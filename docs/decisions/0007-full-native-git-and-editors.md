# ADR-0007 — Full native Git with bundled terminal editors

- Status: Accepted
- Date: 2026-10-19

## Context

Developers need regular Git workflows such as clone, fetch, pull, commit, push,
rebasing and remote management. Git must also be able to start a terminal editor
for commits and other interactive operations. The desktop release should work
without separately installing Git, Nano or Neovim.

## Decision

The `git` command forwards every argument to the native Git executable without
an allowlist, using separate process arguments and inherited terminal streams.
Command failures remain visible and return a failed shell status. The desktop
release bundles a pinned Git runtime, Nano and Neovim; the first desktop session
extracts them to the user's application data directory and puts their
executables on the terminal's process path.

Git keeps the user's existing configuration, credentials and editor choices.
No global Git settings are rewritten. The standalone CLI and development builds
continue to use installed tools from `PATH` unless an explicit executable
environment variable is set.

## Consequences

Every native Git operation is available, including operations that change local
repositories or contact remotes. Users retain the same control they have when
running Git directly in a terminal. Release packaging must pin and verify the
downloaded Windows Nano executable, include matching Nano source, and retain
license notices for the bundled runtime packages. The bundled runtime increases
desktop installer size and first-session setup time.
