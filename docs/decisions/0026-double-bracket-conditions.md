# ADR-0026 — `[[ expression ]]` conditions

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

`[ … ]` (ADR-0016) splits its words and takes `-a`, `-o` and `=` as plain arguments, which is fragile
with variables and cannot match a pattern or a regular expression. Scripts written for other
shells use `[[ $f == *.rs && -f $f ]]` all the time, and in this shell the `&&`, `<` and `|` inside it
were read as chaining and redirection.

## Decision

1. A command that starts with the word `[[` and ends with `]]` is a conditional. The line splitter and the
   pipeline splitter leave `&&`, `||`, `<`, `>` and `|` inside it alone; the closing `]]` ends it, so
   `[[ a ]] && b` still chains. `[[` elsewhere in a line is a plain word.
2. The words are read raw and expanded one by one like the word of a `case`: no brace expansion,
   no word splitting, no file name matching (`[[ $x == a* ]]` with `x="a b"` is true and does not look at files).
3. Operators: `( )`, `!`, `&&`, `||` (short-circuit: the side that is skipped is not evaluated, so
   a `$(…)` there does not run); `-e -f -d -L -h -r -w -x -s -z -n -t -b -c` and `-eq -ne -lt -le -gt -ge`
   through the `test` implementation; `==`, `=` and `!=` against a pattern (what was typed as a wildcard
   matches, a quoted part stands for itself); `<` and `>` compare strings; `=~` matches a regular
   expression with the `regex` crate (a match only: no `BASH_REMATCH`). A lone word is true
   when it is not empty.
4. A true condition has status 0, a false one 1, a mistake (missing `]]` or `)`, bad integer,
   bad regular expression) status 2 with a message, as for `test`.

## Consequences

- New dependency: `regex` (MIT or Apache-2.0, already in the tree through other crates).
- Not included: `-a`/`-o` inside `[[`, `BASH_REMATCH`, `-nt`, `-ot` and `-ef`, and `[[` in the middle of a pipeline.
