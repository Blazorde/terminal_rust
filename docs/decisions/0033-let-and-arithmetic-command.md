# ADR-0033: let and the (( )) arithmetic command

- Status: Accepted
- Date: 2026-11-23
- Decision makers: Terminal Rust team

## Context

[ADR-0017](0017-arithmetic-expansion.md) left assignment out of arithmetic, so a counter had to be written
`i=$((i+1))`. Scripts and interactive habits use `(( i++ ))`, `(( n < 3 ))` as a loop condition and
`let a=b*2`. None of them worked (`unknown command: ((`, `unknown command: let`).

## Decision

1. **`(( expression ))`** is evaluated like `$(( ))` (ADR-0017, 64-bit integers) but may assign. The status is `0` when
   the value is not `0` and `1` when it is `0`; the value of an empty `(( ))` is `0`, so its status is `1`. Any arithmetic error (division by `0`, a
   missing operand, an invalid integer, an overflow) gives status `1` and a message on standard error that starts
   with `((:`. Command substitutions, backquotes and `$NAME` in the body are expanded first, double quotes are
   ignored, and there is no brace expansion or word splitting.
2. **`let arg…`** takes each argument as one statement. An argument is an ordinary word, so `let "x = 3"` needs the
   quotes, and so does a `*`, `<` or `>`. The value of the last statement gives the status as above. `let` alone prints
   `let: expression expected` and gives `1`. An error stops the remaining arguments and starts with `let:`.
3. **Statements** are separated by commas (outside parentheses). Each one is an assignment `name = expr`, a compound
   assignment `name op= expr` (`+= -= *= /= %=`), an increment `name++`, `++name`, `name--` or `--name` (the old value
   for the suffix, the new one for the prefix), or a plain expression. The right side of an assignment may be another
   assignment (`a = b = 3`). An unset or empty variable is `0`; a value that is not an integer is an error that names
   the variable. A `++` or `--` glued to a name inside a larger expression (`i++ < 3`, `x = i-- - 1`) is refused with
   `arithmetic: ++ and -- must stand alone as a statement`, where bash accepts it, because it could also be read as
   two signs. `$(( ))` is unchanged: `$((--x))` is still a double negation. A compound assignment is accepted only
   as a name followed by exactly one of `+ - * / %` glued to `=`; `**=`, `+-=`, `*-=`, `%*=` and `++=` are syntax
   errors (`arithmetic: unexpected '='`, or `'*'` for `%*=`) with status `1`, and the variable keeps its value.
   A chain of assignments (`a = b = c = …`) is limited to 64 levels, like parentheses; beyond that the error is
   `arithmetic: expression nested too deeply`, the status is `1` and no variable changes.
4. **Recognition.** `((` opens an arithmetic command only where a command starts (start of line, after `;`, `&&`,
   `||`, `|`, `!`, `if`, `then`, `elif`, `else`, `do`, `while`, `until`, `{` or `(`), and only if the first `)` at depth
   zero after it (quotes and escapes skipped) is directly followed by another `)`. Otherwise nested subshells stay as
   they were: `((echo a); echo b)` prints `a` then `b`. As in bash, `(((echo a)))` is now an arithmetic expression, and
   an error. The `<`, `>`, `|` and `&` of the body are not shell operators. Text after the closing parentheses
   (`(( 1 )) foo`) makes it an ordinary command, so the shell reports `unknown command: ((`, where bash
   reports a syntax error.
5. **Redirections, pipes and `&`.** A `(( ))` followed by a redirection or `&`, or inside a pipe, is rewritten as
   `let '<body>'` and goes through the pipeline code, so its error message starts with `let:` instead of `((:`. Like
   the other built-in commands it runs in the current shell: `(( x = 5 )) | cat` and `(( x = 5 )) &` change `x`,
   where bash uses a subshell.
6. **Failing status in a sourced script.** The existing rule applies: a script run by `source` stops as soon as a
   command at its top level gives a non-zero status, `false` included. A `(( n++ ))` alone on a line with `n` at `0`
   has the value `0` (the old one), so its status is `1` and the script stops, although `n` is incremented. The shell
   prints `source: post.sh:2: ` (file, failing line and an empty message) on standard error and `source` gives `1`.
   `(( ++n ))` (value `1`) and `n=$((n+1))` (status `0`) do not stop it.
7. **Not included**: the bit operators (`& | ^ ~ << >>`), the ternary `?:`, arrays in an expression (`a[1]`), a comma
   inside parentheses, bases other than decimal and `0x` (`010` is 10, bash reads 8; `2#101` is refused),
   `${y:-3}` inside `(( ))` (`let "x = ${y:-3}"` works because the word is expanded first), the effect of `set -u`
   on the expression, `for (( i=0; i<3; i++ ))`, `time (( ))` and `f() (( … ))`. An overflow is an error that leaves the
   variable as it was, where bash wraps around. The error of a `(( ))` used as the condition of `if` or `while` is not
   printed, like for every condition of those. A line with `<<` in the body (`(( 1 << 2 ))`) is taken for a
   here-document by the here-document reader, which then swallows the lines that follow; the expression would be
   refused anyway. `**` already exists ([ADR-0028](0028-exit-trap-and-power.md)). The literal `-9223372036854775808`
   (the smallest 64-bit integer) is refused with `invalid number '9223372036854775808'`, because the evaluator of
   [ADR-0017](0017-arithmetic-expansion.md) reads the number before applying the sign; bash accepts it.

## Options considered

- Only the `let` command, with `(( ))` rewritten into it: simpler, but a `(( ))` could not stand in the condition of
  an `if` or a `while`.
- Evaluating `++` and `--` inside the evaluator: it would need an evaluator that changes variables, which ADR-0017
  avoided. Only whole statements are handled, outside the evaluator.
- Keeping status `2` for errors, as `[[ ]]` does: bash gives `1` for `(( 1/0 ))`, so `1` is kept.

## Consequences

### Positive

- Counters and conditions are written the usual way (`while (( n < 3 )); do …; (( n++ )); done`) and `let` scripts run.
- The evaluator of `$(( ))` is shared and stays free of side effects; only the statement layer writes variables.
- Nested subshells written with `((` keep working.

### Negative

- `i++ < 3` is refused where bash accepts it, and the bit operators and the ternary are missing.
- `(( ))` is not isolated in a subshell when piped or put in the background.
- `(( n++ ))` alone on a line stops a sourced script while `n` is `0`, and a chain of more than 64 assignments is refused,
  where bash has no such limit.
- `(((cmd)))` no longer runs as three subshells.

## Validation

- Integration tests in `tests/cli_tests.rs`: `let_assigns_calculates_and_reports_the_last_value`,
  `let_reports_errors_and_stops_at_the_first_failing_argument`,
  `an_arithmetic_command_succeeds_when_its_value_is_not_zero`, `compound_assignments_and_the_comma_update_variables`,
  `increments_as_a_statement_give_the_old_or_the_new_value`,
  `arithmetic_errors_name_the_problem_and_leave_the_variables_alone`,
  `the_body_of_an_arithmetic_command_is_expanded_first`, `an_arithmetic_command_follows_and_or_connectors`,
  `an_arithmetic_command_works_in_conditions_loops_functions_and_blocks`,
  `set_e_stops_the_shell_on_a_failing_arithmetic_command_only_where_it_should`,
  `an_arithmetic_command_takes_redirections`, `an_arithmetic_command_runs_in_pipelines_and_in_the_background`,
  `nested_subshells_and_quoted_text_are_not_arithmetic`,
  `arithmetic_errors_do_not_print_escape_sequences_from_variables`,
  `text_after_the_closing_parentheses_makes_it_an_ordinary_command`,
  `a_comma_after_a_closing_parenthesis_still_separates_statements`,
  `a_doubled_sign_before_a_number_is_not_an_increment`,
  `an_increment_glued_to_an_underscore_name_is_refused_inside_an_expression`,
  `an_escaped_double_quote_in_a_string_does_not_end_the_command`,
  `compound_division_and_modulo_by_zero_name_the_error_and_keep_the_variable`,
  `a_pipeline_with_an_arithmetic_command_in_the_background_is_a_job` and
  `a_post_increment_alone_in_a_sourced_script_stops_it_when_the_value_is_zero`.
- Unit tests of the recognition in `src/app/parser/arithmetic_word.rs` (nested subshells, quotes and escapes, a
  comparison or a pipe in the body, redirections and `&` after the command, positions where a command starts) and of the
  refusal of embedded increments in `src/app/command/arithmetic.rs`.
- Mutation testing of the new code: a first round left mutations alive that showed missing tests (text after the
  closing parentheses, a comma after a closing parenthesis, `--5` and `++3`, `_` next to `++`, an escaped double quote
  in a string, a `((` that does not start a stage, a background pipeline job, and the division messages). Those tests
  were added and a second round killed the mutations. Three guards that mutation showed to be redundant (one
  comparison test and two conditions of the `((` branch) were removed.
- Checked by hand in a real terminal (PTY): a `while (( n < 3 ))` loop with `(( n++ ))`, a failing `(( 1/0 ))`
  and `let`.
