# Terminal Rust wiki

Terminal Rust is an open-source, cross-platform educational shell with a Rust
core and a Tauri desktop workspace. It runs real commands through real native
PTY sessions; the browser preview is not a command runner.

## Start here

- [[Installation]] — install the alpha, build from source or launch the native window.
- [[Usage]] — commands, completion, dashboard and desktop shortcuts.
- [[Configuration]] — prompt segments, themes, icons and custom commands.
- [[Neovim]] — download and install Blazorde's Neovim configuration.
- [[Architecture]] — boundaries between the engine, renderer and desktop UI.
- [[Plugins]] — current declarative plugin model and its limits.
- [[Development]] — open-source contribution workflow and quality gates.
- [[Security]] — private reporting and dependency policy.
- [[Releases]] — support status, artifacts and release history.

The versioned source for this wiki is the repository's
[`wiki/`](https://github.com/Blazorde/terminal_rust/tree/main/wiki) directory.
Repository files are the source of truth.

## Current status

`v1.0.0-alpha.1` is the current prerelease; `v0.3.0` remains the latest stable
release. Windows and Linux alpha packages have native runtime and PTY evidence.
The macOS architecture and build configuration are present, but the native
bundle still requires validation on Apple hardware.
