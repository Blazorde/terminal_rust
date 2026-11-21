# Completion and inline suggestions

Interactive Terminal Rust sessions combine path and command completion with
local history suggestions. Suggestions are visual only: Terminal Rust never
executes one until you explicitly submit the line.

| Key | Behavior |
| --- | --- |
| `Tab` | Complete or select the next built-in, custom command or path candidate |
| `Shift+Tab` | Select the previous completion candidate |
| `Right Arrow` | Accept the visible history suffix when the cursor is at the end |
| `Left Arrow` | Move the cursor and hide the inline suggestion without accepting it |
| `Up` / `Down` | Navigate complete history entries |
| `Ctrl+R` | Search backward through history |

The inline suffix is selected deterministically from the most recent matching
history entry. Empty history produces no suggestion and consecutive duplicates
are stored once. Suggestions containing terminal control characters are not
displayed.

A word at the start of a command (the line start, or after `;`, `|`, `&&` or
`||`) completes to the built-in, custom and plugin commands and to the programs
found in `PATH` (the executable files of its directories, without the `.exe`
extension on Windows). Programs are only offered once you have typed a first
character, and the list is reread every ten seconds or when `PATH` changes. A
word that looks like a path (`./run`, `~/bin/x`) completes as a file.

After `git `, `Tab` completes the everyday subcommands, and after `git checkout`,
`switch`, `merge`, `rebase`, `branch`, `cherry-pick`, `diff`, `log`, `show`,
`reset`, `revert`, `push`, `pull` or `fetch` it completes the local branches, the
remote branches (`origin/main`) and the tags. They are read from the repository
files, so Git is not started. When nothing matches, the file names are completed.

Hidden files and directories (`.git`, `.cache`) are offered only when the name
being typed starts with a dot.

After a `$`, `Tab` completes the names of the environment variables, with the
closing brace in the `${NAME}` form. Variables that were set but not exported are
not listed.

The path completer preserves quoted paths, spaces, Unicode and the native path
separator on Windows, macOS and Linux. Directory candidates keep a trailing
separator so another `Tab` can continue into the directory.

History stays local. The suggestion layer does not copy command text into logs
or diagnostics. Commands may still be persisted in the history file, so do not
place secrets directly on a command line. If history loading or saving is
unavailable, completion continues with commands and paths and inline suggestions
start from an empty in-memory history.
