# Terminal Rust

<div align="center">

<img src="docs/assets/terminal-rust-cover.png" alt="Terminal Rust cover: the command prompt mark surrounded by native terminal workspaces" width="100%">

**A compact, cross-platform shell with a real Rust core and a focused desktop workspace.**

[![Release](https://img.shields.io/github/v/release/Blazorde/terminal_rust?display_name=tag&style=flat-square&color=ff6b3d)](https://github.com/Blazorde/terminal_rust/releases)
[![Rust](https://img.shields.io/badge/Rust-2024-edition?style=flat-square&logo=rust&logoColor=white&color=30353a)](Cargo.toml)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-30353a?style=flat-square)](#platform-support)
[![License](https://img.shields.io/badge/license-MIT-30353a?style=flat-square)](LICENSE)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-support%20the%20project-ff6b3d?style=flat-square&logo=kofi&logoColor=white)](https://ko-fi.com/blazorde)

Real commands. Native PTY sessions. Git-aware prompts. No simulated terminal.

[Quick start](#quick-start) · [Download](#download-the-alpha) · [Wiki](https://github.com/Blazorde/terminal_rust/wiki) · [Contribute](#open-source-and-contributions)

</div>

---

## What ships

| Surface | Highlights |
| --- | --- |
| **Interactive shell** | History, completion, persistent aliases, `.trs` scripts and external processes |
| **Desktop app** | Real isolated PTY tabs, responsive settings and persisted themes |
| **Prompt engine** | Configurable path, Git, status, runtime, duration and time segments |
| **Developer tools** | Live system dashboard, optional Fastfetch at desktop session startup, full native Git and bundled Nano/Neovim |
| **Extension foundation** | Versioned manifests, permissions and declarative commands |

Terminal Rust is intentionally smaller than Bash, PowerShell or Zsh. Rust owns
commands, processes, paths and configuration; the CLI and desktop app share the
same behavior.

## Quick start

From the repository root, choose how you want to use Terminal Rust:

| Mode | Command | Use it when |
| --- | --- | --- |
| **Interactive shell (CLI)** | `cargo run` | You want the shell in the current PowerShell/terminal window. Type `help` to see commands; use `exit` to leave. |
| **One command (CLI)** | `cargo run -- dashboard` | You want to run a command once and return to your existing terminal. Replace `dashboard` with another Terminal Rust command. |
| **Desktop app (Tauri)** | `npm run tauri --prefix desktop -- dev` | You want the supported native window, with real terminal sessions, tabs and settings. Install the [desktop prerequisites](docs/development/desktop.md#prerequisites) first; run `npm ci --prefix desktop` once to install its frontend dependencies. |
| **Native UI prototype** | `cargo run -p terminal-rust-native-prototype` | You want to try the experimental Rust-only desktop UI. It requires Zig 0.15.x and is not a replacement for the Tauri app; see the [prototype notes](docs/development/desktop.md#native-ui-migration-prototype). |
| **Browser UI preview** | `npm run dev --prefix desktop` | You are working on the frontend layout only. It does not start a real shell or execute commands. |

The Rust commands require stable Rust and Cargo. For the desktop app, follow
the Tauri prerequisites linked above.

### CLI examples

```bash
git clone https://github.com/Blazorde/terminal_rust.git
cd terminal_rust
cargo run
```

The last command starts the interactive shell in the current terminal. Run
`help` inside it for the authoritative command list. Useful starting points
include `dashboard`, `theme list`, `plugins list` and `exit`. To run a command
without starting an interactive session:

```bash
cargo run -- pwd
cargo run -- ls -la
cargo run -- dashboard
```

### Launch the supported desktop window

Install the [Tauri prerequisites](docs/development/desktop.md#prerequisites),
then install the frontend dependencies once and launch:

```bash
npm ci --prefix desktop
npm run tauri --prefix desktop -- dev
```

This opens the native application and real PTY sessions. The browser-only
`npm run dev --prefix desktop` preview is for frontend layout work and cannot
execute commands. The separate native UI prototype is experimental; see its
[development notes](docs/development/desktop.md#native-ui-migration-prototype).

## Download the alpha

[`v1.0.0-alpha.1`](https://github.com/Blazorde/terminal_rust/releases/tag/v1.0.0-alpha.1)
is a prerelease and does not replace the latest stable version, `v0.3.0`.
Verify downloads with the attached `SHA256SUMS.txt`.

| Platform | Available alpha packages |
| --- | --- |
| Windows x86_64 | Portable `.zip` and NSIS `.exe` |
| Linux x86_64 | Portable `.tar.gz`, `.deb` and `.AppImage` |
| macOS | Build from source; the native-tested package is still pending |

The packages are currently unsigned. Read the
[installation guide](docs/install.md) before granting an operating-system
exception.

## Documentation

The [GitHub wiki](https://github.com/Blazorde/terminal_rust/wiki) is the short,
task-oriented guide. Detailed technical references remain versioned in
[`docs/`](docs/README.md).

| Topic | Start here |
| --- | --- |
| Commands, completion, tabs and dashboard | [Usage](https://github.com/Blazorde/terminal_rust/wiki/Usage) |
| Prompt, themes, icons and custom commands | [Configuration](https://github.com/Blazorde/terminal_rust/wiki/Configuration) |
| Installation and native window | [Installation](https://github.com/Blazorde/terminal_rust/wiki/Installation) |
| Engine and desktop boundaries | [Architecture](https://github.com/Blazorde/terminal_rust/wiki/Architecture) |
| Plugin model and limitations | [Plugins](https://github.com/Blazorde/terminal_rust/wiki/Plugins) |
| Build, test and contribute | [Development](https://github.com/Blazorde/terminal_rust/wiki/Development) |
| Security and releases | [Security](https://github.com/Blazorde/terminal_rust/wiki/Security) · [Releases](https://github.com/Blazorde/terminal_rust/wiki/Releases) |

## Open source and contributions

Terminal Rust is open source under the MIT License. Issues and pull requests
are welcome when they follow the [contribution guide](docs/development/contributing.md),
project documentation, cross-platform requirements and private
[security reporting process](SECURITY.md).

Start focused work from `develop`, include appropriate tests and document
user-visible behavior. Bug reports are most useful with reproducible steps,
expected behavior and the affected operating system.

## Platform support

Windows, macOS and Linux are first-class targets in the architecture. The
`v1.0.0-alpha.1` packages have native runtime evidence on Windows and Linux.
The macOS code path and build configuration exist, but the native bundle and
PTY launch still require validation on Apple hardware.

## Project status

The current product snapshot is `v1.0.0-alpha.1`. Plugin loading remains
deliberately bounded: manifests, compatibility, permissions and safe
declarative commands are supported, while arbitrary third-party native
libraries are not loaded. See [CHANGELOG.md](CHANGELOG.md) for delivered work
and known limits.

## License and support

Terminal Rust is available under the [MIT License](LICENSE). Locked dependency
licenses are recorded in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
If the project is useful to you, you can [support it on Ko-fi](https://ko-fi.com/blazorde).
