# Terminal Rust

<div align="center">

<img src="docs/assets/terminal-rust-cover.png" alt="Terminal Rust cover: the command prompt mark surrounded by native terminal workspaces" width="100%">

**A compact, cross-platform shell with a real Rust core and a focused desktop workspace.**

[![Release](https://img.shields.io/github/v/release/Blazorde/terminal_rust?display_name=tag&style=flat-square&color=ff6b3d)](https://github.com/Blazorde/terminal_rust/releases/latest)
[![Rust](https://img.shields.io/badge/Rust-2024-edition?style=flat-square&logo=rust&logoColor=white&color=30353a)](Cargo.toml)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-30353a?style=flat-square)](#platform-support)
[![License](https://img.shields.io/badge/license-MIT-30353a?style=flat-square)](LICENSE)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-support%20the%20project-ff6b3d?style=flat-square&logo=kofi&logoColor=white)](https://ko-fi.com/blazorde)

Real commands. Native PTY sessions. Git-aware prompts. No simulated terminal.

[Quick start](#quick-start) · [Desktop app](#desktop-workspace) · [Commands](#command-deck) · [Documentation](docs/README.md) · [Wiki](https://github.com/Blazorde/terminal_rust/wiki)

</div>

---

## At a glance

| Surface | What ships today | Built with |
| --- | --- | --- |
| **Interactive shell** | History, completion, quoted arguments, safe errors and built-in file commands | Rust · `rustyline` |
| **Desktop workspace** | Real isolated PTY tabs, responsive settings and persistent preferences | Tauri 2 · React · xterm.js |
| **Prompt engine** | Configurable left/right segments for path, Git, status, runtime, duration and time | Rust · TOML |
| **Developer view** | System dashboard, safe Git shortcuts, themes and a portable Fastfetch preset | `sysinfo` · Git |
| **Extension foundation** | Local manifests, compatibility checks, explicit permissions and declarative commands | Versioned Rust contracts |

Terminal Rust is intentionally smaller than Bash, PowerShell or Zsh. It is a
pedagogical shell whose behavior stays understandable as it grows: Rust owns
commands, processes, paths and configuration; presentation layers consume
bounded contracts.

## Quick start

```bash
git clone https://github.com/Blazorde/terminal_rust.git
cd terminal_rust
cargo run
```

At the prompt, run `help` to explore the command set, `dashboard` to inspect the
machine, or `exit` to close the session. `Tab` cycles command and path
completions; a dim history suggestion can be accepted with `Right Arrow` or
dismissed with `Left Arrow`. See the [completion guide](docs/completion.md).
A single command can run without an interactive session:

```bash
cargo run -- pwd
cargo run -- ls -la
cargo run -- dashboard
cargo run -- git status --short --branch
```

## Command deck

| Command | Purpose |
| --- | --- |
| `pwd` | Print the current directory |
| `cd [path]` / `cd -` | Change directory or return to the previous one |
| `ls [options] [path...]` | List files with long, recursive, sorting and display options |
| `mkdir [directory]` | Create a directory |
| `touch [file]` | Create an empty file or preserve an existing one |
| `cat [file]` | Print a file |
| `rm [options] [path...]` | Remove files or directories with guarded recursive deletion |
| `clear` | Clear an ANSI-capable terminal |
| `dashboard` | Render CPU, memory, disk, network, process and Git context |
| `git status\|branch\|log` | Run an allowlisted, read-only Git operation |
| `theme list` | List bundled prompt themes |
| `theme preview <name>` | Preview a theme using clearly labeled sample data |
| `plugins list` | Inspect installed plugin manifests and their state |
| `commands list` | Inspect user-defined commands loaded from configuration |
| `help` | Show built-in help |
| `exit` / `quit` | Close the shell |

The built-in `ls` supports `-1`, `-a`, `-A`, `-d`, `-F`, `-h`, `-l`, `-p`,
`-R`, `-r`, `-S` and `-t`. The `rm` implementation deliberately remains more
cautious than common Unix shells: recursive removal asks for confirmation even
when combined with `--force`.

Interactive `ls` wraps entries to the current terminal width and highlights
directories, project manifests, source files, scripts and archives with portable
ANSI colors. Use `ls -1` for one entry per line or redirect output for stable
plain text.

In an interactive terminal, `dashboard` refreshes in place once per second
until you press `q`, `Escape` or `Ctrl+C`. Redirected output and small or dumb
terminals receive one stable snapshot instead. See the [dashboard guide](docs/dashboard.md).

## Desktop workspace

The desktop app opens real native PTY sessions managed by Rust. The WebView
renders terminal output and settings but cannot spawn arbitrary processes or
access the filesystem directly.

### Launch the native window from source

Install the [Tauri prerequisites for your platform](docs/development/desktop.md#prerequisites),
then run these commands from the repository root:

```bash
npm ci --prefix desktop
npm run tauri --prefix desktop -- dev
```

The second command opens the real Terminal Rust application window. Keep that
process running while using the terminal. `npm run dev --prefix desktop` is only
a browser preview: it cannot start a PTY or execute commands.

Each tab owns its PTY, working directory, history and prompt context.

| Shortcut | Action |
| --- | --- |
| `Ctrl/⌘ + T` | Open a session tab |
| `Ctrl/⌘ + W` | Close the active tab |
| `Ctrl + Tab` | Select the next tab |
| `Ctrl/⌘ + ,` | Open settings |
| `Escape` | Close settings |

The settings drawer controls theme, monospace fallback, font size, cursor,
density and the status bar. Preferences are validated and saved through the
native Rust adapter; unreadable or invalid data falls back safely.

> A browser-only frontend build is a presentation preview. Real sessions start
> only inside the Tauri window.

See the [desktop development guide](docs/development/desktop.md) for platform
requirements and current validation limits.

## Prompt and Git context

The prompt is assembled from independent segments inspired by Oh My Posh. A
repository can display its branch and dirty state without making Git a startup
requirement:

```text
C:\workspace\terminal_rust  git:develop >
C:\workspace\terminal_rust  git:develop* >
```

Detached HEADs render as `detached@1a2b3c4`. Missing Git, a non-repository
directory or invalid porcelain output simply hides the segment. Results are
cached briefly and invalidated on directory changes.

ASCII is the default. Optional Nerd Font glyphs can be enabled explicitly:

```powershell
$env:TERMINAL_RUST_ICONS = "always"
cargo run
```

On macOS or Linux, use `export TERMINAL_RUST_ICONS=always`. Set `never` for an
ASCII-only prompt or combine `auto` with `TERMINAL_RUST_NERD_FONT=1` when the
active terminal is known to use a compatible font.

## System dashboard

`dashboard` provides a terminal-native snapshot of the system and current
project context:

- operating system, kernel, host and uptime;
- CPU model, load and logical cores;
- memory, swap and process count;
- current-disk usage;
- received and transmitted network totals;
- working directory and Git repository presence.

```bash
cargo run -- dashboard
```

Interactive terminals receive color and Unicode borders. Redirected output,
`TERM=dumb` and `NO_COLOR` automatically select a plain ASCII rendering.

An optional [Fastfetch preset](config/fastfetch/README.md) provides a matching
system overview without requiring a Nerd Font:

```bash
fastfetch --config config/fastfetch/config.jsonc
```

Fastfetch is not installed or required by Terminal Rust.

## Installation

Download the archive for your platform from the
[latest GitHub release](https://github.com/Blazorde/terminal_rust/releases/latest),
extract it and run `terminal_rust help` (`terminal_rust.exe help` on Windows).
Use `SHA256SUMS.txt` to verify the download.

The `v1.0.0-alpha.1` preview is a prerelease and therefore is not selected by
GitHub's `latest` link. Choose that tag explicitly from the Releases page when
testing the alpha.

| Platform | Archive | CLI installer | Desktop bundle |
| --- | --- | --- | --- |
| Windows x86_64 | `.zip` | `scripts/install.ps1` | NSIS `.exe` |
| Linux x86_64 | `.tar.gz` | `scripts/install.sh` | `.deb` + `.AppImage` |
| macOS Apple Silicon | `.tar.gz` | `scripts/install.sh` | `.dmg` |

There is no prebuilt macOS Intel archive yet; building from source remains
supported with a compatible Rust toolchain. Desktop bundles are stored in the
archive's `desktop/` directory and are currently unsigned. Read the full
[installation guide](docs/install.md) and [troubleshooting guide](docs/troubleshooting.md)
before using an installer in a restricted environment.

## Architecture

```text
CLI / Tauri desktop
        │
        ▼
ShellEngine ── command registry ── built-in and declarative commands
        │
        ├── prompt segments ── Git cache ── theme renderer
        ├── typed command errors
        └── platform adapters ── paths · PTY · settings
```

The CLI and desktop use the same Rust behavior. OS-specific code stays behind
bounded adapters; native paths use `Path` and `PathBuf`; missing optional tools
degrade as unavailable capabilities rather than fatal errors.

Start with the [documentation map](docs/README.md), then explore:

- [architecture overview](docs/architecture/overview.md);
- [prompt engine](docs/architecture/prompt-engine.md);
- [desktop boundary](docs/architecture/desktop-gui.md);
- [plugin architecture](docs/plugins/architecture.md);
- [Architecture Decision Records](docs/decisions/README.md).

## Development

Create focused branches from an up-to-date `develop` branch:

```bash
git switch develop
git pull --ff-only origin develop
git switch -c feat/my-feature
```

Run the complete local quality gate before opening a pull request:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
npm run audit:ui --prefix desktop
npm run build --prefix desktop
```

The [contribution guide](docs/development/contributing.md),
[testing strategy](docs/development/testing.md) and
[branch workflow](docs/workflows/README.md) describe the expected evidence.

Security reports must use the private process in [SECURITY.md](SECURITY.md),
not a public issue. Dependency and license checks are documented in the
[security guide](docs/security.md).

## Platform support

Windows, macOS and Linux are first-class targets. The Rust CLI is covered by
portable unit and integration tests. Desktop work additionally requires native
Tauri prerequisites and platform-specific PTY validation. The
[desktop development guide](docs/development/desktop.md) documents the native
checks; an unexecuted CI job is never reported as platform proof.

## Project status

This product snapshot identifies as `v1.0.0-alpha.1`; the latest stable release
remains `v0.3.0`. Plugin loading remains deliberately limited: the current
foundation validates manifests, compatibility and permission grants, and can
execute safe declarative commands, but it does not load arbitrary third-party
native libraries.

See [CHANGELOG.md](CHANGELOG.md) for delivered changes and documented limits.

## License

Terminal Rust is available under the [MIT License](LICENSE). Locked dependency
license metadata is recorded in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Support

If Terminal Rust is useful to you, you can [support the project on Ko-fi](https://ko-fi.com/blazorde).
