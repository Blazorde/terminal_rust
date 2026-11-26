# Completion and inline suggestions

Interactive Terminal Rust sessions combine path and command completion with
local history suggestions. Suggestions are visual only: Terminal Rust never
executes one until you explicitly submit the line.

| Key | Behavior |
| --- | --- |
| `Tab` | Complete what every candidate shares; a second `Tab` lists them in columns |
| `Right Arrow` | Accept the visible history suffix when the cursor is at the end |
| `Left Arrow` | Move the cursor and hide the inline suggestion without accepting it |
| `Up` / `Down` | Navigate complete history entries |
| `Ctrl+R` | Search backward through history |

The inline suffix is selected deterministically from the most recent matching
history entry. Empty history produces no suggestion and consecutive duplicates
are stored once. Suggestions containing terminal control characters are not
displayed.

Above 100 candidates the shell asks before listing them. At most 2,000 are
offered at once.

When no name starts with what is typed in the same case, the names that match in
another case are offered, and the word takes their real case. This applies to
commands, programs, files, directories, Git subcommands and the names read from a
Git repository. It does not apply to options, variables or to the directories
already typed in a path.

A word at the start of a command (the line start; after `;`, `|`, `&`, `&&`,
`||`, `$(`, a backquote, `(` or `{`; after `!`, `if`, `then`, `elif`, `else`,
`while`, `until` or `do`; after `sudo`, `env`, `time`, `nohup`, `xargs`, `exec`
or `command` and their options) completes to the built-in, custom and plugin commands and to the programs
found in `PATH` (the executable files of its directories, without the `.exe`
extension on Windows). Programs are only offered once you have typed a first
character, and the list is reread every ten seconds or when `PATH` changes. A
word that looks like a path (`./run`, `~/bin/x`) completes as a file.

After `git `, `Tab` completes the everyday subcommands, and after `git checkout`,
`switch`, `merge`, `rebase`, `branch`, `cherry-pick`, `diff`, `log`, `show`,
`reset`, `revert`, `push`, `pull` or `fetch` it completes the local branches, the
remote branches (`origin/main`) and the tags, except in the cases below. They are
read from the repository files, so Git is not started. When nothing matches, the
file names are completed.

The first argument of `git fetch`, `pull` and `push`, after any options and their
values, completes to the remotes instead. So does the first argument of
`git remote remove`, `rename`, `show`, `prune`, `set-url` and `get-url`. Later
arguments of `fetch`, `pull` and `push` keep the references. Remotes come from the
`config` file of the repository (the shared one for a linked worktree), not from
`refs/remotes`. Files named by `include.path` or `includeIf`, the global and
system configurations, `config.worktree` and the old `remotes` and `branches`
directories of the Git directory are not read. `git remote rm`, `set-head`,
`set-branches` and `update` are not covered, and an option that takes a value is
not recognised when it is grouped or abbreviated.

After `git stash pop`, `apply`, `drop` or `show`, `Tab` offers the entries,
`stash@{0}` being the latest. They are inserted in quotes (`'stash@{0}'`), and a
word that already starts with a quote still completes. Their messages are not
shown. At most 100 entries are offered, sorted as text, so `stash@{10}` comes
before `stash@{1}`. `git stash branch` is not covered.

Once `git branch` has `-d`, `-D` or `--delete`, alone or grouped as in `-dr`,
`Tab` offers local branches without the one checked out in this working tree
(compared without case on macOS and Windows). With `-r` or `--remotes` it offers
remote branches. Tags are not offered. Branches checked out in another worktree,
or being rebased or bisected, remain in the list. Abbreviated long options are not
recognised.

After `git -C <dir>`, written before the subcommand, the names from the repository
and the file names are those of `<dir>`. Several `-C` add up as for Git: a
relative value starts from the previous one, an absolute one replaces it and an
empty one changes nothing. `git -C <Tab>` offers directories only. Redirections
stay relative to the current directory. When a value already typed, or the
directory part of the one being typed, holds `$`, a backquote, `{`, a history
reference or an unquoted pattern, or does not exist, no names from a repository
and no file or directory names are offered. Subcommands and options are still
completed. The name after that directory part is matched as written, then quoted:
`git -C sub/a{<Tab>` reads `sub/`.
Other global options of Git, such as `--no-pager` or `-c`, are not handled, and
neither `env -C` nor `sudo -D` changes where names are read from.

When several references, remotes or branches that must be quoted or escaped share
a start, the first `Tab` writes that start, but the next one no longer recognises
the opened quote or the escape. Modified files for `git add` and `restore`,
configuration keys and Git aliases are not completed.

A repository that uses the reftable backend gets no references, branches or stash
entries. Its remotes are in `config` and should still be completed, although this
case is not tested.

`cd`, `pushd` and `rmdir` are offered directories only.

A word that starts with `-` completes to the options of a built-in, taken from
its usage line (see `help`) and from what it handles itself, or to the common
options of `git commit`, `checkout`, `switch`, `push`, `pull`, `log`, `diff`,
`branch` and `stash`. Without a matching option the file names are completed.

Hidden files and directories (`.git`, `.cache`) are offered only when the name
being typed starts with a dot.

After a `$`, `Tab` completes the names of the environment variables, with the
closing brace in the `${NAME}` form. Variables that were set but not exported are
not listed.

A name with spaces or special characters is escaped or quoted for the shell, or
written inside the quote already typed. A completed file closes that quote; a
directory keeps it open and takes a trailing separator, so another `Tab` can
continue into it. The directory part of a path stays as typed, and the native
separator is used on Windows. A name that holds a control character is never
offered.

Completion starts no process. It reads directories and, in a Git repository, the
references (`refs/`, `packed-refs`), `.git`, `commondir`, `HEAD`, `config` and the
log of the stash (`logs/refs/stash`). Only regular files are opened, links
followed, although a file replaced between that check and the opening is not
covered. Reads are bounded: 64 KiB for `.git`, `commondir` and `HEAD`, 8 MiB for
`packed-refs`, 1 MiB and 1,000 remotes for `config`, the last 256 KiB of the stash
log, and 5,000 references.

History stays local. The suggestion layer does not copy command text into logs
or diagnostics. Commands may still be persisted in the history file, so do not
place secrets directly on a command line. If history loading or saving is
unavailable, completion continues with commands and paths and inline suggestions
start from an empty in-memory history.
