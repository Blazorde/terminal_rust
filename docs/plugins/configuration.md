# Plugin configuration

Terminal Rust reserves a stable v1 configuration area for activation, explicit
permission grants, and plugin-owned settings.

## TOML format

```toml
[plugins]
enabled = ["example.hello"]

[plugins.grants]
"example.hello" = ["command.register", "output.write"]

[plugins."example.hello"]
greeting = "Welcome"
```

`plugins.enabled` is the ordered list of plugin IDs considered for activation.
An absent or empty list disables every plugin. Removing an ID is enough to
disable it; settings and grants remain so a temporary disable does not destroy
user choices.

`plugins.grants` maps a plugin ID to permissions the user approved. A grant is
effective only when the installed manifest also requests the same permission.
Unknown and duplicate permission names reject the configuration. Activation is
not consent, and a stale grant cannot authorize a permission omitted by the
current manifest.

Every other `[plugins.<id>]` table belongs to that plugin. The core preserves
its TOML values without interpreting or deleting unknown keys. A settings table
is optional.

## IDs

An ID contains 1–64 lowercase ASCII characters. Letters, digits, dots, hyphens,
and underscores are accepted; the first and last character must be a letter or
digit. Both simple IDs such as `git` and qualified IDs such as
`acme.git-tools` are valid.

The same rule applies to activation, grants, and settings table keys. An ID may
appear only once in `enabled`; order is preserved for deterministic discovery,
but never resolves a command conflict.

## Error behavior

| Situation | Behavior |
| --- | --- |
| Missing `[plugins]` | Valid configuration; no plugin is active |
| Invalid or duplicate enabled ID | Configuration rejected with property and ID |
| Invalid grants ID | Configuration rejected |
| Unknown or duplicate grant | Configuration rejected with plugin and permission |
| Settings value is not a table | TOML rejected with the file path |
| Disabled plugin has settings or grants | Values preserved; no contribution exposed |
| Enabled plugin is not installed | Shell continues; plugin remains unavailable |
| Unknown key inside plugin settings | Preserved for the plugin to validate |

As with any global configuration error, the shell prints a diagnostic and uses
safe defaults. Missing plugins and tools are unavailable capabilities, not fatal
shell errors.

## Portability

The format contains no OS-specific path separator and behaves the same on
Windows, macOS, and Linux. Paths stored inside plugin settings remain TOML
strings and must be converted with native path APIs by a scoped host service.

Older v1 files without plugins or grants remain valid. The canonical
[`config/default.toml`](../../config/default.toml) keeps an empty activation
list so a fresh installation never runs an extension implicitly.
