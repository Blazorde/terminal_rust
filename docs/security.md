# Security and dependency policy

The public reporting process lives in [`SECURITY.md`](../SECURITY.md). Suspected
vulnerabilities must use GitHub private vulnerability reporting, never a public
issue containing exploit details.

## Reproducible checks

Install the locked npm dependency graph before running its audit:

```bash
npm ci --prefix desktop
npm audit --prefix desktop --audit-level=high
npm run notices:check --prefix desktop
```

Install `cargo-deny` and run the checked-in Rust policy:

```bash
cargo install --locked cargo-deny
cargo deny check
```

`deny.toml` rejects known Rust vulnerabilities, yanked packages, wildcard
versions, unknown registries, unapproved Git dependencies and licenses outside
the allowlist. Duplicate crate versions remain warnings because Tauri's
cross-platform graph legitimately contains independently versioned platform
adapters; each warning should still be reviewed when dependencies change.

The allowlist contains only license identifiers already needed by the locked
graph: MIT, Apache-2.0, Apache-2.0 with LLVM exception, BSD-2-Clause,
BSD-3-Clause, BSL-1.0, ISC, MPL-2.0, Unicode-3.0 and Zlib. A new license fails
the check until it is deliberately reviewed.

## Automation and limitations

Dependabot proposes weekly updates for Cargo, npm and GitHub Actions against
`develop`. The security workflow repeats Rust policy checks, `npm audit` and
the third-party notice inventory on pull requests, protected branches and a
weekly schedule.

`THIRD_PARTY_NOTICES.md` is generated from `Cargo.lock` plus the npm packages
installed from `desktop/package-lock.json`. Release archives include the
inventory with the project license. Metadata and advisory scanners reduce
risk, but cannot guarantee that upstream metadata is complete or that no
vulnerability exists.

Never commit credentials, local environment files, private paths or generated
signing material. GitHub secrets must be scoped to the workflow that needs them;
the current workflows require no repository secret for ordinary validation.
