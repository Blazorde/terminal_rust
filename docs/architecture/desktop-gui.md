# Desktop GUI architecture

T118 introduced this boundary. [ADR-0005](../decisions/0005-desktop-gui-stack.md)
selects Tauri 2; T121 connects `portable-pty` on the Rust side to xterm.js in
the presentation layer. T122 adds versioned settings whose validation and disk
access remain in Rust.

```text
desktop UI (terminal rendering, settings, focus, shortcuts, responsive layout)
                              │ typed messages and events
Rust desktop adapter (PTY sessions, lifecycle, persistence, OS capabilities)
                              │ GUI-independent Rust API
ShellEngine (results, commands, parser, prompt, Git, configuration)
                     ┌────────┴────────┐
                 CLI runner       OS adapters
```

## Rules

- The Rust library depends on neither Tauri, React nor a GUI toolkit.
- CLI and desktop consume the same engine behavior.
- User input travels as data through bounded APIs, never concatenated shell text.
- Rust owns PTYs, process handles, current directories and disk access.
- The WebView receives no secrets or generic shell/filesystem capability.
- Rust sends typed prompt layouts over private OSC `777`; the GUI does not probe Git or disk.
- Each tab owns exactly one PTY and every event is checked against its native session ID.
- Invalid settings use safe defaults; shipped assets are local with system-font fallbacks.

## Delivery sequence

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
