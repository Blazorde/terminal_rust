# ADR-0008 — Run external programs from the shell

- Status: Accepted
- Date: 2026-11-20
- Decision makers: Terminal Rust team

## Context

Terminal Rust only runs built-in commands, Git, the bundled editors and custom
commands declared in the user configuration. Any other command answers
`unknown command`, so tools such as `cargo`, `python`, `htop` or `ssh` cannot be
used. A terminal that cannot start programs cannot replace the user's default
terminal, which is the goal of the project.

[ADR-0007](0007-full-native-git-and-editors.md) already removed the Git allowlist
and runs the native executable with inherited terminal streams. Running other
programs applies the same rule to every executable.

## Decision

When a command name is not a built-in, a configured custom command or a plugin
command, the shell starts the program of that name:

1. Resolution order is fixed: built-in commands, custom commands, declarative
   plugin commands, then external programs. A built-in always wins.
2. The program is found through `PATH`. A name that contains a path separator is
   used as given. On Windows, `PATHEXT` is honored so `npm` finds `npm.cmd`; the
   current directory is never searched implicitly.
3. Arguments are passed as separate process arguments. The shell does not hand
   the line to another shell, so no extra interpretation happens.
4. The child inherits the terminal streams, the environment and the current
   directory of the shell.
5. A program that cannot be found reports `unknown command: <name>`. A program
   that starts and fails reports its exit status and makes the shell status
   failed.
6. While the interactive shell runs, `Ctrl+C` no longer ends the shell: the
   interrupt reaches the foreground program, and the shell prompt returns.

Declarative plugins keep their boundary: they cannot start programs.

## Options considered

- Keep the current rule, with programs declared one by one in the user
  configuration: safest, but unusable as a daily terminal.
- Run the line through the system shell (`sh -c`, `cmd /C`): supports pipes and
  variables at once, but depends on the platform shell, changes quoting rules and
  adds an injection surface. Rejected.
- Start the program directly with the standard library (chosen): same behavior on
  every platform, no hidden interpretation, and pipes, redirections and variables
  can be added later inside the shell itself.

## Consequences

### Positive

- Any installed tool works, with its own terminal handling (editors, pagers,
  interactive programs).
- Behavior is identical in the CLI and in the desktop app, which share the engine.

### Negative

- The user can run any program with the privileges of the shell, as in every
  other terminal. The shell adds no sandbox.
- A built-in shadows a program of the same name (`ls`, `cat`, `echo`, `date`).
- No job control: there is no background execution, `Ctrl+Z` suspension or
  process groups yet.
- No pipes, redirections or variable expansion yet; they need their own decision.
- A new dependency, `ctrlc`, handles the interrupt on Unix and Windows.

## Validation

- Unit tests for the `PATH` and `PATHEXT` search with a fake file system, for the
  exit status and for a missing program.
- An integration test starts the compiled shell with a real external program and
  checks the output and the exit code.
- A pseudo-terminal check shows that `Ctrl+C` stops a running program and that
  the shell survives.
