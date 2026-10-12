# Pane-based TUI prototype

This T111 experiment does not replace the CLI or Tauri desktop interface and
does not execute commands.

## Technical choice

The optional `tui-prototype` feature uses Ratatui 0.30 with Crossterm 0.29 for
cross-platform layout, differential rendering, keyboard input and resize events.
It remains a Cargo example so the normal CLI does not compile these dependencies:

```console
cargo run --example tui-layout --features tui-prototype
```

Ratatui restores raw mode and the alternate screen on normal exit and panic.
Only key-press events are handled, avoiding duplicate Windows release events.

## Composition and shortcuts

The prototype renders a mock `Terminal` pane, a mock `Project & logs` pane and
a help bar with a 65/35 horizontal or vertical split. It adds no filesystem,
Git, network or shell-engine access.

| Input | Action |
| --- | --- |
| `Tab`, `Shift+Tab`, arrows | Move focus between panes |
| `s` | Toggle horizontal/vertical split |
| `q`, `Esc`, `Ctrl+C` | Exit and restore the terminal |

A future integration must replace mock data with typed views while keeping
command execution in `ShellEngine` and isolated PTY ownership in the desktop
adapter. Promotion requires rendering, shortcut and terminal-restoration proof
on Windows, macOS and Linux.

Sources: [Ratatui installation](https://ratatui.rs/installation/),
[Ratatui backends](https://ratatui.rs/concepts/backends/) and
[Crossterm](https://docs.rs/crate/crossterm/latest).
