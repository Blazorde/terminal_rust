# ADR-0021 — Functions and positional parameters

- Status: Accepted
- Date: 2026-11-21
- Decision makers: Terminal Rust team

## Context

With variables, loops, `case` and substitutions, the language lacks one thing to
write reusable scripts: a way to name a block of commands and give it arguments.
Aliases ([ADR-0008](0008-external-programs.md)) cannot take a parameter in the middle
of a command. Functions are also what people keep in their startup file.

## Decision

1. **Definition**: `name() { LIST; }`, `name () { … }`, `function name { … }` or
   `function name() { … }`. The `{` may be on the next line, and the body may span
   several lines (the `>> ` prompt goes on until the matching `}`). A function name is
   made of letters, digits, `_` and `-`, does not start with a digit or a dash, and is
   not a keyword. `{ LIST; }` alone is a group of commands. `}` must start a command
   (after `;` or a newline). Redefining a function replaces it.
2. **Call**: a function takes the place of any command of the same name, built-in or
   program, and works in pipelines, with redirections and in `&&` / `||` lists.
   `unset -f name` removes it, `type name` says `name is a function`, and Tab
   completes its name. A function lives for the session; put definitions in the
   startup script to have them every time.
3. **Positional parameters**: inside a call, `$1` to `$9` (one digit, so `$10` is `$1`
   and a `0`; `${10}` is the tenth), `${N:-default}`, `${#1}`, `$#` (their number), `$*`
   (all, joined by spaces) and `$@`. `"$@"` and an unquoted `$@` give **one word per
   parameter**, and no word at all when there is none; `"$*"` is one word. `$0` is
   `terminal_rust`. Outside any function they are empty and `$#` is `0`. `shift [N]`
   drops the first parameters; going past the end is an error. They also work inside
   `$(( … ))`.
4. **`return [N]`** leaves the function with status `N` (taken modulo 256; the status of
   the last command when omitted), through loops and `if`. Anywhere else it is an
   error. The status of a function without `return` is the one of its last command,
   and a program's exact exit code is kept.
5. **`local NAME[=value]…`** makes shell variables that end with the function: the
   value they had before comes back, in reverse order of declaration. Scope is dynamic: a
   function called by another sees its locals. `local` outside a function is an error. A
   variable that was exported keeps being the exported one for programs.
6. **`source FILE ARGS…`** gives the arguments to the file as `$1`… for its duration,
   which are then dropped; `return` in a sourced file is an error.
7. **Limits**: at most 64 calls open at once (`name: maximum function nesting depth (64)
   exceeded`, status 1) and command substitutions nested at most 24 deep, so that a
   function that calls itself without end stops with an error and not a crash. A `return`
   inside `$( … )` ends the substitution, not the function.
8. `$(( … ))` now runs the substitutions inside the expression first, which the
   recursive idiom `echo $(( $1 * $(fact $(( $1 - 1 )) ) ))` needs.
9. **Not included**: `declare`, `export -f`, `local -r` and other options, arrays,
   `$0` as the function name, `FUNCNAME`, `${@:2}` slices, `$$`, `$!`, functions in a
   background job, `trap`, and redirections on a whole function or group
   (`f() { … } > file`, `} | cat`).

## Options considered

- Stay with aliases: no parameter, so no `mkcd() { mkdir -p "$1" && cd "$1"; }`.
- Static scope for `local`: closer to other languages, but not what shell scripts expect.
- `"$@"` as a single joined word: simple, and wrong for every argument with a space.

## Consequences

### Positive

- Reusable helpers (`mkcd`, `gco`, wrappers around a program) in the shell and in
  scripts; the recursion idiom works.
- No new `unsafe` code: a call is a frame on a stack held by the shell state.

### Negative

- A function replaces a built-in silently (`ls() { … }`); `unset -f` brings it back.
- Parameters are not exported to programs, and functions are not either.
- A deep recursion through substitutions stops at 24 levels, earlier than in Bash.

## Validation

- Unit tests of the grammar (all header forms, groups, nesting, malformed input,
  unfinished input), of the word parser (`$1`, `${1:-x}`, `$#`, `"$@"` with and without
  parameters, `$*`) and of the arithmetic evaluator with `$1` and `$#`.
- Integration tests with the real binary: arguments and status, pipelines and
  redirections, locals, recursion and its limit, `shift`, `return` through loops,
  multi-line definitions, the exact exit code of a program, `source` with arguments.
- A random test that now also types `f() {`, `}`, `return`, `$1`, `"$@"` and `local`.
- 20 mutation checks (parameter numbering, count, locals not restored, `return` not
  stopping lists and loops, limit removed, dispatch order, `$@` joining, `"$@"` empty,
  `source` frame, unfinished-header detection, exit codes, substitution in arithmetic,
  keyword names, `local` outside a function, restore order) make tests fail.
