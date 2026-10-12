# Distribution builds

`.github/workflows/release.yml` builds optimized Windows, Linux and macOS CLI
archives. It runs manually or for tags matching `v*.*.*`. Manual runs retain
artifacts for 14 days; tag runs also create a GitHub Release, checksums and notes
from `CHANGELOG.md`.

Each platform runs:

```bash
cargo build --release --locked
```

The binary is smoke-tested with `help`. Archives contain the binary at their
root plus `LICENSE`, `THIRD_PARTY_NOTICES.md`, `README.md`, `CHANGELOG.md` and
`config/`:

```text
terminal_rust-linux-<architecture>.tar.gz
terminal_rust-macos-<architecture>.tar.gz
terminal_rust-windows-<architecture>.zip
```

Use [GitHub Releases](https://github.com/Blazorde/terminal_rust/releases/latest)
for stable downloads and verify `SHA256SUMS.txt`. A local build validates only
its own host; cross-platform proof requires executed jobs and manual desktop
evidence where applicable.

## Versioning and publication

Terminal Rust follows Semantic Versioning. Keep unreleased work under
`## [Unreleased]`. For a release, move those notes to
`## [x.y.z] - YYYY-MM-DD` and create a new empty Unreleased section. The
versions in `Cargo.toml`, `Cargo.lock`, Tauri metadata, changelog and Git tag
must agree.

1. Update all version metadata and changelog notes.
2. Run the complete local and platform validation.
3. Merge a focused release preparation PR into `develop`.
4. Open the `develop` to `main` PR only after the owner's explicit approval.
5. After approval and merge, create lowercase tag `vx.y.z` on the `main` commit.
6. Verify all build and publish jobs, archives, checksums and release notes.

The publication job rejects a tag without a matching changelog section. An
Actions job that fails before executing steps is not release evidence.
