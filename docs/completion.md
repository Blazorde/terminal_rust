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
commands, programs, files, directories, Git subcommands and references. It does
not apply to options, variables or to the directories already typed in a path.

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
remote branches (`origin/main`) and the tags. They are read from the repository
files, so Git is not started. When nothing matches, the file names are completed.

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

Completion starts no process: it reads directories and Git reference files.

History stays local. The suggestion layer does not copy command text into logs
or diagnostics. Commands may still be persisted in the history file, so do not
place secrets directly on a command line. If history loading or saving is
unavailable, completion continues with commands and paths and inline suggestions
start from an empty in-memory history.
