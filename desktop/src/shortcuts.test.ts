import assert from 'node:assert/strict'
import { test } from 'node:test'

import { isMacPlatform, shortcutAction, shortcutLabel, type ShortcutEvent } from './shortcuts.ts'

function press(key: string, modifiers: Partial<ShortcutEvent> = {}): ShortcutEvent {
  return { altKey: false, ctrlKey: false, key, metaKey: false, shiftKey: false, ...modifiers }
}

test('Ctrl+W and Ctrl+T reach the shell on Linux and Windows', () => {
  assert.equal(shortcutAction(press('w', { ctrlKey: true }), false), null)
  assert.equal(shortcutAction(press('t', { ctrlKey: true }), false), null)
})

test('every other Ctrl letter of the line editor is left alone', () => {
  for (const letter of 'abcdefghijklmnopqrsuvxyz') {
    assert.equal(shortcutAction(press(letter, { ctrlKey: true }), false), null, `Ctrl+${letter}`)
  }
})

test('tabs use Ctrl+Shift+T and Ctrl+Shift+W outside macOS', () => {
  // With Shift, the browser reports the capital letter.
  assert.equal(shortcutAction(press('T', { ctrlKey: true, shiftKey: true }), false), 'new-tab')
  assert.equal(shortcutAction(press('W', { ctrlKey: true, shiftKey: true }), false), 'close-tab')
})

test('Ctrl+Tab and Ctrl+Shift+Tab switch tabs', () => {
  assert.equal(shortcutAction(press('Tab', { ctrlKey: true }), false), 'next-tab')
  assert.equal(shortcutAction(press('Tab', { ctrlKey: true, shiftKey: true }), true), 'previous-tab')
})

test('Ctrl+comma opens the settings', () => {
  assert.equal(shortcutAction(press(',', { ctrlKey: true }), false), 'toggle-settings')
})

test('macOS keeps Command+T, Command+W and Command+comma', () => {
  assert.equal(shortcutAction(press('t', { metaKey: true }), true), 'new-tab')
  assert.equal(shortcutAction(press('w', { metaKey: true }), true), 'close-tab')
  assert.equal(shortcutAction(press(',', { metaKey: true }), true), 'toggle-settings')
})

test('on macOS Ctrl+W still reaches the shell', () => {
  assert.equal(shortcutAction(press('w', { ctrlKey: true }), true), null)
  assert.equal(shortcutAction(press('t', { ctrlKey: true }), true), null)
})

test('the Super key does not close tabs on Linux', () => {
  assert.equal(shortcutAction(press('w', { metaKey: true }), false), null)
})

test('Alt combinations belong to the shell', () => {
  assert.equal(shortcutAction(press('t', { ctrlKey: true, shiftKey: true, altKey: true }), false), null)
})

test('Escape closes the settings', () => {
  assert.equal(shortcutAction(press('Escape'), false), 'close-settings')
})

test('plain typing is never a shortcut', () => {
  assert.equal(shortcutAction(press('t'), false), null)
  assert.equal(shortcutAction(press('W', { shiftKey: true }), false), null)
})

test('platform detection and labels', () => {
  assert.equal(isMacPlatform('MacIntel'), true)
  assert.equal(isMacPlatform('Linux x86_64'), false)
  assert.equal(isMacPlatform('Win32'), false)
  assert.equal(shortcutLabel('new-tab', false), 'Ctrl+Maj+T')
  assert.equal(shortcutLabel('close-tab', true), '⌘W')
  assert.equal(shortcutLabel('toggle-settings', false), 'Ctrl+,')
})
