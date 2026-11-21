# ADR-0009 — Shell variables and expansion

- Status: Accepted
- Date: 2026-11-20
- Decision makers: Terminal Rust team

## Context

With [ADR-0008](0008-external-programs.md) the shell starts external programs, but
a command line cannot refer to the environment: `echo $HOME`, `cd ~/code` or
`PATH=$PATH:/opt/bin` do not work. Daily use of a terminal needs variables.

The project forbids `unsafe` code, and since Rust 2024 `std::env::set_var` is
`unsafe`. The shell therefore cannot export variables by changing its own
process environment.

## Decision

1. **Expansion happens while the line is parsed**, before the command runs, and
   follows quoting: `$NAME`, `${NAME}`, `$?` and a leading `~` are expanded
   outside quotes; `$NAME`, `${NAME}` and `$?` are expanded inside double quotes;
   nothing is expanded inside single quotes. `\$` writes a literal dollar sign.
2. A variable that is not set expands to an empty string. An unquoted expansion
   that is empty removes its argument. The expanded text is never split into
   several arguments.
3. `~` alone or followed by a separator expands to `HOME` (`USERPROFILE` on
   Windows). `~name` is left as written.
4. `NAME=value` on its own line sets a shell variable. If `NAME` is already part
   of the environment or exported, the assignment changes the exported value, as
   in other shells; otherwise the variable stays local to the shell.
5. `export NAME[=value]` makes a variable visible to the programs the shell
   starts. `unset NAME` removes it, including an inherited one. `export` without
   argument lists the environment as `printenv` does.
6. `$?` is `0` after a command that succeeded. After a program that failed it is
   the exit code of the program, or 128 plus the signal number when a signal
   stopped it (`137` after a kill); `127` for a program that does not exist; and
   `1` for a command that failed inside the shell or a line that cannot be
   parsed. In a pipeline it is the code of the last program. (Until 1.3.0 it
   was only `0` or `1`.)
7. Exported variables live in one table owned by the shell process and are
   applied to every process the shell starts (external programs, Git, editors,
   `fastfetch`, `man`, custom process commands). The process environment of the
   shell itself is never modified.
8. Inside `${...}`: `NAME:-word` and `NAME-word` (default), `NAME:+word` and
   `NAME+word` (alternative), `NAME:?word` and `NAME?word` (error, and the command
   does not run), and `#NAME` (length in characters). The colon makes an empty value
   count as unset. The word is expanded (variables, quotes, nested `${...}`) but never
   split or matched against files. `NAME:=word` and the trimming and replacing forms
   are rejected with an explicit message. (Added in 1.7.0.)

## Options considered

- Modify the process environment with `set_var`: simplest, but `unsafe`, which
  needs a new ADR and exposes a data race with other threads.
- Keep exported variables in the shell state and pass them to each command:
  safe, but every command handler and every Git helper would need a new
  parameter.
- A process-wide table applied at each launch (chosen): safe, small, and the
  shell already owns process-wide state such as the current directory.

## Consequences

### Positive

- `$HOME`, `~`, `${VAR}`, `export` and `PATH=$PATH:dir` work in the CLI and in
  the desktop app.
- No `unsafe` code is added.

### Negative

- The exported table is global, so unit tests that change it must use names
  unique to the test.
- No word splitting and no globbing: `echo $LIST` passes one argument.
- A command that fails inside the shell is always `1`.
- `NAME=value command` was added after 1.3.0: the variables are exported for that
  command only (a program or a pipeline stage) and the previous values are
  restored. A built-in command sees them through the same table. `exit [code]`
  leaves with that code, or with the last status. File name patterns were added
  by [ADR-0012](0012-filename-patterns.md).

## Validation

- Parser tests for each quoting case, escapes, unset variables, empty
  expansions, braces and tilde.
- Tests for assignment, `export`, `unset` and `$?` through the engine.
- A test that an exported variable reaches a started program and that an
  unexported one does not.
