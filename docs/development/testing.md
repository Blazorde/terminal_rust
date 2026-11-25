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

CLI and engine tests use unique temporary directories, including paths with
spaces. Quoted single- and double-quoted paths, escaped spaces, and native
Windows path separators are covered without depending on the user's home
directory.

## Cases to cover

File commands cover valid paths, missing arguments, file/directory mismatches,
spaces and refused operations. Parser and dispatcher tests cover whitespace,
quotes, unknown commands, argument counts, help and completion. External-tool
tests cover valid output, missing executables, failing exit codes, empty or
invalid output and direct execution without a shell.

Environment-dependent tests may skip an extra Git-backed assertion, but missing
tool and output-parser branches must remain testable without Git. Normalize paths
with the Rust standard library and tolerate platform line endings.

## Isolation from the developer's environment

Tests that start the binary (`tests/cli_tests.rs`, the PTY tests of the desktop
crate) run it with an empty `HOME`, `XDG_*`, `APPDATA` and `LOCALAPPDATA` and
without `TERMINAL_RUST_CONFIG`, through `shell_command()` and `isolate()`. Without
this the shell reads the developer's own prompt, aliases and plugins and the
results depend on the machine. A test that needs a configuration passes one
explicitly after the helper. Use the helper for every new test that starts the
binary. To check it, put an unusual `config.toml` and `shell-state.toml` in a
fake `HOME` and run the suite.

## CI

CI targets Ubuntu, Windows and macOS with formatting, compilation, Clippy, unit
tests and CLI integration tests. A documentation-only change skips those jobs;
see the [workflow guide](../workflows/README.md#workflows). A platform result
counts only when its steps actually execute. Record native desktop evidence for every supported platform
before publishing a release.

## Reproduce the Linux gate locally

`scripts/ci-local.sh` replays the whole Linux gate, with the Security checks, in
the pinned validation image and prints one `PASS` or `FAIL` line per step. It
exits with a non-zero status when a step fails and shows the last lines of the
failing log. It also runs the tests of the workflow rules
(`node --test scripts/check-branch-policy.test.mjs scripts/ci-scope.test.mjs`):

```bash
scripts/ci-local.sh            # every step, including cargo-deny
scripts/ci-local.sh --no-deny  # skip cargo-deny
scripts/ci-local.sh --fresh    # drop the cached Cargo registry, target and tools
DOCKER_CONTEXT=default scripts/ci-local.sh   # choose another Docker daemon
```

Three Docker volumes cache the Cargo registry, the build output and the tools, so
a second run takes seconds instead of minutes. The script does not cover Windows,
macOS, a visible desktop window or the test that needs the bundled Git runtime.

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

The same pinned image includes Node.js 24 and `patchelf`, so it can also build
the Linux desktop installers without depending on host tooling:

```bash
docker run --rm --volume "$PWD:/source:ro" \
  terminal-rust-linux-validation bash -c '
    tar --exclude=.git --exclude=target --exclude=node_modules \
      --exclude=desktop/node_modules --exclude=desktop/dist \
      -C /source -cf - . | tar -C /workspace -xf - &&
    cd /workspace &&
    npm ci --prefix desktop &&
    npm run tauri --prefix desktop -- build --bundles deb,appimage --ci --no-sign
  '
```

Successful DEB and AppImage creation is build evidence only. Launch the bundle
in a graphical Linux session and execute a real PTY command before marking the
desktop acceptance criterion complete.
