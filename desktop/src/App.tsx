import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import { useCallback, useEffect, useRef, useState } from 'react'

type ConnectionState = 'connecting' | 'ready' | 'closed' | 'error'

interface PtyOutput {
  sessionId: number
  data: number[]
}

interface PtyExit {
  sessionId: number
  success: boolean
}

interface PtyError {
  sessionId: number
  message: string
}

const encoder = new TextEncoder()

export default function App() {
  const terminalHost = useRef<HTMLDivElement>(null)
  const terminal = useRef<Terminal | null>(null)
  const fitAddon = useRef<FitAddon | null>(null)
  const sessionId = useRef<number | null>(null)
  const closeButton = useRef<HTMLButtonElement>(null)
  const trafficTimer = useRef<number | null>(null)
  const [connection, setConnection] = useState<ConnectionState>('connecting')
  const [error, setError] = useState<string | null>(null)
  const [detailsOpen, setDetailsOpen] = useState(false)
  const [dimensions, setDimensions] = useState('80 × 24')
  const [trafficActive, setTrafficActive] = useState(false)
  const [desktopHost, setDesktopHost] = useState(false)

  const startSession = useCallback(async () => {
    const term = terminal.current
    const fit = fitAddon.current
    if (!term || !fit) return

    setConnection('connecting')
    setError(null)
    fit.fit()

    try {
      const id = await invoke<number>('start_session', {
        cols: term.cols,
        rows: term.rows,
      })
      sessionId.current = id
      setConnection('ready')
      term.focus()
    } catch (reason) {
      setConnection('error')
      setError(String(reason))
    }
  }, [])

  useEffect(() => {
    const host = terminalHost.current
    if (!host) return

    const term = new Terminal({
      allowProposedApi: false,
      convertEol: true,
      cursorBlink: true,
      cursorStyle: 'block',
      fontFamily: "'Cascadia Mono', 'SFMono-Regular', 'DejaVu Sans Mono', monospace",
      fontSize: 14,
      lineHeight: 1.45,
      scrollback: 5000,
      theme: {
        background: '#14171a',
        foreground: '#e7e2d8',
        cursor: '#ff6b3d',
        cursorAccent: '#14171a',
        selectionBackground: '#ff6b3d55',
        black: '#14171a',
        red: '#ff8178',
        green: '#91c99b',
        yellow: '#e6c66f',
        blue: '#82aaff',
        magenta: '#c099ff',
        cyan: '#79c9c5',
        white: '#e7e2d8',
        brightBlack: '#626970',
        brightWhite: '#fffaf0',
      },
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(host)
    terminal.current = term
    fitAddon.current = fit

    const renderPreview = () => {
      fit.fit()
      setDimensions(`${term.cols} × ${term.rows}`)
      setConnection('closed')
      term.writeln('\x1b[38;2;255;107;61mTerminal Rust — aperçu interface\x1b[0m')
      term.writeln('\x1b[38;2;150;156;163mLa session PTY native démarre uniquement dans la fenêtre Tauri.\x1b[0m')
    }

    const resizeObserver = new ResizeObserver(() => fit.fit())
    resizeObserver.observe(host)

    let disposables: Array<{ dispose: () => void }> = []
    let listeners: UnlistenFn[] = []
    let disposed = false

    const connectDesktop = async () => {
      try {
        await invoke<boolean>('is_terminal_rust_desktop')
      } catch {
        if (!disposed) renderPreview()
        return
      }

      if (disposed) return
      setDesktopHost(true)
      disposables = [
        term.onData((data) => {
          const id = sessionId.current
          if (id === null) return
          void invoke('write_session', {
            sessionId: id,
            data: Array.from(encoder.encode(data)),
          }).catch(() => undefined)
        }),
        term.onResize(({ cols, rows }) => {
          setDimensions(`${cols} × ${rows}`)
          const id = sessionId.current
          if (id === null) return
          void invoke('resize_session', { sessionId: id, cols, rows }).catch(() => undefined)
        }),
      ]

      const unlisten = await Promise.all([
        listen<PtyOutput>('pty-output', ({ payload }) => {
          if (payload.sessionId === sessionId.current) {
            term.write(new Uint8Array(payload.data))
            setTrafficActive(true)
            if (trafficTimer.current !== null) window.clearTimeout(trafficTimer.current)
            trafficTimer.current = window.setTimeout(() => setTrafficActive(false), 140)
          }
        }),
        listen<PtyExit>('pty-exit', ({ payload }) => {
          if (payload.sessionId === sessionId.current) {
            setConnection('closed')
            term.writeln(`\r\n\x1b[38;2;150;156;163mSession terminée${payload.success ? '' : ' avec une erreur'}.\x1b[0m`)
          }
        }),
        listen<PtyError>('pty-error', ({ payload }) => {
          if (payload.sessionId === sessionId.current) {
            setConnection('error')
            setError(payload.message)
          }
        }),
      ])

      if (disposed) {
        unlisten.forEach((stop) => stop())
        return
      }
      listeners = unlisten
      await startSession()
    }
    void connectDesktop().catch((reason) => {
      if (!disposed) {
        setConnection('error')
        setError(String(reason))
      }
    })

    return () => {
      disposed = true
      resizeObserver.disconnect()
      disposables.forEach((disposable) => disposable.dispose())
      listeners.forEach((unlisten) => unlisten())
      if (trafficTimer.current !== null) window.clearTimeout(trafficTimer.current)
      const id = sessionId.current
      if (id !== null) void invoke('close_session', { sessionId: id }).catch(() => undefined)
      term.dispose()
      sessionId.current = null
      terminal.current = null
      fitAddon.current = null
    }
  }, [startSession])

  useEffect(() => {
    const onShortcut = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === ',') {
        event.preventDefault()
        setDetailsOpen((open) => !open)
      } else if (event.key === 'Escape') {
        setDetailsOpen(false)
      }
    }
    window.addEventListener('keydown', onShortcut)
    return () => window.removeEventListener('keydown', onShortcut)
  }, [])

  useEffect(() => {
    if (detailsOpen) {
      closeButton.current?.focus()
    } else if (connection === 'ready') {
      terminal.current?.focus()
    }
  }, [detailsOpen, connection])

  const restart = async () => {
    if (!desktopHost) return
    const id = sessionId.current
    sessionId.current = null
    if (id !== null) await invoke('close_session', { sessionId: id })
    terminal.current?.clear()
    await startSession()
  }

  return (
    <main className="workspace">
      <section className={`terminal-window ${error ? 'terminal-window--error' : ''}`} aria-label="Terminal Rust">
        <header className="titlebar">
          <div className="brand-lockup">
            <span className={`activity activity--${connection}`} aria-hidden="true" />
            <strong>Terminal Rust</strong>
            <span className="context-label">atelier / session native</span>
          </div>
          <div className="title-actions">
            <span className="connection-label" role="status">
              {connection === 'ready' && 'Connecté'}
              {connection === 'connecting' && 'Connexion…'}
              {connection === 'closed' && 'Session terminée'}
              {connection === 'error' && 'Connexion impossible'}
            </span>
            <button
              type="button"
              className="quiet-button"
              onClick={() => setDetailsOpen((open) => !open)}
              aria-expanded={detailsOpen}
              aria-controls="session-drawer"
            >
              Session <kbd>Ctrl ,</kbd>
            </button>
          </div>
        </header>

        {error && (
          <div className="error-strip" role="alert">
            <span>{error}</span>
            <button type="button" onClick={() => void restart()}>Réessayer</button>
          </div>
        )}

        <div className="terminal-stage" onMouseDown={() => terminal.current?.focus()}>
          <div ref={terminalHost} className="terminal-host" />
          <div
            className={`traffic-rail traffic-rail--${connection} ${trafficActive ? 'traffic-rail--active' : ''}`}
            aria-hidden="true"
          />
        </div>

        <footer className="statusbar">
          <span>moteur Rust · PTY natif</span>
          <span className="status-metrics">{dimensions} · UTF-8 · historique actif</span>
        </footer>

        <aside
          id="session-drawer"
          className={`session-drawer ${detailsOpen ? 'session-drawer--open' : ''}`}
          aria-hidden={!detailsOpen}
          aria-labelledby="session-heading"
        >
          <div className="drawer-heading">
            <div>
              <h2 id="session-heading">Session</h2>
              <p>Le processus et les entrées restent côté Rust.</p>
            </div>
            <button
              ref={closeButton}
              type="button"
              className="close-button"
              onClick={() => setDetailsOpen(false)}
              aria-label="Fermer le panneau"
            >
              Fermer
            </button>
          </div>
          <dl className="session-facts">
            <div><dt>Transport</dt><dd>PTY natif</dd></div>
            <div><dt>Moteur</dt><dd>Terminal Rust 0.3</dd></div>
            <div><dt>Taille</dt><dd>{dimensions}</dd></div>
            <div><dt>État</dt><dd>{connection}</dd></div>
          </dl>
          <div className="shortcut-list">
            <h3>Raccourcis</h3>
            <p><kbd>↑</kbd><kbd>↓</kbd><span>Historique</span></p>
            <p><kbd>Ctrl C</kbd><span>Interrompre</span></p>
            <p><kbd>Ctrl L</kbd><span>Effacer</span></p>
          </div>
          <button
            type="button"
            className="restart-button"
            onClick={() => void restart()}
            disabled={!desktopHost || connection === 'connecting'}
          >
            {desktopHost ? 'Redémarrer la session' : 'Disponible dans la fenêtre Tauri'}
          </button>
        </aside>
      </section>
    </main>
  )
}
