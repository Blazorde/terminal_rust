import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import { memo, useEffect, useRef, useState } from 'react'

import { emptyPromptLayout, parsePromptLayout, type PromptLayout } from './PromptSegments'
import { applyTerminalSettings, type DesktopSettings } from './settings'
import type { ConnectionState } from './terminal-types'

interface PtyOutput { sessionId: number; data: number[] }
interface PtyExit { sessionId: number; success: boolean }
interface PtyError { sessionId: number; message: string }

export interface TerminalTabPatch {
  connection?: ConnectionState
  dimensions?: string
  error?: string | null
  promptLayout?: PromptLayout
  trafficActive?: boolean
}

interface TerminalPaneProps {
  active: boolean
  desktopHost: boolean
  focusEnabled: boolean
  onPatch: (tabId: string, patch: TerminalTabPatch) => void
  restartToken: number
  settings: DesktopSettings
  tabId: string
}

const encoder = new TextEncoder()
const promptMetadataMarkers = ['\x1b]777;', '\u2190]777;']
const noWrapStartMarkers = ['\x1b[?7l', '\u2190[?7l']
const noWrapEndMarkers = ['\x1b[?7h', '\u2190[?7h']

function stripTrailingMarker(value: string, markers: string[]): string {
  const marker = markers.find((candidate) => value.endsWith(candidate))
  return marker ? value.slice(0, -marker.length) : value
}

function stripLeadingMarker(value: string, markers: string[]): string {
  const marker = markers.find((candidate) => value.startsWith(candidate))
  return marker ? value.slice(marker.length) : value
}

function createPromptMetadataFilter(onPromptLayout: (layout: PromptLayout) => void) {
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
      let payloadEnd = -1
      let promptLayout: PromptLayout | null = null
      for (let index = payloadStart; index < pending.length; index += 1) {
        if (pending[index] !== '}') continue
        const candidate = pending.slice(payloadStart, index + 1).replace(/[\r\n]/g, '')
        promptLayout = parsePromptLayout(candidate)
        if (promptLayout) {
          payloadEnd = index + 1
          break
        }
      }
      if (payloadEnd < 0 || !promptLayout) {
        pending = pending.slice(marker.index)
        break
      }

      onPromptLayout(promptLayout)
      pending = pending.slice(payloadEnd)
      if (pending.startsWith('\x07')) pending = pending.slice(1)
      if (pending.startsWith('\x1b\\')) pending = pending.slice(2)
      pending = stripLeadingMarker(pending, noWrapEndMarkers)
    }

    return visible
  }
}

function TerminalPane({
  active,
  desktopHost,
  focusEnabled,
  onPatch,
  restartToken,
  settings,
  tabId,
}: TerminalPaneProps) {
  const host = useRef<HTMLDivElement>(null)
  const terminal = useRef<Terminal | null>(null)
  const fitAddon = useRef<FitAddon | null>(null)
  const refitTerminal = useRef<(() => void) | null>(null)
  const [reduceMotion, setReduceMotion] = useState(() => window.matchMedia('(prefers-reduced-motion: reduce)').matches)

  useEffect(() => {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)')
    const update = ({ matches }: MediaQueryListEvent) => setReduceMotion(matches)
    query.addEventListener('change', update)
    return () => query.removeEventListener('change', update)
  }, [])

  useEffect(() => {
    const element = host.current
    if (!element) return

    const term = new Terminal({
      allowProposedApi: false,
      convertEol: true,
      letterSpacing: 0,
      lineHeight: 1.2,
      rescaleOverlappingGlyphs: true,
      scrollback: 5000,
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    applyTerminalSettings(term, settings, reduceMotion)
    term.open(element)
    terminal.current = term
    fitAddon.current = fit

    let sessionId: number | null = null
    let listeners: UnlistenFn[] = []
    let terminalDisposables: Array<{ dispose: () => void }> = []
    let trafficTimer: number | null = null
    let fitFrame: number | null = null
    let inputQueue: Promise<void> = Promise.resolve()
    let disposed = false
    const filterPromptMetadata = createPromptMetadataFilter((promptLayout) => onPatch(tabId, { promptLayout }))

    const fitToHost = () => {
      if (disposed || element.clientWidth === 0 || element.clientHeight === 0) return
      fit.fit()
      onPatch(tabId, { dimensions: `${term.cols} × ${term.rows}` })
    }

    const scheduleFit = () => {
      if (fitFrame !== null) window.cancelAnimationFrame(fitFrame)
      fitFrame = window.requestAnimationFrame(() => {
        fitFrame = window.requestAnimationFrame(() => {
          fitFrame = null
          fitToHost()
        })
      })
    }

    const refreshFontMetrics = () => {
      if (disposed) return
      term.clearTextureAtlas()
      scheduleFit()
    }

    refitTerminal.current = scheduleFit
    const resizeObserver = new ResizeObserver(scheduleFit)
    resizeObserver.observe(element)
    window.visualViewport?.addEventListener('resize', scheduleFit)
    document.fonts.addEventListener('loadingdone', refreshFontMetrics)
    void document.fonts.ready.then(refreshFontMetrics)

    const connect = async () => {
      fitToHost()
      if (!desktopHost) {
        onPatch(tabId, { connection: 'closed', promptLayout: emptyPromptLayout })
        term.writeln('\x1b[38;2;255;107;61mTerminal Rust — aperçu interface\x1b[0m')
        term.writeln('\x1b[38;2;150;156;163mLes sessions et onglets réels démarrent uniquement dans la fenêtre Tauri.\x1b[0m')
        return
      }

      onPatch(tabId, { connection: 'connecting', error: null })
      terminalDisposables = [
        term.onData((data) => {
          if (sessionId === null) return
          const targetSessionId = sessionId
          const bytes = Array.from(encoder.encode(data))
          inputQueue = inputQueue
            .then(() => invoke('write_session', { sessionId: targetSessionId, data: bytes }))
            .then(() => undefined)
            .catch(() => undefined)
        }),
        term.onResize(({ cols, rows }) => {
          onPatch(tabId, { dimensions: `${cols} × ${rows}` })
          if (sessionId !== null) {
            void invoke('resize_session', { sessionId, cols, rows }).catch(() => undefined)
          }
        }),
      ]

      listeners = await Promise.all([
        listen<PtyOutput>('pty-output', ({ payload }) => {
          if (payload.sessionId !== sessionId) return
          const visible = filterPromptMetadata(new Uint8Array(payload.data))
          if (visible) term.write(visible)
          onPatch(tabId, { trafficActive: true })
          if (trafficTimer !== null) window.clearTimeout(trafficTimer)
          trafficTimer = window.setTimeout(() => onPatch(tabId, { trafficActive: false }), 140)
        }),
        listen<PtyExit>('pty-exit', ({ payload }) => {
          if (payload.sessionId !== sessionId) return
          onPatch(tabId, { connection: 'closed' })
          term.writeln(`\r\n\x1b[38;2;150;156;163mSession terminée${payload.success ? '' : ' avec une erreur'}.\x1b[0m`)
        }),
        listen<PtyError>('pty-error', ({ payload }) => {
          if (payload.sessionId === sessionId) {
            onPatch(tabId, { connection: 'error', error: payload.message })
          }
        }),
      ])
      if (disposed) {
        terminalDisposables.forEach((disposable) => disposable.dispose())
        listeners.forEach((unlisten) => unlisten())
        terminalDisposables = []
        listeners = []
        return
      }

      const createdSessionId = await invoke<number>('start_session', { cols: term.cols, rows: term.rows })
      if (disposed) {
        void invoke('close_session', { sessionId: createdSessionId }).catch(() => undefined)
        return
      }
      sessionId = createdSessionId
      onPatch(tabId, { connection: 'ready' })
      if (active) term.focus()

    }

    void connect()
      .catch((reason) => onPatch(tabId, { connection: 'error', error: String(reason) }))

    return () => {
      disposed = true
      resizeObserver.disconnect()
      window.visualViewport?.removeEventListener('resize', scheduleFit)
      document.fonts.removeEventListener('loadingdone', refreshFontMetrics)
      if (fitFrame !== null) window.cancelAnimationFrame(fitFrame)
      terminalDisposables.forEach((disposable) => disposable.dispose())
      listeners.forEach((unlisten) => unlisten())
      if (trafficTimer !== null) window.clearTimeout(trafficTimer)
      if (sessionId !== null) void invoke('close_session', { sessionId }).catch(() => undefined)
      term.dispose()
      terminal.current = null
      fitAddon.current = null
      if (refitTerminal.current === scheduleFit) refitTerminal.current = null
    }
  }, [desktopHost, restartToken, tabId, onPatch])

  useEffect(() => {
    const term = terminal.current
    const fit = fitAddon.current
    if (!term || !fit) return
    applyTerminalSettings(term, settings, reduceMotion)
    term.clearTextureAtlas()
    refitTerminal.current?.()
  }, [settings, reduceMotion])

  useEffect(() => {
    if (!active || !focusEnabled) return
    const term = terminal.current
    window.requestAnimationFrame(() => {
      refitTerminal.current?.()
      term?.focus()
    })
  }, [active, focusEnabled])

  return (
    <div
      className={`terminal-pane ${active ? 'terminal-pane--active' : ''}`}
      id={`panel-${tabId}`}
      role="tabpanel"
      aria-labelledby={`tab-${tabId}`}
      hidden={!active}
      onMouseDown={() => terminal.current?.focus()}
    >
      <div ref={host} className="terminal-host" />
    </div>
  )
}

export default memo(TerminalPane)
