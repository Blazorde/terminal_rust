# Installation

## Download

The [`v1.11.0` release](https://github.com/Blazorde/terminal_rust/releases/tag/v1.11.0)
contains the following unsigned packages:

| Platform | Packages |
| --- | --- |
| Windows x86_64 | Portable ZIP and NSIS installer |
| Linux x86_64 | Portable tarball, DEB and AppImage |
| macOS | Archive built by the release workflow; not yet tested on Apple hardware |

Verify every download with the release's `SHA256SUMS.txt` before granting an
operating-system exception.

On Linux, see [[Linux]] to use Terminal Rust as your terminal.

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
