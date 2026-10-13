# Development

Terminal Rust is open source under the MIT License. GitHub issues and pull
requests are welcome when they follow the project documentation, stay focused,
preserve Windows, macOS and Linux compatibility, include appropriate tests and
document user-visible behavior. Security vulnerabilities must use the private
process in `SECURITY.md` rather than a public issue.

Create work from an up-to-date `develop` branch and open pull requests back
into `develop`. Commits use English Conventional Commits.

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
npm run audit:ui --prefix desktop
npm run build --prefix desktop
```

Public behavior changes include tests and documentation in the same pull
request. A CI job that executes zero steps is infrastructure state, not
validation evidence.

See [contributing](https://github.com/Blazorde/terminal_rust/blob/main/docs/development/contributing.md),
[testing](https://github.com/Blazorde/terminal_rust/blob/main/docs/development/testing.md)
and [workflow](https://github.com/Blazorde/terminal_rust/blob/main/docs/workflows/README.md).
