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
Desktop release packages include Git 2.56.0 and the runtime libraries needed by
that build. The first terminal session extracts and relocates Git into the
application's writable data directory; subsequent sessions reuse it. Git
settings in the user's home directory continue to apply. Local development and
the standalone CLI keep using Git from `PATH` unless
`TERMINAL_RUST_GIT` points to a specific executable. If a bundled runtime is
missing or cannot be prepared, the desktop session reports the setup error.

The release workflow builds the runtime from the pinned conda-forge Git package
for each operating system and includes the runtime package inventory and
license texts in the desktop resources. Update `.github/git-runtime.yml` to
change the bundled Git version.
