# ADR-0028 — `trap … EXIT` and `**` in arithmetic

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

Scripts clean up after themselves with `trap 'rm -f "$tmp"' EXIT`, and `$(( 2 ** 10 ))` is the usual way to
write a power. Both were missing (`unknown command: trap`, `unexpected '*'`).

## Decision

1. **`trap 'command' EXIT`** (also `0`) remembers a command that runs once when the shell ends: `exit`, the end of
   the input (a script, a pipe, `Ctrl+D`) and the interactive prompt of the terminal and the desktop app.
   `trap - EXIT` removes it, `trap` alone shows it, a new `trap` replaces it. The status the shell leaves with is
   the one it had before the trap ran. The front ends call `ShellEngine::finish()` after their loop.
2. **No signal traps**: `trap … INT`, `TERM`, `ERR`, `DEBUG` and the others are refused with a message instead of being
   accepted and ignored. Catching a signal would need handlers that the shell, which forbids `unsafe`, does not have
   (ADR-0024 uses only `SIGTSTP` through a safe crate).
3. **`a ** b`** is the integer power: right to left (`2 ** 3 ** 2` is 512), tighter than `*`, and below the unary
   operators as in Bash (`-2 ** 2` is 4). A negative exponent and an overflow are errors.

## Consequences

- A trap set in a subshell (ADR-0027) is kept after it ends, like the aliases.
- A shell that is killed by a signal does not run the trap.
