# Changelog

All notable changes to Terminal Rust are documented in this file. Versions use
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- Custom commands and aliases of the `process` kind ran with their output
  captured and no standard input, so interactive programs (a prompt, `python`,
  `ssh`, `less`) could not work and the output only appeared at the end. They now
  share the terminal like any other program, and a failure reports the exact exit
  status (`name: exited with status 3`) instead of a doubled message.
- Pressing Tab after a no-break space or another multi-byte whitespace crashed
  the shell, because the completion assumed a whitespace takes one byte. The
  word start is now computed from the character width and the cursor position is
  moved back to a character boundary.
- The remove command could delete what the user did not mean to: asking it to
  remove the current directory or its parent emptied them. Those two targets and
  the root directory are now refused, before anything is removed, as in GNU `rm`.
- `rm` now keeps going after a failure and reports every failing target instead of
  stopping at the first one, can remove a broken symlink, and never follows a
  symlink to a directory.
- A `rm` confirmation read from a piped standard input blocked the shell for ever,
  and the question was not shown in a terminal until after the answer.

## [1.2.1] - 2026-11-20

### Added

- `scripts/ci-local.sh` replays the Linux gate of the CI, with the Security checks,
  in the pinned Docker image and summarizes each step, for use when hosted
  runners are unavailable.

### Fixed

- A command typed while another program was still running was never executed:
  the terminal turned Enter into a line feed, and the shell bound that key
  (`Ctrl+J`) to "insert a new line" instead of accepting the line. `Ctrl+J` now
  validates the line like Enter, as in other shells. Multi-line input still uses
  a trailing backtick, and `Shift+Enter` in the desktop terminal.
- Lines pasted or typed faster than the shell reads them were lost: only the
  first command of a paste ran. The line editor read ahead into a buffer that it
  dropped after each prompt; enabling its `buffer-redux` feature keeps that
  buffer between prompts (Unix).

## [1.2.0] - 2026-11-20

### Added

- Command chaining: `;` runs the next command, `&&` only after a success and
  `||` only after a failure. Each command is expanded right before it runs, so
  `$?` and variables set earlier on the line are visible, and output stays in the
  order of the commands even when programs are involved. See ADR-0010.

### Changed

- An unquoted `|`, `<`, `>` or `&` is now rejected with a clear message instead
  of being passed to a program as an argument, until pipes, redirections and
  background jobs are designed.

- Variables in command lines: `$NAME`, `${NAME}`, `$?` and a leading `~` are
  expanded outside and inside double quotes (not inside single quotes), `\$`
  writes a dollar sign, and `NAME=value` sets a variable. `export NAME[=value]`
  passes a variable to the programs the shell starts, `unset NAME` removes it,
  and `PATH=$PATH:dir` updates the search path. See ADR-0009.
- Any command that is not built in now runs the program of that name found in
  `PATH` (`PATHEXT` is honored on Windows), with the terminal streams, the
  environment and the current directory of the shell. A missing program still
  reports `unknown command`, and a failing one reports its exit status. See
  ADR-0008.
- `Ctrl+C` no longer ends the interactive shell while a program is running.

## [1.1.0] - 2026-11-20

### Added

- Bundled Git, Nano and Neovim into desktop releases so full Git workflows and
  editor commands work without separate installations.
- Added `nano`, `nvim` and `neovim` commands and made the bundled editors
  available to Git's configured interactive editor.
- Added an `echo` command that prints its arguments separated by spaces.
- Added a `man` command: built-in commands show their name and synopsis, and any
  other topic is handed to the system `man` when it is installed.
- Added a whoami command that prints the current user’s username.
- Added an arch command that prints the system architecture.
- Added an nproc command that prints the number of available processing units.
- Added an uptime command that prints how long the system has been running.
- Added a date command that prints the current date and time.
- Added a hostname command that prints the current host’s name.
- Added a printenv command that prints the environment variables.
- Added Linux terminal integration to the desktop app: `--working-directory` /
  `-w`, `-e` and `--` launch options for the first session, a `TerminalEmulator`
  desktop entry, and an `x-terminal-emulator` alternative registered with a low
  priority by the DEB. See the Linux wiki page.

### Changed

- Replaced the README cover with a lighter hand-drawn illustration.
- Renamed the `refractor/` branch prefix to `refactor/` in the branch and pull
  request policies.

### Fixed

- Fixed the desktop crate failing to compile on Linux and macOS.
- Restored the CI: bundled runtime preparation (Nano version parsing, Git
  license file name, Windows download retries), resource permissions and the
  desktop PTY tests on Linux and macOS.
- Replaced the yanked `clipboard-win` 5.4.1 with 5.4.3 and refreshed the
  third-party notices.

## [1.0.0-alpha.1] - 2026-10-13

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
- Added a live cross-platform system dashboard that refreshes in place until
  `q`, `Escape` or `Ctrl+C`, with static fallbacks for redirected, dumb and
  undersized terminals.
- Added dim inline history suggestions accepted with `Right Arrow`, circular
  `Tab`/`Shift+Tab` completion and tested quoted Unicode path handling.
- Added keyboard-accessible desktop tabs backed by independent native PTY
  sessions, with isolated input, output, working directory, history and prompt
  context.

### Changed

- Redesigned the desktop shell around a frameless native window with branded
  title chrome, clear PTY state, compact session tabs and responsive controls.
- Desktop prompts now keep the editable line compact while the status bar owns
  full path, user, host, runtime and timing context.
- Interactive `ls` output now wraps to the available width and uses portable
  semantic colors for directories, project manifests, source, tools and archives.
- Linux desktop packaging now declares the complete native icon set and embeds
  the MIT license alongside third-party notices and bundled configuration.
- Expanded the desktop palette to Graphite, Bone, Nord, Amber, Moss and Dusk,
  while migrating the previous Paper and Midnight settings automatically.
- The desktop terminal now refreshes xterm font metrics and dimensions after
  font, density, tab, viewport and window changes so typed text stays aligned
  with every cursor style.

### Fixed

- Prevented structured prompt metadata from appearing as raw JSON or shifting
  Rustyline's visible prompt width in Windows and Linux PTY sessions.
- Serialized desktop keyboard writes so rapid input and long paths cannot reach
  the native PTY out of order or lose characters.
- Restored native drag and edge/corner resize behavior for the frameless window.
- Standardized contributor, architecture, configuration, desktop, plugin,
  installation, release and troubleshooting documentation in concise English.
- Clarified the source commands that open the native Tauri terminal window and
  added the project's optional Ko-fi support link.
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
