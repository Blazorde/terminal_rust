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
Desktop release packages include Git 2.56.0, Neovim 0.12.5, Nano 9.2 and their
runtime dependencies. The first terminal session extracts and relocates these
tools into the application's writable data directory; subsequent sessions
reuse them. The desktop shell can use `git`, `nano`, `nvim` and `neovim` without
the user installing those programs separately. Git configuration, SSH keys,
credential-manager state and editor preferences in the user's home directory
continue to apply. Git's executable search path includes the bundled tools, so
`git config --global core.editor nano` and `git config --global core.editor nvim`
work with commits that open an editor. Nano is the runtime's system-level
default only when the user has not configured a different editor.

Local development and the standalone CLI use the tools in `PATH` unless
`TERMINAL_RUST_GIT`, `TERMINAL_RUST_NANO` or `TERMINAL_RUST_NVIM` points to a
specific executable. If bundled resources are missing or cannot be prepared,
the desktop session reports the setup error.

The release workflow builds the runtime from pinned conda-forge packages for
each operating system. It also includes Nano's matching source archive, the
Windows Nano executable, a package inventory and the license texts in desktop
resources. Update `.github/git-runtime.yml` and
`.github/actions/prepare-git-runtime/action.yml` together when changing tool
versions.
