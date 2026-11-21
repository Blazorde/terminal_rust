// Pulls the prompt layout the shell sends as `ESC ] 777 ; <json> BEL` out of the
// terminal output. Anything else, including other `ESC ] 777` notifications from
// a program, goes to the terminal untouched.

const promptMetadataMarkers = ['\x1b]777;', '\u2190]777;']
const noWrapStartMarkers = ['\x1b[?7l', '\u2190[?7l']
const noWrapEndMarkers = ['\x1b[?7h', '\u2190[?7h']

// A real layout is well under this. A longer unterminated sequence is not ours,
// so it is released instead of being held back for ever.
export const maxPromptPayloadLength = 4096

function stripTrailingMarker(value: string, markers: string[]): string {
  const marker = markers.find((candidate) => value.endsWith(candidate))
  return marker ? value.slice(0, -marker.length) : value
}

function stripLeadingMarker(value: string, markers: string[]): string {
  const marker = markers.find((candidate) => value.startsWith(candidate))
  return marker ? value.slice(marker.length) : value
}

// Where the OSC sequence ends: the first BEL or ST (`ESC \`) from `from`.
function findTerminator(value: string, from: number): { index: number; length: number } | null {
  const bell = value.indexOf('\x07', from)
  const stringTerminator = value.indexOf('\x1b\\', from)
  if (bell < 0 && stringTerminator < 0) return null
  if (stringTerminator < 0 || (bell >= 0 && bell < stringTerminator)) {
    return { index: bell, length: 1 }
  }
  return { index: stringTerminator, length: 2 }
}

export function createPromptMetadataFilter<Layout>(
  parseLayout: (payload: string) => Layout | null,
  onPromptLayout: (layout: Layout) => void,
) {
  const decoder = new TextDecoder()
  let pending = ''

  return (data: Uint8Array): string => {
    pending += decoder.decode(data, { stream: true })
    let visible = ''

    while (pending.length > 0) {
      const marker = promptMetadataMarkers
        .map((value) => ({ index: pending.indexOf(value), value }))
        .filter(({ index }) => index >= 0)
        .sort((left, right) => left.index - right.index)[0]

      if (!marker) {
        const partialLength = promptMetadataMarkers.reduce((longest, value) => {
          for (let length = Math.min(value.length - 1, pending.length); length > longest; length -= 1) {
            if (pending.endsWith(value.slice(0, length))) return length
          }
          return longest
        }, 0)
        visible += pending.slice(0, pending.length - partialLength)
        pending = pending.slice(pending.length - partialLength)
        break
      }

      visible += stripTrailingMarker(pending.slice(0, marker.index), noWrapStartMarkers)
      const payloadStart = marker.index + marker.value.length
      const terminator = findTerminator(pending, payloadStart)

      if (!terminator) {
        if (pending.length - payloadStart > maxPromptPayloadLength) {
          visible += pending.slice(marker.index)
          pending = ''
        } else {
          // The rest of the sequence has not arrived yet.
          pending = pending.slice(marker.index)
        }
        break
      }

      const payload = pending.slice(payloadStart, terminator.index).replace(/[\r\n]/g, '')
      const sequenceEnd = terminator.index + terminator.length
      const layout = payload.length <= maxPromptPayloadLength ? parseLayout(payload) : null

      if (layout === null) {
        // Not a prompt layout: let the terminal handle the whole sequence.
        visible += pending.slice(marker.index, sequenceEnd)
        pending = pending.slice(sequenceEnd)
        continue
      }

      onPromptLayout(layout)
      pending = stripLeadingMarker(pending.slice(sequenceEnd), noWrapEndMarkers)
    }

    return visible
  }
}
