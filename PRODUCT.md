# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Delegated: Tauri 2 wraps a local React and TypeScript interface built with
Vite. Rust owns the shell engine, PTY sessions, filesystem access and
configuration. The WebView owns presentation only.

## Users

Developers and students who want a compact, understandable terminal for daily
project navigation, Git context and system inspection on Windows, macOS and
Linux.

## Product Purpose

Terminal Rust is a pedagogical mini-shell that makes common terminal actions
approachable without hiding their real filesystem and Git effects. Success
means the CLI and desktop interface run the same Rust behavior with clear
fallbacks when Git, ANSI color or enhanced fonts are unavailable.

## Positioning

The product combines a deliberately small, auditable Rust command engine with
an Oh My Posh-inspired segmented prompt and a desktop workshop surface. The GUI
is not a simulated terminal: it connects to a native PTY owned by Rust.

## Operating Context

Users keep the terminal open beside an editor while navigating repositories,
checking Git state, creating files and inspecting machine performance. Keyboard
input, command history, selection and focus are primary interactions.

## Capabilities and Constraints

- Windows, macOS and Linux are first-class targets.
- CLI behavior remains available without the desktop toolchain.
- No command execution, generic filesystem access or secrets live in the
  WebView.
- The application must start and remain usable without network access, Git, a
  Nerd Font or advanced ANSI support.
- Native paths use Rust `Path` and `PathBuf`; OS-specific behavior is isolated.

## Brand Commitments

The product name is Terminal Rust. The supplied Atelier prototype is the
binding composition reference: a focused terminal surface that fills its
native window, with a compact title bar, contextual footer and dense settings
drawer. Its fake commands and remote fonts are not product behavior.

## Evidence on Hand

- `C:/Users/Blaz/Downloads/Terminal de ligne de commande.zip` contains the
  approved Atelier visual prototype.
- The repository contains the real Rust engine, cross-platform tests, themes,
  dashboard and Git-aware prompt.
- No testimonials, usage metrics or external brand assets are available and
  none may be fabricated.

## Product Principles

- Real terminal behavior before visual theater.
- One Rust source of truth for CLI and desktop.
- Keyboard fluency with visible, recoverable states.
- Cross-platform fallbacks are product behavior, not edge cases.
- Dense information presented with calm hierarchy.

## Accessibility & Inclusion

All controls require keyboard access, visible focus, scalable text and at least
WCAG AA contrast. Status cannot rely on color alone. Reduced motion and system
monospace fallbacks must remain functional.
