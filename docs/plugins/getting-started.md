# Build your first Terminal Rust plugin

This tutorial assumes you are learning Rust. You need a recent stable Rust
toolchain and a checkout of Terminal Rust. You do not need to understand the
shell internals, Tauri, PTYs, or the future plugin runtime.

> Current limitation: the repository can validate, discover, list, and unit-test
> plugins, but it does not execute third-party plugin code yet. The tutorial
> produces a complete plugin implementation and proves its contract locally.

## 1. Understand the four pieces

The example contains only four important files:

```text
plugins/example-plugin/
├── Cargo.toml
├── README.md
├── terminal-rust-plugin.toml
└── src/
    └── lib.rs
```

- `Cargo.toml` describes the Rust crate and depends on the small public SDK.
- `terminal-rust-plugin.toml` is data read by the host before any code runs.
- `src/lib.rs` implements one typed command and its unit test.
- `README.md` records how to build, test, and share the plugin.

## 2. Copy and rename the template

Copy `plugins/example-plugin` to a separate directory. For this tutorial, use
the ID `learning.greeting` and crate name `learning-greeting-plugin`.

Update the package section:

```toml
[package]
name = "learning-greeting-plugin"
version = "0.1.0"
edition = "2024"
```

When developing inside this repository, keep the SDK path dependency:

```toml
[dependencies]
terminal-rust-plugin-sdk = { path = "../../crates/terminal-rust-plugin-sdk" }
```

If you move the plugin elsewhere, adjust the relative path to your Terminal
Rust checkout. A published SDK dependency can replace it only after the SDK is
published.

## 3. Write the manifest

Start with the smallest command manifest:

```toml
schema_version = 1
id = "learning.greeting"
name = "Greeting"
version = "0.1.0"
description = "Greets a developer"
host_api = "^1.0"
terminal_version = ">=0.3, <2.0"
authors = ["Your name"]
permissions = ["command.register", "output.write"]

[[commands]]
name = "greet"
summary = "Greet one person"
```

The ID uses lowercase ASCII letters, digits, `.`, `_`, and `-`, with an
alphanumeric first and last character. Treat it as permanent after sharing the
plugin. The manifest and crate versions should always match.

The compatibility ranges say what you tested. Do not use `*` for a published
plugin. See the [compatibility policy](compatibility.md).

Permissions are requests, not automatic grants. This command needs to register
itself and return rendered output, so it requests exactly two permissions. See
the [security policy](security.md) before adding anything else.

## 4. Implement one command

The SDK receives parsed arguments and returns typed output. A compact command
looks like this:

```rust
#![forbid(unsafe_code)]

use terminal_rust_plugin_sdk::{
    CommandRequest, CommandResponse, CommandStatus, OutputChunk, OutputStream,
    Plugin, PluginError,
};

#[derive(Default)]
pub struct GreetingPlugin;

impl Plugin for GreetingPlugin {
    fn execute(
        &mut self,
        request: CommandRequest,
    ) -> Result<CommandResponse, PluginError> {
        if request.contribution_id != "learning.greeting/greet" {
            return Err(PluginError {
                code: "unknown_contribution".into(),
                message: "unknown greeting contribution".into(),
                retryable: false,
            });
        }

        let name = request.arguments.first().map_or("world", String::as_str);
        Ok(CommandResponse {
            status: CommandStatus::Success,
            output: vec![OutputChunk {
                stream: OutputStream::Stdout,
                text: format!("Hello, {name}!\n"),
                semantic_style: Some("success".into()),
            }],
            directory_change: None,
        })
    }
}
```

Important boundaries:

- the contribution ID is `<plugin-id>/<command-name>`;
- arguments are already split; do not parse a shell command string;
- return semantic styles, not ANSI escape sequences;
- return a `PluginError` for expected failures instead of panicking;
- a directory change uses a host-issued resource token, never a raw path.

## 5. Add a focused test

Create a `CommandRequest`, call the plugin directly, and assert the public
result. The repository template contains a complete test you can rename. At a
minimum, test the normal response and an unknown contribution. Add boundary
tests whenever you accept arguments.

Run the checks from the copied plugin directory:

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

## 6. Test discovery locally

Find the platform data directory:

| Platform | Plugin root |
| --- | --- |
| Windows | `%LOCALAPPDATA%\terminal-rust\plugins` |
| macOS | `$HOME/Library/Application Support/terminal-rust/plugins` |
| Linux | `$XDG_DATA_HOME/terminal-rust/plugins`, or `$HOME/.local/share/terminal-rust/plugins` |

Create this exact versioned path and copy the manifest into it:

```text
<plugin-root>/learning.greeting/0.1.0/terminal-rust-plugin.toml
```

Enable the ID in the Terminal Rust user configuration:

```toml
[plugins]
enabled = ["learning.greeting"]

[plugins.grants]
"learning.greeting" = ["command.register", "output.write"]
```

Then run:

```bash
terminal_rust plugins list
```

Check the ID, version, `enabled` state, path, and requested permissions. If it
is `incompatible`, the next line tells you which API or application range to
change. The `greet` command is not exposed yet because there is no isolated
runtime; discovery alone never loads your Rust library.

## 7. Declare a prompt segment later

A manifest can reserve a prompt contribution:

```toml
permissions = ["prompt.segment.register"]

[[prompt_segments]]
name = "greeting"
summary = "Short greeting state"
timeout_ms = 20
cache_ttl_ms = 30000
```

This declaration is validated but not executed today. Prompt segments have a
strict time budget and return plain typed values; Terminal Rust owns colors,
icons, separators, and ANSI rendering.

## Common errors

| Symptom | Cause and fix |
| --- | --- |
| `invalid manifest ... missing field` | Add the named required key and keep TOML types exact. |
| `unknown variant` for a permission | Use an identifier from the permission catalog; unknown permissions fail closed. |
| `commands require command.register` | Add `command.register` or remove the command contribution. |
| `duplicate permission` | Keep each permission once. |
| Plugin is `disabled` | Add the exact ID to `plugins.enabled`; installation alone does not enable it. |
| Plugin is `incompatible` | Read the following API/application diagnostic and update the correct range or component. |
| Plugin is absent | Check the native data root and the `<id>/<version>/terminal-rust-plugin.toml` layout. |
| Unit test cannot find the SDK | Correct the relative SDK path in `Cargo.toml`. |
| Command does not run in the shell | Expected until the isolated runtime milestone; test the trait directly. |

## Debugging habits

- Keep request and response fixtures small and deterministic.
- Assert error codes as well as user messages.
- Never put secrets, tokens, or personal absolute paths in fixtures.
- Test paths with spaces and Unicode when the plugin eventually uses resource
  tokens.
- Test on Windows, macOS, and Linux before claiming portability.
- Keep `unsafe` forbidden and treat all host input as untrusted.

## Share responsibly

Before sharing, choose a license, document supported platforms and ranges,
minimize permissions, run strict checks, and review the manifest as users will
see it. The [local catalog design](catalog.md) describes a future distribution
flow; until it is implemented, sharing is manual and must not imply trusted or
automatic installation.
