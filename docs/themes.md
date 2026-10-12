# Theme format

Themes map semantic roles to rendering without collecting data or executing
commands. Built-in presets are `default`, `minimal`, `git-focus` and `powerline`.
Unknown names report a diagnostic and fall back to `default`. Loading arbitrary
theme paths is not supported yet.

`config/themes/default.toml` is the canonical example. Themes define palette
roles, icon names with ASCII fallbacks and optional separators. User
`prompt.icons = "always"` or `"never"` overrides theme preference.

```text
C:\workspace\terminal_rust  git:main* >
```

This plain rendering remains the minimum contract. No theme may assume a Unix
terminal, 24-bit color or a specific font.

`minimal` disables icons and uses a reduced ANSI palette. `git-focus` preserves
ASCII fallbacks and emphasizes a dirty repository. `powerline` uses `U+E0B0`
between segments, colored with the previous segment's semantic role. Without
`TERMINAL_RUST_NERD_FONT=1`, an interactive terminal, or when icons are disabled,
the separator becomes ` | `.

## Inspect presets

```bash
terminal_rust theme list
terminal_rust theme preview default
terminal_rust theme preview minimal
terminal_rust theme preview powerline
```

Preview data is fixed and explicitly illustrative: folder `terminal-rust`, dirty
branch `main*` and symbol `>`. Redirected output uses plain ASCII. Unknown names
list valid choices and return a failing exit code.

The `render::icons` registry owns glyphs for `folder`, `git_branch`, `error`,
`user` and `time`; themes reference symbolic names rather than copying Unicode.
Unknown or unavailable glyphs always use the declared ASCII fallback.
