# Plugin manifest

Every installed version provides a `terminal-rust-plugin.toml` file. The
registry reads this file without executing the plugin. The initial format is
strict: an unknown key, unsupported permission, missing field, or malformed
SemVer requirement produces a diagnostic tied to the manifest path.

```toml
schema_version = 1
id = "acme.git-tools"
name = "Git tools"
version = "0.1.0"
description = "Adds Git diagnostics without modifying the repository"
host_api = "^1.0"
terminal_version = ">=0.3, <2.0"
authors = ["Acme"]
permissions = ["command.register", "output.write", "prompt.segment.register"]

[[commands]]
name = "git-clean-check"
summary = "Check whether the repository is clean"
response = "Repository check is available.\n"

[[prompt_segments]]
name = "git-review"
summary = "Current review state"
timeout_ms = 20
cache_ttl_ms = 5000
```

`version` follows SemVer. `host_api` and `terminal_version` are SemVer
requirements evaluated by the [compatibility policy](compatibility.md). `id`
follows the rules in [plugin configuration](configuration.md). A command name
contains only lowercase ASCII letters, digits, `-`, or `_`.

For backward compatibility, an existing schema v1 manifest that omits
`terminal_version` is interpreted as `*`. New manifests and every republished
version should declare the narrowest range tested by the author.

The accepted v1 permissions are:

- `context.cwd.read`, `context.env.read`, and `config.read`;
- `command.register`, `completion.register`, `output.write`, and
  `prompt.segment.register`;
- the sensitive declarations `filesystem.read`, `filesystem.write`,
  `process.spawn`, and `network.connect`.

A manifest request is not a user grant. The sensitive declarations have no host
service yet and cannot perform an action. See the
[security policy](security.md) for the authorization sequence.

A contribution requires its matching registration permission. Declared prompt
segments have a 1–100 ms budget and a maximum cache lifetime of 300,000 ms. The
defaults are 25 ms and no cache.

An optional declarative command `response` is limited to 4 KiB, rejects NUL and
escape characters, accepts no arguments, and requires `output.write`. It is
rendered as static data; it does not execute the plugin library.

An invalid manifest remains visible with the `error` state. A version whose API
or application requirement rejects the current host remains visible with the
`incompatible` state and an actionable explanation.
