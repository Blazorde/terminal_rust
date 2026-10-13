# Security

Do not publish suspected vulnerabilities in an issue, discussion or pull
request. Follow the repository [security policy](https://github.com/Blazorde/terminal_rust/blob/main/SECURITY.md)
and use a private GitHub security advisory. Include the affected version and
platform, impact and minimal reproduction without unrelated personal data or
credentials.

The repository checks Rust advisories, yanked crates, dependency sources and
an explicit license allowlist with `cargo-deny`. npm dependencies are installed
from the lockfile and audited. Dependabot monitors Cargo, npm and GitHub Actions.

Automated analysis reduces risk but cannot guarantee that the program or its
dependencies are vulnerability-free. See the full
[dependency policy](https://github.com/Blazorde/terminal_rust/blob/main/docs/security.md).
