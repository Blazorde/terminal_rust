# Desktop development

The desktop app wraps the existing Rust engine in Tauri 2. Tauri creates a
native PTY and starts the same executable with internal `--pty-shell`; React
does not reimplement input, history, commands or prompt behavior.

## Prerequisites

- stable Rust;
- Node.js 24 and npm;
- Tauri 2 platform dependencies.

Debian/Ubuntu needs WebKitGTK 4.1, `libayatana-appindicator3-dev`,
`librsvg2-dev` and `libxdo-dev`. Windows uses WebView2 and macOS uses WKWebView.

## Launch the native window

From the repository root, install the frontend dependencies once, then start
Tauri in development mode:

```bash
npm ci --prefix desktop
npm run tauri --prefix desktop -- dev
```

The second command builds the Rust adapter and opens the real native Terminal
Rust window with PTY-backed sessions. Keep it running while testing. If the
command stops before a window appears, confirm the platform prerequisites above
and use the [troubleshooting guide](../troubleshooting.md).

Verify real `pwd`, history, `dashboard`, `clear`, safe Git, `Ctrl+C`, resize,
settings (`Ctrl/⌘+,`, `Escape`) and tabs (`Ctrl/⌘+T`, `Ctrl/⌘+W`, `Ctrl+Tab`).
Open two tabs, change directory in one, and confirm the other keeps independent
directory, history and prompt context. Keyboard tab-list navigation supports
arrows, `Home`, `End` and `Delete`.

The native window uses a cross-platform custom title bar so the logo, PTY state,
session controls and window controls remain visually consistent. Verify drag,
minimize, maximize/restore and close on every target OS. The Rust adapter emits
structured prompt metadata through the PTY; the frontend removes its OSC frame
before rendering output and uses it only for tab labels and the status bar.
Rustyline receives separate plain and styled prompt forms, and the desktop
editable prompt is reduced to folder, Git and prompt symbol. This prevents long
project paths or hidden metadata from shifting the cursor. Frontend PTY writes
are serialized so fast typing and paste preserve byte order.

Interactive `ls` uses the detected terminal width to wrap entries across a
small number of readable rows. Directories and important project folders,
manifests, source files, scripts and archives receive portable ANSI colors.
`ls -1` remains one item per line, `ls -l` remains detailed, and redirected
output remains stable plain text.

The appearance selector exposes Graphite, Bone, Nord, Amber, Moss and Dusk.
Older persisted `paper` and `midnight` values migrate to Bone and Nord when
loaded, so existing users keep a close visual equivalent.

The **Démarrage** settings can run Fastfetch before the prompt in each PTY
session. Fastfetch is an optional, separately installed executable: install the
package for Windows, macOS or Linux and make sure `fastfetch` is available in
the application's `PATH`. Save the setting and restart the active session, or
open a new tab, to apply it. It is disabled by default. If the executable is
missing or exits with an error, the terminal reports the issue and continues to
the prompt.

Terminal glyph metrics use integer letter spacing and a stable line height.
The desktop waits for local fonts to settle, clears xterm's glyph cache, and
refits after font, density, tab, viewport or window changes. This keeps the
cursor aligned with typed text without platform-specific pixel offsets.

## Terminal regression checklist

Run these checks in the real desktop window, not only in the browser preview:

1. Enter a command, use `Up` and `Down` to recall it, move the cursor within it,
   edit it, and execute it.
2. Run `cd` to a directory with and without spaces, then `pwd`; repeat with
   `cd -` and verify the prompt follows the current directory.
3. Type a command longer than the visible row, move the cursor, insert and
   delete characters, then repeat after resizing the window.
4. Use `Shift+Enter` after a trailing backtick, continue typing, and execute;
   verify the shell remains responsive afterward.
5. Print accented text, CJK text and emoji, then compare cursor placement and
   wrapping at several window sizes.
6. If Fastfetch is installed, run `ff` and inspect the reported GPU line; note
   the OS, window size and displayed PTY grid dimensions if it wraps.
7. Select each theme in settings, verify the live terminal colors, save, restart
   the active session and relaunch the app to check persistence.
8. Open multiple tabs, run `pwd` and change directory in each, switch between
   them, close one, and verify the remaining session is unaffected.
9. Run an interactive full-screen program where available, resize the window,
   exit the program, and confirm the shell prompt and input are restored.
10. Close a session while a long-running child is active, then close and reopen
    the application; verify the session ends and no child remains.

## Frontend-only preview

```bash
npm run dev --prefix desktop
npm run audit:ui --prefix desktop
npm run build --prefix desktop
```

The browser preview explicitly reports that no native session exists. It must
never fabricate command output. Use it for layout work only; use
`npm run tauri --prefix desktop -- dev` whenever terminal input, commands, tabs
or native settings need to be tested.

## Native UI migration prototype

The product target is a compiled Rust desktop UI without a WebView. The
experimental application lives in `desktop/native-prototype/` and does not
replace the Tauri app yet. It uses `eframe`/`egui`, the `egui_tty` terminal
widget, Ghostty's VT engine and the existing Rust interactive runner over a
real `portable-pty` session.

Building the Ghostty VT engine requires Zig 0.15.x on `PATH`:

```bash
zig version
cargo run -p terminal-rust-native-prototype
```

The prototype follows the operating system's light/dark preference by default.
Launch-time overrides are available for one-off appearance checks:

```powershell
cargo run -p terminal-rust-native-prototype -- --theme dark
cargo run -p terminal-rust-native-prototype -- --theme light --color "1=#FF4050"
```

The native window starts at 1000 × 640 points and stays at least 800 points
wide so common hardware summary lines fit without wrapping at the default font
size. Its header shows the PTY grid dimensions; output wider than the current
grid still wraps as normal terminal output.

Inside the terminal, `appearance show` inspects the active settings.
`appearance theme system`, `appearance theme dark` or `appearance theme light`
select the color mode. `appearance foreground "#RRGGBB"`,
`appearance background "#RRGGBB"` and `appearance color <index> "#RRGGBB"`
(index 0-15) customize individual colors. These settings persist across
launches and update the current session. `appearance reset` restores the system
theme and vivid default ANSI palette. Settings live in `shell-state.toml`; the
native window watches that file for changes.

The prototype has one real shell session, selection-to-copy and right-click
paste. Its right-click paste strips terminal control characters and wraps text
in bracketed-paste markers. It does not yet have the production app's tabs,
full settings UI, accessibility review, packaging, OS default-terminal
registration or cross-platform validation. Keep using the Tauri app for normal
desktop use.

The prototype's Rust entry point delegates argument handling to `launch.rs`,
window and rendering behavior to `app.rs`, and PTY lifecycle and input to
`pty.rs`. Shared appearance settings and ANSI palette generation live in
`src/config/appearance.rs`; `src/config/shell_state.rs` owns persistence for
aliases, startup scripts and appearance preferences.

The supported Tauri adapter keeps PTY/session lifecycle and event serialization
in `desktop/src-tauri/src/pty.rs`; `lib.rs` wires those commands into Tauri and
retains desktop runtime setup. The browser terminal remains in `desktop/src/`
and communicates only through the typed PTY commands and events.

Do not declare the native app the default terminal or remove Tauri until the
Windows, Fedora/Linux and macOS migration gates in
[ADR-0006](../decisions/0006-native-desktop-ui-prototype.md) pass.

## Rust validation

```bash
cargo test -p terminal-rust-desktop
cargo test -p terminal-rust-desktop --test pty_shell
cargo test -p terminal-rust-desktop --test pty_shell \
  desktop_performance_probe_reports_three_trials -- --ignored --nocapture
```

The manual probe records three real PTY prompt and `pwd` round trips. Repeat the
same commands and manual path on Windows, macOS and Linux before claiming full
desktop support.

## Security boundary

Capabilities expose only the typed Tauri commands required by the interface.
The CSP allows local application and development IPC origins and no remote
content. Rust validates session IDs, dimensions and settings. The WebView has no
generic process or filesystem API.

Settings are stored atomically in the platform-native configuration directory.
Invalid JSON, versions or values report a diagnostic and use safe defaults.
