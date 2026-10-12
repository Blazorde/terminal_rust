# Desktop GUI quality audit

Status: Windows audit completed on 2026-10-11; macOS and Linux evidence is still
missing because no runner has executed the required steps.

## Result

| Dimension | Score | Main evidence |
| --- | ---: | --- |
| Accessibility | 4/4 | Keyboard navigation, bounded modal focus and AA contrast |
| Performance | 3/4 | Reproducible measurements and split bundle; WebView memory remains high |
| Responsive | 4/4 | No overflow at 1280×720, 560×400 or 320×240 |
| Themes | 4/4 | All three palettes exceed 4.5:1 for text |
| Integrity | 4/4 | No detector findings and no simulated data |
| **Total** | **19/20** | **Excellent locally; two OS validations remain** |

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
| Linux | not run | not run | not run | not run | pending |

For each missing system, run the [desktop guide](desktop.md), record hardware
and OS, three startup/memory trials and captures at 1280×720 and 560×400. CI
that fails before executing steps is not evidence; T124 remains open.
