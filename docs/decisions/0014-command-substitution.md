# ADR-0014 — Command substitution

- Status: Accepted
- Date: 2026-11-21
- Decision makers: Terminal Rust team

## Context

[ADR-0009](0009-shell-variables.md) gave the shell variables,
[ADR-0011](0011-pipelines-and-redirections.md) pipes and
[ADR-0012](0012-filename-patterns.md) patterns. The most common thing a shell
user still cannot write is the output of one command inside another:
`cd $(dirname file)`, `echo today is $(date)`, `NAME=$(git rev-parse HEAD)`.

## Decision

1. `$(line)` is replaced by what `line` writes on standard output. `line` is a
   full command line: chains, pipes, redirections, patterns and nested
   substitutions all work. Trailing newlines are removed. What the command writes
   on standard error goes to the terminal as usual.
2. The substitution runs when its command runs, after the commands before it on
   the line, so `cd project ; echo $(pwd)` sees the new directory and `$?`.
3. **Quoting.** In double quotes the result is one word, unchanged. Unquoted, it
   is split on spaces, tabs and newlines into separate words, and an empty result
   gives no word at all. The words are never matched against file names, so a
   result such as `*.rs` stays a plain word. Single quotes and a backslash
   (`\$(`) keep `$(` literal. In an assignment word (`NAME=$(cmd)`, `export
   NAME=$(cmd)`) the result is not split.
4. Programs started inside a substitution write to the substitution, including
   Git and custom `process` commands. A program that reads the terminal still
   gets the terminal.
5. The substitution is not a subshell: `cd` or `export` inside it takes effect in
   the shell, as in a pipeline stage (ADR-0011). `exit` inside it does not close
   the shell. Its status is not the status of the line: `$?` is the one of the
   command that uses it.
6. An unclosed `$(` rejects the whole line before anything runs. `$((...))`
   arithmetic is not supported and says so. Backquotes are not supported.
   Substitutions nest up to 8 levels.

## Options considered

- Parse and substitute while splitting words: needs the interpreter inside the
  parser. Instead the text of a segment is substituted first, each result
  re-quoted so it can only ever be words and never operators, then parsed as
  usual.
- Splitting on every character of `IFS`: no `IFS` variable exists yet.
- Backquotes: a second syntax for the same thing, with confusing nesting.

## Consequences

### Positive

- Everyday one-liners work, for built-in commands and programs alike.
- Because results are re-quoted, a command cannot inject `;`, `|` or `>` through
  its output.

### Negative

- No arithmetic expansion, no process substitution (`<(...)`), no `IFS`. Backquotes
  (`` `line` ``) were added later and work like `$(line)`: `` \` ``, `\\` and `\$` are unescaped
  inside them, they are active in double quotes and not in single quotes, and a line that
  ends with the closing backquote of a pair is complete (the trailing backtick continues
  a line only when the backquotes of the line are odd in number).
- Results are not matched against file names, unlike POSIX shells.
- A substitution that reads and writes the terminal at once may behave oddly.

## Validation

- Parser tests: operators inside `$(...)` do not split the line, quotes and
  parentheses inside, an unclosed substitution, `$((`.
- Integration tests with real programs: the replacement and its trailing
  newlines, splitting, quoting, nesting, a failing command, pipes and chains
  inside, the substitution in redirections and assignments, injection safety.
- Mutation checks: not capturing programs, not re-quoting and not splitting make
  tests fail.
