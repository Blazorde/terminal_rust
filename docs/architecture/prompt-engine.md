# Prompt engine

Status: typed model, enriched context collection, ANSI renderer and desktop
transport are implemented.

## Goal

Build an Oh My Posh-inspired prompt from independent segments while remaining
readable without color, icons or Git.

```text
~/project  git:main* >
C:\project>
```

`Prompt` lives for the interactive session. The Git provider collects branch
and worktree state with one porcelain-v2 command; `GitPromptCache` retains both
positive and unavailable results for 250 ms and invalidates on directory change.

`PromptSegment` is the collection-to-rendering contract. It carries a value and
semantic style/icon references, never ANSI. Missing values hide the segment.
Supported kinds cover path, Git, symbol, user, host, exit status, local runtime,
duration and time. One typed `PromptLayout` feeds the CLI and desktop status bar.

`render::palette` defines semantic roles and `render::ansi` maps them to portable
terminal output with a reset after every styled segment. Redirected output,
`NO_COLOR` and `TERM=dumb` use plain text. Windows enables color only for a known
ANSI-capable terminal.

`render::icons` and `render::separators` select Nerd Font or Powerline glyphs
only when policy and capability allow them; every glyph has an ASCII fallback.
Color and icon policies remain independent.

## Configuration contract

The [customization specification](../customization.md) separates segment choice
from appearance. User configuration selects theme, order and behavior; themes
describe rendering. Neither format executes arbitrary commands.

```toml
version = 1
theme = "default"

[prompt]
icons = "auto"
segments = ["user", "host", "path", "git", "symbol"]
right_segments = ["status", "runtime", "duration", "time"]
```

Canonical examples live in `config/default.toml` and
`config/themes/default.toml`. The loader validates order, uniqueness and side.

For desktop sessions, the child process serializes the layout as JSON through
private OSC `777`. xterm consumes it before display, so the WebView never probes
Git, runtime or duration. Ordinary CLI sessions never emit this metadata.

Add context providers only when their cost is bounded, evolve themes without
coupling segments to colors, and version the OSC contract before adding another
consumer. ADR-0003 defines the segment-based direction.
