// Tests the branch naming rules of two workflows:
//   - .github/workflows/branch-policy.yml (name of any pushed branch)
//   - .github/workflows/pr-policy.yml (head branch of a pull request)
//
// The rules are read from the workflows themselves, never copied here: the
// test runs the real bash steps with their environment variables set, and also
// applies the extracted regexes with JavaScript so that a bash/JS divergence on
// these fixtures is caught. The two workflows hold the same regexes on purpose
// (a workflow cannot import from another); a test checks they do not drift.
//
// Run with: node --test scripts/check-branch-policy.test.mjs
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const branchPolicyPath = process.env.BRANCH_POLICY_WORKFLOW
  ?? `${repoRoot}.github/workflows/branch-policy.yml`;
const prPolicyPath = process.env.PR_POLICY_WORKFLOW
  ?? `${repoRoot}.github/workflows/pr-policy.yml`;

function extractRunBlock(yaml, stepName) {
  const lines = yaml.split("\n");
  const nameIndex = lines.findIndex((line) => line.includes(`- name: ${stepName}`));
  assert.notEqual(nameIndex, -1, `step "${stepName}" not found`);
  const runIndex = lines.findIndex((line, i) => i > nameIndex && /^\s+run: \|\s*$/.test(line));
  assert.notEqual(runIndex, -1, "run block not found");
  const baseIndent = lines[runIndex].match(/^\s*/)[0].length;
  const body = [];
  for (const line of lines.slice(runIndex + 1)) {
    if (line.trim() !== "" && line.match(/^\s*/)[0].length <= baseIndent) break;
    body.push(line.slice(baseIndent + 2));
  }
  return body.join("\n");
}

function loadStep(path, stepName) {
  const script = extractRunBlock(readFileSync(path, "utf8"), stepName);
  const regexSource = (name) => {
    const match = script.match(new RegExp(`^${name}='(.*)'$`, "m"));
    assert.ok(match, `${name} not found in ${path}`);
    return match[1];
  };
  const humanSource = regexSource("allowed_work_branch_regex");
  const dependabotSource = regexSource("dependabot_branch_regex");
  return {
    script,
    humanSource,
    dependabotSource,
    human: new RegExp(humanSource),
    dependabot: new RegExp(dependabotSource),
  };
}

const branchPolicy = loadStep(branchPolicyPath, "Enforce branch naming convention");
const prPolicy = loadStep(prPolicyPath, "Enforce branch flow");

const bypassOwner = readFileSync(`${repoRoot}.github/bypass-owners.txt`, "utf8")
  .split("\n")
  .map((line) => line.trim())
  .find((line) => line !== "" && !line.startsWith("#"));

const bashAvailable = spawnSync("bash", ["--version"], { stdio: "ignore" }).status === 0;
const needsBash = { skip: !bashAvailable && "bash not available" };
const notBypassed = "someone-not-in-bypass-owners";

function runBash(script, env) {
  return spawnSync("bash", ["-c", script], {
    cwd: repoRoot,
    env: { ...process.env, ...env },
    encoding: "utf8",
  }).status;
}

function runBranchPolicy(branch, actor = notBypassed) {
  return runBash(branchPolicy.script, { REF_NAME: branch, REF_TYPE: "branch", ACTOR: actor });
}

function runPrPolicy(base, head, actor = notBypassed) {
  return runBash(prPolicy.script, { BASE_BRANCH: base, HEAD_BRANCH: head, ACTOR: actor });
}

function isWorkOrDependabot(rules, branch) {
  if (rules.human.test(branch)) return true;
  return rules.dependabot.test(branch) && !branch.endsWith(".lock") && !branch.includes(".lock/");
}

// Names of real Dependabot branches of this repository (target-branch develop).
const realDependabot = [
  "dependabot/cargo/develop/sysinfo-0.40.4",
  "dependabot/cargo/develop/jiff-1.0.1",
  "dependabot/cargo/develop/rust-minor-and-patch-3fd7713de0",
  "dependabot/cargo/develop/toml-1.1.8spec-1.1.0",
  "dependabot/github_actions/develop/actions-e7f8b6bf0b",
  "dependabot/npm_and_yarn/desktop/develop/npm-minor-and-patch-37aae3811d",
];

// Shapes Dependabot can produce that are not in the history above.
const syntheticDependabot = [
  "dependabot/github_actions/develop/EmbarkStudios/cargo-deny-action-2.0.18",
  "dependabot/cargo/develop/serde_json-1.0.140",
  "dependabot/cargo/develop/pkg+build-1.0.0",
  "dependabot/x",
];

// Valid work branch names, without the protected names "main" and "develop".
const validWork = [
  "feat/x",
  "fix/a-b",
  "chore/bump-jiff",
  "refactor/a.b_c",
  "hotfix/1-2-3",
  "feat/0",
  ...realDependabot,
  ...syntheticDependabot,
];

const invalid = [
  "",
  "Feat/x",
  "feature/x",
  "ci/x",
  "docs/x",
  "test/x",
  "main-x",
  "feat",
  "feat/",
  "feat/-x",
  "feat/.x",
  "feat/A",
  "feat/a b",
  "feat//x",
  "feat/x/y",
  "feat/x\n",
  "feat/x\ny",
  "feat/\u00e9",
  "dependabot",
  "dependabot/",
  "dependabotx/y",
  "xdependabot/y/z",
  "Dependabot/x",
  "DEPENDABOT/x",
  "dependabot//x",
  "dependabot/a//b",
  "dependabot/a/",
  "dependabot/a/../b",
  "dependabot/..",
  "dependabot/a..b",
  "dependabot/.x",
  "dependabot/a/.b",
  "dependabot/a.",
  "dependabot/a.lock",
  "dependabot/a.lock/b",
  "dependabot/-x",
  "dependabot/ x",
  "dependabot/a b",
  "dependabot/a\tb",
  "dependabot/a\n",
  "dependabot/a\nb",
  "dependabot/a;b",
  "dependabot/$(x)",
  "dependabot/a:b",
  "dependabot/a~b",
  "dependabot/é",
];

const show = (name) => JSON.stringify(name);

// Head branches a pull request to develop must refuse, "main" and "develop" included.
const invalidForDevelop = ["main", "develop", ...invalid];

test("both workflows hold identical regexes", () => {
  assert.equal(prPolicy.humanSource, branchPolicy.humanSource);
  assert.equal(prPolicy.dependabotSource, branchPolicy.dependabotSource);
});

test("javascript regexes accept and refuse the expected names", () => {
  for (const rules of [branchPolicy, prPolicy]) {
    for (const name of validWork) assert.equal(isWorkOrDependabot(rules, name), true, `expected valid: ${show(name)}`);
    for (const name of invalid) assert.equal(isWorkOrDependabot(rules, name), false, `expected invalid: ${show(name)}`);
  }
});

test("branch policy step exits 0 on valid names and 1 on invalid names", needsBash, () => {
  for (const name of ["main", "develop", ...validWork]) assert.equal(runBranchPolicy(name), 0, `expected exit 0: ${show(name)}`);
  for (const name of invalid) assert.equal(runBranchPolicy(name), 1, `expected exit 1: ${show(name)}`);
});

test("pr policy step into develop accepts work and dependabot heads only", needsBash, () => {
  for (const name of validWork) assert.equal(runPrPolicy("develop", name), 0, `expected exit 0: ${show(name)}`);
  for (const name of invalidForDevelop) assert.equal(runPrPolicy("develop", name), 1, `expected exit 1: ${show(name)}`);
});

test("pr policy step into main accepts develop only", needsBash, () => {
  assert.equal(runPrPolicy("main", "develop"), 0);
  for (const name of ["feat/x", "main", "", ...realDependabot, ...syntheticDependabot]) {
    assert.equal(runPrPolicy("main", name), 1, `expected exit 1: ${show(name)}`);
  }
});

test("pr policy step refuses any other base branch", needsBash, () => {
  for (const base of ["", "release/1.0.0", "feat/x", "dependabot/cargo/develop/jiff-1.0.1"]) {
    for (const head of ["develop", "feat/x", realDependabot[0]]) {
      assert.equal(runPrPolicy(base, head), 1, `expected exit 1: base ${show(base)} head ${show(head)}`);
    }
  }
});

test("the bypass owners file still bypasses both gates", needsBash, () => {
  assert.ok(bypassOwner, "no owner listed in .github/bypass-owners.txt");
  assert.equal(runBranchPolicy("Feat/not-a-valid-name", bypassOwner), 0);
  assert.equal(runPrPolicy("main", "feat/x", bypassOwner), 0);
});
