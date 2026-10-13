# Vision for an extensible Terminal Rust

Status: long-term direction; the current MVP executes only safe declarative
plugin commands. See [plugin architecture](plugins/architecture.md) and
[ADR-0006](decisions/0006-plugin-boundary.md).

## Intent

Terminal Rust can grow from an educational mini-shell into a customizable
platform without losing a shared Rust engine, explicit behavior, portability or
graceful fallback. The useful lessons from Neovim are a stable API, named
extension points, composable configuration and lazy loading—not unrestricted
implicit script authority.

```text
user configuration -> plugin registry -> validated manifest -> granted permissions
                              │                    │
                              └-> typed messages -> isolated runtime adapter
                                      ├ commands
                                      ├ prompt segments
                                      ├ completion
                                      ├ themes and shortcuts
                                      └ session events
```

The engine never loads an untrusted dynamic library directly. An adapter maps
plugin messages to the same bounded services used by CLI and GUI.

## Extension points

Declarative themes, commands, prompt segments, completion and shortcuts come
first. Session events, GUI panels and developer tools remain long-term and use
closed event lists or validated view models rather than WebView access.

By default, a plugin cannot read arbitrary paths or the full environment,
history or secrets; access network or processes; invoke a shell; receive PTY,
Tauri or engine handles; shadow a native command; block the prompt; change its
permissions; or remain registered after shutdown failure.

Sensitive capabilities are deny-by-default, manifest-declared and explicitly
granted. Paths are normalized and scoped; calls have deadlines, cancellation
and quotas. A crash or invalid result disables the plugin, not the shell.

## Contracts before runtime

Stabilize identity and provenance, host/API compatibility, closed capabilities,
versioned DTOs, deterministic lifecycle, stable errors and privacy-preserving
observability before choosing a code runtime. WebAssembly with WIT or a
serialized child process is preferable to a dynamic Rust ABI.

Discovery reads manifests only; runtime startup is lazy. Prompt segments have a
short budget and may reuse cached output. Ordering is deterministic: native
core, official plugins, user plugins, then explicit permitted overrides.

The roadmap moves from the current engine, themes and validated TOML through
declarative registry, permission and official-plugin proofs before any isolated
runtime. Rhai may later serve explicitly trusted local scripting; it does not
replace the WebAssembly preference for untrusted plugins.

A runtime is acceptable only after Windows, macOS and Linux prove crash
isolation, denied permissions, deadlines, hot disable, version compatibility
and startup cost. Until then, extensions remain declarative.
