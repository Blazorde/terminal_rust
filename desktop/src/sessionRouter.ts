// Delivers the events of one terminal session to its tab.
//
// The shell starts writing as soon as the session exists on the Rust side, but
// the tab only learns the session id when `start_session` answers. Events that
// arrive in between used to be dropped, which could lose the first prompt. They
// are held, in order, and delivered once the id is known.

// Enough for a burst of start-up output, small enough never to matter in memory.
export const maxEarlyEvents = 1000

interface EarlyEvent {
  deliver: () => void
  sessionId: number
}

export interface SessionRouter {
  /** The id returned by `start_session`: delivers what arrived before it. */
  attach: (sessionId: number) => void
  /** An event for `sessionId`: delivered now, held, or ignored if it is not ours. */
  receive: (sessionId: number, deliver: () => void) => void
}

export function createSessionRouter(): SessionRouter {
  let current: number | null = null
  let early: EarlyEvent[] = []

  return {
    attach(sessionId) {
      current = sessionId
      const held = early
      early = []
      for (const event of held) {
        if (event.sessionId === sessionId) event.deliver()
      }
    },
    receive(sessionId, deliver) {
      if (current === null) {
        early.push({ deliver, sessionId })
        if (early.length > maxEarlyEvents) early.shift()
      } else if (sessionId === current) {
        deliver()
      }
    },
  }
}
