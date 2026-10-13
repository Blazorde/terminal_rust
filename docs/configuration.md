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

## Minimal example

```toml
version = 1

[prompt]
icons = "never"
```
