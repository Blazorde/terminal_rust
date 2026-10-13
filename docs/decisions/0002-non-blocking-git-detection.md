# ADR-0002 — External, non-blocking Git detection

- Status: Accepted
- Date: 2026-10-09
- Decision makers: Terminal Rust team

## Context

Prompt Git detection must work on Windows, macOS and Linux when Git may be
missing and `.git` may be a directory or file for worktrees and submodules.

## Decision

Run Git directly without a shell:

```text
git -C <path> rev-parse --is-inside-work-tree
```

Return a boolean. Missing executables, invalid paths, failing exits and
unexpected output mean `false`; expected diagnostic stderr is hidden because
repository context is optional decoration.

## Trade-offs

Walking parents for `.git` is faster but incorrect for some Git layouts. A Git
library offers more API than this first capability needs. Calling installed Git
matches its real configuration and stays portable, at the cost of a process.
Richer prompt data therefore uses one grouped command plus a short cache.

## Validation

Tests cover positive and invalid output, missing Git, a non-repository directory
and a temporary initialized repository when Git is available.
