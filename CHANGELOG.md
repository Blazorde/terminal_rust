# Changelog

All notable changes to Terminal Rust are documented in this file. Versions use
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Documented the post-MVP plugin vision, extension boundaries, permission model
  and staged roadmap inspired by Neovim without enabling third-party code.
- Defined the target plugin architecture, declarative-first lifecycle, stable
  message boundary, future source layout and cross-platform compatibility rules.
- Specified the proposed plugin API DTOs, host capabilities, error model,
  version negotiation and runtime-independent validation rules.
- Added a validated, backward-compatible plugin configuration schema with
  explicit activation and preserved plugin-owned settings.
- Added a cross-platform local plugin registry, validated versioned manifests,
  and a read-only `plugins list` command with explicit health states.
- Centralized native command syntax and summaries so dispatch, generated help,
  and completion can share the same future plugin-aware registry boundary.
- Added validated prompt-segment declarations with bounded execution and cache
  budgets while keeping all rendering under host control.
- Added a runtime-independent plugin SDK and a copyable, compiling `hello`
  plugin template with a validated manifest and tests.
- Added structured custom commands for built-in aliases and directly spawned
  processes, including listing, completion, conflict checks, and safe arguments.
- Compared Lua, Rhai, and WebAssembly for future scripting and proposed a
  bounded Rhai experiment after the plugin permission model is complete.
- Added an optional Ratatui layout prototype with two focusable panes,
  horizontal and vertical splits, keyboard shortcuts, and resize handling.
- Prioritized future project detection and task shortcuts while preserving
  native developer tools and the existing no-implicit-shell boundary.
- Added a fail-closed plugin permission gate, sensitive capability requests,
  visible permission review, and explicit denial diagnostics.
- Added explicit Terminal Rust version requirements to plugin manifests and
  actionable API/application compatibility diagnostics in the registry.
- Defined a local-first plugin catalog, transactional installation lifecycle,
  command surface, and security boundary for future discovery work.
- Added an English beginner plugin tutorial covering the template, manifest,
  typed command, tests, local discovery, debugging, and responsible sharing.
- Added the first official declarative plugin command runtime with explicit
  grants, static bounded responses, command dispatch, and disable-safe tests.
- Added a reusable `ShellEngine` library API with typed execution status,
  captured standard streams, errors, and directory-change metadata for CLI and
  future desktop consumers.
- Added the first Tauri desktop window backed by a native Rust-owned PTY, with
  real shell input and output, keyboard focus, resize handling, session restart,
  an xterm.js renderer, and an Atelier-inspired responsive interface.
- Added desktop build and native PTY integration checks to the cross-platform
  CI matrix.
- Added a keyboard-accessible desktop settings drawer with six local themes,
  terminal typography and cursor controls, compact density, optional status
  bar, and versioned persistence with validated safe fallbacks.
- Added Rust-owned prompt context for user, host, path, Git, last status,
  project runtime, command duration, and local time, with configurable left and
  right layouts shared with the desktop status bar.
- Added keyboard-accessible desktop tabs backed by independent native PTY
  sessions, with isolated input, output, working directory, history and prompt
  context.

### Changed

- Expanded the desktop palette to Graphite, Bone, Nord, Amber, Moss and Dusk,
  while migrating the previous Paper and Midnight settings automatically.
- Standardized contributor, architecture, configuration, desktop, plugin,
  installation, release and troubleshooting documentation in concise English.
- The CLI now executes built-in commands through the same public Rust engine
  intended for the desktop adapter.
- The desktop terminal surface now fills the native window without an outer
  decorative canvas, redundant border, or shadow.
- Desktop themes now meet WCAG AA text contrast, the settings dialog traps and
  restores keyboard focus, reduced motion has targeted fallbacks, and frontend
  dependencies are emitted as bounded chunks.

## [0.3.0] - 2026-10-10

This release introduces the configurable, segmented prompt engine while
preserving graceful fallbacks on Windows, Linux, and macOS.

### Added

- Defined the versioned prompt customization and theme contracts with default
  TOML examples and icon-free fallbacks.
- Added a reusable prompt segment model that keeps data, semantic styles, and
  optional icons independent from ANSI rendering.
- Added a portable ANSI prompt renderer with foreground, background, emphasis,
  per-segment resets, and automatic plain-text fallback.
- Added an injectable semantic color palette so themes can remap prompt colors
  without changing path or Git segment logic.
- Added centralized Nerd Font icons with deterministic ASCII fallbacks and
  `auto`, `always`, and `never` display modes.
- Added validated, cross-platform TOML user configuration loading with safe
  defaults and actionable file-specific diagnostics.
- Added configurable prompt segment selection and ordering, including separator,
  newline, path style, Git status marker, and symbol options.
- Added selectable `default`, `minimal`, `git-focus`, and `powerline` theme
  presets, including automatic ASCII fallback without a Nerd Font.
- Added `theme list` and `theme preview <name>` commands for safe theme
  discovery without changing the user configuration.
- Verified user-level installers for Windows x86_64, Linux x86_64, and macOS
  Apple Silicon, with authenticated private-release support.

### Improved

- Added the repository's MIT license file and bundled the license, README,
  changelog, and configuration examples in release archives.
- Added end-to-end CLI coverage and a reproducible MVP quality checklist for
  public commands and terminal fallbacks.
- Added copyable onboarding, a supported-platform matrix, and the complete
  contributor validation workflow to the README.
- Added installer syntax validation to the cross-platform CI matrix.
- Removed duplicate CI matrices for work-branch pushes; the full matrix remains
  required on pull requests and protected-branch updates.
- Centralized prompt rendering so the interactive shell and theme previews use
  the same ANSI, icon, separator, and fallback behavior.

### Known limitations

- Custom theme files are not loaded yet; this release provides four built-in
  presets.
- Powerline glyphs and prompt icons require a compatible Nerd Font and explicit
  capability confirmation; ASCII fallbacks remain available everywhere.
- Terminal Rust remains a pedagogical mini-shell and does not replace Bash,
  PowerShell, or Zsh.

## [0.2.0] - 2026-10-09

### Added

- Persistent command history and the `clear` built-in command.
- Non-blocking Git repository detection and read-only `git status`, `git branch`,
  and `git log` shortcuts.
- A cross-platform `dashboard` command for CPU, memory, storage, network, and Git
  context.
- A portable Fastfetch preset and setup documentation.
- Cross-platform CLI integration tests and release builds for Windows, Linux,
  and macOS.

### Improved

- Added project architecture, decision records, contribution guidance, and
  release documentation.
- Added automated GitHub Release publication with checksums for version tags.

### Known limitations

- Terminal Rust remains a pedagogical mini-shell and does not yet replace Bash,
  PowerShell, or Zsh.
- The prompt segment system inspired by Oh My Posh is planned but not included
  in this release.

## [0.1.0] - 2026-06-20

### Added

- First public release with the interactive shell, command history for the
  current session, completion, and quoted-argument parsing.
- Core `pwd`, `cd`, `ls`, `mkdir`, `touch`, `cat`, and `rm` commands.
- Cross-platform CI for formatting, compilation, Clippy, and tests.
- Windows x86_64 binary and checksum attached to the GitHub Release.
