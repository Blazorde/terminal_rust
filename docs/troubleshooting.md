# Troubleshooting

## Installation script fails

- Run without administrator rights in a normal terminal.
- Check access to `github.com` and temporary disk space.
- On Linux/macOS, install `curl` or `wget` and `sha256sum` or `shasum`.
- On Windows, use `-NoPathUpdate` if policy blocks user PATH changes.
- Fall back to the manual steps in [install.md](install.md).

## Command not found

Confirm the binary directory is in PATH, reopen the terminal after changes and
test the full path first:

```powershell
& "$env:LOCALAPPDATA\Programs\TerminalRust\terminal_rust.exe" help
```

```bash
"$HOME/.local/bin/terminal_rust" help
```

On Linux/macOS, restore the executable bit with
`chmod +x "$HOME/.local/bin/terminal_rust"`.

## Platform blocks the binary

On macOS, verify the release origin and SHA-256 before allowing the unsigned
binary in Privacy & Security. Do not disable Gatekeeper. `bad CPU type in
executable` usually means an Intel Mac with the current Apple Silicon archive.

On Windows, extract the ZIP, verify the official release and checksum, and
inspect publisher/provenance before a one-time SmartScreen allowance. Do not
disable SmartScreen globally.

## Checksum mismatch

Do not execute the binary. Delete it, download again from the official release
and recheck. Report a second mismatch through the repository's security channel.

## Git or history missing

Git is optional. Check `git --version` and that the current directory is inside
a repository; core commands remain available without it. For persistent history,
set `TERMINAL_RUST_HISTORY_FILE` to a file whose parent directory already exists.

## Removing a directory

`rm -r <directory>` asks for confirmation before recursively removing a
directory. Add `-f` to skip that confirmation (`rm -rf <directory>`); the next
shell command can then run immediately. `rm -i` always asks for confirmation,
including when combined with `-f`.

## Ask for help

Record operating system, architecture, Terminal Rust version, command and full
error, then open a [GitHub issue](https://github.com/Blazorde/terminal_rust/issues)
unless the report contains a security vulnerability.
