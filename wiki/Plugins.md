# Plugins

The current plugin foundation supports local manifests, version compatibility,
explicit permission grants, discovery, declarative commands and bounded prompt
contributions. It deliberately does not load arbitrary third-party native
libraries.

From the terminal, run `plugins help` (or `plugin help`) for a short
introduction and `plugins list` (or `plugin list`) to inspect installed
manifests, status, compatibility and requested permissions. `plugin` and
`plugins` are equivalent command names.

Plugins declare requested permissions; users grant them separately. A missing
grant keeps the contribution unavailable. Invalid, incompatible or disabled
plugins are reported without preventing the shell from starting.

Start with the [plugin tutorial](https://github.com/Blazorde/terminal_rust/blob/main/docs/plugins/getting-started.md),
then review the [API](https://github.com/Blazorde/terminal_rust/blob/main/docs/plugins/api.md),
[manifest](https://github.com/Blazorde/terminal_rust/blob/main/docs/plugins/manifest.md)
and [security model](https://github.com/Blazorde/terminal_rust/blob/main/docs/plugins/security.md).
