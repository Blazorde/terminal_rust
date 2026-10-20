# ADR-0006 — Native desktop UI migration prototype

- Status: Accepted for prototype; production migration is gated
- Date: 2026-10-19
- Decision makers: Terminal Rust team

## Context

The product goal changed: the desktop terminal must not depend on a system
WebView. It should be a lightweight, installable terminal application that can
be selected as the default terminal emulator on Windows, macOS and Linux. The
existing Rust shell and CLI remain the source of truth.

Firmware/BIOS integration is not an application-level target. UEFI runs before
the operating system and cannot launch an ordinary desktop terminal. Registering
Terminal Rust as an OS terminal emulator is separate from replacing the user's
login shell.

## Decision

Build a reversible native Rust UI prototype alongside the current Tauri app.
Keep the existing engine and PTY behavior; do not replace the production
desktop application until the prototype proves the terminal path.

The prototype uses `eframe`/`egui` for the native window and `egui_tty` for
terminal rendering and input. `egui_tty` uses Ghostty's VT engine and supports
real PTYs through `portable-pty`. A prototype session launches the same Rust
interactive runner through the native executable's internal `--pty-shell`
entry point.

The prototype also exercises system clipboard behavior: text selection copies
to the clipboard and right-click pastes sanitized, bracketed text. Clipboard
behavior must be validated on each supported desktop platform before it is
treated as a product contract.

## Consequences and risks

- The prototype is a native compiled window without a WebView; egui still draws
  its own widgets rather than using each OS's native widget toolkit.
- Building the Ghostty VT dependency requires Zig 0.15.x. This is a developer
  and build-pipeline prerequisite, not a runtime dependency for end users.
- `egui_tty` is an early-stage dependency; its API, release cadence, platform
  behavior and licensing notices must be reviewed before production adoption.
- The prototype currently has one terminal session. It does not replace the
  current Tauri app, settings, tabs, packaging, or OS terminal registration.
- The current Tauri UI remains the supported desktop application until
  Windows, Fedora/Linux and macOS builds and interactive tests pass.
- Default-terminal registration is an OS-specific packaging/integration task.
  Login-shell replacement is optional, separate, and must never be required for
  normal use.
- Distribution adoption requires stable packaging, upgrade/uninstall behavior,
  security maintenance and platform-specific support; a prototype is not enough.

## Migration gates

1. Build and run the native prototype against the real Terminal Rust shell on
   Windows, Fedora/Linux and macOS.
2. Verify resize, ANSI/VT output, keyboard input, selection, clipboard, paste,
   Ctrl+C, process exit and clean shutdown.
3. Measure release binary size, cold startup and idle/active memory against the
   current Tauri app on the same hardware.
4. Port settings, tabs, accessibility and desktop integrations without changing
   shell behavior.
5. Package and test OS default-terminal registration before retiring Tauri.

## Alternatives considered

- Keep Tauri and only hide developer tools: does not meet the no-WebView goal.
- Implement a VT parser and renderer from scratch: duplicates mature terminal
  work and increases correctness and security risk.
- Integrate the whole shell at firmware level: not a normal desktop-app
  capability and not a useful cross-platform target.
