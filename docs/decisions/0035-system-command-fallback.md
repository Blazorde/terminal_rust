# ADR-0035: Hand a built-in over to the system program

- Status: Accepted
- Date: 2026-11-25
- Decision makers: Terminal Rust team

## Context

The built-ins `ls`, `cat`, `mkdir`, `rm` and a few others handle a small set of options. A user who wants
the terminal as a daily shell types `ls -l`, `cat -n` or `date -u`. The built-ins refused some of these,
ignored others, and `mkdir` and `touch` could read `-x` as the name of a file to create. The built-ins stay:
they make the shell usable on Windows, where most of these programs do not exist.

## Decision

1. One table, `commands::system_fallback`, declares what each built-in handles: short and long options,
   operands, and whether it reads its input. The covered built-ins are `ls`, `cat`, `mkdir`, `touch`, `date`,
   `printenv`, `whoami`, `arch`, `nproc`, `uptime`, `hostname`, `clear` and `rm`.
2. When the arguments go beyond the table, the system program of the same name receives the whole command
   with the original arguments. On Unix its first argument is the name of the built-in.
3. The program is searched in the absolute entries of `PATH` only. A relative entry never provides it.
4. Without a program, an option the built-in does not handle is an error (`mkdir: unknown option: -m`). Extra
   operands without an option stay with the built-in. `--` ends the options.
5. `ls` with `l` in a short group goes to the system program even though the built-in accepts it, because
   the built-in long format has no permissions, owner or date. Without a program the built-in answers.
6. Functions and aliases keep their priority. An alias of a built-in and `command name` reach the same rule.
7. The system program is started like any other program: redirections, pipelines, command substitution,
   exit status, `set -e`, jobs and `Ctrl+C` follow. A system `cat` receives the input the shell holds.
8. `rm` is delegated under two conditions. Only listed options are handed over: `-I`, `--one-file-system`,
   `--preserve-root`, `--preserve-root=all`, `--interactive=never`, `--interactive=once`,
   `--interactive=always`, `--help`, `--version`, along with the options of the built-in. Anything else,
   such as `--no-preserve-root` or an abbreviated long option, is an error. Before the program starts, the
   shell applies the protections of the built-in to every operand: `.`, `..` and the root directory are
   refused, and it asks before a recursive removal without `-f`, or for each operand with `-i`.

## Options considered

- Remove the built-ins and always run system programs: Windows would lose basic file commands.
- Add the missing options to the built-ins: a long reimplementation that would still lag behind.
- Never delegate `rm`: safest, but `rm -I` and `rm --one-file-system` would stay unavailable.
- Delegate any `rm` option behind the guard: an unknown or abbreviated option could lift a protection.

## Consequences

### Positive

- The usual options work wherever the system program exists.
- An unknown option is no longer read as a file name.

### Negative

- Output and messages of a delegated command depend on the system (GNU, BSD or none).
- A system `rm` may ask again after the shell has asked.
- The question of the shell before a delegated `rm` is answered on the input of the shell, not through a pipe
  or a here-string given to `rm`.
- A new option of a built-in must be added to the table as well, or it is handed over.

## Validation

- Unit tests of the decision table and of the `rm` guard.
- `tests/system_fallback_tests.rs` and `tests/captured_program_tests.rs`, with stand-in programs as the whole
  `PATH`: arguments, redirections, pipelines, substitution, status, relative `PATH` entries, priority of
  functions, input of blocks, and the protections of `rm`.
