# ADR-0005 — Tauri desktop layer around the Rust engine

- Status: Accepted
- Date: 2026-10-10
- Decision makers: Terminal Rust team

## Context

T118 requires an Atelier-inspired desktop interface on Windows, macOS and Linux
without replacing the Rust engine or CLI. It needs real terminal sessions,
responsive settings and full keyboard navigation with no remote font or asset.

## Decision

Use Tauri 2 for the window and local TypeScript/React for presentation. Rust
continues to own commands, Git, paths, configuration and PTY sessions.

1. The Rust library exposes commands, context, prompt and errors without a GUI dependency.
2. The CLI remains a consumer of that library and preserves public behavior.
3. Tauri Rust owns PTYs, filesystem and settings and exposes minimal typed messages.
4. The WebView renders xterm and settings but never starts processes directly.
5. Capabilities are limited to the main window and explicit commands; no remote content loads.

`portable-pty` provides the initial native PTY adapter across Unix and Windows.

## Options considered

| Criterion | Tauri 2 | `iced` | `egui` / `eframe` |
| --- | --- | --- | --- |
| Atelier fidelity | High through CSS and responsive layout | Medium | Medium to low |
| Rust engine reuse | Typed commands/events | Direct | Direct |
| Terminal work | xterm renderer, Rust PTY | Custom widget | Custom widget |
| Accessibility | Web semantics, still audited | Widget-dependent | Custom-control verification |
| Main risk | IPC and two toolchains | Experimental ecosystem | Frequent breaking changes |

Tauri's system WebView adds WebKitGTK requirements on Linux and may render
slightly differently between platforms. IPC security, content policy and message
validation become explicit responsibilities.

## Rejected for the first prototype

- Importing the React mock: it simulates commands and loads remote fonts.
- Rewriting the engine in TypeScript: Rust remains the source of truth.
- Starting with `iced` or `egui`: a custom terminal delays product validation.

## Validation

The desktop must prove real sessions on all three platforms, unchanged CLI
behavior, keyboard and small-window usability, offline startup without Git or a
Nerd Font, minimal Tauri capabilities and documented startup/memory/input data.
