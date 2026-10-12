# Research — scripting runtime

- Status: long-term recommendation, not implemented
- Date: 2026-10-12
- Scope: explicitly approved local automation, separate from third-party plugins

## Recommendation

Rhai is the preferred prototype for local, explicitly trusted scripts. Its Rust
integration is direct, it receives only host-registered functions, and it offers
limits for depth, calls, operations, variables, modules, strings and collections.
This is not a sandbox guarantee: Rhai runs in-process and every exposed Rust
function expands its authority.

WebAssembly Component Model remains the preferred direction in ADR-0006 for
untrusted third-party plugins. Lua is not selected for the first prototype
because the host would need to construct its sandbox policy and maintain an
additional C integration.

| Criterion | Lua 5.4 | Rhai | WebAssembly + Wasmtime |
| --- | --- | --- | --- |
| Beginner use | Compact and well documented | Rust/JS-like syntax | Compilation and tooling required |
| Integration | C API | Embedded Rust crate | Runtime plus WIT bindings |
| Default isolation | Depends on opened libraries | No external access without registration, same process | Isolated memory and explicit imports |
| Resource limits | Host allocator/hooks | Built-in operation and size limits | Fuel, epochs, stack and resource limiter |
| Target use | Not first-tier | Trusted local scripts | Isolated third-party plugins after a spike |

Scripting follows manifest, permissions, compatibility and supervision. Simple
automation should use structured custom commands in the meantime.

## Prototype requirements

A future Rhai spike must use an immutable engine, forbid `unchecked`, expose no
filesystem/network/process APIs by default, cap all documented resources,
support cancellation at host call boundaries, sanitize diagnostics and pass the
same scenarios on Windows, macOS and Linux. It must be removable without
affecting the native shell.

Primary sources: [Rhai sandbox](https://rhai.rs/book/safety/sandbox.html),
[Rhai limits](https://rhai.rs/book/safety/index.html),
[Lua libraries](https://www.lua.org/manual/5.4/manual.html#6),
[Wasmtime security](https://docs.wasmtime.dev/security.html) and
[WIT interfaces](https://component-model.bytecodealliance.org/design/wit.html).
