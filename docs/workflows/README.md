# Workflow and Branch Protection

This project uses GitHub Actions plus GitHub branch protection rules.

## Branch flow

- `main` only accepts pull requests from `develop`.
- `develop` only accepts pull requests from:
  - `feat/name`
  - `fix/name`
  - `chore/name`
  - `refractor/name`
  - `hotfix/name`

Examples:

```text
feat/autocomplete
fix/cd-home
chore/update-ci
refractor/ls-modules
hotfix/windows-path
```

Invalid examples:

```text
feature/autocomplete
bugfix/cd
test
feat/
Feat/autocomplete
```

## Workflows

### CI

File: `.github/workflows/ci.yml`

Runs on pull requests and pushes. It checks:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

The CI runs on Ubuntu and Windows.

### PR policy

File: `.github/workflows/pr-policy.yml`

This blocks invalid pull request routes:

- PR to `main` must come from `develop`.
- PR to `develop` must come from `feat/*`, `fix/*`, `chore/*`, `refractor/*`, or `hotfix/*`.

### Branch policy

File: `.github/workflows/branch-policy.yml`

This fails if a pushed or created branch does not match the naming rules.

Important: GitHub Actions cannot prevent the branch from being created before the workflow starts. To block creation completely, configure a GitHub Ruleset as described below.

## Required GitHub settings

Configure these in GitHub:

```text
Repository -> Settings -> Rules -> Rulesets
```

The workflow files can fail invalid changes, but they cannot mark themselves as required and they cannot decide who is allowed to bypass. Required checks and bypass permissions must be configured in GitHub Rulesets.

Repository owner allowed to bypass:

```text
Blazorde
```

Bypass owners are listed in:

```text
.github/bypass-owners.txt
```

Put one GitHub username per line. Example:

```text
Blazorde
AnotherMaintainer
```

The policy workflows read this file for policy-gate bypasses. GitHub Rulesets do not automatically read repository files, so you must also add the same users manually as bypass actors in GitHub Rulesets.

### Ruleset for `main`

Recommended settings:

- Target branch: `main`
- Require pull request before merging
- Require status checks to pass
- Required checks:
  - `PR policy / Required PR policy gate`
  - `CI / Required CI gate`
- Block force pushes
- Block deletions
- Require linear history if desired
- Restrict bypass permissions to the users listed in `.github/bypass-owners.txt`
- Do not allow direct pushes

### Ruleset for `develop`

Recommended settings:

- Target branch: `develop`
- Require pull request before merging
- Require status checks to pass
- Required checks:
  - `PR policy / Required PR policy gate`
  - `CI / Required CI gate`
- Block force pushes
- Block deletions
- Restrict bypass permissions to the users listed in `.github/bypass-owners.txt`
- Do not allow direct pushes

### Ruleset for branch names

Create a ruleset for branch creation and updates.

Allow only:

```text
main
develop
feat/*
fix/*
chore/*
refractor/*
hotfix/*
```

Block or reject every other branch name.

## How to update the project

1. Create a branch from `develop`:

```bash
git checkout develop
git pull
git checkout -b feat/my-change
```

2. Make changes and run checks locally:

```bash
cargo fmt
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

3. Open a pull request into `develop`.

4. After review and CI success, merge into `develop`.

5. To release or promote stable work, open a pull request from `develop` into `main`.

## Bypass policy

Only users listed in `.github/bypass-owners.txt` should be allowed to bypass required checks.

This cannot be safely enforced by a workflow file alone. Configure it in GitHub Rulesets by limiting bypass actors to the same users listed in `.github/bypass-owners.txt`.

If GitHub asks for required status check names, use:

```text
CI / Required CI gate
PR policy / Required PR policy gate
```

The CI gate depends on every Rust matrix job. If Ubuntu or Windows fails, the gate fails too.
