# ADR-0029 — Indexed arrays

- Status: Accepted
- Date: 2026-11-22
- Decision makers: Terminal Rust team

## Context

Lists of words are everywhere in scripts: `files=(*.rs)`, `for f in "${files[@]}"`, `${#args[@]}`. Without
arrays a script has to pack words into one string and split it again, which is what ADR-0009 avoids.

## Decision

1. **Assignment**: `name=(a b c)` sets an array; the words are expanded like the arguments of a command (braces,
   `$(…)`, variables, file name patterns), so `files=(*.rs)` and `n=({1..3})` work and `name=()` is empty.
   `name+=(d e)` appends, `name[2]=value` sets one element (the array grows with empty elements), and an index may
   be a number or a variable (`name[i]`, `name[$i]`). An array replaces a plain variable of the same name,
   and `unset name` removes it.
2. **Expansion**: `${name[1]}` (a negative index counts from the end), `${name[@]}`, `${name[*]}`, `${#name[@]}` (the
   number of elements), `${!name[@]}` (the indices) and `${name[1]:-default}`. `$name` is the first element.
   `"${name[@]}"` gives one word per element, none for an empty array, exactly like `"$@"`; `"${name[*]}"` joins with
   spaces. An element is never split again, quoted or not (ADR-0009, `${=…}` still splits on request).
3. A subshell gets a copy of the arrays and gives them back (ADR-0027). A `(` inside a command of a subshell
   (`( x=(1 2); … )`) is a plain parenthesis, closed by its own `)`.
4. **Not included**: associative arrays (`declare -A`), `declare -a`, `local -a`, slices (`${name[@]:1:2}`), `read -a`,
   exporting arrays, `unset name[1]`, `${name[@]/x/y}`, and arrays in the prefix of a command (`x=(1 2) cmd`).

## Consequences

- The variable lookup of the parser gets synthetic names (`name[1]`, `#name[@]`) that the shell state resolves, so the
  parser needs no knowledge of arrays beyond the syntax of the references.
- Indices are not shown by `set`, `env` or `export`.
