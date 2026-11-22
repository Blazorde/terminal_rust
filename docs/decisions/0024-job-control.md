# ADR-0024 — Job control: Ctrl+Z, `bg` and `fg` in safe Rust

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

An interactive program (an editor, a pager, `ssh`, a dev server) that the user wants to
leave for a moment had no way to be stopped: `Ctrl+Z` did nothing useful and `bg` did not
exist. Job control needs process groups, the terminal's foreground group and the wait for
a *stopped* child, which the standard library does not expose. The usual way is `unsafe`
code (`libc`, `pre_exec`), and this project is `#![forbid(unsafe_code)]`.

## Decision

1. **Safe wrappers only.** `rustix` (`getpgrp`, `setpgid`, `waitpid` with `UNTRACED`,
   `kill_process_group`, `tcgetpgrp`/`tcsetpgrp`, `tcgetattr`/`tcsetattr`) and `nix`
   (`pthread_sigmask`) give these calls as safe functions, `std` gives
   `CommandExt::process_group`, and `signal-hook` catches a signal without `unsafe`. The
   crate keeps `forbid(unsafe_code)`. Everything lives in `src/utils/job_control.rs`.
2. **Only on a terminal.** Job control is turned on at startup when the standard input is a
   terminal that the shell owns (its group is the foreground group). In a script, a pipe,
   `terminal -c` or on Windows nothing changes: programs are started and waited for as before.
3. **Foreground programs lead a process group.** A program, or the programs of a pipeline
   together, are started in a new group. The shell hands the terminal to that group while it
   waits, so `Ctrl+C` and `Ctrl+Z` reach the program and not the shell, and takes it back
   afterwards. A program ended by `SIGINT` still stops the loops of the shell, as `Ctrl+C`
   did before.
4. **Stopped jobs.** The wait asks to be told about stopped children. A group that was
   stopped becomes a job, announced at once (`[1]+  Stopped  vim notes.txt`), with the terminal
   modes it left, which are put back when it continues. `$?` is 148.
5. **`fg [%n]`** gives the terminal back to the job, restores its modes, sends `SIGCONT` and waits
   (it can be stopped again). **`bg [%n]`** sends `SIGCONT` and leaves it in the background.
   `jobs` shows `Stopped`; `wait` does not wait for a stopped job, as it would never end.
6. **`exit` warns once** (`There are stopped jobs.`) and leaves on the next `exit`.
7. **At the prompt `Ctrl+Z` does nothing**: it is not bound to a suspend, and the shell catches
   `SIGTSTP` so that it is never stopped while it has the terminal.
8. **No signal mask is inherited.** `SIGTTOU` and `SIGTTIN` are blocked only for the calls that set
   the foreground group and the terminal modes, which the shell makes while it is in the
   background, and unblocked right after. Blocking them for good would be inherited by the
   programs and `Ctrl+Z` would not stop them (found by trying it in a PTY).

## Consequences

- Every place that starts a program in the foreground goes through
  `job_control::run_foreground` (external commands, user commands, `man`, the editors,
  `fastfetch`, Git) or through the pipeline code; a new one must do the same.
- A stopped pipeline leaves its feeding threads running until the pipeline continues.
- The tests run real `sh` children that stop themselves (`kill -STOP $$`), and a PTY test
  drives `Ctrl+Z`, `jobs`, `bg`, `fg`, `Ctrl+C` and the `exit` warning in the desktop shell.
- Not included: `disown`, `%+`/`%-`/`%name` specs, `suspend`, background jobs that read the
  terminal (their standard input stays empty), and Windows.
