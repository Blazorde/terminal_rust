# Plugin prompt segments

A plugin can declare a future prompt segment in its manifest. The declaration
reserves an identity and resource budgets; it does not load or execute code. A
contribution becomes visible only when the plugin is enabled, compatible,
started and granted `prompt.segment.register`.

```toml
permissions = ["prompt.segment.register"]

[[prompt_segments]]
name = "node"
summary = "Node version for the current project"
timeout_ms = 20
cache_ttl_ms = 30000
```

`name` follows plugin identifier rules. `summary` is required and cannot contain
control characters. `timeout_ms` defaults to 25 and stays between 1 and 100 ms.
`cache_ttl_ms` defaults to zero and cannot exceed five minutes.

## Evaluation lifecycle

```text
filtered PromptContext -> valid cache? -> deadline-bound request -> typed value
                                                  |
                                            timeout or error
                                                  v
                                           omit the segment
```

The context contains only authorized data: an opaque directory token, last
status, duration and explicitly granted variables. Plugins receive no history,
complete environment or terminal handle.

A timeout, crash or invalid value hides the segment for that render and emits a
rate-limited diagnostic. Cache keys include plugin, segment, configuration and
relevant context. Disabling the plugin or changing configuration or directory
invalidates related entries.

## Configuration and rendering

Segment settings live under `[plugins.<id>]` and are validated by the plugin at
startup. Prompt order can eventually reference `<plugin-id>.<segment>`; missing
references degrade like unavailable capabilities.

Results carry UTF-8 text and bounded semantic roles, never ANSI sequences, raw
colors or mandatory glyphs. The renderer selects palette, separators, ASCII or
Nerd Font icons and text fallbacks for CLI, GUI, no-color terminals and tests.
