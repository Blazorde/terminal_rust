# Terminal Rust plugin template

This directory is a minimal copyable template. It contains a registry-compatible
manifest, one `hello` command, and a unit test. The code uses only SDK message
types and does not choose the future transport between the host and plugin.

For a guided explanation, start with the
[beginner tutorial](../../docs/plugins/getting-started.md).

## Copy the template

1. Copy this directory outside `plugins/example-plugin`, or rename it.
2. Change `name` in `Cargo.toml`.
3. Replace `id`, `name`, `version`, `description`, and `authors` in
   `terminal-rust-plugin.toml`.
4. Update `example.hello/hello` and the response in `src/lib.rs`.
5. Keep only permissions the plugin actually needs.

The plugin ID is stable and should not change after publishing. The manifest
and crate versions must remain identical.

## Build and test

From the repository root:

```bash
cargo check -p terminal-rust-example-plugin --all-targets
cargo clippy -p terminal-rust-example-plugin --all-targets -- -D warnings
cargo test -p terminal-rust-example-plugin --all-targets
```

The test calls the implementation directly and checks its typed response. Core
tests also parse the manifest. The template is not dynamically loaded: the
isolated runtime remains a separate milestone.

## Test local discovery

Create the following tree in the
[local plugin directory](../../docs/plugins/registry.md), then copy the manifest:

```text
terminal-rust/plugins/example.hello/0.1.0/terminal-rust-plugin.toml
```

Enable and authorize the declarative command, then run:

```toml
[plugins]
enabled = ["example.hello"]

[plugins.grants]
"example.hello" = ["command.register", "output.write"]
```

```bash
terminal_rust plugins list
```

The plugin should appear as `enabled` with its requested permissions. The static
`hello` response is executable now. The host still never loads the copied Rust
library; its typed implementation remains a testable contract for the future
isolated runtime.

## Sharing checklist

- valid, stable ID and SemVer;
- matching manifest and crate versions;
- tested API and Terminal Rust compatibility ranges;
- minimum requested permissions;
- no secret or personal path in examples and tests;
- formatting, strict Clippy, and tests pass;
- concise, actionable user errors;
- behavior documented for Windows, macOS, and Linux.
