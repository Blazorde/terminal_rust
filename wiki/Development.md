# Development

Create focused work from an up-to-date `develop` branch and open pull requests
back into `develop`. The final release route is `develop` to `main` and requires
explicit owner approval.

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
npm run audit:ui --prefix desktop
npm run build --prefix desktop
```

Commits use English Conventional Commits. Public behavior changes include tests
and documentation in the same pull request. A CI job that executes zero steps
is infrastructure state, not validation evidence.

See [contributing](https://github.com/Blazorde/terminal_rust/blob/develop/docs/development/contributing.md),
[testing](https://github.com/Blazorde/terminal_rust/blob/develop/docs/development/testing.md)
and [workflow](https://github.com/Blazorde/terminal_rust/blob/develop/docs/workflows/README.md).

## Synchronize this wiki

The repository `wiki/` directory is the source of truth. Validate locally:

```bash
node scripts/sync-wiki.mjs --check
```

Publish after the corresponding repository PR is merged:

```bash
node scripts/sync-wiki.mjs --publish
```

The publisher clones the dedicated GitHub wiki repository into a temporary
directory, replaces Markdown pages from the mirror, commits only when content
changed, pushes, then removes the temporary checkout.
