# Terminal Rust documentation

This directory is the technical documentation entry point. The root README is
product-oriented; design and contribution details live here.

The same user-facing guides are also published in the
[GitHub wiki](https://github.com/Blazorde/terminal_rust/wiki). Repository
Markdown remains authoritative.

## Documentation map

| Goal | Document |
| --- | --- |
| Understand the system | [Architecture overview](architecture/overview.md) |
| Use the shared engine | [Shell engine API](architecture/shell-engine.md) |
| Understand the desktop boundary | [Desktop GUI architecture](architecture/desktop-gui.md) |
| Configure the shell | [User configuration](configuration.md) |
| Use the live system dashboard | [Dashboard](dashboard.md) |
| Use native Git commands | [Git commands](git.md) |
| Use Nano or Neovim in the terminal | [Bundled editors](editors.md) |
| Download the public Neovim configuration | [Neovim setup](../wiki/Neovim.md) |
| Use completion and inline suggestions | [Completion](completion.md) |
| Customize the prompt | [Prompt customization](customization.md) |
| Create user commands | [Custom commands](custom-commands.md) |
| Create a theme | [Theme format](themes.md) |
| Understand plugin direction | [Extensibility vision](vision-plugins.md) |
| Build a first plugin | [Beginner plugin tutorial](plugins/getting-started.md) |
| Review plugin security | [Permissions and security](plugins/security.md) |
| Develop the desktop app | [Desktop guide](development/desktop.md) |
| Check portability | [Cross-platform guide](architecture/cross-platform.md) |
| Review decisions | [ADR register](decisions/README.md) |
| Contribute | [Contribution guide](development/contributing.md) |
| Test a change | [Testing strategy](development/testing.md) |
| Install Terminal Rust | [Installation](install.md) |
| Troubleshoot | [Troubleshooting](troubleshooting.md) |
| Build a release | [Release builds](release.md) |
| Review security and dependency policy | [Security](security.md) |
| Configure GitHub | [Workflows and protections](workflows/README.md) |

## Principles

- Documentation describes the current state or clearly labels future work.
- Structural decisions receive an ADR instead of an isolated pull-request note.
- Examples run on Windows, macOS and Linux or state their platform explicitly.
- Internal links remain relative to the repository root for GitHub and editors.
