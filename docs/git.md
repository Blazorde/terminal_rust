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
