# v1.0.0 release snapshot

Status: preparation only. Do not open the final `develop` to `main` pull
request without the owner's explicit approval.

The stable branch is a product snapshot, while `develop` also carries working
notes and contribution automation. The v1.0.0 promotion therefore needs one
temporary cleanup commit on `develop`. After the stable squash merge, restore
the development-only files on `develop` in a separate pull request. Do not
rewrite either branch.

## Stable snapshot inventory

Keep these groups in the v1.0.0 snapshot:

- Rust CLI, shared engine, desktop application, plugin SDK and example plugin;
- `Cargo.toml`, `Cargo.lock`, desktop npm/Tauri manifests and lockfile;
- default configuration, themes and the Fastfetch example;
- user documentation: `README.md`, `CHANGELOG.md`, `docs/install.md`,
  `docs/configuration.md`, `docs/customization.md`, `docs/custom-commands.md`,
  `docs/completion.md`, `docs/dashboard.md`, `docs/themes.md`,
  `docs/troubleshooting.md`, `docs/security.md` and `docs/plugins/`;
- public technical documentation in `docs/architecture/`, `docs/decisions/`
  and the development build, testing and contribution guides;
- `LICENSE`, `SECURITY.md`, `THIRD_PARTY_NOTICES.md` and `deny.toml`;
- installer scripts and GitHub validation, security, release and PR-policy
  workflows.

Remove these development-only paths from the temporary release snapshot:

| Path | Reason | Restore on `develop` |
| --- | --- | --- |
| `AGENTS.md` | automated contribution instructions | yes |
| `PRODUCT.md` | internal product brief and local reference path | yes |
| `DESIGN.md` | internal visual direction | yes |
| `docs/agents/` | ticket automation process | yes |
| `docs/research/` | future-work research, not shipped behavior | yes |
| `docs/development/gui-quality-audit.md` | internal evidence log | yes |
| `docs/development/mvp-quality-checklist.md` | internal stabilization checklist | yes |
| `docs/development/v1-release-snapshot.md` | this internal release checklist | yes |
| `docs/architecture/tui-layout-prototype.md` | superseded prototype architecture | yes |
| `examples/tui_layout.rs` | optional superseded TUI prototype | yes |
| `wiki/` | source mirror already published as the GitHub wiki | yes |
| `.github/workflows/wiki.yml` | develop-side wiki publication automation | yes |
| `scripts/sync-wiki.mjs` | develop-side wiki publication tool | yes |
| redundant tracked `.gitkeep` files in non-empty directories | no runtime value | optional |

Removing the TUI prototype also requires removing the `tui-prototype` feature,
the `ratatui` dependency and the `[[example]]` entry from `Cargo.toml`, then
refreshing `Cargo.lock`. This keeps the stable dependency surface aligned with
the shipped product.

## Version and release gates

The release candidate is not ready while any item below is unchecked:

- [ ] Set the root crate, plugin SDK, example plugin, desktop Rust crate,
  desktop npm package and Tauri bundle to `1.0.0`.
- [ ] Regenerate both lockfiles without unrelated dependency upgrades.
- [ ] Move the complete Unreleased changelog into a dated `1.0.0` section and
  create a new empty Unreleased section.
- [ ] Run the required Rust format, check, Clippy and test commands.
- [ ] Run the frontend UI audit, production build and notice inventory check.
- [ ] Build and launch the native desktop bundle on Windows, macOS and Linux;
  confirm that a real PTY accepts a command on each platform.
- [ ] Record startup, memory, latency, responsive and keyboard evidence for the
  remaining macOS and visible Linux audits.
- [ ] Verify that every archive contains the expected CLI, desktop installer,
  license, third-party notices, configuration and public documentation.
- [ ] Review `git diff origin/main...develop` for secrets, local paths,
  generated output and internal-only files.
- [ ] Obtain explicit owner approval before opening the final pull request.

## Promotion sequence

1. Complete the platform evidence on normal focused branches and merge them
   into `develop`.
2. Create a final `chore/v1-release-snapshot` branch from current `develop`.
3. Apply the version, changelog and inventory cleanup in that branch; run every
   gate and merge it back into `develop`.
4. Present the final diff and evidence to the owner. Stop until approval is
   explicit.
5. Open exactly one `develop` to `main` pull request and squash-merge it so the
   first-parent history of `main` receives one v1.0.0 release commit.
6. Tag that exact `main` commit as `v1.0.0`; verify archives, checksums and
   release notes before announcing availability.
7. Restore the development-only paths on a new branch from `develop`, merge
   that branch only into `develop`, and republish the wiki if required.

The existing `main` ancestry is preserved. Rewriting published history merely
to flatten older releases would invalidate existing clones, tags and release
references; the one-release-commit policy applies to the first-parent history
from v1.0.0 onward.
