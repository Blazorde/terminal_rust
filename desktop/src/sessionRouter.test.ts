import assert from 'node:assert/strict'
import { test } from 'node:test'

import { createSessionRouter, maxEarlyEvents } from './sessionRouter.ts'

function recorder() {
  const seen: string[] = []
  return { seen, add: (label: string) => () => seen.push(label) }
}

test('events of the attached session are delivered at once', () => {
  const { seen, add } = recorder()
  const router = createSessionRouter()
  router.attach(7)

  router.receive(7, add('first'))
  router.receive(7, add('second'))

  assert.deepEqual(seen, ['first', 'second'])
})

test('events of another session are ignored', () => {
  const { seen, add } = recorder()
  const router = createSessionRouter()
  router.attach(7)

  router.receive(8, add('other'))

  assert.deepEqual(seen, [])
})

test('output that arrives before the session id is known is not lost', () => {
  const { seen, add } = recorder()
  const router = createSessionRouter()

  router.receive(7, add('prompt part 1'))
  router.receive(7, add('prompt part 2'))
  assert.deepEqual(seen, [], 'nothing is shown before the id is known')

  router.attach(7)

  assert.deepEqual(seen, ['prompt part 1', 'prompt part 2'])
})

test('held events keep their order across output and exit', () => {
  const { seen, add } = recorder()
  const router = createSessionRouter()

  router.receive(3, add('output'))
  router.receive(3, add('exit'))
  router.attach(3)
  router.receive(3, add('late'))

  assert.deepEqual(seen, ['output', 'exit', 'late'])
})

test('held events of another tab session are dropped when ours is attached', () => {
  const { seen, add } = recorder()
  const router = createSessionRouter()

  router.receive(2, add('belongs to another tab'))
  router.receive(5, add('ours'))
  router.attach(5)

  assert.deepEqual(seen, ['ours'])
})

test('what is held is bounded and keeps the most recent events', () => {
  const { seen, add } = recorder()
  const router = createSessionRouter()

  for (let index = 0; index < maxEarlyEvents + 50; index += 1) {
    router.receive(1, add(String(index)))
  }
  router.attach(1)

  assert.equal(seen.length, maxEarlyEvents)
  assert.equal(seen[0], '50')
  assert.equal(seen.at(-1), String(maxEarlyEvents + 49))
})

test('events are not delivered twice after attach', () => {
  const { seen, add } = recorder()
  const router = createSessionRouter()
  router.receive(4, add('early'))

  router.attach(4)
  router.attach(4)

  assert.deepEqual(seen, ['early'])
})
