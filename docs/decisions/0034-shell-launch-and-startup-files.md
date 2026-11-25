# ADR-0034: How the shell is launched and which files it reads

- Status: Accepted
- Date: 2026-11-25
- Decision makers: Terminal Rust team

## Context

The binary knew two forms: no argument (a prompt) and anything else (one command made of the arguments).
Programs that start a shell expect more: `-c 'string'`, a script file (also through a `#!` line), piped
input without a prompt, `-i`, and a login shell. There was no interactive startup file either. Login
profiles (`/etc/profile`, `~/.profile`) are written for a POSIX shell and use syntax this shell does not
read (`if [ -d … ]; then`, `.`, `case` forms, parameter forms).

## Decision

### Arguments

1. Options come first: `-c`, `-i`, `-l`, alone or grouped (`-lc`), `--login`, and `--` which ends them. A
   `argv[0]` that starts with `-` also means a login shell. An unknown option is not an option: it is the
   first operand, as before. There is no `--help` or `--version`.
2. With `-c`, the first operand is the command string, the next one is `$0` and the others are `$1`, `$2`, …
3. Without `-c`, the first operand decides, in this order:
   1. an existing file is a script, even when a command has the same name;
   2. otherwise, an operand that contains `/` (or `\` on Windows) or ends with `.sh` or `.trs` is a script
      that cannot be run;
   3. otherwise the operands are one command, as before (`terminal_rust ls`).
4. A missing script gives status `127`. A directory, a file that cannot be read, a file that is not text
   or one larger than 1 MiB gives `126`. A usage error gives `2`.

### Running without a prompt

5. `-c`, scripts and standard input that is not a terminal run one complete command at a time. A command
   that fails does not stop the next ones; `exit`, `set -e` and a syntax error do (status `2` for the
   latter). The status is that of the last command. Nothing prints a prompt, and no startup file is read.
   `-i` keeps the prompt on piped input. `source` and `script run` still stop at the first failure.

### Startup files

6. Every interactive shell reads `~/.terminal_rust_rc` in the current shell, login shells and the desktop
   app included; `-i` also reads it before `-c`, a script or a one-line command. A missing file is silent.
   An error names the file and the line and the file goes on; a syntax error stops reading it; `exit` ends
   the shell. It runs before the script set by `script startup`. A relative home directory is ignored, so
   that no file of the current directory is run.
7. A login shell on Unix does not interpret `/etc/profile` and `~/.profile` itself. `/bin/sh` runs them
   (`~/.profile` only under an absolute home directory), then prints its environment with `/usr/bin/env -0`
   between two markers, and the shell imports the variables that are new or changed. Windows ignores `-l`.

The script given to `/bin/sh` is a constant: no outside text is added to it. What the profiles print goes
to standard error, and the environment travels on a separate descriptor that they do not get. Reading
stops at the end marker, so a program left running by a profile cannot hang the shell. Traps send the
environment even after `exit`, an error or `Ctrl+C`. `TSTP` is ignored while the profiles run, because a
stopped `sh` would never be resumed. `sh` stays in the process group of the shell, and its output has to
be read while it runs, so `job_control::run_foreground` is not used. When nothing usable comes back, one
warning is shown and the environment stays as it was.

## Options considered

- Run the profiles in this shell: no second process, aliases and functions would be kept. A profile of the
  distribution or of a package can use any POSIX construct, so this fails on files the user did not write
  and may leave the environment half set. Rejected.
- Let `/bin/sh` run them and import the environment: the profiles are read by the language they are written
  for. Only exported variables survive. Selected.
- `exec /bin/sh -l` into this shell: nothing to parse, but `sh` does not hand over cleanly on every system
  and a profile that fails would take the session with it. Rejected.
- For operands, require a path (`./name`) before treating a file as a script: `terminal_rust deploy.sh`
  and `terminal_rust build` would not work as with other shells. Rejected.

## Consequences

### Positive

- Programs and `#!/usr/bin/env terminal_rust` scripts can use the shell as they use `bash`.
- A broken profile or rc file never prevents reaching the prompt.

### Negative

- Piped input no longer prints a prompt (`-i` restores it).
- A file in the current directory named like a command wins over the command. In a directory someone else
  filled (a cloned repository), `terminal_rust ls` or `terminal_rust git status` runs that file.
- A path to a program (`terminal_rust /usr/bin/ls`) is read as a script and gives `126`.
- From the profiles, `umask`, `ulimit`, `cd`, aliases, functions, plain variables and removed variables are
  not taken. `~/.bash_profile` and `~/.bash_login` are not read.
- `HOME`, `XDG_CONFIG_HOME`, `TERMINAL_RUST_CONFIG`, `TERM` or `NO_COLOR` exported by a profile or the rc
  file reach the programs, but the settings of the shell itself still come from its process environment.
- Programs started by a profile inherit an ignored `TSTP`.
- In a script, a command cannot go on after a trailing `&&`, `||`, `|` or `\` (the continuation character is
  the backtick), and a quoted string cannot span lines. `Ctrl+C` ends a script without running the `EXIT` trap.
- A script has to be a regular file: `/dev/stdin` and process substitutions give `126`.
- When standard input is not a terminal, lines are read one byte at a time, so that `read` and the programs
  that follow get the rest of the input.

## Validation

Unit tests cover the argument rules and the reading of the environment. `tests/launch_tests.rs` and
`tests/startup_tests.rs` run the binary with a temporary home. Login behavior, the rc file, `Ctrl+C` during
a profile and job control after a login start were checked in a real pseudo-terminal on Linux.
