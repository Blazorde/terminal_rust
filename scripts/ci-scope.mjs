// Decides whether .github/workflows/ci.yml can skip its compilation jobs.
//
//   node scripts/ci-scope.mjs <base-revision> <head-revision>
//
// Prints "code=false" only when every changed path is documentation that no
// build or test reads. Anything else, including a diff that cannot be computed,
// prints "code=true" so that the checks run.
import { execFileSync } from "node:child_process";

// THIRD_PARTY_NOTICES.md and LICENSE are absent on purpose: they are shipped.
const documentation = [
  /^(docs|wiki)\/.*\.(md|png|jpg|svg)$/,
  /^\.github\/ISSUE_TEMPLATE\/.*\.(md|yml)$/,
  /^(README|CHANGELOG|SECURITY)\.md$/,
];

function changedPaths(base, head) {
  // --no-renames lists both ends of a move: moving a source file into docs/ counts.
  const output = execFileSync(
    "git",
    ["diff", "--name-only", "--no-renames", "-z", base, head, "--"],
    { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  return output.split("\0").filter(Boolean);
}

function needsBuild(paths) {
  return paths.length === 0
    || paths.some((path) => !documentation.some((rule) => rule.test(path)));
}

const [base, head] = process.argv.slice(2);
let code = true;
try {
  if (!base || !head) throw new Error("expected a base and a head revision");
  code = needsBuild(changedPaths(base, head));
} catch (error) {
  console.error(`::warning::Cannot list the changed files, running every check: ${error.message}`);
}
console.log(`code=${code}`);
