# Workflow and branch protection

## Branch flow

- `main` accepts pull requests only from `develop`.
- `develop` accepts `feat/*`, `fix/*`, `chore/*`, `refractor/*` and `hotfix/*`.

## Workflows

`.github/workflows/ci.yml` runs formatting, all-target/all-feature checks,
Clippy with warnings denied, workspace tests and platform installer parsing on
Ubuntu, Windows and macOS. Plugin SDK, plugin template and desktop are workspace
members and are validated explicitly. `CI / Required CI gate` succeeds only
after the matrix passes.

`.github/workflows/release.yml` builds, smoke-tests and packages optimized
binaries. Manual runs retain artifacts for 14 days. Version tags create a
GitHub Release with changelog notes, all archives and `SHA256SUMS.txt`.

`.github/workflows/pr-policy.yml` enforces PR routes, while
`.github/workflows/branch-policy.yml` reports invalid branch names. Workflows
cannot prevent branch creation before they run; use GitHub Rulesets for that.

## Required GitHub settings

Create rulesets for `main` and `develop` that require pull requests, block force
pushes and deletions, and require:

```text
PR policy / Required PR policy gate
CI / Required CI gate
```

Allow bypass only for maintainers listed in `.github/bypass-owners.txt`, and
configure the same actors manually in Rulesets. A branch-name ruleset should
allow only `main`, `develop` and the documented prefixes.

## Contribution path

```bash
git switch develop
git pull --ff-only origin develop
git switch -c feat/my-change
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Open the PR into `develop`. Stable promotion uses a `develop` to `main` PR.
Required checks should be bypassed only under the documented maintainer policy,
with local evidence recorded when hosted jobs cannot execute.
