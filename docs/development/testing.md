# Testing strategy

## Current pyramid

Fast unit tests live beside modules. Integration tests in `tests/cli_tests.rs`
also launch the real binary and verify output and exit codes as a user would.

## Local suite

Run before every pull request:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Validate all workspace members together with:

```bash
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

CLI tests use unique temporary directories, including paths with spaces, and do
not depend on the user's home directory.

## Cases to cover

File commands cover valid paths, missing arguments, file/directory mismatches,
spaces and refused operations. Parser and dispatcher tests cover whitespace,
quotes, unknown commands, argument counts, help and completion. External-tool
tests cover valid output, missing executables, failing exit codes, empty or
invalid output and direct execution without a shell.

Environment-dependent tests may skip an extra Git-backed assertion, but missing
tool and output-parser branches must remain testable without Git. Normalize paths
with the Rust standard library and tolerate platform line endings.

## CI

CI targets Ubuntu, Windows and macOS with formatting, compilation, Clippy, unit
tests and CLI integration tests. A platform result counts only when its steps
actually execute. Complete automation with the [MVP checklist](mvp-quality-checklist.md).

## Reproduce the Linux gate locally

When hosted runners are unavailable, Docker can execute the complete Rust gate
against WebKitGTK and the other native Tauri dependencies used by Linux. Build
the pinned validation image from the repository root:

```bash
docker build --file .github/docker/linux-validation.Dockerfile \
  --tag terminal-rust-linux-validation .
```

Then mount the checkout read-only, copy it into the ephemeral container and
keep Cargo output inside the container. The copy is required because Tauri's
build script generates permission manifests beside its source configuration:

```bash
docker run --rm --volume "$PWD:/source:ro" \
  terminal-rust-linux-validation bash -c '
    tar --exclude=.git --exclude=target --exclude=node_modules \
      --exclude=desktop/node_modules --exclude=desktop/dist \
      -C /source -cf - . | tar -C /workspace -xf - &&
    cd /workspace &&
    cargo fmt --all -- --check &&
    cargo check --workspace --all-targets --all-features --locked &&
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings &&
    cargo test --workspace --all-targets --all-features --locked
  '
```

This proves Linux compilation and headless Rust/PTY behavior. It does not prove
that a visible WebKitGTK window renders correctly; the manual GUI evidence in
the desktop audit is still required for that claim.
