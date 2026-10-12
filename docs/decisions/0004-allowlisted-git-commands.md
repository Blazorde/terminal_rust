# ADR-0004 — Allowlist built-in Git operations

- Status: Accepted
- Date: 2026-10-09

## Context

The shell should help developers inspect status, branches and history without
exposing destructive or remote Git operations through an unrestricted passthrough.

## Decision

The built-in `git` command delegates only `status`, `branch` and `log` to the
installed executable. It preserves stdout, stderr and exit status, forces
`--no-pager`, and rejects mutating branch arguments and `log --output` before
launch. It runs `git -C <current directory>` consistently on all platforms.

## Consequences

Common inspection works without reimplementing Git, while `push`, `reset`,
`clean` and other operations are unavailable through this command. Missing Git
produces a clear recoverable error. Every new subcommand requires explicit
allowlisting, tests and a security justification.
