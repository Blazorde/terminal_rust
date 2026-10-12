# Contribution guide

## Prepare the environment

Install Rust with `rustup`, then verify:

```bash
rustc --version
cargo --version
git --version
```

Git enriches the prompt but must not be required for core shell commands.

## Choose a branch

Start from `develop` and select the matching prefix:

```bash
git switch develop
git pull --ff-only origin develop
git switch -c feat/short-description
```

| Prefix | Use |
| --- | --- |
| `feat/` | User-facing capability |
| `fix/` | Behavior correction |
| `chore/` | Documentation, CI or maintenance |
| `refractor/` | Refactor without behavior change |
| `hotfix/` | Urgent production correction |

## Add a command

1. Create its module in `src/commands/`.
2. Add a type in `src/errors/` when multiple domain failures are possible.
3. Export the required modules.
4. Register the command in `app/command.rs`.
5. Add help and verify completion.
6. Test success, invalid arguments and filesystem failures.
7. Document the command in the README.

## Change architecture

Create an ADR when a change introduces a structural dependency, modifies module
boundaries, selects a cross-platform strategy, affects several future tickets
or replaces an existing decision.

## Pull requests

Target `develop` and include the problem or ticket, important choices, executed
validation and known limits or follow-up work. See the complete
[GitHub workflow guide](../workflows/README.md).
