// Window shortcuts of the desktop app. A terminal must hand almost every Ctrl
// combination to the shell: Ctrl+W deletes a word and Ctrl+T swaps two characters
// in the line editor, so tabs use Ctrl+Shift+T and Ctrl+Shift+W (as GNOME
// Terminal and Windows Terminal do) and only macOS uses plain Command+T/W.

export type ShortcutAction =
  | 'toggle-settings'
  | 'new-tab'
  | 'close-tab'
  | 'next-tab'
  | 'previous-tab'
  | 'close-settings'

export interface ShortcutEvent {
  altKey: boolean
  ctrlKey: boolean
  key: string
  metaKey: boolean
  shiftKey: boolean
}

export function isMacPlatform(platform: string): boolean {
  return /mac|iphone|ipad/i.test(platform)
}

export function shortcutAction(event: ShortcutEvent, mac: boolean): ShortcutAction | null {
  if (event.key === 'Escape') return 'close-settings'
  if (event.altKey) return null

  const key = event.key.toLowerCase()
  const ctrl = event.ctrlKey
  const meta = event.metaKey

  if (ctrl && !meta && key === 'tab') return event.shiftKey ? 'previous-tab' : 'next-tab'

  if (mac) {
    // Command is never sent to a terminal program, so it cannot clash with it.
    if (!meta || ctrl) return null
    if (key === ',' && !event.shiftKey) return 'toggle-settings'
    if (key === 't' && !event.shiftKey) return 'new-tab'
    if (key === 'w' && !event.shiftKey) return 'close-tab'
    return null
  }

  if (!ctrl || meta) return null
  // Ctrl+, is not a line-editor binding, the tab keys need Shift.
  if (key === ',' && !event.shiftKey) return 'toggle-settings'
  if (key === 't' && event.shiftKey) return 'new-tab'
  if (key === 'w' && event.shiftKey) return 'close-tab'
  return null
}

export function shortcutLabel(action: 'toggle-settings' | 'new-tab' | 'close-tab', mac: boolean): string {
  const key = { 'toggle-settings': ',', 'new-tab': 'T', 'close-tab': 'W' }[action]
  if (mac) return `⌘${key}`
  return action === 'toggle-settings' ? `Ctrl+${key}` : `Ctrl+Maj+${key}`
}
