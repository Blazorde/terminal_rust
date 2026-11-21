# ADR-0015 — Brace expansion

- Status: Accepted
- Date: 2026-11-21
- Decision makers: Terminal Rust team

## Context

`mkdir -p project/{src,tests}`, `cp config{,.bak}` and `echo {1..5}` are among
the first things typed in a shell. After file name patterns
([ADR-0012](0012-filename-patterns.md)) and command substitution
([ADR-0014](0014-command-substitution.md)) they were the last common expansion
still missing.

## Decision

1. In an unquoted word, `{a,b,c}` gives one word per alternative, each a copy of
   the word with that alternative in place: `file{.txt,.md}` is `file.txt
   file.md`. Several groups give every combination, the first group varying
   slowest (`{a,b}{1,2}` is `a1 a2 b1 b2`), and groups nest (`{a,{b,c}}`).
   An alternative may be empty (`f{,.bak}`).
2. Sequences: `{1..5}`, `{5..1}`, `{-2..2}`, `{a..e}`, with a step
   (`{1..10..3}`) and zero padding when an end has a leading zero (`{01..10}`).
3. Anything that is not a group stays as written: `{}`, `{a}`, a brace never
   closed, `{a..bb}`. A group needs a comma or a valid sequence.
4. Quoting and escaping keep braces literal, and a `${NAME}` is a variable, not
   a group. A `$(...)` is opaque. Quotes and escapes inside an alternative are
   kept for the parser.
5. Braces are expanded before command substitutions and before the line is
   parsed, on the text of the segment, so the results are ordinary words that go
   through variable expansion and file name patterns afterwards. A word that
   holds an unquoted operator (`|`, `<`, `>`, `&`, `;`, parentheses) is left
   alone, so `echo {a,b}>out` redirects as before.
6. One expansion may produce at most 10 000 words; more is an error
   (`brace expansion would produce too many words`) and nothing runs.

## Options considered

- Expand inside the word parser: loses the quoting information that the later
  steps need.
- Expand after variables, as for patterns: a value such as `{a,b}` would then
  expand, which is not what POSIX shells do.

## Consequences

### Positive

- The usual one-liners (`mkdir -p a/{b,c}`, `cp f{,.bak}`, `touch f{1..3}`) work.

### Negative

- Braces in a word that also holds a redirection are not expanded.
- No `{a,b}` in a redirection target, and no multi-character sequences.

## Validation

- Unit tests for groups, nesting, sequences, padding, literal cases, quotes,
  `${}`, substitutions, operators, spacing and the limit.
- Integration tests with real commands, and mutation checks (segment not
  expanded, no recursion) that make tests fail.
