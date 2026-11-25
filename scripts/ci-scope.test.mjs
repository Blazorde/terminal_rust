// Tests scripts/ci-scope.mjs on real Git repositories, and the final gate of
// .github/workflows/ci.yml by running its bash step.
//
// Run with: node --test scripts/ci-scope.test.mjs
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { devNull, tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const script = fileURLToPath(new URL("ci-scope.mjs", import.meta.url));
const workflow = fileURLToPath(new URL("../.github/workflows/ci.yml", import.meta.url));
const isolated = { ...process.env, GIT_CONFIG_GLOBAL: devNull, GIT_CONFIG_NOSYSTEM: "1" };

function git(directory, ...args) {
  execFileSync("git", ["-c", "user.name=test", "-c", "user.email=test@example.invalid", ...args], {
    cwd: directory,
    env: isolated,
    stdio: "pipe",
  });
}

function write(directory, path, content) {
  mkdirSync(dirname(join(directory, path)), { recursive: true });
  writeFileSync(join(directory, path), content);
}

// Commits a base tree, applies `change`, commits again and classifies the diff.
function classify(change, revisions = ["HEAD^1", "HEAD"]) {
  const directory = mkdtempSync(join(tmpdir(), "ci-scope-"));
  try {
    git(directory, "init", "--quiet");
    write(directory, "src/main.rs", "fn main() {}\n");
    write(directory, "README.md", "base\n");
    git(directory, "add", "--all");
    git(directory, "commit", "--quiet", "--message", "base");
    change(directory);
    git(directory, "add", "--all");
    git(directory, "commit", "--quiet", "--allow-empty", "--message", "change");
    const result = spawnSync(process.execPath, [script, ...revisions], {
      cwd: directory,
      env: isolated,
      encoding: "utf8",
    });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout.trim();
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

const touch = (...paths) => (directory) => {
  for (const path of paths) write(directory, path, "changed\n");
};

test("documentation alone skips the compilation", () => {
  const change = touch(
    "README.md",
    "CHANGELOG.md",
    "SECURITY.md",
    "docs/decisions/0099-example.md",
    "docs/assets/cover.png",
    "docs/with space/é.md",
    "wiki/Usage.md",
    ".github/ISSUE_TEMPLATE/bug.yml",
  );
  assert.equal(classify(change), "code=false");
});

test("any other file runs the checks, even next to documentation", () => {
  for (const path of [
    "src/main.rs",
    "Cargo.lock",
    ".github/workflows/ci.yml",
    "scripts/install.sh",
    "docs/tool.sh",
    "config/themes/default.toml",
    "THIRD_PARTY_NOTICES.md",
    "LICENSE",
    "src/README.md",
  ]) {
    assert.equal(classify(touch("README.md", path)), "code=true", path);
  }
});

test("removing or moving a source file into the documentation runs the checks", () => {
  assert.equal(classify((directory) => rmSync(join(directory, "src/main.rs"))), "code=true");
  const move = (directory) => {
    mkdirSync(join(directory, "docs"));
    renameSync(join(directory, "src/main.rs"), join(directory, "docs/main.md"));
  };
  assert.equal(classify(move), "code=true");
});

test("an empty or unreadable diff runs the checks", () => {
  assert.equal(classify(() => {}), "code=true");
  assert.equal(classify(touch("README.md"), ["0".repeat(40), "HEAD"]), "code=true");
  assert.equal(classify(touch("README.md"), []), "code=true");
});

function gate(changes, code, rust) {
  const lines = readFileSync(workflow, "utf8").split(/\r?\n/);
  const name = lines.findIndex((line) => line.includes("- name: Require all Rust checks"));
  assert.notEqual(name, -1, "gate step not found");
  const run = lines.findIndex((line, index) => index > name && line.trim() === "run: |");
  assert.notEqual(run, -1, "gate run block not found");
  const body = [];
  for (const line of lines.slice(run + 1)) {
    if (line !== "" && !line.startsWith("          ")) break;
    body.push(line.slice(10));
  }
  return spawnSync("bash", ["-c", body.join("\n")], {
    env: { ...process.env, CHANGES_RESULT: changes, CODE_CHANGED: code, RUST_RESULT: rust },
  }).status;
}

test("the required gate passes only on green checks or a documentation change", () => {
  assert.equal(gate("success", "true", "success"), 0);
  assert.equal(gate("success", "false", "skipped"), 0);
  for (const [changes, code, rust] of [
    ["success", "true", "failure"],
    ["success", "true", "cancelled"],
    ["success", "true", "skipped"],
    ["success", "", "skipped"],
    ["success", "false", "failure"],
    ["success", "false", "cancelled"],
    ["failure", "", "skipped"],
    ["failure", "false", "skipped"],
    ["cancelled", "false", "skipped"],
  ]) {
    assert.equal(gate(changes, code, rust), 1, `${changes} ${code} ${rust}`);
  }
});
