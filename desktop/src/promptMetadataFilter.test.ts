import assert from 'node:assert/strict'
import { test } from 'node:test'

import { createPromptMetadataFilter, maxPromptPayloadLength } from './promptMetadataFilter.ts'

interface Layout {
  left: unknown[]
  right: unknown[]
}

const encoder = new TextEncoder()

// Accepts the same shape as the real parser: an object with `left` and `right` lists.
function parseLayout(payload: string): Layout | null {
  try {
    const value: unknown = JSON.parse(payload)
    if (typeof value !== 'object' || value === null) return null
    const layout = value as Layout
    return Array.isArray(layout.left) && Array.isArray(layout.right) ? layout : null
  } catch {
    return null
  }
}

function setup() {
  const layouts: Layout[] = []
  const filter = createPromptMetadataFilter(parseLayout, (layout) => layouts.push(layout))
  const feed = (text: string) => filter(encoder.encode(text))
  return { feed, layouts }
}

const layoutJson = JSON.stringify({ left: [{ kind: 'path' }], right: [] })

test('a prompt layout ended by BEL is consumed and delivered', () => {
  const { feed, layouts } = setup()

  const visible = feed(`before\x1b]777;${layoutJson}\x07after`)

  assert.equal(visible, 'beforeafter')
  assert.deepEqual(layouts, [{ left: [{ kind: 'path' }], right: [] }])
})

test('a prompt layout ended by ESC backslash is consumed too', () => {
  const { feed, layouts } = setup()

  const visible = feed(`\x1b]777;${layoutJson}\x1b\\prompt`)

  assert.equal(visible, 'prompt')
  assert.equal(layouts.length, 1)
})

test('another program notification is passed through untouched and does not freeze the output', () => {
  const { feed, layouts } = setup()
  const notification = '\x1b]777;notify;Build;done\x07'

  assert.equal(feed(`${notification}next line\r\n`), `${notification}next line\r\n`)
  assert.equal(feed('and more output'), 'and more output')
  assert.equal(layouts.length, 0)
})

test('output keeps flowing after a notification and a real layout still works', () => {
  const { feed, layouts } = setup()

  const visible = feed(`\x1b]777;notify;a;b\x07text\x1b]777;${layoutJson}\x07end`)

  assert.equal(visible, '\x1b]777;notify;a;b\x07textend')
  assert.equal(layouts.length, 1)
})

test('a layout split across several chunks is held until it is complete', () => {
  const { feed, layouts } = setup()

  assert.equal(feed(`x\x1b]777;${layoutJson.slice(0, 10)}`), 'x')
  assert.equal(layouts.length, 0)
  assert.equal(feed(`${layoutJson.slice(10)}\x07y`), 'y')
  assert.equal(layouts.length, 1)
})

test('a marker split between two chunks is not lost', () => {
  const { feed, layouts } = setup()

  assert.equal(feed('a\x1b]7'), 'a')
  assert.equal(feed(`77;${layoutJson}\x07b`), 'b')
  assert.equal(layouts.length, 1)
})

test('an unterminated sequence longer than the limit is released as plain text', () => {
  const { feed, layouts } = setup()
  const long = `\x1b]777;${'x'.repeat(maxPromptPayloadLength + 1)}`

  const visible = feed(long)

  assert.equal(visible, long)
  assert.equal(layouts.length, 0)
  assert.equal(feed('later'), 'later')
})

test('a short unterminated sequence waits for more output instead of being lost', () => {
  const { feed } = setup()

  assert.equal(feed('\x1b]777;notify;Build'), '')
  assert.equal(feed(';done\x07rest'), '\x1b]777;notify;Build;done\x07rest')
})

test('a terminated payload that is too long is passed through', () => {
  const { feed, layouts } = setup()
  const long = `\x1b]777;${'x'.repeat(maxPromptPayloadLength + 1)}\x07`

  assert.equal(feed(long), long)
  assert.equal(layouts.length, 0)
})

test('the no-wrap markers around a layout are removed', () => {
  const { feed } = setup()

  const visible = feed(`\x1b[?7l\x1b]777;${layoutJson}\x07\x1b[?7hprompt> `)

  assert.equal(visible, 'prompt> ')
})

test('plain output is returned unchanged', () => {
  const { feed, layouts } = setup()

  assert.equal(feed('ls\r\nfile.txt\r\n'), 'ls\r\nfile.txt\r\n')
  assert.equal(layouts.length, 0)
})
