import { execFileSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const source = join(root, "wiki");
const requiredPages = [
  "Home.md",
  "_Sidebar.md",
  "Installation.md",
  "Usage.md",
  "Configuration.md",
  "Architecture.md",
  "Plugins.md",
  "Development.md",
  "Security.md",
  "Releases.md",
];

function git(args, options = {}) {
  return execFileSync("git", args, {
    cwd: options.cwd ?? root,
    encoding: "utf8",
    stdio: options.capture ? "pipe" : "inherit",
    env: options.env ?? process.env,
  });
}

function pages() {
  return readdirSync(source).filter((file) => file.endsWith(".md")).sort();
}

function validate() {
  const available = new Set(pages().map((file) => basename(file, ".md")));
  const errors = [];

  for (const required of requiredPages) {
    if (!existsSync(join(source, required))) errors.push(`Missing required page: ${required}`);
  }

  for (const file of pages()) {
    const content = readFileSync(join(source, file), "utf8");
    for (const match of content.matchAll(/\[\[([^\]]+)\]\]/g)) {
      const target = match[1].split("|").at(-1).trim().replaceAll(" ", "-");
      if (!available.has(target)) errors.push(`${file}: missing wiki page ${target}`);
    }
    if (/<script\b|@import\s+url|font-family\s*:/i.test(content)) {
      errors.push(`${file}: remote scripts or font styling are not allowed`);
    }
  }

  if (errors.length) throw new Error(errors.join("\n"));
  console.log(`Validated ${pages().length} versioned wiki pages.`);
}

function repositorySlug() {
  if (process.env.GITHUB_REPOSITORY) return process.env.GITHUB_REPOSITORY;
  const remote = git(["remote", "get-url", "origin"], { capture: true }).trim();
  const match = remote.match(/github\.com[/:]([^/]+)\/([^/]+?)(?:\.git)?$/);
  if (!match) throw new Error("Cannot derive the GitHub repository from origin.");
  return `${match[1]}/${match[2]}`;
}

function authenticationEnvironment() {
  const token = process.env.GITHUB_TOKEN;
  if (!token) return process.env;
  return {
    ...process.env,
    GIT_CONFIG_COUNT: "1",
    GIT_CONFIG_KEY_0: "http.extraHeader",
    GIT_CONFIG_VALUE_0: `AUTHORIZATION: basic ${Buffer.from(`x-access-token:${token}`).toString("base64")}`,
  };
}

function publish() {
  const slug = repositorySlug();
  const token = process.env.GITHUB_TOKEN;
  const remote = token
    ? `https://github.com/${slug}.wiki.git`
    : `git@github.com:${slug}.wiki.git`;
  const temporaryRoot = mkdtempSync(join(tmpdir(), "terminal-rust-wiki-"));
  const checkout = join(temporaryRoot, "checkout");
  const env = authenticationEnvironment();

  try {
    try {
      git(["clone", remote, checkout], { env });
    } catch {
      rmSync(checkout, { recursive: true, force: true });
      mkdirSync(checkout);
      git(["init"], { cwd: checkout, env });
      git(["remote", "add", "origin", remote], { cwd: checkout, env });
    }

    for (const file of readdirSync(checkout)) {
      if (file.endsWith(".md")) rmSync(join(checkout, file));
    }
    for (const file of pages()) cpSync(join(source, file), join(checkout, file));

    git(["add", "-A"], { cwd: checkout, env });
    const changed = git(["status", "--porcelain"], {
      cwd: checkout,
      capture: true,
      env,
    }).trim();
    if (!changed) {
      console.log("GitHub wiki is already synchronized.");
      return;
    }

    git(["config", "user.name", "terminal-rust-maintainer"], { cwd: checkout, env });
    git(["config", "user.email", "maintainers@users.noreply.github.com"], {
      cwd: checkout,
      env,
    });
    git(["commit", "-m", "docs: sync wiki from repository"], { cwd: checkout, env });
    git(["push", "origin", "HEAD:master"], { cwd: checkout, env });
    console.log(`Published ${pages().length} pages to https://github.com/${slug}/wiki`);
  } finally {
    rmSync(temporaryRoot, { recursive: true, force: true });
  }
}

validate();
if (process.argv.includes("--publish")) publish();
else if (!process.argv.includes("--check")) {
  throw new Error("Use --check or --publish.");
}
