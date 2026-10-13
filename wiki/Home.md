# Terminal Rust wiki

Terminal Rust is a compact, cross-platform educational shell with a Rust core
and an optional Tauri desktop workspace. It runs real commands and real native
PTY sessions; the browser preview is not a command runner.

## Start here

- [[Installation]] — build or install the CLI and launch the native window.
- [[Usage]] — commands, completion, dashboard and desktop shortcuts.
- [[Configuration]] — prompt segments, themes, icons and custom commands.
- [[Architecture]] — boundaries between the engine, renderer and desktop UI.
- [[Plugins]] — current declarative plugin model and its limits.
- [[Development]] — contribution workflow and required quality gates.
- [[Security]] — private reporting and dependency policy.
- [[Releases]] — support status, artifacts and publication evidence.

The versioned source for this wiki is the repository's [`wiki/`](https://github.com/Blazorde/terminal_rust/tree/develop/wiki)
directory. Repository files are the source of truth; direct edits in GitHub's
wiki are overwritten by the documented synchronization process.

## Current status

The latest stable release is `v0.3.0`. Work toward `v1.0.0` remains on
`develop`. Windows has local CLI, PTY and desktop evidence. macOS and Linux are
designed and tested through portable code paths, but they are not declared
release-verified until their CI jobs actually execute and native evidence is
recorded.
