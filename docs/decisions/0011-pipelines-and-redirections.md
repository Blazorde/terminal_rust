# ADR-0011 — Pipelines and redirections

- Status: Accepted
- Date: 2026-11-21
- Decision makers: Terminal Rust team

## Context

[ADR-0010](0010-command-chaining.md) joins commands with `;`, `&&` and `||`
and rejects `|`, `<`, `>` and `&` on purpose, because the output of the built-in
commands was only available as text captured in memory. Without pipes and
redirections a one-liner such as `ls | grep rs`, `cargo test 2> errors.txt` or
`sort < names.txt > sorted.txt` still needs another shell, which blocks the goal
of using Terminal Rust as the default terminal.

## Decision

1. Inside one `;`/`&&`/`||` segment, an unquoted `|` separates the **stages** of
   a pipeline. The status of a pipeline is the status of its last stage.
2. Redirections apply to one stage and are written anywhere among its words:

   | Syntax | Meaning |
   | --- | --- |
   | `< file` | standard input from `file` |
   | `> file`, `1> file` | standard output to `file`, truncated |
   | `>> file` | standard output appended to `file` |
   | `2> file`, `2>> file` | standard error to `file` |
   | `2>&1` | standard error where standard output goes |
   | `>&2`, `1>&2` | standard output where standard error goes |

   The target of a redirection is expanded like any argument (variables, `~`,
   quotes) and must give exactly one word. Relative paths use the current
   directory. Files are created with the default permissions of the process.
3. A stage is either a **program** or an **in-process command** (built-in,
   custom built-in alias, declarative plugin command, assignment).
   - Programs are started with their streams connected: stdout of one stage is
     the stdin of the next through an operating-system pipe, so the stages run
     at the same time and `yes | head` stops. A redirection replaces the pipe.
   - In-process commands keep writing through the output layer, which captures
     their text for the duration of the stage. That text is the stdin of the next
     stage, or goes to its file, or is printed if the stage is the last one.
   - Custom commands of the `process` kind are programs.
4. Built-in commands do not read standard input yet, except where documented:
   in the middle of a pipeline they ignore it. They produce output, which is the
   useful direction (`ls | grep txt`, `echo hi > file`).
5. In-process commands run in the shell, not in a copy of it: `cd` or `export`
   in a pipeline stage takes effect for the next commands. This differs from
   POSIX shells, which run each stage in a subshell, and is documented.
6. A lone `&` runs a command in the background (see
   [ADR-0013](0013-background-jobs.md)); right after `>` it is part of `>&`.
7. A syntax error (an empty stage, a redirection without a target, an
   unsupported form such as `>&3`) rejects the whole line before anything runs. A
   file that cannot be opened is an error for that stage: the stage does not run
   and its status is a failure.
8. A program that cannot start fails its own stage only; the other stages run,
   as in POSIX shells, and the error is reported on standard error.

## Options considered

- Hand the line to the system shell (`sh -c`, `cmd /C`): complete, but changes
  quoting and variables per platform, which ADR-0008 rejected.
- Keep rejecting pipes: honest, but leaves the main daily-use gap open.
- Run every stage as a copy of the shell (subshell): POSIX-exact for `cd`, but
  Rust has no `fork`, and built-ins would need a process model of their own.
- Stream the text of in-process commands through threads: more concurrent, with
  little benefit, because built-in commands produce all their output at once.

## Consequences

### Positive

- The common one-liners work in the CLI, the desktop app and scripts, with real
  streaming between programs.
- Redirections cover files for both output streams and for input.

### Negative

- No here-documents (`<<`), no process substitution, no file descriptors above 2,
  no `pipefail`, no `|&`, no globs (separate work, see ADR-0012).
- In-process stages cannot be interrupted while they run, and a built-in in the
  middle of a pipeline does not read its stdin.
- `cd` and `export` in a pipeline are not isolated.

## Validation

- Parser tests: stages, every redirection form, quoting, `2` as an argument,
  empty stages, missing targets, `>&` only after `>`.
- Execution tests with real programs: a pipe between programs, built-in to
  program, program to built-in, files for input, output, append and errors,
  `2>&1`, the status of the last stage, a missing program in the middle.
- Mutation checks: removing the pipe wiring, the append flag and the stderr
  redirection each make a test fail.
