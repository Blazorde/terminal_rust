# ADR-0020 — Recursive patterns (`**`)

- Status: Accepted
- Date: 2026-11-21
- Decision makers: Terminal Rust team

## Context

[ADR-0012](0012-filename-patterns.md) matches a pattern one directory level at a
time, so `*` never crosses a `/` and a segment `**` behaved like `*`. Finding every
`*.rs` file of a project (`ls **/*.rs`, `rm **/*.tmp`, `for f in **/*.log`) needed
`find` or `grep -r`, which Windows does not have.

## Decision

1. A path segment that is exactly `**` stands for any number of directories,
   including none: `**/*.rs` is every `.rs` file of the current directory and of all
   the directories below it, and `src/**/mod.rs` does the same under `src`.
2. A `**` at the end (`**`, `src/**`) lists everything below the directory, files and
   directories, not the directory itself.
3. As with `*`, names that start with `.` are skipped, so hidden directories such as
   `.git` are not entered. A `**` that is not a whole segment (`a**`, `**b`) is an
   ordinary `*`, and an escaped or quoted `**` is text.
4. Symbolic links to directories are listed but not followed, so a link back to a
   parent cannot make the walk endless.
5. A `**` goes at most 32 directories deep and one pattern gives at most 50 000
   paths: a `**` typed in a huge tree stops there instead of running for minutes.
6. The results are sorted and unique, and a pattern that matches nothing stays as
   written, as before.
7. **Not included**: `**` as a part of a name (`foo**/bar` as a recursion), the
   `(...)` qualifiers of zsh, and following links.

## Options considered

- A separate `-r` option on each command: not available to commands that are programs.
- Following symbolic links: finds more, but needs cycle detection and can leave the
  tree the user means.
- Making `**` opt-in (Bash `shopt -s globstar`): a hidden setting for what almost
  everybody expects from `**/`.

## Consequences

### Positive

- `ls **/*.rs`, `for f in **/*.log; do …; done` and `rm **/*.tmp` work on every system.

### Negative

- A pattern that began as one level (`**`) is now recursive: `echo **` lists the
  whole tree below the current directory.
- A tree deeper than 32 levels or larger than 50 000 matches is cut short without a
  message.

## Validation

- Unit tests on fixture trees: every depth, zero directories, in the middle, trailing,
  hidden directories, a link back to the root, an embedded or escaped `**`, and the
  depth limit for both forms.
- An integration test with the real binary: `echo`, `for` and `cat` over `**`, quoting,
  no match, and a trailing `**`.
- Mutation checks: `**` read as `*`, no zero-directory step, hidden directories
  entered, links followed, the depth limit removed (in the middle and at the end), and
  a trailing `**` without its entries make tests fail.
