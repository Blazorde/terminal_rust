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

## Lifecycle

- `Proposed`: discussion or prototype in progress.
- `Accepted`: rule for new work.
- `Superseded`: a newer decision replaces it.
- `Rejected`: documented option that was not selected.

Copy the [template](template.md), choose the next four-digit number and add the
ADR here. Do not rewrite accepted ADRs to hide history; supersede them.
