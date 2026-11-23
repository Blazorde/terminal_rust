# ADR-0017 — Arithmetic expansion

- Status: Accepted. Item 6 is partly replaced by [ADR-0033](0033-let-and-arithmetic-command.md): assignment, the comma, `let` and `((...))`.
- Date: 2026-11-21
- Decision makers: Terminal Rust team

## Context

Loops ([ADR-0016](0016-control-flow.md)) need counters: `i=$((i+1))`,
`echo $((total / 3))`. Until now `$((...))` was rejected with an explicit error,
because reading it as a command substitution would run `(...)` in a subshell.

## Decision

1. `$((expression))` is replaced, before the line is parsed, by the value of the
   expression written in decimal. It works unquoted and inside double quotes, not
   inside single quotes, and nests with the other expansions through the result
   (`echo $(( $(echo 2) ))` is not supported: see 5).
2. Numbers are 64-bit signed integers: decimal or `0x` hexadecimal.
3. Operators, from the loosest binding: `||`, `&&`, `==` `!=`, `<` `<=` `>` `>=`,
   `+` `-`, `*` `/` `%`, then the prefix `-`, `+` and `!`, and parentheses. The
   comparisons and the logical operators give `1` or `0`; `/` truncates toward
   zero and `%` takes the sign of the left operand.
4. A variable is written `NAME`, `$NAME` or `${NAME}`, and `$?` is the last status.
   An unset or empty variable is `0`; a value that is not an integer is an error
   that names the variable.
5. Every operation is checked: an overflow, a division or remainder by `0`, an
   unknown character, a missing operand or parenthesis is an error, the command
   does not run, and the status is `1`. Parentheses and prefix operators nest at
   most 64 deep. A `$(...)` command substitution inside an expression is refused.
6. **Not included**: assignment (`=`, `+=`, `++`), `**`, bit operators (`& | ^ ~ << >>`),
   the ternary `?:`, the comma, other bases (`2#101`, octal), `let` and `((...))`.
   Write `i=$((i+1))` as an ordinary assignment.

## Options considered

- Run `expr` or `bc`: absent on Windows and a process for every sum.
- Floating point: surprising for counters and for `%`, and not what other shells do.
- Allow assignments inside the expression: it needs the expansion to change the
  shell state, which `${NAME:=word}` also avoids ([ADR-0009](0009-shell-variables.md)).

## Consequences

### Positive

- Counters, ranges and simple maths work in loops and scripts, on every system,
  without starting a process.
- An evaluator with checked operations and a depth limit cannot panic or exhaust
  the stack on a hostile expression.

### Negative

- No assignment or bit operators, so some shell scripts need a rewrite.
- A value such as `NAME=1+1` is not evaluated again as an expression.

## Validation

- Unit tests for each operator, precedence, unary signs, variables, hexadecimal,
  errors, overflow at the limits of `i64`, and deep nesting; a random test over
  200 000 expressions that must never panic.
- Integration tests with real lines: sums in arguments, assignments, loops with a
  counter, quoting, a failing expansion that prevents the command from running.
- Mutation checks: wrapping instead of checking, `&&` as `||`, `<` before `<=`, the
  depth guard removed, and a lone `)` closing the expression make tests fail.
