# Custom commands

Custom commands create portable shortcuts without changing the binary. Each
command is a structured TOML table; Terminal Rust never passes its value to
`sh -c`, `cmd /C` or PowerShell.

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
