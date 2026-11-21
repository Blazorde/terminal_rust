// Copying the terminal selection. Ctrl+C must stay the interrupt key, so outside
// macOS the shortcut is Ctrl+Shift+C (or Ctrl+Insert), as in other terminals.

export interface CopyEvent {
  altKey: boolean
  ctrlKey: boolean
  key: string
  metaKey: boolean
  shiftKey: boolean
}

export function isCopyShortcut(event: CopyEvent, mac: boolean): boolean {
  if (event.altKey) return false
  const key = event.key.toLowerCase()
  if (mac) return event.metaKey && !event.ctrlKey && !event.shiftKey && key === 'c'
  if (event.metaKey) return false
  if (event.ctrlKey && event.shiftKey && key === 'c') return true
  return event.ctrlKey && !event.shiftKey && key === 'insert'
}

/** Put `text` on the system clipboard. Resolves to whether it worked. */
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    // Not allowed or not available in this webview: use the old way.
  }
  const area = document.createElement('textarea')
  area.value = text
  area.setAttribute('readonly', '')
  area.style.position = 'fixed'
  area.style.opacity = '0'
  document.body.appendChild(area)
  area.select()
  let copied = false
  try {
    copied = document.execCommand('copy')
  } catch {
    copied = false
  }
  area.remove()
  return copied
}
