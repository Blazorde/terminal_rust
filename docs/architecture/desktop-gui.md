# Desktop GUI architecture

The current supported desktop app uses the Tauri architecture recorded in
[ADR-0005](../decisions/0005-desktop-gui-stack.md): `portable-pty` on the Rust
side and xterm.js in the presentation layer. The product target has since
changed to a native Rust UI; [ADR-0006](../decisions/0006-native-desktop-ui-prototype.md)
records the reversible prototype and the gates required before replacing Tauri.

```text
desktop UI (terminal rendering, settings, focus, shortcuts, responsive layout)
                              │ typed messages and events
Rust desktop adapter (PTY sessions, lifecycle, persistence, OS capabilities)
                              │ GUI-independent Rust API
ShellEngine (results, commands, parser, prompt, Git, configuration)
                     ┌────────┴────────┐
                 CLI runner       OS adapters
```

## Current application rules

- The Rust library depends on neither Tauri, React nor a GUI toolkit.
- CLI and desktop consume the same engine behavior.
- User input travels as data through bounded APIs, never concatenated shell text.
- Rust owns PTYs, process handles, current directories and disk access.
- Optional startup tools are launched by the Rust PTY process, not by the WebView.
- The WebView receives no secrets or generic shell/filesystem capability.
- Rust sends typed prompt layouts over private OSC `777`; the GUI does not probe Git or disk.
- Each tab owns exactly one PTY and every event is checked against its native session ID.
- Invalid settings use safe defaults; shipped assets are local with system-font fallbacks.

## Native migration prototype

`desktop/native-prototype/` is a separately built Rust application using
`eframe`/`egui`, `egui_tty`, Ghostty's VT engine and `portable-pty`. It starts
the existing Rust interactive runner in a real PTY. It is an experiment, not
the default or production desktop app.

The prototype is intentionally isolated: the current settings, tab model,
Tauri adapter and packaging remain unchanged until the native terminal path is
validated on Windows, Fedora/Linux and macOS. Selection-to-copy and
right-click-to-paste behavior are being evaluated as native clipboard
interactions.

## Historical delivery sequence

T119 selected the stack; T120 exposed the shared engine; T121 connected a real
PTY; T122 added settings, themes and persistence; T123 shared typed prompt
segments; T125 added isolated tabs. T124 retains final accessibility,
performance and three-platform validation.

A GUI cannot be called cross-platform from one machine's evidence. Windows,
macOS and Linux each require build, PTY and manual evidence.

## Atelier reference

The owner-provided `Terminal de ligne de commande.zip` is a composition
reference only. Its simulated commands, fake data and remote fonts are not
imported. The outer decorative grid was removed so the terminal fills the native
window, following product feedback from 2026-10-11.
