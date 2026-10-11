# Terminal Rust desktop design

## Direction

**Graphite workshop.** The application should feel like a precise workbench:
dark mineral surfaces, hairline construction marks, compact labels and one warm
orange signal. The terminal content is the artifact; chrome recedes around it.
The Atelier reference supplies composition, while implementation removes its
decorative blur, remote fonts and simulated content.

## First viewport

A near-full-window terminal sits inside a restrained 32-pixel workspace margin.
The title bar names Terminal Rust, exposes live connection state and keeps a
single settings action. The PTY fills the central surface immediately. A slim
footer reports the native shell, dimensions and platform. The memorable detail
is a calibrated orange activity rail that responds to live PTY traffic without
competing with terminal text.

## Tokens

```css
:root {
  --page: #0b0d0f;
  --surface: #14171a;
  --surface-raised: #1a1e22;
  --ink: #e7e2d8;
  --muted: #969ca3;
  --faint: #626970;
  --accent: #ff6b3d;
  --success: #91c99b;
  --danger: #ff8178;
  --line: #30353a;
  --focus: #ffb199;
  --radius-window: 14px;
  --radius-control: 8px;
  --shadow-window: 0 24px 64px rgb(0 0 0 / 45%);
}
```

## Typography

- Interface: `Instrument Sans` only when bundled locally; otherwise
  `Segoe UI`, `Inter`, system sans-serif.
- Terminal: `JetBrains Mono` only when bundled locally; otherwise `Cascadia
  Mono`, `SFMono-Regular`, `DejaVu Sans Mono`, monospace.
- Monospace is reserved for terminal content, paths, dimensions and shortcuts.

## Layout and behavior

- Desktop margin scales from 12 to 32 pixels; the terminal always keeps useful
  height in a small window.
- The grid belongs to the workshop canvas and stays below 4% visual contrast.
- Borders define structure; shadow is reserved for the main window.
- Settings use a persistent side drawer on wide windows and an overlay drawer
  on narrow windows.
- Motion is limited to the live activity rail and drawer transition, both
  disabled by `prefers-reduced-motion`.

## States

- Connecting: terminal remains visible with a textual status in the title bar.
- Ready: focus moves to the terminal and the status names the native shell.
- Error: a persistent recovery strip explains the failure and offers retry.
- Closed: output remains readable and a restart action replaces the status.
- Focus: every interactive control uses the warm focus token, never color alone.

## Anti-patterns

- No fake command transcript, repository or performance data.
- No remote font or asset request.
- No gradient text, decorative glass, nested cards or generic icon tiles.
- No JavaScript access to process spawning or unrestricted filesystem APIs.
