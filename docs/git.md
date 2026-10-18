# Git commands

Git is optional. The prompt shows branch and working-tree information when Git
is installed and the current directory belongs to a repository. The shell's
built-in Git helpers run the installed Git executable from the current
directory.

Run `git help` inside Terminal Rust for a concise overview. The supported
read-only commands are:

| Command | Purpose |
| --- | --- |
| `git status [options]` | Inspect working-tree status. |
| `git branch [options]` | List branches. Branch creation, deletion and renaming are refused. |
| `git log [options]` | Inspect commit history. Writing output to a file is refused. |

Examples:

```text
git status --short
git branch --all
git log -5 --oneline
```

Other Git subcommands, including `push`, `reset` and `clean`, are unavailable
through the built-in helper. Use a regular system terminal for Git operations
outside this read-only allowlist. The `git help` output is built into Terminal
Rust and remains available when Git itself is not installed.

## Platform support

The same read-only helpers are supported on Windows, macOS and Linux when Git
is available on `PATH`. Terminal Rust invokes Git directly with separate
arguments and passes the current directory through `git -C`; it does not build
a platform-specific shell command. The integration suite exercises status,
branch and log from a temporary repository whose path contains spaces and
Unicode. CI runs these checks on all three operating systems. If Git is absent,
the prompt simply omits Git context and explicit Git commands report that the
executable is unavailable.
