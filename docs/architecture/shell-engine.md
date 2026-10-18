# Shell engine API

`ShellEngine` is the Rust boundary shared by the CLI and desktop adapters. It
accepts an entered line or argument list, dispatches through the command
registry and returns data without imposing a presentation layer.

```rust
use terminal_rust::{ExecutionStatus, ShellEngine};

let mut engine = ShellEngine::new();
let result = engine.execute_line("pwd");

if result.status == ExecutionStatus::Failed {
    eprintln!("{}", result.error.as_deref().unwrap_or("unknown error"));
} else {
    print!("{}", result.stdout);
}
```

## Contract

- `status` distinguishes continue, requested exit and recoverable failure.
- `stdout` and `stderr` remain separate and are not written by the engine.
- `error` contains dispatch or command diagnostics for failed results.
- `current_dir` reports the accessible directory after execution.
- `directory_change` reports the paths before and after a successful `cd`.

The engine retains session state for `cd -`. Each instance therefore belongs
to one session and must not be shared between concurrent terminals. The PTY
adapter owns external commands, interactive confirmations and process lifetime.

`execute_line` tokenizes single- and double-quoted arguments before dispatch.
Quotes keep whitespace inside one argument and are removed from the resulting
value; escaped whitespace is also supported. Backslashes in native Windows
paths are preserved. `execute_args` accepts an already-tokenized argument list
and does not parse quote characters.

## Compatibility

The API uses `PathBuf` and does not depend on Unix or PowerShell commands.
Consumers must use `directory_change` instead of parsing output to detect `cd`.
