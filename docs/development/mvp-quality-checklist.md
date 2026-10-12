# MVP quality checklist

Run this checklist from the repository root on an up-to-date `develop` branch
before a release or stabilization pull request.

## Automated checks

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Windows, Linux and macOS CI must then execute successfully. CLI tests cover paths
with spaces and platform line endings.

## Manual path

Start with `cargo run`, then verify:

1. `help` lists every public command.
2. `pwd`, `mkdir quality-check`, `touch "quality-check/note.txt"`, `ls -la quality-check` and `cat "quality-check/note.txt"` work.
3. `cd quality-check`, `cd ..` and `cd -` navigate correctly.
4. Git shortcuts remain limited to the allowed read-only subcommands.
5. `dashboard` reports system, CPU, memory, storage, network and context without Fastfetch or a Nerd Font.
6. `clear` clears a compatible terminal without closing the shell.
7. Up arrow recalls command history.
8. `rm quality-check/note.txt` and `rm -d quality-check` remove the fixture.
9. An unknown command reports a useful error and keeps the shell alive.
10. `exit` closes cleanly and saves history when persistence is available.

## Fallbacks

- Outside Git, the prompt remains usable without a Git segment.
- Without Git, only Git capabilities are unavailable.
- Redirected output, `TERM=dumb` or `NO_COLOR` selects plain dashboard output.
- Without a detectable home directory, the shell continues without persistent history.

Remove temporary data after the pass. Put substantial defects in focused tickets
instead of expanding the stabilization pull request.
