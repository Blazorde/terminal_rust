# Architecture

```text
CLI / Tauri desktop
        |
        v
ShellEngine -- command registry -- built-ins and declarative commands
        |
        +-- prompt segments -- Git cache -- theme renderer
        +-- typed command errors
        +-- platform adapters -- paths, PTY, settings
```

The shared Rust engine owns parsing, commands, processes, paths and prompt
context. `runner` owns the interactive lifecycle. Rendering is centralized so
plain, ANSI and desktop consumers do not duplicate business behavior.

The Tauri backend owns native PTY sessions and validated settings. React and
xterm.js render bounded events in the WebView; they do not receive arbitrary
filesystem or process access.

Platform-specific dependencies remain behind adapters or `cfg(...)` with a
documented fallback. Native paths use `Path` and `PathBuf`; optional Git or font
capabilities are never startup requirements.

Read the [architecture overview](https://github.com/Blazorde/terminal_rust/blob/main/docs/architecture/overview.md)
and [decision register](https://github.com/Blazorde/terminal_rust/blob/main/docs/decisions/README.md).
