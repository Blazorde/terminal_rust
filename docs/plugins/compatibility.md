# Plugin compatibility policy

Terminal Rust versions both the plugin protocol and the host application. A
plugin manifest must state a SemVer range for each:

```toml
host_api = "^1.0"
terminal_version = ">=0.3, <2.0"
```

`host_api` protects the message and host-service contract. `terminal_version`
protects behavior tied to the application release. The registry evaluates both
ranges before treating an enabled plugin as runnable.

## Compatibility rules

- The host currently provides plugin API `1.0.0`.
- The Terminal Rust version comes from the package version at build time.
- Legacy schema v1 manifests without `terminal_version` are interpreted as `*`
  for backward compatibility; new and republished plugins must declare a tested
  range explicitly.
- Both requirements must match. The API requirement is checked first because a
  protocol mismatch prevents any safe initialization.
- An incompatible plugin remains visible in `plugins list`, but its status is
  `incompatible` and no contribution may be started.
- The diagnostic prints the required range, installed version, and advises the
  user to update either the plugin or Terminal Rust.

Examples:

```text
requires plugin API ^2, but the host provides 1.0.0; update the plugin or Terminal Rust
requires Terminal Rust >=9, but 0.3.0 is installed; update the plugin or Terminal Rust
```

An incompatibility is isolated to that registry entry. It cannot panic the
shell or hide other valid plugins.

## Evolution and deprecation

The plugin API follows Semantic Versioning:

- patch releases fix behavior without changing the contract;
- minor releases may add optional fields or capabilities that older receivers
  can ignore;
- major releases may remove or redefine required behavior.

A feature scheduled for removal must first be documented as deprecated in the
changelog, remain supported for at least one minor compatibility window, and
include its replacement. Removing it requires a new API major version. A plugin
author should choose the narrowest range tested in CI instead of using `*`.

## Compatibility test matrix

Every contract change must cover:

| Case | Expected result |
| --- | --- |
| Current API and current terminal range | `compatible` |
| Future API major | blocked with current and required API versions |
| Older unsupported API major | blocked with current and required API versions |
| Future Terminal Rust requirement | blocked with installed and required app versions |
| Compatible plugin beside incompatible plugin | valid plugin remains available |
| Malformed SemVer requirement | manifest is `error`, no code is loaded |

Before a runtime is described as portable, the same fixtures must pass on
Windows, macOS, and Linux.

Terminal Rust prereleases use the compatibility rules of their corresponding
base release. For example, `1.0.0-alpha.1` satisfies a plugin requirement that
accepts `1.0.0`, while diagnostics still report the complete prerelease version.
