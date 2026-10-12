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

The appearance selector exposes Graphite, Bone, Nord, Amber, Moss and Dusk.
Older persisted `paper` and `midnight` values migrate to Bone and Nord when
loaded, so existing users keep a close visual equivalent.

Terminal glyph metrics use integer letter spacing and a stable line height.
The desktop waits for local fonts to settle, clears xterm's glyph cache, and
refits after font, density, tab, viewport or window changes. This keeps the
cursor aligned with typed text without platform-specific pixel offsets.

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
