# Live system dashboard

Run `dashboard` inside Terminal Rust to open a live performance view. CPU,
memory, swap, processes, disk, network totals, the current directory and Git
context refresh once per second without appending lines.

```text
dashboard
```

Press `q`, `Escape` or `Ctrl+C` to close the view. Terminal Rust uses an
alternate screen and restores the cursor, input mode and previous terminal
contents before returning to the prompt.

The live view requires an interactive input and output terminal at least 70
columns by 24 rows. It works in the native Tauri window and in a compatible
terminal on Windows, macOS or Linux. The native window advertises
`TERM=xterm-256color` to its child shell; no Unix command or PowerShell process
is used to collect metrics. The layout adjusts to narrower terminal widths and
truncates long values without breaking the borders; resizing the live view
refreshes its width.

## Safe fallbacks

The same command renders one static, ASCII-safe snapshot when output is
redirected, `TERM=dumb` is set, the terminal is too small or dimensions cannot
be determined outside the managed desktop PTY. In a visible terminal, the
snapshot fits the available width; redirected output uses the default width.
This makes reports and scripts deterministic:

```bash
cargo run -- dashboard > dashboard.txt
```

`NO_COLOR` keeps the live refresh behavior but removes color. Closing the view
does not terminate the shell.
