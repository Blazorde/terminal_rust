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
      lineHeight: 1.45,
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
    let disposed = false

    const resize = () => {
      fit.fit()
      onPatch(tabId, { dimensions: `${term.cols} × ${term.rows}` })
      if (sessionId !== null) {
        void invoke('resize_session', { sessionId, cols: term.cols, rows: term.rows }).catch(() => undefined)
      }
    }
    const resizeObserver = new ResizeObserver(resize)
    resizeObserver.observe(element)

    const connect = async () => {
      resize()
      if (!desktopHost) {
        onPatch(tabId, { connection: 'closed', promptLayout: emptyPromptLayout })
        term.writeln('\x1b[38;2;255;107;61mTerminal Rust — aperçu interface\x1b[0m')
        term.writeln('\x1b[38;2;150;156;163mLes sessions et onglets réels démarrent uniquement dans la fenêtre Tauri.\x1b[0m')
        return
      }

      onPatch(tabId, { connection: 'connecting', error: null })
      terminalDisposables = [
        term.parser.registerOscHandler(777, (payload) => {
          const promptLayout = parsePromptLayout(payload)
          if (promptLayout) onPatch(tabId, { promptLayout })
          return true
        }),
        term.onData((data) => {
          if (sessionId === null) return
          void invoke('write_session', {
            sessionId,
            data: Array.from(encoder.encode(data)),
          }).catch(() => undefined)
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
          term.write(new Uint8Array(payload.data))
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
      terminalDisposables.forEach((disposable) => disposable.dispose())
      listeners.forEach((unlisten) => unlisten())
      if (trafficTimer !== null) window.clearTimeout(trafficTimer)
      if (sessionId !== null) void invoke('close_session', { sessionId }).catch(() => undefined)
      term.dispose()
      terminal.current = null
      fitAddon.current = null
    }
  }, [desktopHost, restartToken, tabId, onPatch])

  useEffect(() => {
    const term = terminal.current
    const fit = fitAddon.current
    if (!term || !fit) return
    applyTerminalSettings(term, settings, reduceMotion)
    window.requestAnimationFrame(() => fit.fit())
  }, [settings, reduceMotion])

  useEffect(() => {
    if (!active || !focusEnabled) return
    const fit = fitAddon.current
    const term = terminal.current
    window.requestAnimationFrame(() => {
      fit?.fit()
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
