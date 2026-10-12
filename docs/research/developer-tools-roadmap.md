# Developer tools roadmap

- Status: long-term planning, not implemented
- Date: 2026-10-12
- Scope: explicit shortcuts built on top of installed tools

## Principle

Terminal Rust does not replace Cargo, npm, Python, Git, or their configuration
files. An integrated command detects context, explains the proposed action, and
starts the original executable directly with structured arguments. A missing
tool is an unavailable capability with an actionable message, not a fatal shell
error.

Existing building blocks already cover part of this need:

- `git status|branch|log` exposes a non-destructive Git allowlist;
- `commands list` and structured user commands provide local shortcuts without
  an implicit shell;
- `dashboard` summarizes the host system;
- `ShellEngine` exposes typed results shared by the CLI and desktop app.

Future work should compose these capabilities instead of creating another task
runner.

## Priorities

### P0 — read-only context

1. `project info` detects the nearest project and prints its root, types, and
   available tools.
2. `project tasks` lists known actions, their source, and the command that would
   run, without executing anything.

The first detectors are all optional:

| Type | Markers | Proposed actions |
| --- | --- | --- |
| Rust | `Cargo.toml` | `cargo check`, `cargo test`, and `cargo run` when an executable target exists |
| Node.js | `package.json` | scripts declared in `scripts`, using the package manager selected by the present lockfile |
| Python | `pyproject.toml` | explicitly declared tasks; never infer a task from a random `.py` file |

A repository may contain several project types. Detection walks upward with
`Path` from the current directory, stops at the repository or volume root, and
never follows a link outside the project. It starts no executable.

### P1 — bounded execution

3. `project run <task>` starts a task originating from the detected manifest.
4. `project check` and `project test` are aliases for identified tasks, never
   silently invented commands.

The interface must display the executable, arguments, and working directory.
Execution uses `std::process::Command`, without implicit `sh -c`, `cmd /C`, or
PowerShell. Output and exit status flow through a typed result; the desktop app
can present that result without reimplementing detection.

### P2 — summaries and convenience

5. `project git` composes non-destructive information already exposed by the
   Git module: branch, short status, and latest commit.
6. A failure view groups the exit code and latest lines without hiding raw
   output or inventing a diagnosis.
7. The TUI prototype may consume these models as a task palette, but detection
   never depends on the TUI.

A new Git shortcut is justified only when it provides a clearer summary. Native
Git commands remain accessible and documented.

## Initial non-goals

- installing tools or dependencies automatically;
- executing a script found outside a recognized manifest;
- evaluating a string through the system shell;
- modifying Git state (`commit`, `push`, `reset`, or branch switching);
- continuously watching the file system;
- a recipe marketplace or OS-specific heuristics;
- replacing ecosystem task runners.

These boundaries keep the first phase testable and cross-platform. General
scripting remains a separate topic in the
[runtime comparison](scripting-runtime.md).

## Delivery slices

Each slice should be an independent pull request:

1. a `ProjectContext` model, pure detectors, and fixtures covering paths with
   spaces, LF/CRLF line endings, and nested projects;
2. `project info` and `project tasks` rendering, with no execution;
3. explicit executable resolution and direct task spawning;
4. a Git summary composed from the existing module;
5. CLI, desktop, and optionally TUI presentation adapters.

Before promotion, the Windows, macOS, and Linux matrix must cover at least one
project of each type, missing tools, an invalid manifest, and a polyglot
project.

## Decision rule

An integrated action is accepted only when it is frequent, deterministic,
explainable before execution, and simpler than the native command without
removing any capability. If it requires a complex policy, an implicit write, or
non-portable heuristics, it remains a documented user command.
