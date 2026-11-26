# ADR-0037: Git completion of remotes, stash entries and `git -C`

- Status: Accepted
- Date: 2026-11-26
- Decision makers: Terminal Rust team

Completes [ADR-0019](0019-git-completion.md) and
[ADR-0036](0036-smarter-completion.md). It replaces these parts:

- remote names, stash entries and `git -C dir`, left out by point 6 of ADR-0019,
  and remotes and stash entries, left out by point 8 of ADR-0036;
- point 2 of ADR-0019 for the first argument of `fetch`, `pull` and `push`, and
  for `git branch` when it deletes;
- point 4 of ADR-0019 for the `git remote` and `git stash` actions listed below.

The rest still applies, including the rule that no process is started (point 7 of
ADR-0036). Of point 3 of ADR-0019, what remains here is that Git is never started.
With `git -C`, the search for the repository starts from the directory of `-C`,
not from the current one.

## Context

`git push <Tab>` offered branches and tags where Git expects a remote, and
`git branch -d <Tab>` offered tags and the current branch. Stash entries had to be
typed by hand. After `git -C dir`, only file names of the current directory were
offered. The repository files were also read whole and whatever their type, so a
FIFO or a link to a device in their place could block `Tab` or fill the memory.

## Decision

1. The `-C <dir>` pairs before the subcommand add up as for Git: a relative value
   starts from the previous one, an absolute value replaces it and an empty one
   changes nothing. References, remotes, stash entries, branches and file names
   are then those of that directory. Subcommands and options come from fixed
   lists, whatever the directory, and are still completed after `-C`. The value of
   `-C` completes to directories only. Redirections stay relative to the current
   directory.
2. Nothing is evaluated. This applies to the values of `-C` already typed and to
   the directory part of the one being typed. When one of them holds `$`, a
   backquote, `{`, a history reference or an unquoted pattern, or names no
   existing directory, nothing is read from any directory: neither names from a
   repository nor file or directory names. Subcommands and options are still
   completed. The name after that directory part is compared as written: with
   `git -C sub/a{<Tab>`, `sub/` is read, `a{` is matched literally and a match is
   written in quotes.
3. The first argument of `fetch`, `pull` and `push`, after the options and their
   values, and of `git remote remove`, `rename`, `show`, `prune`, `set-url` and
   `get-url`, completes to the remotes instead of the references. Without a match
   the file names are completed. Later arguments of `fetch`, `pull` and `push`
   keep the references. Remotes are read from the `config` file of the repository,
   the shared one for a linked worktree, and not from `refs/remotes`.
4. After `git stash pop`, `apply`, `drop` or `show`, the entries are `stash@{n}`,
   counted from the end of `logs/refs/stash` in the shared Git directory: the last
   line is `stash@{0}`. They are inserted quoted (`'stash@{0}'`), and a word that
   starts with a quote still completes. Messages are not shown. At most 100
   entries are offered, sorted as text.
5. Once `git branch` has `-d`, `-D` or `--delete`, alone or grouped as in `-dr`,
   the following arguments complete to local branches without the one checked out
   in the working tree (compared without case on macOS and Windows). With `-r` or
   `--remotes` they complete to remote branches. Tags are never offered.
6. Only regular files are opened, links followed, and each read is bounded: 64 KiB
   for `.git`, `commondir` and `HEAD`, 8 MiB for `packed-refs`, 1 MiB and 1,000
   remotes for `config`, the last 256 KiB of the stash log, and 5,000 references.
7. **Not included**: modified files for `git add` and `restore`, configuration
   keys, Git aliases, other global options of Git (`--no-pager`, `-c`), the `-C`
   of `env` and the `-D` of `sudo`, `git remote rm`, `set-head`, `set-branches`
   and `update`, and `git stash branch`.

## Options considered

- Ask Git (`git remote`, `git stash list`, `git branch`): follows the included and
  global configurations, the other worktrees and every reference format, but
  starts a process on `Tab`.
- Take the remotes from `refs/remotes`: it holds tracking branches, so a remote
  that was never fetched would be missing.
- Fall back to the current directory when the value of `-C` is unknown: it would
  offer the names of another repository.

## Consequences

### Positive

- The names offered match what the command expects at that place.
- A special file in place of a repository file no longer blocks `Tab` or fills the
  memory.

### Negative

- Remotes declared in files named by `include.path` or `includeIf`, in the global
  or system configuration, in `config.worktree` or in the old `remotes` and
  `branches` directories of the Git directory are not offered.
- A branch checked out in another worktree, or being rebased or bisected, is still
  offered for deletion.
- The options that take a value are listed by hand. For remotes, such an option
  written in a group or abbreviated is not recognised. For `git branch`,
  abbreviated long options are not recognised.
- When several references, remotes or branches that must be quoted or escaped
  share a start, the first `Tab` writes that start and the next one no longer
  recognises it. Only stash entries handle this.
- With the reftable backend there are no references, branches or stash entries.
  Remotes stay in `config` and should still be completed; this is deduced from the
  code and not tested.
- A file replaced between the check of its type and its opening is not covered.

## Validation

- Tests on real repositories made by `git init` with an isolated configuration.
- An integration test runs, with the shell binary, the lines that `Tab` writes for
  stash entries.
- Mutations of the main rules make tests fail.
- Validation was done on Linux. Nothing was run on Windows or macOS.
