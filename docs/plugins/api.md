# Public plugin contract

Status: design contract for API `1.0`; executable third-party code is not loaded.

The public API is a versioned message protocol, independent of runtime and
language. Rust may use Serde, WebAssembly may use WIT and a process may use JSON;
the same fields and rules apply. Major versions break required semantics; minor
versions add optional, ignorable fields or capabilities. Activation stops before
startup when no compatible major exists.

## Common envelope

```rust
struct PluginRequest {
    api_version: ApiVersion,
    request_id: RequestId,
    plugin_id: PluginId,
    deadline_ms: u64,
    body: RequestBody,
}

struct PluginResponse {
    api_version: ApiVersion,
    request_id: RequestId,
    body: Result<ResponseBody, PluginError>,
}
```

Identifiers and resource tokens are opaque. A response repeats its request ID;
late or unknown responses are ignored and counted as runtime faults.

## Context and capabilities

Context is an immutable snapshot containing only the current session token,
display path, explicitly granted environment names, terminal capabilities, last
status and this plugin's settings. It exposes no history, edit buffer, other
session, launch arguments, secret, PTY or Tauri object. Filesystem operations
use host-issued resource tokens, never reconstructed display strings.

Initial capabilities are `context.cwd.read`, `context.env.read`, `config.read`,
`command.register`, `prompt.segment.register`, `completion.register` and
`output.write`. `filesystem.read`, `filesystem.write`, `process.spawn` and
`network.connect` are outside API 1.0 and require a later decision, scoped
parameters and consent UI.

Commands receive host-parsed arguments and return typed status, bounded output
chunks and an optional validated directory token. Prompt segments return an
optional UTF-8 value plus semantic style/icon keys and cache duration. Completion
returns bounded candidates. Plugins never provide executable ANSI styling.

## Errors and lifecycle

Errors distinguish invalid request/response, denied permission, unsupported API,
timeout, cancellation, unavailable runtime and plugin failure. User-facing
messages contain no secrets, unauthorized paths or full traces. Plugin failures
never become core panics; repeated faults disable the plugin with recovery help.

Initialization negotiates the API, grants and configuration, then verifies that
reported contributions match the manifest. Shutdown removes contributions,
cancels calls and stops the runtime by a deadline; a plugin cannot veto removal.

The private `PluginRuntime` port may evolve and is not author-facing API. Its
DTOs obey this contract.

## Validation requirements

- Round-trip every DTO, including unknown optional fields.
- Reject incompatible majors before runtime start.
- Expose no `core`, Tauri, PTY or `rustyline` type.
- Test deadline, cancellation and maximum sizes with deterministic clocks.
- Cover spaces, Unicode, LF and CRLF.
- Keep messages equivalent across Windows, macOS and Linux.
- Fuzz message parsing and never `unwrap` plugin-controlled data.
