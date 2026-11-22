# ADR-0030 — Operators on the value of a parameter

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

`${f%.txt}`, `${path##*/}`, `${s/old/new}` and `${s:1:3}` are how scripts cut file names and words without a
program. Only `${NAME:-w}`, `${NAME:+w}`, `${NAME:?w}` and `${#NAME}` existed (ADR-0009).

## Decision

Implemented in `src/app/parameter.rs`, applied by the parameter expansion of the parser:

1. `${v#pattern}` / `${v##pattern}` remove the shortest / longest prefix that matches the pattern, `${v%pattern}` /
   `${v%%pattern}` the suffix.
2. `${v/pattern/text}` replaces the first match, `${v//pattern/text}` every match, `${v/#pattern/text}` a match at the
   start, `${v/%pattern/text}` at the end; the longest match wins, a match of nothing is not replaced, and the text
   may be left out to delete the match. A `/` in the pattern is written `\/`.
3. `${v:offset}` and `${v:offset:length}` count characters; a negative offset counts from the end and is written
   with a space or parentheses (`${v: -3}`, `${v:(-3)}`) because `${v:-3}` is the default value; a negative length
   stops before the end. The numbers may be `$name` or a bare name. On `${name[@]}` and `${@}` the same slice takes
   elements (`${@:2}` is from `$2`, `${@:0}` includes `$0`) and keeps them as separate words.
4. `${v^}` and `${v^^}` make the first character, or all of them, upper case; `${v,}` and `${v,,}` lower case.
5. Patterns are the shell patterns of ADR-0012 (`*`, `?`, `[...]`), after variables are expanded. A quote around a
   wildcard does not make it literal in a pattern here (an escaped one, `\*`, does).
6. **Not included**: `${v@Q}` and the other `@` transformations, case operators with a pattern (`${v^^[aeiou]}`),
   `${v/pattern/text}` on arrays element by element, `${!name}` indirection.

## Consequences

- An operator that is not recognised after a valid name stays `bad substitution`, as before.
- Matching tries every prefix or suffix length, so it is quadratic in the length of the value, which is fine for
  names and lines.
