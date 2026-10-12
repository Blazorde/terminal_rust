# Target plugin architecture

This design implements the [extensibility vision](../vision-plugins.md),
[public contract](api.md) and [ADR-0006](../decisions/0006-plugin-boundary.md).
The current MVP has no arbitrary third-party code runtime.

A plugin is an identified, versioned package declaring public-API
contributions. It is not a `ShellEngine` reference, an implicitly trusted shell
script or an in-process Rust library.

```text
plugin directory -> ManifestLoader -> ManifestValidator -> PluginRegistry
configuration -> activation -> PermissionSet                 |
ShellEngine <- HostServices <- PluginSupervisor <- PluginRuntime
```

The loader reads only manifests inside a native data root. Validation covers
schema, identity, versions, compatibility, relative paths and permissions. The
registry owns installed versions and resolved contributions but no execution
handle. Effective permissions are the intersection of requested, understood
and user-granted capabilities. Host services are the sole bounded bridge to the
engine. A supervisor applies deadlines, cancellation, size limits, health and
shutdown. The runtime remains an internal replaceable port.

## Storage and lifecycle

```text
<data-dir>/terminal-rust/plugins/<plugin-id>/<version>/
├── terminal-rust-plugin.toml
├── assets/
└── future component.wasm or executable
```

Installation and activation are separate. Future installers write to a
temporary directory and replace atomically where supported; old versions remain
available for explicit rollback until cleanup.

```text
absent -> discovered -> validated -> installed -> enabled -> started
                    \-> rejected              \-> faulted
started -> suspended -> stopped -> disabled
```

Disabling first unregisters contributions, then cancels calls, stops the runtime
and persists state. Faulted plugins remain disabled after restart until explicit
action.

## Core boundary and compatibility

Public DTOs contain bounded UTF-8 strings, numbers, stable identifiers, lists
and versioned objects. They never expose raw `PathBuf`, mutable engine state,
private errors, process/PTY/window handles, full environments, history or
retained callbacks.

Identifiers use stable `publisher.name` form. Manifests declare SemVer, schema
and host-API range. Native commands win name conflicts; conflicting plugins are
both disabled unless explicit policy resolves them. Filesystem discovery order
never chooses a winner.

Package paths are relative, host-normalized and cannot traverse with `..`.
Executable artifacts are target-specific; portable WASM remains common only
when it imports no platform capability. Tests cover spaces, line endings,
Unicode, missing plugins, unknown manifests, incompatible versions, denied
permissions, timeout and shutdown during a call on all three target systems.
