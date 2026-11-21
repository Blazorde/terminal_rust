# Local plugin registry

The local registry inventories manifests without loading libraries or starting
processes. The canonical layout is:

```text
<data-dir>/terminal-rust/plugins/<plugin-id>/<version>/terminal-rust-plugin.toml
```

| System | `<data-dir>` |
| --- | --- |
| Windows | `%LOCALAPPDATA%` |
| macOS | `$HOME/Library/Application Support` |
| Linux | `$XDG_DATA_HOME`, falling back to `$HOME/.local/share` |

`plugins list` shows every version deterministically as `enabled`, `disabled`,
`incompatible`, `superseded` or `error`. Activation comes from `plugins.enabled`.
A missing directory means no plugin is installed.

- The `id` in a manifest must be the name of its `<plugin-id>` folder: grants and
  the enabled list are keyed by that id. A manifest that disagrees is reported as
  an `error`.
- When several compatible versions of one plugin are installed, only the newest
  is used and the others are `superseded`. Without this rule an upgrade would
  block every command of the plugin as a name conflict.
- A folder that cannot be read is listed as an `error` with its path and cause,
  and the other plugins keep working. Only an unreadable plugin root fails the
  discovery.

Discovery stays inside the expected tree, ignores symbolic links and never
follows a path supplied by a manifest. This limits traversal and keeps behavior
consistent across platforms.

Compatible, enabled entries may contribute safe declarative commands when their
permissions are granted. The registry does not load arbitrary native libraries
or start plugin processes.
