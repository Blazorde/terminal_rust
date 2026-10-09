# Changelog

All notable changes to Terminal Rust are documented in this file. Versions use
[Semantic Versioning](https://semver.org/).

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

- First public release with the interactive shell and core filesystem commands.
- Windows x86_64 binary and checksum attached to the GitHub Release.
