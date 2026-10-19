# User configuration

Terminal Rust loads an optional TOML file when an interactive session starts.
Missing configuration is valid and uses the defaults in `config/default.toml`.

## Locations

- Windows: `%APPDATA%\terminal-rust\config.toml`
- macOS: `$HOME/Library/Application Support/terminal-rust/config.toml`
- Linux: `$XDG_CONFIG_HOME/terminal-rust/config.toml`, falling back to
  `$HOME/.config/terminal-rust/config.toml`

`TERMINAL_RUST_CONFIG` can select another absolute or relative native path. A
missing explicit path reports a diagnostic, unlike automatic discovery.

## Version 1 format

The [example configuration](../config/default.toml) documents every option.
Unknown sections or keys, invalid TOML, unsupported versions, unknown segments
and control characters produce a file-specific diagnostic. The shell then
starts with safe defaults.

Missing fields use defaults. Prompt icon policy, segment order and position,
separator, newline mode, path style, Git status and final symbol are applied to
the prompt.

`theme` accepts `default`, `minimal`, `git-focus` and `powerline`. See the
[theme guide](themes.md) for rendering and icon-option precedence.

`[plugins]` controls extension activation and permission grants. A plugin is
inactive unless its identifier appears in `enabled`. See
[plugin configuration](plugins/configuration.md).

`[commands]` defines structured aliases for built-ins or external processes.
See [custom commands](custom-commands.md) for syntax and security boundaries.
Interactive aliases and `.trs` scripts are also documented in that guide.
Their persistent state is kept in `shell-state.toml` beside `config.toml` so
the versioned TOML configuration remains backward-compatible.

The desktop app's **Réglages → Démarrage** option can launch the separately
installed `fastfetch` command before each PTY session's first prompt. It is
disabled by default and requires `fastfetch` to be available in the desktop
app's `PATH`. The option is stored with the desktop appearance settings rather
than in this CLI TOML file; it applies to new sessions (or after restarting the
current one). Missing Fastfetch is reported without preventing the shell from
starting. See the [desktop guide](development/desktop.md).

## Minimal example

```toml
version = 1

[prompt]
icons = "never"
```
