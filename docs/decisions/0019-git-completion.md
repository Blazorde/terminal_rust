# ADR-0019 — Git completion

- Status: Accepted
- Date: 2026-11-21
- Partly replaced: options (points 4 and 6) are completed since
  [ADR-0036](0036-smarter-completion.md)
- Decision makers: Terminal Rust team

## Context

Git is the command most often typed after the shell's own. Completion already
covers commands, programs of `PATH`, variables and files, but `git ch<Tab>` and
`git checkout fe<Tab>` offered nothing useful, and the file completion also showed
hidden entries such as `.git/` as soon as `Tab` was pressed in a directory.

## Decision

1. After `git ` (also after `;`, `|` or `&`), `Tab` completes the everyday
   subcommands from a fixed list.
2. After `git checkout`, `switch`, `merge`, `rebase`, `branch`, `cherry-pick`,
   `diff`, `log`, `show`, `reset`, `revert`, `push`, `pull` or `fetch`, `Tab` completes the
   local branches, the remote branches (`origin/main`) and the tags of the
   repository. When none matches the typed text, the file names are completed as
   before, so `git checkout src/main.rs` still works.
3. The references are read from the repository files (`refs/heads`,
   `refs/remotes`, `refs/tags` and `packed-refs`), found from the current directory
   upwards, including a linked worktree (`.git` file and `commondir`). Git is never
   started: completing is instant and cannot hang. The symbolic `HEAD` of a remote
   is not offered, and at most 5 000 references are read.
4. Options (`-…`), other subcommands (`add`, `rm`, `mv`, `restore`…) and every
   other command keep the file name completion.
5. File name completion no longer offers hidden entries (names starting with `.`)
   unless the name being typed starts with a dot, as in other shells.
6. **Not included**: completion of remote names, stash entries, configuration keys,
   options, `git -C dir …` and aliases defined in the Git configuration.

## Options considered

- Run `git for-each-ref`: always right (including exotic ref storage), but it starts
  a process on each `Tab`, can be slow on large repositories and needs Git on the
  `PATH`.
- Parse Git's own shell completion script: not portable and not available on Windows.

## Consequences

### Positive

- `git ch<Tab>`, `git checkout fe<Tab>` and `git merge origin/<Tab>` work at once and
  without a process.
- `ls <Tab>` no longer lists `.git`, `.cache` and other hidden entries.

### Negative

- A repository that stores references in another format (the reftable backend) shows no
  references until Git supports it in files; subcommands still complete.
- The subcommand list is fixed and does not include aliases or plugins of Git.

## Validation

- Unit tests with fixture repositories: loose and packed references, nested
  branches, remote branches, tags, the remote `HEAD`, a linked worktree, a subdirectory,
  a broken `.git`, separators, files and options.
- A test through the completion helper itself, and a manual check in a real terminal.
- Mutation checks: another command completed as git, `HEAD` offered, `packed-refs`
  ignored, `commondir` ignored, no walk upwards, tags not read, references for every
  subcommand, and the hidden-file filter always on or always off make tests fail.
