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
`incompatible` or `error`. Activation comes from `plugins.enabled`. A missing
directory means no plugin is installed; read failures name the affected path.

Discovery stays inside the expected tree, ignores symbolic links and never
follows a path supplied by a manifest. This limits traversal and keeps behavior
consistent across platforms.

Compatible, enabled entries may contribute safe declarative commands when their
permissions are granted. The registry does not load arbitrary native libraries
or start plugin processes.
