import assert from 'node:assert/strict'
import { test } from 'node:test'

import { isCopyShortcut, type CopyEvent } from './clipboard.ts'

function press(key: string, modifiers: Partial<CopyEvent> = {}): CopyEvent {
  return { altKey: false, ctrlKey: false, key, metaKey: false, shiftKey: false, ...modifiers }
}

test('Ctrl+C stays the interrupt key outside macOS', () => {
  assert.equal(isCopyShortcut(press('c', { ctrlKey: true }), false), false)
})

test('Ctrl+Shift+C and Ctrl+Insert copy outside macOS', () => {
  assert.equal(isCopyShortcut(press('C', { ctrlKey: true, shiftKey: true }), false), true)
  assert.equal(isCopyShortcut(press('Insert', { ctrlKey: true }), false), true)
})

test('Command+C copies on macOS and Ctrl+C does not', () => {
  assert.equal(isCopyShortcut(press('c', { metaKey: true }), true), true)
  assert.equal(isCopyShortcut(press('c', { ctrlKey: true }), true), false)
  assert.equal(isCopyShortcut(press('C', { ctrlKey: true, shiftKey: true }), true), false)
})

test('the Super key does not copy on Linux and Windows', () => {
  assert.equal(isCopyShortcut(press('c', { metaKey: true }), false), false)
})

test('Alt combinations and other keys are not copy', () => {
  assert.equal(isCopyShortcut(press('C', { ctrlKey: true, shiftKey: true, altKey: true }), false), false)
  assert.equal(isCopyShortcut(press('v', { ctrlKey: true, shiftKey: true }), false), false)
  assert.equal(isCopyShortcut(press('Insert', { shiftKey: true }), false), false)
  assert.equal(isCopyShortcut(press('c'), false), false)
})
