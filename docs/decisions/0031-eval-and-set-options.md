# ADR-0031 — `eval` and the options of `set`

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

Scripts start with `set -eu`, build commands in variables and run them with `eval`, and reset their arguments with
`set --`. None of it existed (`unknown command: set`, `unknown command: eval`).

## Decision

1. **`eval args…`** joins the arguments with spaces and runs the result as a command line in the current shell, so
   what the expansion produced (`;`, quotes, `&&`) is read again. It has no arguments: nothing runs, status 0.
2. **`set`** takes `-e`, `-u`, `-x` and `+e`, `+u`, `+x` (also combined, `-eu`) and `-o` / `+o` with `errexit`, `nounset`
   or `xtrace`. Any other option is refused with a message. `$-` is the letters of the options that are on.
   - **`-e`**: when a command fails the shell leaves with its status, except in the condition of `if`, `while` or
     `until`, on the left of `&&` and `||`, and after `!`.
   - **`-u`**: expanding an unset variable (`$x`, `${x}`) is an error `x: unbound variable` and the command does
     not run; `${x:-default}`, `$?`, `$#`, `$@`, `$*`, `$$`, `$!`, `$-` and `$0` are fine.
   - **`-x`**: each command is shown on standard error as `+ command` before it runs, as typed (not expanded).
3. **`set -- a b`** and **`set a b`** replace the positional parameters (of the function that runs, or of the shell);
   `set --` clears them. `set` alone shows the variables.
4. **Not included**: `pipefail`, `-f`, `-n`, `-v`, `-C`, `set -o` with no name, the trap on `ERR`, the expanded text in the
   `-x` trace, and arrays and functions in the `set` listing.

## Consequences

- `-u` is a flag of the expansion code (thread local), so it also applies inside a subshell and a function.
- `-e` is checked after each command of a list, so a failing command in the middle of a pipeline is not seen on its own.
