# Custom commands

Custom commands create portable shortcuts without changing the binary. Each
command is a structured TOML table; Terminal Rust never passes its value to
`sh -c`, `cmd /C` or PowerShell.

Run `commands help` in the terminal for a short overview and `commands list` to
inspect the shortcuts currently loaded from your user configuration.

## Built-in alias

```toml
[commands.git-summary]
kind = "builtin"
target = "git"
args = ["status", "--short", "--branch"]
append_args = false
description = "Show a concise Git summary"
```

`target` must name an existing built-in. Native commands always win: defining
`help`, `git` or any other native name disables all custom commands and reports
a diagnostic.

## External process

```toml
[commands.serve]
kind = "process"
target = "npm"
args = ["run", "dev"]
append_args = true
description = "Start the project development server"
```

The program starts directly through `std::process::Command`; every `args` item
remains separate. Pipes, redirects, command substitution and shell variables
are never interpreted. The child still inherits the working directory and
environment, so configured processes are trusted user code and may read secrets
available to Terminal Rust.

When `append_args` is true, arguments entered after the alias are appended.
When false, extra arguments produce an error. Standard output, standard error
and the process exit code are preserved.

## Discovery and errors

`commands list` shows configured commands, types, targets and descriptions.
They are also included in interactive completion.

Names contain 1–64 lowercase ASCII letters, digits, hyphens or underscores and
must begin and end with a letter or digit. Loading rejects:

- conflicts with native commands;
- unknown built-in targets;
- empty targets;
- NUL bytes in targets or arguments;
- control characters in descriptions.

Invalid configuration does not block the shell: custom commands are disabled,
the diagnostic is shown, and native commands remain available. External tools
are inherently platform-dependent; document prerequisites and avoid hard-coded
path separators.

## Interactive aliases

For shortcuts that do not need a configuration edit, use:

```text
alias ll ls -la
alias --save gs git status --short --branch
alias
unalias ll
unalias --save gs
```

An alias without `--save` lasts for the current shell session. `--save` writes
the alias to `shell-state.toml` beside the active user configuration and makes
it available in later CLI and desktop sessions. `unalias` hides a saved alias
for the current session; `unalias --save` removes it from persistent storage.
Native commands cannot be shadowed. Aliases target a built-in command or start
an external executable directly, with arguments passed as separate values.

## `.trs` command scripts

Scripts contain one Terminal Rust command per line; they do not run through an
operating-system shell:

```text
script create setup-project
script run scripts/setup-project.trs
script list
script startup scripts/setup-project.trs
script startup off
```

Edit the created `.trs` file with any text editor. Scripts stop at the first
failed command, support a trailing PowerShell-style backtick for line
continuation, and can run another script up to a bounded nesting depth. Startup
scripts run before the first prompt. Script paths are resolved from the current
working directory when invoked; selecting a startup script stores its absolute
path.

Use `Shift+Enter` in the desktop terminal to add a continuation line. In the
interactive CLI, end a line with a backtick and press Enter.

## Fastfetch

`fastfetch` and its short form `ff` invoke the separately installed Fastfetch
executable. Fastfetch is optional and is never installed automatically. Ensure
it is available on `PATH`; otherwise the command reports an actionable
installation diagnostic. The desktop **Réglages → Démarrage** option can run
it before each session prompt.
