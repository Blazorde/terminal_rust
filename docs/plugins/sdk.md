# Minimal plugin SDK

The `terminal-rust-plugin-sdk` crate is the public, portable boundary used by
the example template. It depends on neither the shell core, Tauri, nor a plugin
runtime. Its structures are serializable so a future adapter can transport
messages through WebAssembly or an isolated process without changing the
business contract.

The SDK exposes:

- `HOST_API_VERSION` and `ApiVersion`;
- `CommandRequest`, parsed arguments, and a filtered `PluginContext`;
- `CommandResponse`, `CommandStatus`, and stdout/stderr `OutputChunk` values;
- `ResourceToken`, which avoids exposing a raw host path;
- `PluginError`, with a stable code and retryable flag;
- the minimal `Plugin` trait for testing an implementation without a transport.

The trait is not a dynamic Rust ABI, and the core does not load the template's
`.rlib`. A future runtime will adapt serialized messages to this logic. This
keeps security and compatibility independent from Rust ABI stability.

Read the [beginner tutorial](getting-started.md) to build a plugin step by step,
or the [template README](../../plugins/example-plugin/README.md) for the shortest
copy/build/test path.
