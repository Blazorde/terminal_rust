# ADR-0013 — Background jobs

- Status: Accepted
- Date: 2026-11-21
- Decision makers: Terminal Rust team

## Context

[ADR-0010](0010-command-chaining.md) and
[ADR-0011](0011-pipelines-and-redirections.md) reject `&`. Starting a server,
a watcher or a long build and keeping the prompt is the main thing left that
forces a second terminal. Full job control with `Ctrl+Z` needs process groups,
`tcsetpgrp` and signal handling, which the project cannot do without `unsafe`
code or a new dependency (`#![forbid(unsafe_code)]`), and which behaves very
differently on Windows.

## Decision

1. A lone `&` ends a command and runs it in the background: `server &`,
   `build & test`. The shell prints `[1] 4242` (the job number and the process id
   of the last program) and the prompt comes back at once. `$?` is `0`.
2. A job is one pipeline. `&` after `&&` or `||` (`a && b &`) is rejected: the
   whole list would have to run in the background, which is not supported.
3. A background program does not read the terminal: its standard input is empty
   (unless redirected). Its output goes to the terminal, so it may appear in the
   middle of what you type.
4. In-process commands in a background pipeline (a built-in, a plugin command)
   run at once in the shell and their output is shown; only programs run in the
   background.
5. New built-in commands:
   - `jobs` lists the jobs as `[1]+ Running  command` or `Done`.
   - `fg [%n]` waits for a job (the most recent without an argument), shows its
     result and removes it; `$?` is its exit code.
   - `wait [%n]` waits for one job, or for all of them, without a result of
     its own beyond `$?`.
   - `kill %n` stops a job.
6. Finished jobs are reported once, before the next prompt:
   `[1]+  Done  command` or `[1]+  Exit 3  command`, and forgotten.
7. Leaving the shell does not stop the jobs, the programs keep running. In the
   desktop app, closing the terminal ends them with the session.
8. **Not included**: `Ctrl+Z`, `bg`, stopped jobs, `disown`, `%-` and `%+`
   job specs beyond a number, `wait` for a process id, and giving the terminal
   to a job (a background program that insists on the terminal can still fight
   the shell for it).

## Options considered

- Full job control with process groups: right behavior, but needs `unsafe` or a
  dependency and a Windows design of its own.
- Keep rejecting `&`: honest, but leaves the long-running-process case open.
- Run background jobs on threads inside the shell: cannot host programs that read
  or write the terminal, and cannot be killed cleanly.

## Consequences

### Positive

- Servers and long commands run while the shell stays usable, with no `unsafe`
  code and no dependency.
- Same behavior on Linux, macOS and Windows.

### Negative

- No suspension (`Ctrl+Z`) and no handing the terminal over: an interactive
  program must run in the foreground.
- Output of a background job can mix with the prompt line.
- The shell notices a finished job at the next prompt, not the moment it ends.

## Validation

- Parser tests for `&` at the end, in the middle, after `&&`, and for `2>&1`.
- Integration tests with real programs: the prompt returns while a program
  runs, `jobs`, `fg`, `wait`, `kill`, the completion notice and its exit code,
  empty standard input.
- Mutation checks: not detaching, and not reporting finished jobs, make a test
  fail.
