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

The path completer preserves quoted paths, spaces, Unicode and the native path
separator on Windows, macOS and Linux. Directory candidates keep a trailing
separator so another `Tab` can continue into the directory.

History stays local. The suggestion layer does not copy command text into logs
or diagnostics. Commands may still be persisted in the history file, so do not
place secrets directly on a command line. If history loading or saving is
unavailable, completion continues with commands and paths and inline suggestions
start from an empty in-memory history.
