# Releases

Stable versions live on `main`; ongoing work lives on `develop`. The latest
stable line is currently `0.3.x`. A `v1.0.0` pull request will not be opened
until the remaining release criteria are satisfied and the owner explicitly
approves the final route to `main`.

Release tags build locked CLI binaries and native desktop bundles for Windows,
macOS and Linux, smoke-test the CLI, generate SHA-256 checksums and publish
notes from the changelog. Archives retain the project license and generated
third-party notice inventory. Desktop bundles are currently unsigned and must
be verified against the release checksum before installation.

A local build proves only its host. A platform is release-verified only after
its CI steps actually execute and required native evidence is recorded. See the
[release procedure](https://github.com/Blazorde/terminal_rust/blob/develop/docs/release.md)
and [GUI evidence matrix](https://github.com/Blazorde/terminal_rust/blob/develop/docs/development/gui-quality-audit.md).
