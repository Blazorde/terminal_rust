# ADR-0025 — `!`, `command`, `exec`, `$PPID` and `$RANDOM`

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

Trying ordinary shell usage in the shell showed small gaps that stopped everyday lines:
`if ! test -f x`, `command -v git`, `exec cmd`, `$PPID` and `$RANDOM` all failed with
`unknown command` or an empty value.

## Decision

1. **`! command`**: a segment that starts with the word `!` runs the rest and turns its
   status around: `$?` is `0` after a failure and `1` after a success. The message of a failure
   is still shown. It works as a condition (`if ! cmd; then`, `while ! cmd`) and with `&&` and
   `||`. A lone `!` is a syntax error. Only a leading `!` followed by a blank counts, so `"a ! b"`
   and the history references `!!` and `!$` are unchanged. `! a | b` negates the whole pipeline.
2. **`command`**: `command -v name...` prints what each name runs (the name for a keyword, a
   function, a built-in or an alias; the path for a program) with status `1` if one is missing;
   `command -V` says it like `type`; `command name args...` runs the command without the
   function of that name. It does not bypass an alias.
3. **`exec command args...`** runs the command and leaves the shell with its status. The
   process is not replaced (this shell has no `execve`, which would need `unsafe`). Without
   a command it does nothing.
4. **`$PPID`** is the parent process id of the shell (Unix). **`$RANDOM`** is a number from 0 to
   32767, a new one at each use (xorshift seeded from the clock and the process id). A variable
   of the same name set by the user wins.

## Consequences

- `exec` does not free the shell process before the command runs, and `$-`, `$LINENO` and
  `trap` are still missing.
- `RANDOM` is not a source of secrets.
