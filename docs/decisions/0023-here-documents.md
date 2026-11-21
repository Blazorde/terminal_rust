# ADR-0023 — Here-documents and here-strings

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

`cat <<EOF > config`, `sort <<END` and `read a b <<< "x y"` are how people write a few
lines to a file or feed a command from the terminal or from a script. Without them
the shell has no way to give a command several lines of input except through a file.

## Decision

1. **Here-document**: `command <<WORD`, the lines that follow, and a line made of `WORD`
   alone. The body is the standard input of the command. The prompt asks for more
   (`>> `) until the delimiter line, in the terminal, in piped input, in scripts and in
   `source`d files, where the body keeps its blanks, its indentation and its `#` lines.
2. **Expansion**: with a bare word the body is expanded like text in double quotes
   (`$name`, `${…}`, `$(…)`, `$((…))`, and `\$` for a literal dollar) and `"` and `'`
   are plain characters. With a quoted word (`<<'EOF'`, `<<"EOF"`, `<<\EOF`) the body is
   taken literally.
3. **`<<-WORD`** removes the leading tabs of the body lines and of the delimiter line, so
   that a here-document can be indented in a script.
4. Several here-documents on a line take their bodies in order (`cmd <<A <<B`), and the rest
   of the line works as usual: `cat <<EOF > file`, `sort <<EOF | head`, `while read l; do …;
   done <<EOF`, and a function body can hold one.
5. **Here-string**: `command <<< word` gives the expanded word and a newline as input.
   It is also what a here-document is turned into.
6. An operator inside quotes, a `$(…)` or after a backslash is text. A here-document that
   reaches the end of the input before its delimiter is an error and runs nothing; a `<<`
   with no word is a syntax error.
7. **Implementation**: before a line is parsed, each here-document is rewritten as a
   here-string and its body lines are removed. Pipelines, redirections, expansions and the
   counting of unfinished blocks only see the result, so none of them needed to learn about
   here-documents; the last of `<` and `<<<` on a stage is its input.
8. **Not included**: `<<<` with several words that must stay separate, here-documents inside a
   `$( … )` that spans lines of the outer command (they work inside the substitution's own
   text), and a here-document in the middle of a background job.

## Options considered

- Parse here-documents in the grammar with a body field on each stage: touches every layer
  (splitting, blocks, pipelines, scripts); the rewrite touches one.
- Read the body lazily when the command runs: breaks the "reject the whole input before
  anything runs" rule and cannot work with the prompt's continuation.

## Consequences

### Positive

- Multi-line input to commands and files from the prompt, from piped input and from scripts.
- No new state: a here-document is plain text until the rewrite.

### Negative

- The body is held in memory as one word; very large bodies belong in a file.
- A delimiter inside a body line that looks like an operator is only recognised by quotes,
  as in other shells.

## Validation

- Unit tests of the rewrite (bodies, delimiters spaced, quoted, escaped, `-` and tabs,
  several per line, unfinished input, text that only looks like an operator, 100 000 random
  inputs without a panic) and of the prompt's continuation.
- Integration tests with the real binary: expansions, literal bodies, tabs, files, pipes,
  loops, functions, programs as readers, `source`d files, errors.
- A check in a real terminal.
- 16 mutation checks (delimiter matching, tabs, quoting of the body, substitution and quote
  handling, continuation, block counting, scripts, the here-string input, the final newline,
  the first line ending) make tests fail.
