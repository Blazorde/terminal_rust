export type PromptSegmentKind =
  | 'user'
  | 'host'
  | 'path'
  | 'git'
  | 'status'
  | 'runtime'
  | 'duration'
  | 'time'
  | 'symbol'

export interface PromptSegment {
  kind: PromptSegmentKind
  label: string | null
  value: string | null
  styleKey: string
  icon: { name: string; fallback: string } | null
}

export interface PromptLayout {
  left: PromptSegment[]
  right: PromptSegment[]
}

export const emptyPromptLayout: PromptLayout = { left: [], right: [] }

export function parsePromptLayout(payload: string): PromptLayout | null {
  try {
    const value: unknown = JSON.parse(payload)
    if (!isObject(value) || !Array.isArray(value.left) || !Array.isArray(value.right)) return null
    if (![...value.left, ...value.right].every(isPromptSegment)) return null
    return value as unknown as PromptLayout
  } catch {
    return null
  }
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function isPromptSegment(value: unknown): boolean {
  return isObject(value)
    && typeof value.kind === 'string'
    && (value.label === null || typeof value.label === 'string')
    && (value.value === null || typeof value.value === 'string')
    && typeof value.styleKey === 'string'
}

function SegmentGroup({ segments, side }: { segments: PromptSegment[]; side: 'left' | 'right' }) {
  return (
    <span className={`prompt-segments prompt-segments--${side}`}>
      {segments.map((segment) => (
        <span
          key={segment.kind}
          className="prompt-segment"
          data-style={segment.styleKey}
          title={segment.label ?? segment.kind}
        >
          {segment.label && <span className="prompt-segment__label">{segment.label}</span>}
          <span className="prompt-segment__value">{segment.value}</span>
        </span>
      ))}
    </span>
  )
}

export default function PromptSegments({
  backgroundInert,
  layout,
  dimensions,
}: {
  backgroundInert: boolean
  layout: PromptLayout
  dimensions: string
}) {
  const hasSegments = layout.left.length > 0 || layout.right.length > 0

  return (
    <footer className="statusbar" aria-label="Contexte du prompt Rust" inert={backgroundInert}>
      {hasSegments ? (
        <>
          <SegmentGroup segments={layout.left} side="left" />
          <SegmentGroup segments={layout.right} side="right" />
        </>
      ) : (
        <span className="prompt-awaiting">moteur Rust · contexte en attente</span>
      )}
      <span className="status-metrics">{dimensions} · UTF-8</span>
    </footer>
  )
}
