# Desktop GUI quality audit

Status: Windows audit completed on 2026-10-11. A pinned Debian Bookworm
container completed the Linux Rust, native dependency and headless PTY gate on
2026-10-13. macOS and a visible Linux WebKitGTK audit are still missing.

## Result

| Dimension | Score | Main evidence |
| --- | ---: | --- |
| Accessibility | 4/4 | Keyboard navigation, bounded modal focus and AA contrast |
| Performance | 3/4 | Reproducible measurements and split bundle; WebView memory remains high |
| Responsive | 4/4 | No overflow at 1280×720, 560×400 or 320×240 |
| Themes | 4/4 | All six palettes exceed 4.5:1 for text |
| Integrity | 4/4 | No detector findings and no simulated data |
| **Total** | **19/20** | **Excellent locally; two visual OS audits remain** |

The audit fixed low-contrast faint text, focus escaping the settings drawer and
an overbroad reduced-motion rule. The drawer now traps focus while open, returns
it to Settings on close and makes the background inert. Reduced motion disables
only the activity rail, connection pulse, drawer transition and cursor blink.

`npm run audit:ui --prefix desktop` checks actual CSS tokens and fails below
4.5:1 for text or 3:1 for focus. The minimum native window is 560×400; the
drawer remains scrollable and the tab strip scrolls independently.

## Windows reference measurements

Reference machine: Windows 11 Pro `10.0.26200`, Intel Core Ultra 9 185H, 31.5
GiB, Rust 1.95.0 and Node 24.11.1. Release-build results:

| Trial | Startup | Processes | Memory |
| ---: | ---: | ---: | ---: |
| Cold | 541.1 ms | 10 | 409.5 MiB |
| Warm 1 | 174.4 ms | 10 | 406.7 MiB |
| Warm 2 | 184.8 ms | 10 | 406.6 MiB |

The ignored PTY probe measured first prompt at 70.360–86.049 ms and `pwd`
round trips at 0.566–0.850 ms. These figures are a Windows baseline, not proof
for WebKitGTK or WKWebView.

Frontend bundle: application 236.38 kB (74.07 kB gzip), xterm 332.16 kB (83.42
kB gzip), Tauri API 1.14 kB (0.55 kB gzip), CSS 14.59 kB (3.99 kB gzip).

## Platform matrix

| System | Build | Rust tests | PTY probe | Manual check | State |
| --- | --- | --- | --- | --- | --- |
| Windows 11 | passed | passed | passed | passed | locally validated |
| macOS | not run | not run | not run | not run | pending |
| Linux (Debian Bookworm) | DEB and AppImage passed | passed | packaged AppImage passed | not run | headless validated; visible GUI pending |

The Linux gate used Rust 1.95.0 and the native Tauri dependencies declared in
the pinned validation image. Formatting, all-target/all-feature compilation,
Clippy with warnings denied and 182 automated tests passed; one manual
performance probe remained intentionally ignored. During validation, the PTY
test was made prompt-synchronous and environment-dependent tests were made
hermetic so that clean Linux installations behave like Windows checkouts.

The pinned image also built the native Tauri packages after adding Node.js 24,
`patchelf` and an explicit square icon set. The DEB was 4,197,358 bytes with
SHA-256 `6426b8cb9091554f5c3e900e69f120a91b3103219ea2edde94b3604def32a708`;
the AppImage was 99,920,376 bytes with SHA-256
`5c30bfd914469e7d0ab2aeb6582f40f6806c3802ca403c5be2a1b7d0d9f36cea`.
Both packages contain the executable, MIT license, third-party notices and
configuration. The AppImage stayed alive under Xvfb, spawned its packaged
`--pty-shell` child and executed `pwd` successfully through a pseudo-terminal.
This is reproducible headless launch evidence, not a visible WebKitGTK audit.

For each missing visual audit, run the [desktop guide](desktop.md), record
hardware and OS, three startup/memory trials and captures at 1280×720 and
560×400. CI that fails before executing steps is not evidence; T124 remains
open until macOS and the visible Linux window are checked.
