import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = resolve(desktopRoot, "..");
const outputPath = join(repositoryRoot, "THIRD_PARTY_NOTICES.md");

function normalizeRepository(repository) {
  if (typeof repository === "string") return repository;
  return repository?.url ?? "";
}

function rustPackages() {
  const metadata = JSON.parse(
    execFileSync("cargo", ["metadata", "--format-version", "1", "--locked"], {
      cwd: repositoryRoot,
      encoding: "utf8",
      maxBuffer: 32 * 1024 * 1024,
    }),
  );

  const workspace = new Set(metadata.workspace_members);
  return metadata.packages
    .filter((entry) => !workspace.has(entry.id))
    .map((entry) => ({
      ecosystem: "Rust",
      name: entry.name,
      version: entry.version,
      license: entry.license ?? "UNKNOWN",
      source: entry.repository ?? entry.homepage ?? "crates.io",
    }));
}

function packageName(lockPath) {
  const marker = "node_modules/";
  const index = lockPath.lastIndexOf(marker);
  return index === -1 ? "" : lockPath.slice(index + marker.length);
}

function npmPackages() {
  const lockfile = JSON.parse(
    readFileSync(join(desktopRoot, "package-lock.json"), "utf8"),
  );
  const packages = new Map();

  for (const [path, manifest] of Object.entries(lockfile.packages ?? {})) {
    const name = packageName(path);
    if (!name || !manifest.version) continue;
    const item = {
      ecosystem: "npm",
      name,
      version: manifest.version,
      license: manifest.license ?? "UNKNOWN",
      source:
        normalizeRepository(manifest.repository) ||
        manifest.homepage ||
        `https://www.npmjs.com/package/${name}`,
    };
    packages.set(`${item.name}@${item.version}`, item);
  }

  return [...packages.values()];
}

function render(packages) {
  const lines = [
    "# Third-party notices",
    "",
    "Terminal Rust is licensed under MIT. The following locked dependencies retain",
    "their own copyright and license terms. This inventory is generated from Cargo",
    "metadata and the npm lockfile; it is not legal advice and does not",
    "replace the license files distributed by upstream projects.",
    "",
    "Regenerate with `npm run notices --prefix desktop` after `npm ci --prefix desktop`.",
    "CI rejects a stale inventory with `npm run notices:check --prefix desktop`.",
    "",
    "| Ecosystem | Package | Version | Declared license | Source |",
    "| --- | --- | --- | --- | --- |",
  ];

  for (const item of packages.sort((left, right) =>
    `${left.ecosystem}:${left.name}:${left.version}`.localeCompare(
      `${right.ecosystem}:${right.name}:${right.version}`,
    ),
  )) {
    const cell = (value) => String(value).replaceAll("|", "\\|").replaceAll("\n", " ");
    lines.push(
      `| ${cell(item.ecosystem)} | ${cell(item.name)} | ${cell(item.version)} | ${cell(item.license)} | ${cell(item.source)} |`,
    );
  }

  return `${lines.join("\n")}\n`;
}

const generated = render([...rustPackages(), ...npmPackages()]);
if (process.argv.includes("--check")) {
  const current = readFileSync(outputPath, "utf8");
  if (current !== generated) {
    console.error("THIRD_PARTY_NOTICES.md is stale; run npm run notices --prefix desktop.");
    process.exit(1);
  }
} else {
  writeFileSync(outputPath, generated, "utf8");
  console.log(`Updated ${outputPath}`);
}
