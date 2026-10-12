# Plugin permissions and security

Terminal Rust treats every plugin as untrusted. Installation never means
activation, a manifest request never means consent, and an enabled plugin gets
only the intersection of permissions it requested and the user granted.

No third-party runtime is active yet. The permission gate in this milestone is
the host-side contract that every future host service must call before doing
work. It fails closed and returns a user-facing reason.

## Permission catalog

| Permission | Capability | Initial policy |
| --- | --- | --- |
| `context.cwd.read` | Read the display form of the working directory | Read-only, opaque resource token for operations |
| `context.env.read` | Read explicitly named environment values | No bulk environment access |
| `config.read` | Read this plugin's configuration | No access to other plugin settings |
| `command.register` | Register a command contribution | Native commands always win conflicts |
| `completion.register` | Register completion candidates | No implicit file-system access |
| `output.write` | Return text and semantic styles | ANSI supplied by a plugin is treated as text |
| `prompt.segment.register` | Register a prompt segment | Strict timeout and output limits |
| `filesystem.read` | Ask an host service to read files | Sensitive; denied until scope and user consent exist |
| `filesystem.write` | Ask an host service to write files | Sensitive; denied until scope and user consent exist |
| `process.spawn` | Ask an host service to start a process | Sensitive; no implicit system shell |
| `network.connect` | Ask an host service to connect to a network endpoint | Sensitive; denied until endpoint scope and consent exist |

The four sensitive permissions can be declared now so installation and review
can expose intent before activation. They do not create a file, process, or
network service. Implementing one requires a separate scoped host-service
design and tests on Windows, macOS, and Linux.

## Decision sequence

For every host operation:

1. the permission must be present in `permissions` in
   `terminal-rust-plugin.toml`;
2. the user must have granted that exact permission;
3. the host service must validate the resource scope and request payload;
4. the supervisor must enforce time, output, and resource limits;
5. a denial returns an explicit error naming the plugin and permission.

An item present only in the grant store but absent from the manifest is still
denied. This prevents a stale consent record from silently expanding a new
plugin version's authority. Duplicate permissions make the manifest invalid.

## Review before activation

`plugins list` prints every plugin's requested permissions, including disabled
and incompatible plugins. A future activation screen must show the same sorted
list and distinguish previously granted, newly requested, and removed
permissions. An update that adds a permission returns to a pending-consent state.

Example manifest:

```toml
permissions = [
  "command.register",
  "output.write",
  "filesystem.read",
]
```

This example requests read access; it does not grant it. Until a scoped read
service and explicit consent exist, `filesystem.read` authorization fails with
an explanation.

## Non-negotiable boundaries

- Unknown and duplicate permissions invalidate the manifest.
- Undeclared or ungranted operations are blocked before any side effect.
- Plugins never receive raw handles, unrestricted `PathBuf` values, secrets,
  the complete environment, or another session's state.
- Process requests use a direct executable plus argument vector, never a shell
  command string.
- A permission error never becomes a core panic and does not disable the shell.
