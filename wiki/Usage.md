# Usage

Run `help` inside Terminal Rust for the authoritative command list.

| Command | Purpose |
| --- | --- |
| `pwd` | Print the working directory |
| `cd`, `ls`, `mkdir`, `touch`, `cat`, `rm` | Portable file operations |
| `clear` | Clear an ANSI-capable terminal |
| `dashboard` | Open the live system and project dashboard |
| `git status\|branch\|log` | Run an allowlisted read-only Git operation |
| `theme list`, `theme preview` | Inspect built-in prompt themes |
| `plugins list`, `plugin list` | Inspect plugin manifests and permissions |
| `plugins help`, `plugin help` | Explain what plugins support and how to inspect them |
| `commands list` | Inspect configured custom commands |
| `commands help` | Explain custom commands and where to configure them |
| `echo [-n] [text...]` | Print text, without a trailing newline with `-n` |
| `exit`, `quit` | Close the session |

## Paths containing spaces

Quote a path containing spaces with single or double quotes so the command
receives it as one argument:

```text
cat "notes from today.txt"
rm 'notes from today.txt'
rm -d "old project"
```

The quotes are removed by the command parser. You can also escape spaces with
a backslash (`cat notes\ from\ today.txt`). Native path separators are
preserved, including backslashes in Windows paths.

The dashboard refreshes once per second. Press `q`, `Escape` or `Ctrl+C` to
leave it. Redirected output and limited terminals receive a stable text
snapshot.

`Tab` and `Shift+Tab` cycle completions. Accept a dim history suggestion with
`Right Arrow` and hide it with `Left Arrow`.

Plugins are declarative extensions described by local manifests. `plugins list`
shows their status and requested permissions; `plugin` is an alias for
`plugins`. Custom commands are shortcuts from the user configuration:
`commands list` shows configured shortcuts, and `commands help` explains them.
See [custom commands](../docs/custom-commands.md) and the
[plugin tutorial](../docs/plugins/getting-started.md) for setup details.

Desktop sessions are isolated per tab. Use `Ctrl/Command+T` to create a tab,
`Ctrl/Command+W` to close it, `Ctrl+Tab` to switch tabs and `Ctrl/Command+,` to
open settings.

More detail: [dashboard](https://github.com/Blazorde/terminal_rust/blob/main/docs/dashboard.md)
and [completion](https://github.com/Blazorde/terminal_rust/blob/main/docs/completion.md).
