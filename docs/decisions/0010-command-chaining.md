# ADR-0010 — Command chaining and unsupported operators

- Status: Accepted
- Date: 2026-11-20
- Decision makers: Terminal Rust team

## Context

With external programs ([ADR-0008](0008-external-programs.md)) and variables
([ADR-0009](0009-shell-variables.md)), a line often needs several commands:
`cargo build && cargo test`, `cd project; ls`, `make || echo failed`.

Until now an unquoted `|`, `>`, `<` or `&` reached the program as an ordinary
argument, so `cat file | grep x` ran `cat` with a `|` argument and gave a
misleading result.

## Decision

1. A line is split at the `;`, `&&` and `||` that are neither quoted nor
   escaped. The operators are evaluated left to right with equal priority, as in
   POSIX shells: `;` always runs the next command, `&&` runs it only after a
   success, `||` only after a failure. A skipped command does not change the
   status.
2. The text of each segment is expanded and parsed right before it runs, so `$?`
   and variables set earlier on the same line are visible to the next segment.
3. The failure of a segment that is followed by another command that runs is
   printed immediately. The result of the last command that ran is the result of
   the line, and `exit` stops the line.
4. A syntax error (a missing command around an operator, an unclosed quote)
   rejects the whole line before anything runs.
5. Pipes (`|`), redirections (`<`, `>`) and background jobs (`&`) are rejected
   with a clear message until they are designed, instead of being passed to the
   program as arguments. Quoting or escaping them keeps them literal. Pipes and
   redirections were designed in [ADR-0011](0011-pipelines-and-redirections.md);
   background jobs are still rejected.
6. When output is going to a terminal, the output captured from built-in
   commands is written before a child process starts, so output keeps the order
   of the commands. The engine API, which returns the captured output, does not
   do this.
7. Script files (`.trs`) use the same line execution.

## Options considered

- Leave `|`, `>`, `<` and `&` as arguments: no work, but silently wrong results.
- Reject them now and add them later (chosen): honest behavior today, and the
  syntax stays free for a real design.
- Parse every operator now, including pipes: needs a way to capture or stream
  the output of built-in commands first, which is a separate decision.

## Consequences

### Positive

- Common build and test chains work in the CLI, the desktop app and scripts.
- A user who types a pipe gets a clear message instead of a wrong result.

### Negative

- No pipes, redirections or background jobs yet, so many one-liners still need
  another shell.
- No grouping with parentheses or braces, and no `!` negation.
- Splitting happens before expansion, so a variable cannot contain an operator.

## Validation

- Parser tests for each operator, quoting, escapes, trailing `;` and errors.
- Engine tests for the evaluation rules, `$?` inside a line, `exit` and the
  final status.
- An integration test with a real program shows the output stays in order; a
  mutation check (disabling the flush) makes that test fail.
