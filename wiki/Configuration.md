# Configuration

Terminal Rust reads versioned TOML configuration. Missing or invalid optional
capabilities degrade safely instead of preventing startup.

The prompt is composed from independent left and right segments for the current
folder, Git state, command status, runtime, duration and time. Built-in themes
include Graphite, Bone, Nord, Amber, Moss and Dusk. ASCII remains the default;
Nerd Font icons require explicit opt-in.

User commands map a stable name to a built-in action or external process
without passing through a system shell. This keeps quoting and errors portable
across Windows, macOS and Linux.

Detailed references:

- [User configuration](https://github.com/Blazorde/terminal_rust/blob/main/docs/configuration.md)
- [Prompt customization](https://github.com/Blazorde/terminal_rust/blob/main/docs/customization.md)
- [Themes](https://github.com/Blazorde/terminal_rust/blob/main/docs/themes.md)
- [Custom commands](https://github.com/Blazorde/terminal_rust/blob/main/docs/custom-commands.md)
