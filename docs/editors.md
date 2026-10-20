# Nano and Neovim

The desktop release bundles Nano 9.2 and Neovim 0.12.5 with the runtime
libraries they need. They are installed into the application's writable data
directory the first time a terminal session starts and are reused afterward.
No administrator access or separate editor installation is required.

```text
nano notes.txt
nvim src/main.rs
neovim README.md
```

`neovim` is an alias for `nvim`. File names and options are passed to the
editor as separate arguments. Quoted file paths with spaces work in the same
way as other shell commands.

Git uses the editor configured in the user's existing Git configuration. When
no editor is configured, desktop releases use Nano as their system-level
default. To choose Neovim for commands such as `git commit`, set a user-level
preference once:

```text
git config --global core.editor nano
git config --global core.editor nvim
```

The desktop shell exposes the bundled editors to Git through its process path.
The runtime default is lower priority than the user's global configuration, so
an existing editor preference wins. Development builds and the standalone CLI
use an editor available in `PATH`; the executable can also be selected with
`TERMINAL_RUST_NANO` or `TERMINAL_RUST_NVIM`.

The Windows Nano binary is downloaded from the official Nano distribution and
checked against a pinned SHA-256 value during release packaging. Its matching
Nano source archive and the runtime package license files ship in the desktop
resources. The package inventory is listed in `GIT-RUNTIME-NOTICES.md`.

## Public Neovim configuration

The [CodeOpsHQ Neovim configuration](../config/nvim/README.md) is available
for download. The [wiki installation guide](../wiki/Neovim.md) provides
Windows, macOS and Linux install commands. Existing Neovim configurations are
backed up before the public setup is installed.

Choose **CodeOpsHQ Full** on first launch to load the complete profile used by
the author's Windows setup. The configuration files and plugin revisions
pinned in `lazy-lock.json` are shared across platforms. Platform-specific
language servers and formatters are installed separately through Mason; AI
provider keys must be configured in the environment. The optional image
converter used by the author's dashboard is not included.
