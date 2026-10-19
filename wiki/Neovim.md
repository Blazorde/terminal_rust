# Neovim configuration

Download Blazorde's public [CodeOpsHQ Neovim configuration](https://github.com/Blazorde/terminal_rust/tree/feat/bundled-editors/config/nvim).
The install scripts fetch only this folder from the public repository and back
up an existing configuration before replacing it.

## Install on Windows

Open PowerShell and run:

```powershell
$installer = Join-Path $env:TEMP 'install-terminal-rust-nvim.ps1'
Invoke-WebRequest -Uri 'https://raw.githubusercontent.com/Blazorde/terminal_rust/feat/bundled-editors/scripts/install-nvim-config.ps1' -OutFile $installer
& $installer
```

The installer needs Git. Terminal Rust desktop releases include Git; run the
same PowerShell commands from inside a Terminal Rust session if Git is not
installed separately. The existing config is moved to a timestamped
`nvim.backup-*` folder under `%LOCALAPPDATA%`.

## Install on macOS or Linux

Run this command in a terminal with Git installed:

```sh
curl -fsSL https://raw.githubusercontent.com/Blazorde/terminal_rust/feat/bundled-editors/scripts/install-nvim-config.sh | sh
```

The config is installed under `${XDG_CONFIG_HOME:-~/.config}/nvim`. Any
existing config is backed up next to it before installation.

## First launch in Terminal Rust

Open Neovim from the Terminal Rust command line:

```text
nvim
```

The first launch bootstraps `lazy.nvim` and downloads the plugins pinned in
`lazy-lock.json`, so it needs an internet connection. In the interactive
profile menu, choose **CodeOpsHQ Full**, **Essential**, **Coding**, **AI**,
**Custom**, or **Craftzdog / LazyVim**. Essential is the lightest profile;
Full is the complete configuration.

The AI profile is optional. Avante reads provider keys from environment
variables; for example, set `DEEPSEEK_API_KEY` or `OPEN_ROUTER_API_KEY` before
starting Terminal Rust. Do not put API keys in the config files. Language
servers and formatters may install on first use through Mason.

See the [configuration source](https://github.com/Blazorde/terminal_rust/tree/feat/bundled-editors/config/nvim)
and the [Terminal Rust editor guide](https://github.com/Blazorde/terminal_rust/blob/feat/bundled-editors/docs/editors.md).
