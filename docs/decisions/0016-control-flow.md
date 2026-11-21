# ADR-0016 — Control flow

- Status: Accepted
- Date: 2026-11-21
- Decision makers: Terminal Rust team

## Context

[ADR-0010](0010-command-chaining.md) joins commands with `;`, `&&` and `||`.
Everyday one-liners still need a loop or a test: `for f in *.log; do gzip $f;
done`, `while ! ping -c1 host; do sleep 1; done`, `if grep -q x file; then
echo found; fi`. Without them the shell cannot run the scripts people already
have.

## Decision

1. Three compound commands, recognised at the start of a command (the start of a
   line, or after `;`, `&&`, `||` or a newline):

   ```text
   for NAME in WORD...; do LIST; done
   while LIST; do LIST; done        until LIST; do LIST; done
   if LIST; then LIST; [elif LIST; then LIST;]... [else LIST;] fi
   ```

   `LIST` is any sequence of commands, chains and pipelines, and compound
   commands nest. A newline works like `;`, so the same code can be written over
   several lines.
2. `for` expands its words like the arguments of a command (braces, `$(...)`,
   variables, patterns), sets the shell variable `NAME` for each, and leaves it
   set after the loop. With no words the body does not run.
3. A condition is true when its last command succeeds. The error message of a
   failing condition is not shown, except for an unknown command, because a test
   that fails is not an error. The condition is not a command whose failure
   stops a `&&` chain.
4. `break` and `continue` stop the innermost loop or skip to its next turn, and
   are an error outside a loop. `true` and `false` are built in. The status of a
   compound command is the one of the last command it ran, or `0` if none ran.
5. `Ctrl+C` stops a loop between two commands and prints `interrupted`: the
   interrupt reaches the program that runs, and the loop notices it. `exit` in a
   loop leaves the shell.
6. A compound command is complete only at its `done` or `fi`. Interactive input
   and scripts keep reading lines (`>> ` prompt) until it is, so a block can be
   typed or pasted over several lines. A syntax error (a missing `do`, `done`,
   `then` or `fi`, or a stray keyword) rejects the whole input before anything
   runs.
7. **Not included**: `case`, functions, `select`, `break N`, `[[ ]]`, arithmetic
   `for ((...))`, redirections or pipes on a whole compound (`done > file`,
   `done | cat`), and keywords written inside a word. Use the programs `test` or
   `[` for tests, where the system has them.

## Options considered

- Leave loops to external shells: keeps the grammar small, but blocks real use.
- A separate script language: more to learn and document than three keywords.
- `break N`, `case` and functions now: a larger grammar and runtime for less
  common needs, left for later.

## Consequences

### Positive

- The common one-line loops and tests work interactively, pasted, and in `.trs`
  scripts.
- The structure is parsed once, before anything runs, so a typo does not leave
  half a loop executed.

### Negative

- The compound commands have no redirections or pipes of their own, and there are
  no functions or `case`.
- On Windows there is no `test` program, so conditions are limited to commands
  that succeed or fail by themselves.
- A multi-line block typed interactively is one history entry per line.

## Validation

- Parser tests for each construct on one line and over several lines, nesting,
  syntax errors, and the detection of an unfinished block.
- Integration tests with real programs: loops over words, patterns and
  substitutions, conditions with exit codes, `break`, `continue`, nested loops,
  status after a loop, an interrupted loop, scripts and piped multi-line input.
- Mutation checks: ignoring `break`, running the wrong branch, and not stopping a
  loop on interrupt make tests fail.
