# Install Terminal Rust

Prebuilt releases are available from [GitHub Releases](https://github.com/Blazorde/terminal_rust/releases/latest).
Read installation scripts before running them. Both installers download an
official archive plus `SHA256SUMS.txt`, verify integrity, install without
administrator rights and smoke-test `terminal_rust help`.

## Automated installation

Windows:

```powershell
.\scripts\install.ps1
.\scripts\install.ps1 -NoPathUpdate -Version v0.3.0
```

The default directory is `%LOCALAPPDATA%\Programs\TerminalRust`; PATH changes
affect only the current user.

Linux x86_64 or macOS Apple Silicon:

```bash
chmod +x scripts/install.sh
./scripts/install.sh
```

The default is `~/.local/bin`; the script prints the required PATH line instead
of editing shell files. Override version and directory with
`TERMINAL_RUST_VERSION` and `TERMINAL_RUST_INSTALL_DIR`.

For a private repository, set `GITHUB_TOKEN` temporarily to a read-only token.
Never store it in a script or commit. Unix installation additionally uses `gh`.

## Archives

| System | Archive | Desktop bundle inside `desktop/` |
| --- | --- | --- |
| Windows x86_64 | `terminal_rust-windows-X64.zip` | NSIS `.exe` installer |
| Linux x86_64 | `terminal_rust-linux-X64.tar.gz` | `.deb` and `.AppImage` |
| macOS Apple Silicon | `terminal_rust-macos-ARM64.tar.gz` | `.dmg` |

No macOS Intel archive is currently published; build from source on Intel Macs.

Verify downloads before execution:

```powershell
Get-FileHash .\terminal_rust-windows-X64.zip -Algorithm SHA256
```

```bash
sha256sum terminal_rust-linux-X64.tar.gz
shasum -a 256 terminal_rust-macos-ARM64.tar.gz
```

On Linux/macOS, extract, mark executable and install to `~/.local/bin`. On
Windows, extract the ZIP to a stable directory and add that directory to the
user PATH if desired. To install the GUI, open the platform bundle under
`desktop/` after verifying the archive checksum. Desktop bundles are not yet
code-signed or notarized; verify their origin before granting a one-time
allowance. Never disable Windows protection or macOS Gatekeeper globally.

## Build from source

```bash
git clone https://github.com/Blazorde/terminal_rust.git
cd terminal_rust
cargo build --release --locked
```

The result is `target/release/terminal_rust`, or
`target\release\terminal_rust.exe` on Windows.
