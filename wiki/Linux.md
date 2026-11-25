# Linux

Terminal Rust Desktop can be chosen as your terminal on Linux. This is an
opt-in setting: installing the package never changes your system default.

## Launch options

The desktop app accepts the options that desktop environments and other
programs use to open a terminal:

| Option | Effect |
| --- | --- |
| `-w DIR`, `--working-directory DIR` | Start the first session in `DIR` |
| `--working-directory=DIR` | Same, in a single argument |
| `-e COMMAND [ARGS...]` | Run `COMMAND` instead of the Terminal Rust shell |
| `-- COMMAND [ARGS...]` | Same as `-e` |

```bash
terminal-rust-desktop -w ~/code
terminal-rust-desktop -e htop
terminal-rust-desktop -w ~/code -- git log --oneline
```

Only the first session uses these options. Tabs opened afterwards start a
regular shell. A missing directory is ignored, and an invalid option is reported
on stderr without preventing the window from opening.

The shell of each session reads `~/.terminal_rust_rc`. The command-line binary
also accepts `-c`, `-i` and `-l`; see [Starting the shell](Usage.md#starting-the-shell).

## Desktop entry

The DEB package installs a desktop entry in the `TerminalEmulator` category, so
launchers and settings panels list Terminal Rust as a terminal.

## Make it your default terminal

### Debian, Ubuntu and derivatives

The DEB registers Terminal Rust as an `x-terminal-emulator` alternative with a
low priority. Select it explicitly:

```bash
sudo update-alternatives --config x-terminal-emulator
```

Removing the package removes the alternative.

### GNOME

```bash
gsettings set org.gnome.desktop.default-applications.terminal exec 'terminal-rust-desktop'
gsettings set org.gnome.desktop.default-applications.terminal exec-arg '-e'
```

Some GNOME components launch their own terminal regardless of this setting.

### KDE Plasma

Open **System Settings → Applications → Default Applications → Terminal
emulator** and pick Terminal Rust.

### AppImage

The AppImage does not register system alternatives. Put it on your `PATH` under a
stable name, then point your desktop settings at it:

```bash
install -Dm755 Terminal.Rust_*.AppImage ~/.local/bin/terminal-rust-desktop
```

## Current limits

- This is an alpha. Keep your previous terminal available until you are
  comfortable with the desktop app.
- The window, moving, resizing and input have been checked in a real Linux
  session under WSLg, and the AppImage has been checked headless. Other Wayland
  and X11 compositors are not yet covered.
- Closing behavior when the `-e` command exits follows the regular session
  behavior of the desktop app.
