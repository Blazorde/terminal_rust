# ADR-0032 — `pushd`, `popd`, `dirs`, `time` and `$'…'`

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

Trying ordinary usage showed three more gaps: jumping between a few directories (`pushd`, `popd`, `dirs`), timing a
command (`time`), and writing a tab or a newline in an argument (`$'a\tb'`).

## Decision

1. **Directory stack.** `pushd dir` goes to `dir` and puts the directory it left at the top of the stack; `pushd` alone
   swaps the current directory with the top; `popd` goes to the top and removes it; each prints the stack as `dirs`
   does: the current directory, then the others, most recent first, with the home directory written `~`. An empty
   stack is an error (`popd: directory stack empty`, `pushd: no other directory`) and a `cd` that fails leaves the
   stack as it was. `cd -` and `$PWD` follow. A subshell gets the stack back as it was (ADR-0027).
2. **`time command args…`** runs the command in the shell and prints `real<TAB>XmY.YYYs` on standard error after it,
   with the status of the command. Only the elapsed time is shown, not the user and system times, which need a
   system call that the safe crates the shell uses do not offer. `time` alone shows zero. It times one command,
   not a pipeline.
3. **`$'text'`** is a quoted word where the escapes are read: `\n \t \r \a \b \e \f \v \\ \"`, octal (`\101`),
   `\xHH`, `\uHHHH` and `\UHHHHHHHH`. An unknown escape is kept as typed. A `\'` cannot be written inside, because the
   line splitters read the quote as the end of the string. Inside double quotes `$'` is plain text.

## Consequences

- The stack is not shared with `cd`: `cd` does not push anything.
- `time` has no `-p` option and no `TIMEFORMAT`.
