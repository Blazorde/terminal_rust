# Terminal Rust documentation

This directory is the technical documentation entry point. The root README is
product-oriented; design and contribution details live here.

The repository also keeps the source of its navigable GitHub wiki in
[`wiki/`](../wiki/). Repository Markdown is authoritative; the GitHub wiki is a
published mirror.

## Documentation map

| Goal | Document |
| --- | --- |
| Understand the system | [Architecture overview](architecture/overview.md) |
| Use the shared engine | [Shell engine API](architecture/shell-engine.md) |
| Understand the desktop boundary | [Desktop GUI architecture](architecture/desktop-gui.md) |
| Configure the shell | [User configuration](configuration.md) |
| Use the live system dashboard | [Dashboard](dashboard.md) |
| Use completion and inline suggestions | [Completion](completion.md) |
| Customize the prompt | [Prompt customization](customization.md) |
| Create user commands | [Custom commands](custom-commands.md) |
| Create a theme | [Theme format](themes.md) |
| Understand plugin direction | [Extensibility vision](vision-plugins.md) |
| Build a first plugin | [Beginner plugin tutorial](plugins/getting-started.md) |
| Review plugin security | [Permissions and security](plugins/security.md) |
| Develop the desktop app | [Desktop guide](development/desktop.md) |
| Review GUI evidence | [GUI quality audit](development/gui-quality-audit.md) |
| Check portability | [Cross-platform guide](architecture/cross-platform.md) |
| Review decisions | [ADR register](decisions/README.md) |
| Contribute | [Contribution guide](development/contributing.md) |
| Test a change | [Testing strategy](development/testing.md) |
| Validate the MVP | [MVP quality checklist](development/mvp-quality-checklist.md) |
| Install Terminal Rust | [Installation](install.md) |
| Troubleshoot | [Troubleshooting](troubleshooting.md) |
| Build a release | [Release builds](release.md) |
| Prepare the first v1 alpha snapshot | [v1.0.0-alpha.1 release snapshot](development/v1-release-snapshot.md) |
| Review security and dependency policy | [Security](security.md) |
| Process a ticket | [Ticket workflow](agents/ticket-workflow.md) |
| Configure GitHub | [Workflows and protections](workflows/README.md) |
| Synchronize the GitHub wiki | [Wiki development page](../wiki/Development.md#synchronize-this-wiki) |

## Principles

- Documentation describes the current state or clearly labels future work.
- Structural decisions receive an ADR instead of an isolated pull-request note.
- Examples run on Windows, macOS and Linux or state their platform explicitly.
- Internal links remain relative to the repository root for GitHub and editors.
