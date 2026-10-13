# Terminal Rust Fastfetch preset

This preset provides a compact, colorful system overview on Windows, macOS and
Linux. It uses standard Fastfetch modules and does not require a Nerd Font.

## Run from the repository

```bash
fastfetch --config config/fastfetch/config.jsonc
```

Fastfetch must be installed separately. Terminal Rust does not install it, and
the built-in `dashboard` command works without it.

## Use as the default configuration

Create the Fastfetch configuration directory, back up any existing file and
copy `config.jsonc` to:

```text
~/.config/fastfetch/config.jsonc
```

Fastfetch recognizes this path on Windows, macOS and Linux. The file follows
the [official Fastfetch schema](https://github.com/fastfetch-cli/fastfetch/blob/dev/doc/json_schema.json).

## Customize

- Change `keyColor` to recolor a category.
- Remove `gpu` or `battery` when the machine does not expose that information.
- Set `logo.type` to `auto` to use the full system logo.
- Keep `$schema` for validation and editor completion.
