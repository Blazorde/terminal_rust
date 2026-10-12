# Automated contribution guide

This file applies to the entire repository.

## Project goal

Terminal Rust is a reliable, educational, cross-platform mini-shell. Every
change must remain compatible with Windows, macOS and Linux. Its prompt engine
is inspired by Oh My Posh: independent segments, readable configuration and
graceful degradation when Git or a terminal capability is unavailable.

## Sources of truth

- The Notion ticket defines scope and acceptance criteria.
- `docs/architecture/` defines current and target technical boundaries.
- `docs/decisions/` records structural decisions and trade-offs.
- Code and tests win when documentation is stale; update the documentation in
  the same pull request.

## Workflow

1. Start from an up-to-date `develop` branch.
2. Use `feat/*`, `fix/*`, `chore/*`, `refractor/*` or `hotfix/*` as appropriate.
3. Keep one coherent group of tickets per branch. Group tickets only when they
   share a technical boundary and can be validated together.
4. Preserve unrelated local changes.
5. Add or update tests with the implementation.
6. Update the README, guides or an ADR when public behavior or architecture changes.
7. Open the pull request against `develop` and record its link in the ticket.

## Rust rules

- Keep `runner` focused on the shell lifecycle.
- Register built-in commands in `app/command.rs`.
- Keep business logic in `commands/`, `git/` or a dedicated module.
- Centralize output in `render` for testability and terminal adaptation.
- Prefer typed errors and useful messages over `unwrap` or `panic!`.
- Treat a missing external tool as an unavailable capability when it is not fatal.
- Use `Path` and `PathBuf`, never manual `/` or `\\` concatenation.
- Do not call Unix or PowerShell commands for work available in the Rust standard library.

## Compatibility

- Do not assume `HOME`, Git, a Nerd Font or advanced ANSI sequences exist.
- Test paths containing spaces and both LF and CRLF when relevant.
- Keep OS-specific dependencies behind `cfg(...)` with a documented fallback.

## Required validation

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Document checks that cannot run locally. Never mark a ticket complete without
evidence for its acceptance criteria.

## Git

Use English Conventional Commits:

```text
feat(git): expose repository status
fix(prompt): handle missing home directory
chore(docs): document architecture decisions
```

See `docs/workflows/README.md` for branch, pull-request and protection details.
