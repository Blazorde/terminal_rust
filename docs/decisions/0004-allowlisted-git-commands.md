# ADR-0004 — Allowlist built-in Git operations

- Status: Superseded by [ADR-0007](0007-full-native-git-and-editors.md)
- Date: 2026-10-09

## Context

The shell should help developers inspect status, branches and history without
exposing destructive or remote Git operations through an unrestricted passthrough.

## Decision

The built-in `git` command provides local `git help` output and delegates only
`status`, `branch` and `log` to the installed executable. Help remains available
without Git installed. Delegated commands preserve stdout, stderr and exit status,
force `--no-pager`, and reject mutating branch arguments and `log --output` before
launch. They run `git -C <current directory>` consistently on all platforms.

## Consequences

Common inspection works without reimplementing Git, while `push`, `reset`,
`clean` and other operations are unavailable through this command. Missing Git
produces a clear recoverable error. Every new subcommand requires explicit
allowlisting, tests and a security justification.
