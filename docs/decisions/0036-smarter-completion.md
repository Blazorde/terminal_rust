# ADR-0036 — Smarter completion

- Status: Accepted
- Date: 2026-11-25
- Partly replaced: remotes and stash entries (point 8) are completed since
  [ADR-0037](0037-git-completion-remotes-stash-and-directory.md)
- Decision makers: Terminal Rust team

Completes [ADR-0019](0019-git-completion.md). It replaces one part of it:
options, left to the file name completion by its points 4 and 6, are now
completed. The rest still applies.

## Context

`Tab` inserted one candidate after the other without showing them, found a command
only after `;`, `|` and `&`, offered files to `cd`, knew no option and required the
exact case. Users of bash on Linux expect a visible list and completion that
follows what the line means.

## Decision

1. The line editor uses its list mode: `Tab` inserts what every candidate shares,
   a second `Tab` lists them. The history hint is unchanged.
2. Every source is first asked for names in the case typed. Only when no source
   has any are the names in another case offered, in this order for an argument:
   exact references, exact files, other-case references, other-case files.
3. A pure reader of the line before the cursor finds the word and its role. A
   command goes at the start, after separators, the openings `$(`,
   backquote, `(`, `{`), the keywords that precede a command and the wrappers
   `sudo`, `env`, `time`, `nohup`, `xargs`, `exec` and `command`.
4. `cd`, `pushd` and `rmdir` get directories only.
5. Options of built-ins come from their usage line and from the table of
   [ADR-0035](0035-system-command-fallback.md). Options of nine Git subcommands
   are a table of the completion, as nothing in this repository defines them. The
   case counts for options.
6. File names are completed by the shell and no longer by the completer of the
   line editor, which cannot fall back to another case. Names are quoted by the
   rules of the parser. A name with a control character is not offered. On Windows
   the last `\` typed becomes `/` when the written name starts with a quote or a
   `\`, which it would otherwise escape.
7. No process is started. At most 2,000 candidates are offered, keeping what the
   whole list shares: the line editor fails above 65,535.
8. Not included: remotes, modified files for `git add` and `restore`, stash
   entries, completion for `cargo`, `ssh`, `systemctl` or `dnf`, and the
   completion scripts of bash.

## Options considered

- Keep cycling through candidates: shows one name at a time.
- Keep the file completer of rustyline: no other-case fallback on Linux, no
  directories-only filter, no bound.
- Use the scripts of bash-completion: needs bash, not available on Windows.
- Write names with control characters as `$'…'`: they could be completed, at the
  cost of a third quoting style for a rare case.

## Consequences

### Positive

- The choices are visible, and completion follows the structure of the line.
- The analysis of the line and the quoting are tested without a disk or a terminal.

### Negative

- `Shift+Tab` no longer goes back through candidates.
- The reader of the line is separate from the parser and must follow its rules.
- The table of Git options is maintained by hand.
- The other-case fallback applies to the last name of a path only.
- A path that starts with a variable (`$HOME/…`) is not completed, as before.

## Validation

- Unit tests of the line reader, of the quoting (the parser reads every written
  name back), of the case fallback, of the bound and of the options.
- Tests through the completion helper with temporary directories.
- Mutations of these rules make tests fail.
- A check in a real terminal: list, history hint, case, `sudo gi`, `cd`, names
  with spaces.
