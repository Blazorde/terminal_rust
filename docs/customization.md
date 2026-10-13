# Prompt customization

Terminal Rust builds a smaller, declarative Oh My Posh-inspired prompt from
independent segments.

## Responsibilities

1. Providers collect data such as the current directory or Git state.
2. User configuration selects segment order and behavior.
3. Themes convert semantic data into text, colors, icons and separators.

A theme never executes a command. Missing segments disappear without making the
shell unusable.

## Configuration locations

- Windows: `%APPDATA%\terminal-rust\config.toml`
- macOS: `$HOME/Library/Application Support/terminal-rust/config.toml`
- Linux: `$XDG_CONFIG_HOME/terminal-rust/config.toml`, otherwise `$HOME/.config/terminal-rust/config.toml`

`TERMINAL_RUST_CONFIG` selects an explicit file. Built-in defaults remain active
when no location is available.

```toml
version = 1
theme = "default"

[prompt]
segments = ["user", "host", "path", "git", "symbol"]
right_segments = ["status", "runtime", "duration", "time"]
icons = "auto"
separator = "  "
newline = false

[segments.path]
style = "full"

[segments.git]
show_status = true

[segments.symbol]
value = ">"
```

## Version 1 segments

| ID | Data | Fallback |
| --- | --- | --- |
| `user` | process user | omitted when unavailable |
| `host` | system host name | omitted when unavailable |
| `path` | current directory, full or folder style | `Path::display` output |
| `git` | branch and dirty state | omitted outside Git or without Git |
| `status` | previous command result | omitted before the first command |
| `runtime` | local Cargo, Node, Python or Go marker | omitted when unknown |
| `duration` | previous command duration | omitted before the first command |
| `time` | local collection time | offline text |
| `symbol` | final prompt symbol | `>` |

Array order is preserved. Left segments also form the CLI prompt; right segments
appear in the desktop status bar. Duplicates across either side and unknown IDs
are rejected. Version 1 does not evaluate expressions or arbitrary commands.

## Capabilities and safety

`icons = "never"` forces ASCII, `always` requests theme glyphs, and conservative
`auto` requires explicit capability confirmation. `TERMINAL_RUST_ICONS` can
override the setting; `TERMINAL_RUST_NERD_FONT=1` confirms a compatible font for
auto mode. Non-interactive output, `NO_COLOR` and `TERM=dumb` disable ANSI.

Configuration values are never executed. Strings are bounded, control
characters are rejected, paths use native APIs, and invalid user files report
their location before safe defaults take over. See the [theme format](themes.md).
