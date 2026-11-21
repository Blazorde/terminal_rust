# ADR-0022 — Redirections and pipes on blocks

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

[ADR-0016](0016-control-flow.md) and [ADR-0021](0021-functions.md) left one thing out:
a block could not be redirected or piped. The most common shell idioms use exactly
that: `while read line; do …; done < file`, `cmd | while read x; do …; done`,
`for f in *.log; do …; done > report.txt`, `{ a; b; } | sort`.

## Decision

1. What follows the closing `done`, `fi`, `esac` or `}` of a block may be redirections
   (`< file`, `> file`, `>> file`, `2> file`, `2>&1`, `>&2`) and a pipeline (`| cmd …`).
   Anything else after it is a syntax error. A block after a pipe is allowed too:
   `cmd | while …; done`, `cmd | { read a; echo $a; }`, `a | b | for …; done | c`.
2. The block is **one stage** of the pipeline. It runs inside the shell, so
   variables it sets and the directory it changes stay (unlike Bash, which runs a
   piped block in a subshell). Everything it writes, programs it starts included, is
   collected and sent to its redirection or to the next stage, so a loop that
   starts `sort` or `sh -c` can be redirected whole. Its output is therefore shown
   when the block ends, not as it goes.
3. A function body or definition is not redirected (`f() { … } > file` is an error);
   redirect the call instead. A pipeline holds at most one block (`… | for … done |
   while … done` is not available).
4. The input of a stage (`< file`, or the pipe before it) is available to the
   commands inside the block, **line by line**: `read` takes the next line, and
   `cat` takes everything that is left. A command with no input of its own inside
   the block shares the block's input, as in Bash. When the input is read to its
   end, the next `read` finds the end of the file; it does not fall back to the
   keyboard. Without any redirection `read` still reads the keyboard.
5. A redirection that cannot be opened (`done < missing.txt`, an unwritable
   file) fails the block before it runs anything, with status 1 and a message.
6. The status of a block with redirections is the one of its last command; a program's
   exact exit code is kept when the block is alone. In a pipeline, as always, it is
   the one of the last stage.
7. The block that follows `&&` or `||` keeps its own connector, so
   `false || { echo failed; } > log` runs.
8. **Not included**: here-documents (`<<`) and here-strings (`<<<`), `&` after a block,
   a block in the middle of a *background* pipeline, streaming output while a
   redirected block runs, and `exec` redirections.

## Options considered

- Run the block in a subshell: needs a copy of the whole shell state, and surprises
  people who set a variable in the loop (`count=$((count+1))`).
- Stream the output of the block: needs the pipeline to run stages concurrently with
  in-process commands, a much larger change.
- Add here-documents first: they need a multi-line input mode of their own.

## Consequences

### Positive

- The usual file-processing loops work, including `while read` over a file or a pipe.
- Variables set in a piped loop are visible afterwards, which is what most people
  expect to be able to rely on.

### Negative

- A long-running redirected block shows nothing until it ends.
- Differences from Bash to know about: no subshell, and a block piped into another
  program is complete before the program starts.

## Validation

- Unit tests of the line-wise input (lines, blank lines, no final newline, the end, a
  scope that shares or replaces the outer input) and of the grammar (all redirection
  forms after each kind of block, a block after a pipe, quotes and `$(…)` not mistaken
  for a pipe, malformed forms, unfinished input).
- Integration tests with the real binary: `read` loops over a file and over a pipe, a
  command inside the loop sharing the input, writing and appending, programs
  collected, pipelines of blocks, `&&` and `||`, statuses and errors.
- A check in a real terminal (multi-line loops, `>> ` prompt, keyboard `read`).
- 15 mutation checks (input consumption and sharing, end of input, capture of programs,
  the tail ignored, the pipe detection, connectors, statuses, exact exit code, text
  after a block, unfinished input, functions) make tests fail.
