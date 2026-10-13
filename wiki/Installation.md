# Installation

## Run the shell from source

Install a current Rust toolchain, then run from the repository root:

```bash
cargo run
```

Run one command without opening an interactive session:

```bash
cargo run -- help
cargo run -- dashboard
```

## Launch the native desktop window

Install the platform-specific Tauri prerequisites plus Node.js, then run:

```bash
npm ci --prefix desktop
npm run tauri --prefix desktop -- dev
```

The second command opens the native application and its real PTY sessions.
`npm run dev --prefix desktop` opens only a browser presentation preview and
cannot execute shell commands.

## Release archives

Stable archives are published on [GitHub Releases](https://github.com/Blazorde/terminal_rust/releases).
Verify downloads with `SHA256SUMS.txt`. Each archive includes the MIT license,
third-party notices, changelog, README and default configuration.

See the full [installation guide](https://github.com/Blazorde/terminal_rust/blob/develop/docs/install.md)
for supported archive shapes, source prerequisites and current platform limits.
