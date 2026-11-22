# Architecture Decision Records

An ADR records a decision that affects multiple modules, supported platforms or
future work. It explains why the choice was made and which trade-offs follow.

## Register

| ADR | Status | Decision |
| --- | --- | --- |
| [0001](0001-command-registry.md) | Accepted | Central built-in command registry |
| [0002](0002-non-blocking-git-detection.md) | Accepted | External, non-blocking Git detection |
| [0003](0003-segmented-prompt-engine.md) | Accepted | Segment-based prompt engine |
| [0004](0004-allowlisted-git-commands.md) | Superseded | Read-only allowlist for Git shortcuts |
| [0005](0005-desktop-gui-stack.md) | Accepted | Tauri desktop layer around the Rust engine |
| [0006](0006-plugin-boundary.md) | Accepted | Declarative, isolatable plugin boundary |
| [0007](0007-full-native-git-and-editors.md) | Accepted | Full native Git with bundled terminal editors |
| [0008](0008-external-programs.md) | Accepted | Run external programs from the shell |
| [0009](0009-shell-variables.md) | Accepted | Shell variables, expansion, `export` and `unset` |
| [0010](0010-command-chaining.md) | Accepted | Command chaining with `;`, `&&` and `||` |
| [0011](0011-pipelines-and-redirections.md) | Accepted | Pipelines and redirections |
| [0012](0012-filename-patterns.md) | Accepted | File name patterns `*`, `?` and `[...]` |
| [0013](0013-background-jobs.md) | Accepted | Background jobs with `&`, `jobs`, `fg`, `wait` and `kill` |
| [0014](0014-command-substitution.md) | Accepted | Command substitution `$(...)` |
| [0015](0015-brace-expansion.md) | Accepted | Brace expansion `{a,b}` and `{1..5}` |
| [0016](0016-control-flow.md) | Accepted | Control flow: `for`, `while`, `until`, `if`, `break`, `continue` |
| [0017](0017-arithmetic-expansion.md) | Accepted | Arithmetic expansion: `$((...))` |
| [0018](0018-history-expansion.md) | Accepted | History expansion: `!!`, `!$`, `!N`, `!prefix` and `history` |
| [0019](0019-git-completion.md) | Accepted | Git completion: subcommands, branches and tags |
| [0020](0020-recursive-patterns.md) | Accepted | Recursive patterns: `**` |
| [0021](0021-functions.md) | Accepted | Functions and positional parameters |
| [0022](0022-redirected-blocks.md) | Accepted | Redirections and pipes on blocks, `while read` |
| [0023](0023-here-documents.md) | Accepted | Here-documents and here-strings |
| [0024](0024-job-control.md) | Accepted | Job control: Ctrl+Z, `bg` and `fg` in safe Rust |
| [0025](0025-negation-command-exec.md) | Accepted | `!`, `command`, `exec`, `$PPID` and `$RANDOM` |

## Lifecycle

- `Proposed`: discussion or prototype in progress.
- `Accepted`: rule for new work.
- `Superseded`: a newer decision replaces it.
- `Rejected`: documented option that was not selected.

Copy the [template](template.md), choose the next four-digit number and add the
ADR here. Do not rewrite accepted ADRs to hide history; supersede them.
