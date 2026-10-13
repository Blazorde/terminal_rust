# Installation

## Download the alpha

The [`v1.0.0-alpha.1` prerelease](https://github.com/Blazorde/terminal_rust/releases/tag/v1.0.0-alpha.1)
contains the following unsigned packages:

| Platform | Packages |
| --- | --- |
| Windows x86_64 | Portable ZIP and NSIS installer |
| Linux x86_64 | Portable tarball, DEB and AppImage |
| macOS | Build from source; native-tested bundle pending |

Verify every download with the release's `SHA256SUMS.txt` before granting an
operating-system exception.

## Run from source

Install a current Rust toolchain, clone the repository and run:

```bash
cargo run
```

Run one command without opening an interactive session:

```bash
cargo run -- help
cargo run -- dashboard
```

## Launch the native desktop window

Install Node.js and the platform-specific Tauri prerequisites, then run:

```bash
npm ci --prefix desktop
npm run tauri --prefix desktop -- dev
```

The second command opens the native application and real PTY sessions.
`npm run dev --prefix desktop` is only a browser presentation preview.

See the full [installation guide](https://github.com/Blazorde/terminal_rust/blob/main/docs/install.md)
for prerequisites, package contents and platform limitations.
