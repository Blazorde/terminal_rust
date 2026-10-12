# ADR-0006 — Declarative, isolatable plugin boundary

- Status: Accepted
- Date: 2026-10-12
- Decision makers: Terminal Rust team

## Context

Extensions may contribute commands, prompt segments, completion, themes and
tools while touching a real terminal. The boundary must survive missing or
failed plugins and must not expose Rust internals, PTY handles or implicit system
access. Rust's native ABI is not a stable third-party plugin contract.

## Decision

Use two tiers behind one versioned message boundary:

1. Declarative plugins provide manifests, themes, shortcuts and bounded host capabilities without arbitrary code.
2. A future code tier implements a private `PluginRuntime` using public DTOs only.
3. The core owns registry, validation, conflicts, permissions, deadlines and host-service conversion.
4. Third-party Rust dynamic libraries are excluded; a later measured spike compares WebAssembly components with a child-process protocol.
5. Discovery starts nothing. Activation is explicit and reversible; invalid or incompatible plugins stay disabled with diagnostics.

```text
ShellEngine / Prompt / Completion
              │ bounded host services
        versioned plugin DTOs
              │
 registry — permissions — supervisor
              │
     declarative | WASM | isolated process
```

Declarative formats are portable and auditable but intentionally limited.
Child processes isolate crashes but retain user permissions unless additionally
sandboxed. WebAssembly provides typed WIT interfaces and deny-by-default
resources but adds runtime and tooling cost. Neither future runtime is selected
without security, portability and performance evidence.

## Consequences

The public contract can be tested before choosing a runtime, early extensions
cannot access disk or processes arbitrarily, and the core stays runtime-neutral.
The cost is DTO conversion and initially limited plugins. Process isolation
alone must never be described as a sandbox.

## Validation

The API, configuration, registry, manifest, SDK and official plugin are delivered
incrementally. Any code runtime must prove denied permissions, deadlines,
cancellation, crash isolation and Windows/macOS/Linux behavior. The repository
suite stays green with no plugin installed, and a rejected plugin never blocks
CLI or GUI startup.
