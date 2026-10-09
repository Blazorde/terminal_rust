# Changelog

All notable changes to Terminal Rust are documented in this file. Versions use
[Semantic Versioning](https://semver.org/).

## [Unreleased]

The next planned minor release is **0.3.0**. It will focus on the configurable,
segmented prompt engine while preserving graceful fallbacks on Windows, Linux,
and macOS.

### Added

- Verified user-level installers for Windows x86_64, Linux x86_64, and macOS
  Apple Silicon, with authenticated private-release support.

### Improved

- Added copyable onboarding, a supported-platform matrix, and the complete
  contributor validation workflow to the README.
- Added installer syntax validation to the cross-platform CI matrix.

## [0.2.0] - 2026-10-09

### Added

- Persistent command history and the `clear` built-in command.
- Non-blocking Git repository detection and read-only `git status`, `git branch`,
  and `git log` shortcuts.
- A cross-platform `dashboard` command for CPU, memory, storage, network, and Git
  context.
- A portable Fastfetch preset and setup documentation.
- Cross-platform CLI integration tests and release builds for Windows, Linux,
  and macOS.

### Improved

- Added project architecture, decision records, contribution guidance, and
  release documentation.
- Added automated GitHub Release publication with checksums for version tags.

### Known limitations

- Terminal Rust remains a pedagogical mini-shell and does not yet replace Bash,
  PowerShell, or Zsh.
- The prompt segment system inspired by Oh My Posh is planned but not included
  in this release.

## [0.1.0] - 2026-06-20

### Added

- First public release with the interactive shell, command history for the
  current session, completion, and quoted-argument parsing.
- Core `pwd`, `cd`, `ls`, `mkdir`, `touch`, `cat`, and `rm` commands.
- Cross-platform CI for formatting, compilation, Clippy, and tests.
- Windows x86_64 binary and checksum attached to the GitHub Release.
