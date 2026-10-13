import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const desktopRoot = fileURLToPath(new URL('..', import.meta.url))
const css = readFileSync(`${desktopRoot}/src/styles.css`, 'utf8')
const app = readFileSync(`${desktopRoot}/src/App.tsx`, 'utf8')
const drawer = readFileSync(`${desktopRoot}/src/SettingsDrawer.tsx`, 'utf8')
const settings = readFileSync(`${desktopRoot}/src/settings.ts`, 'utf8')
const terminalPane = readFileSync(`${desktopRoot}/src/TerminalPane.tsx`, 'utf8')

function block(selector) {
  const start = css.indexOf(`${selector} {`)
  assert.notEqual(start, -1, `missing CSS block: ${selector}`)
  const end = css.indexOf('}', start)
  return css.slice(start, end + 1)
}

function variables(selector, inherited = {}) {
  const result = { ...inherited }
  for (const [, name, value] of block(selector).matchAll(/--([\w-]+):\s*(#[\da-f]{6});/gi)) {
    result[name] = value
  }
  return result
}

function rgb(hex) {
  return [1, 3, 5].map((offset) => Number.parseInt(hex.slice(offset, offset + 2), 16))
}

function luminance(hex) {
  return rgb(hex)
    .map((channel) => {
      const value = channel / 255
      return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
    })
    .reduce((sum, value, index) => sum + value * [0.2126, 0.7152, 0.0722][index], 0)
}

function contrast(first, second) {
  const [lighter, darker] = [luminance(first), luminance(second)].sort((a, b) => b - a)
  return (lighter + 0.05) / (darker + 0.05)
}

const base = variables(':root')
const themes = {
  graphite: base,
  bone: variables(".workspace[data-theme='bone']", base),
  nord: variables(".workspace[data-theme='nord']", base),
  amber: variables(".workspace[data-theme='amber']", base),
  moss: variables(".workspace[data-theme='moss']", base),
  dusk: variables(".workspace[data-theme='dusk']", base),
}
const ratios = {}

for (const [theme, tokens] of Object.entries(themes)) {
  ratios[theme] = {}
  for (const foreground of ['ink', 'muted', 'faint', 'accent', 'danger']) {
    const ratio = contrast(tokens[foreground], tokens.surface)
    ratios[theme][`${foreground}/surface`] = Number(ratio.toFixed(2))
    assert.ok(ratio >= 4.5, `${theme} ${foreground}/surface contrast is ${ratio.toFixed(2)}:1`)
  }
  for (const [foreground, background] of [['accent-ink', 'accent'], ['danger-ink', 'danger']]) {
    const ratio = contrast(tokens[foreground], tokens[background])
    ratios[theme][`${foreground}/${background}`] = Number(ratio.toFixed(2))
    assert.ok(ratio >= 4.5, `${theme} ${foreground}/${background} contrast is ${ratio.toFixed(2)}:1`)
  }
  const focusRatio = contrast(tokens.focus, tokens.surface)
  ratios[theme]['focus/surface'] = Number(focusRatio.toFixed(2))
  assert.ok(focusRatio >= 3, `${theme} focus/surface contrast is ${focusRatio.toFixed(2)}:1`)
}

assert.match(css, /@media\s*\(prefers-reduced-motion:\s*reduce\)/)
assert.match(css, /\.activity--connecting,\s*\.traffic-rail--active\s*\{\s*animation:\s*none;/)
assert.match(css, /\.session-drawer\s*\{\s*transition:\s*none;/)
assert.doesNotMatch(css, /animation-duration:\s*\.01ms|transition-duration:\s*\.01ms/)
assert.match(css, /html, body, #root\s*\{[^}]*width:\s*100%;[^}]*height:\s*100%;/)
assert.match(block('.terminal-window'), /width:\s*100%;[\s\S]*height:\s*100%;[\s\S]*overflow:\s*hidden;/)
assert.match(settings, /cursorBlink\s*=\s*settings\.cursorBlink\s*&&\s*!reduceMotion/)
assert.match(terminalPane, /letterSpacing:\s*0/)
assert.match(terminalPane, /lineHeight:\s*1\.2/)
assert.match(terminalPane, /rescaleOverlappingGlyphs:\s*true/)
assert.match(terminalPane, /document\.fonts\.ready\.then\(refreshFontMetrics\)/)
assert.match(terminalPane, /new ResizeObserver\(scheduleFit\)/)
assert.match(terminalPane, /visualViewport\?\.addEventListener\('resize', scheduleFit\)/)
assert.match(terminalPane, /promptMetadataMarkers = \['\\x1b\]777;', '\\u2190\]777;'\]/)
assert.match(terminalPane, /createPromptMetadataFilter/)
assert.match(terminalPane, /inputQueue = inputQueue/)
assert.match(app, /startDragging/)
assert.match(app, /startResizeDragging/)
assert.match(block('.terminal-host'), /overflow:\s*hidden;/)
assert.match(drawer, /role="dialog"/)
assert.match(drawer, /onKeyDown=\{keepFocusInside\}/)
assert.ok((app.match(/inert=\{detailsOpen\}/g) ?? []).length >= 3, 'background regions must become inert')

console.log(JSON.stringify({ status: 'passed', ratios }, null, 2))
