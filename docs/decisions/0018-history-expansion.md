# ADR-0018 — History expansion

- Status: Accepted
- Date: 2026-11-21
- Decision makers: Terminal Rust team

## Context

The line editor already recalls earlier lines with the arrow keys and search, but
habits from other shells need text references: `sudo !!` after forgetting a
prefix, `cd !$` to enter the directory just created, `!git` to run the last `git`
command again. There was also no way to list the history from the shell.

## Decision

1. At the interactive prompt, before a line runs, these references are replaced
   by text from the history:

   | Reference | Replaced by |
   | --- | --- |
   | `!!` | the previous line |
   | `!$` | the last word of the previous line (the command itself if it has no argument) |
   | `!^` | the first argument of the previous line |
   | `!N` / `!-N` | line number `N`, or the `N`th line back |
   | `!prefix` | the most recent line that starts with `prefix` |

2. The words keep their quotes as typed, so `cat !$` after `echo > "my file"`
   gives `cat "my file"`.
3. A `!` stays literal inside any quotes, after a backslash or a `$`, at the end
   of a line and before a blank, `=` or `(`: `echo hi!`, `[ ! -f x ]`, `a != b`.
   This differs from Bash, which also expands inside double quotes.
4. When something was replaced, the resulting line is shown, then run, and
   **recorded as expanded**. A reference to a line that does not exist
   (`!99`, `!nothing`, `!!` with an empty history) prints `…: event not found`;
   nothing runs and nothing is recorded.
5. An expansion is not expanded again, and it only happens at the interactive
   prompt: scripts, `source`, piped input and `-c` style runs never expand, so a
   `!` in a script is always literal.
6. `history [N]` lists the lines typed at the prompt, oldest first, numbered with
   the same numbers `!N` uses (the history file loaded at start comes first). A
   line identical to the previous one is not recorded twice, as the editor already
   did.
7. **Not included**: word designators and modifiers (`!!:1`, `!:$`, `!!:s/a/b/`,
   `^a^b`), `!?text?`, `!*`, `history -c` and `-d`, and a configurable expansion
   character.

## Options considered

- Expand inside double quotes like Bash: `echo "Hello!!"` would silently run the
  previous command, a well-known trap.
- Expand in the parser: the parser also runs for scripts and substitutions, where
  a `!` must stay literal.
- Leave it to the editor (rustyline): it has no text expansion.

## Consequences

### Positive

- `sudo !!`, `cd !$` and `!git` work, and `history` shows what was typed.
- The text of the expanded line is visible before it runs.

### Negative

- Some Bash habits (`!!:1`, `^a^b`) are missing.
- A line typed in a block over several lines is one history entry.

## Validation

- Unit tests for every reference, quotes and escapes, missing events, the
  non-references, no re-expansion, and a random test over 100 000 inputs.
- A PTY test with the real binary: `!!`, `!$` and a missing event, and what is
  recorded.
- Mutation checks on the references, the negative index, the prefix order, the
  quote and `$` protections, and on the interactive loop (expanded line ignored,
  missing event run anyway).
