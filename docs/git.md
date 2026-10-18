# Git commands

Terminal Rust forwards every `git` command and argument to the native Git
executable without a subcommand or option allowlist. This includes commands
that modify repositories or contact remotes. Review the command before running
it, just as you would in another terminal.

Git receives the terminal's standard input, output and error streams. This
allows native Git pagers, editors, SSH authentication, credential helpers and
interactive prompts to work through Terminal Rust's PTY.

```text
git status --short
git switch -c feature/example
git add -- "file with spaces.txt"
git commit
git push --force-with-lease
git help rebase
```

The Git prompt segment is optional. It displays branch and working-tree
information when Git is available and the current directory is a repository.

## Platform support

Terminal Rust invokes Git directly with separate arguments and sets the child
process working directory, avoiding platform-specific shell quoting. The
integration suite verifies native Git commands on Windows, macOS and Linux.
Git must be available to the application; when it cannot be started, the shell
reports the operating-system error and the optional prompt segment is omitted.
