# ADR-0012 — File name patterns

- Status: Accepted
- Date: 2026-11-21
- Decision makers: Terminal Rust team

## Context

[ADR-0009](0009-shell-variables.md) listed `*` patterns as not available, and
[ADR-0011](0011-pipelines-and-redirections.md) added pipes and redirections.
`rm *.tmp`, `ls src/*.rs` or `cat logs/*.txt | wc -l` still did nothing useful:
the pattern reached the command as a literal argument.

## Decision

1. An unquoted `*`, `?` or `[...]` in a command argument is a pattern. It is
   replaced by the sorted names it matches, as separate arguments.
   - `*` matches any run of characters, `?` one character, `[abc]`, `[a-z]` and
     `[!a-z]` (or `[^a-z]`) one character of a set. `]` first in a set, and a `[`
     that is never closed, are ordinary characters.
   - A pattern is matched one path segment at a time: `*` never matches `/`.
     `src/*/mod.rs` works, there is no `**`.
   - A name that starts with `.` is only matched by a segment that starts with a
     literal `.`.
2. A pattern that matches nothing is left as written, as in POSIX shells, so the
   command reports the missing file itself.
3. Quoting protects a pattern: `'*.txt'`, `"*.txt"`. A backslash protects it too
   where it is not a path separator (Unix and macOS): `\*`. In a word that mixes
   quoted and unquoted parts only the unquoted wildcards count (`"[draft]"*.md`).
4. The value of a variable is never a pattern, so `$NAME` is safe to use with
   files whose names hold wildcards. This differs from POSIX shells.
5. Expansion applies to the arguments of commands and pipeline stages. It does
   not apply to the target of a redirection (`> *.log` writes a file called
   `*.log`) nor to the command name. `~` and variables are expanded first.
6. The matcher is part of the project (`utils/glob.rs`): no dependency is added.
   Names that are not valid UTF-8 are skipped.

## Options considered

- The `glob` crate: complete, but a new dependency for a small matcher, and its
  handling of quoting and of hidden files would need wrapping anyway.
- Leave expansion to each command: inconsistent, and external programs would
  still receive the literal pattern.
- Expand patterns inside variable values too, like POSIX shells: surprising and
  a known source of accidents.

## Consequences

### Positive

- The everyday `rm`, `ls`, `cat`, `mv` and `grep` patterns work, for built-ins
  and programs alike, with the same rules.

### Negative

- No `**`, no brace expansion (`{a,b}`), no extended patterns, no sorting by
  locale: names are sorted by their bytes.
- On Windows only `/` separates segments in a pattern; `src\*.rs` is not
  expanded and stays as written. Backslash does not protect a wildcard there.
- `rm *` is as dangerous as in any shell.

## Validation

- Matcher tests: `*` with backtracking, `?`, classes, ranges, negation,
  unclosed brackets, escapes, hidden names.
- Expansion tests on a temporary tree: sorting, several segments, no match.
- Parser tests for quoted, escaped, mixed and variable words, and integration
  tests through the shell. Mutation checks: no expansion, hidden files included
  and unprotected quotes each make tests fail.
