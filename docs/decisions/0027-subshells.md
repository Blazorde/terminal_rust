# ADR-0027 — Subshells `( list )`

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

`(cd build && make)` and `(export X=1; run)` are everyday idioms: run a few commands without changing the
shell that is typing them. The shell had only `{ list; }`, which shares everything.

## Decision

1. A `(` that starts a command (at the start of a line, after `;`, `&&`, `||`, `|`, a newline, or after
   `if`, `then`, `elif`, `else`, `do`, `while`, `until`, `{`, or a function header `name() {`) opens a
   subshell, and the `)` that closes it ends it. The line splitter turns them into the words `(` and `)`, so
   the flow parser reads them like `{` and `}`: `( … ) && x`, `( … ) > file`, `( … ) | cmd`, `if (cmd); then`.
2. The body runs in the same process, in a copy of the state: variables, functions, positional
   parameters, the previous directory, the current directory and the exports are put back when it ends.
   `break`, `continue` and `return` end there; `exit [code]` leaves only the subshell. The status of the
   subshell is the status of its last command.
3. **Not included**: aliases and the history changed inside a subshell are kept; `! ( … )` is a syntax
   error; `(( … ))` arithmetic commands; a `case` statement inside a subshell, because its pattern `)` would
   close the subshell; a `case` arm pattern with a leading `(`; and running the body in another process
   (so background jobs and `$$` are those of the shell).

## Consequences

- No new process: a subshell costs a clone of the variable table, and it cannot be interrupted separately.
- Another thread of the shell would see the directory of the subshell while it runs.
