import { invoke } from '@tauri-apps/api/core'
import { useCallback, useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from 'react'

import PromptSegments, { emptyPromptLayout, type PromptLayout } from './PromptSegments'
import SettingsDrawer from './SettingsDrawer'
import { defaultSettings, type DesktopSettings, type SettingsLoad } from './settings'
import TerminalPane, { type TerminalTabPatch } from './TerminalPane'
import type { ConnectionState } from './terminal-types'

interface TerminalTab {
  connection: ConnectionState
  dimensions: string
  error: string | null
  id: string
  label: string
  ordinal: number
  promptLayout: PromptLayout
  restartToken: number
  trafficActive: boolean
}

function createTab(ordinal: number): TerminalTab {
  return {
    connection: 'connecting',
    dimensions: '80 × 24',
    error: null,
    id: `session-${ordinal}`,
    label: `Terminal ${ordinal}`,
    ordinal,
    promptLayout: emptyPromptLayout,
    restartToken: 0,
    trafficActive: false,
  }
}

function labelFromLayout(tab: TerminalTab, layout: PromptLayout): string {
  const path = [...layout.left, ...layout.right].find((segment) => segment.kind === 'path')?.value
  const folder = path?.split(/[\\/]/).filter(Boolean).at(-1)
  return folder ? `${folder} · ${tab.ordinal}` : `Terminal ${tab.ordinal}`
}

export default function App() {
  const closeButton = useRef<HTMLButtonElement>(null)
  const nextOrdinal = useRef(2)
  const [desktopHost, setDesktopHost] = useState(false)
  const [detailsOpen, setDetailsOpen] = useState(false)
  const [settings, setSettings] = useState<DesktopSettings>(defaultSettings)
  const [settingsDiagnostic, setSettingsDiagnostic] = useState<string | null>(null)
  const [settingsPath, setSettingsPath] = useState<string | null>(null)
  const [saveState, setSaveState] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle')
  const [tabs, setTabs] = useState<TerminalTab[]>(() => [createTab(1)])
  const [activeTabId, setActiveTabId] = useState('session-1')

  const activeTab = tabs.find((tab) => tab.id === activeTabId) ?? tabs[0]

  const focusTabButton = useCallback((tabId: string) => {
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => document.getElementById(`tab-${tabId}`)?.focus())
    })
  }, [])

  const patchTab = useCallback((tabId: string, patch: TerminalTabPatch) => {
    setTabs((current) => current.map((tab) => {
      if (tab.id !== tabId) return tab
      const next = { ...tab, ...patch }
      if (patch.promptLayout) next.label = labelFromLayout(tab, patch.promptLayout)
      return next
    }))
  }, [])

  const addTab = useCallback(() => {
    const ordinal = nextOrdinal.current++
    const tab = createTab(ordinal)
    setTabs((current) => [...current, tab])
    setActiveTabId(tab.id)
    setDetailsOpen(false)
  }, [])

  const closeTab = useCallback((tabId: string) => {
    const index = tabs.findIndex((tab) => tab.id === tabId)
    if (index < 0) return
    if (tabs.length === 1) {
      const replacement = createTab(nextOrdinal.current++)
      setTabs([replacement])
      setActiveTabId(replacement.id)
      return
    }
    const next = tabs.filter((tab) => tab.id !== tabId)
    setTabs(next)
    if (activeTabId === tabId) setActiveTabId(next[Math.min(index, next.length - 1)].id)
  }, [activeTabId, tabs])

  const cycleTab = useCallback((direction: 1 | -1) => {
    setActiveTabId((active) => {
      const index = tabs.findIndex((tab) => tab.id === active)
      const next = (index + direction + tabs.length) % tabs.length
      return tabs[next]?.id ?? active
    })
    setDetailsOpen(false)
  }, [tabs])

  const navigateTabList = (event: ReactKeyboardEvent<HTMLButtonElement>, tabId: string) => {
    const index = tabs.findIndex((tab) => tab.id === tabId)
    if (index < 0) return
    let target: TerminalTab | undefined
    if (event.key === 'ArrowRight') target = tabs[(index + 1) % tabs.length]
    if (event.key === 'ArrowLeft') target = tabs[(index - 1 + tabs.length) % tabs.length]
    if (event.key === 'Home') target = tabs[0]
    if (event.key === 'End') target = tabs.at(-1)
    if (event.key === 'Delete') {
      event.preventDefault()
      const remaining = tabs.filter((tab) => tab.id !== tabId)
      const nextId = remaining[Math.min(index, remaining.length - 1)]?.id
      closeTab(tabId)
      if (nextId) focusTabButton(nextId)
      return
    }
    if (!target) return
    event.preventDefault()
    setActiveTabId(target.id)
    setDetailsOpen(false)
    focusTabButton(target.id)
  }

  useEffect(() => {
    let disposed = false
    const connectDesktop = async () => {
      try {
        await invoke<boolean>('is_terminal_rust_desktop')
      } catch {
        return
      }
      if (disposed) return
      const loaded = await invoke<SettingsLoad>('load_desktop_settings')
      if (disposed) return
      setDesktopHost(true)
      setSettings(loaded.settings)
      setSettingsDiagnostic(loaded.diagnostic)
      setSettingsPath(loaded.path)
    }
    void connectDesktop().catch((reason) => setSettingsDiagnostic(String(reason)))
    return () => { disposed = true }
  }, [])

  useEffect(() => {
    const onShortcut = (event: KeyboardEvent) => {
      const modifier = event.ctrlKey || event.metaKey
      if (modifier && event.key === ',') {
        event.preventDefault()
        setDetailsOpen((open) => !open)
      } else if (modifier && event.key.toLowerCase() === 't') {
        event.preventDefault()
        addTab()
      } else if (modifier && event.key.toLowerCase() === 'w') {
        event.preventDefault()
        closeTab(activeTabId)
      } else if (event.ctrlKey && event.key === 'Tab') {
        event.preventDefault()
        cycleTab(event.shiftKey ? -1 : 1)
      } else if (event.key === 'Escape') {
        setDetailsOpen(false)
      }
    }
    window.addEventListener('keydown', onShortcut, true)
    return () => window.removeEventListener('keydown', onShortcut, true)
  }, [activeTabId, addTab, closeTab, cycleTab])

  useEffect(() => {
    if (detailsOpen) closeButton.current?.focus()
  }, [detailsOpen])

  const restart = () => {
    if (!activeTab) return
    setTabs((current) => current.map((tab) => tab.id === activeTab.id
      ? { ...tab, restartToken: tab.restartToken + 1, error: null, promptLayout: emptyPromptLayout }
      : tab))
  }

  const updateSettings = (nextSettings: DesktopSettings) => {
    setSettings(nextSettings)
    setSaveState('idle')
  }

  const saveSettings = async () => {
    if (!desktopHost) return
    setSaveState('saving')
    try {
      await invoke('save_desktop_settings', { settings })
      setSettingsDiagnostic(null)
      setSaveState('saved')
    } catch (reason) {
      setSettingsDiagnostic(String(reason))
      setSaveState('error')
    }
  }

  return (
    <main className="workspace" data-theme={settings.theme} data-density={settings.windowDensity}>
      <section
        className={`terminal-window ${activeTab?.error ? 'terminal-window--error' : ''} ${settings.showStatusBar ? '' : 'terminal-window--status-hidden'}`}
        aria-label="Terminal Rust"
      >
        <header className="titlebar">
          <div className="brand-lockup">
            <span className={`activity activity--${activeTab?.connection ?? 'connecting'}`} aria-hidden="true" />
            <strong>Terminal Rust</strong>
          </div>

          <div className="tab-list" role="tablist" aria-label="Sessions terminal">
            {tabs.map((tab) => (
              <div className={`tab-item ${tab.id === activeTabId ? 'tab-item--active' : ''}`} key={tab.id}>
                <button
                  id={`tab-${tab.id}`}
                  type="button"
                  role="tab"
                  aria-selected={tab.id === activeTabId}
                  aria-controls={`panel-${tab.id}`}
                  tabIndex={tab.id === activeTabId ? 0 : -1}
                  onClick={() => { setActiveTabId(tab.id); setDetailsOpen(false) }}
                  onKeyDown={(event) => navigateTabList(event, tab.id)}
                >
                  <span className={`tab-state tab-state--${tab.connection}`} aria-hidden="true" />
                  <span>{tab.label}</span>
                </button>
                <button type="button" className="tab-close" onClick={() => closeTab(tab.id)} aria-label={`Fermer ${tab.label}`}>×</button>
              </div>
            ))}
            <button type="button" className="tab-add" onClick={addTab} aria-label="Nouvel onglet" title="Nouvel onglet (Ctrl/⌘+T)">+</button>
          </div>

          <div className="title-actions">
            <span className="connection-label" role="status">
              {activeTab?.connection === 'ready' && 'Connecté'}
              {activeTab?.connection === 'connecting' && 'Connexion…'}
              {activeTab?.connection === 'closed' && 'Session terminée'}
              {activeTab?.connection === 'error' && 'Connexion impossible'}
            </span>
            <button
              type="button"
              className="quiet-button"
              onClick={() => setDetailsOpen((open) => !open)}
              aria-expanded={detailsOpen}
              aria-controls="session-drawer"
            >
              Réglages <kbd>Ctrl/⌘ ,</kbd>
            </button>
          </div>
        </header>

        {activeTab?.error && (
          <div className="error-strip" role="alert">
            <span>{activeTab.error}</span>
            <button type="button" onClick={restart}>Réessayer</button>
          </div>
        )}

        <div className="terminal-stage">
          {tabs.map((tab) => (
            <TerminalPane
              key={tab.id}
              active={tab.id === activeTabId}
              desktopHost={desktopHost}
              focusEnabled={!detailsOpen}
              onPatch={patchTab}
              restartToken={tab.restartToken}
              settings={settings}
              tabId={tab.id}
            />
          ))}
          <div
            className={`traffic-rail traffic-rail--${activeTab?.connection ?? 'connecting'} ${activeTab?.trafficActive ? 'traffic-rail--active' : ''}`}
            aria-hidden="true"
          />
        </div>

        {settings.showStatusBar && (
          <PromptSegments layout={activeTab?.promptLayout ?? emptyPromptLayout} dimensions={activeTab?.dimensions ?? '80 × 24'} />
        )}

        <SettingsDrawer
          closeButton={closeButton}
          connection={activeTab?.connection ?? 'connecting'}
          detailsOpen={detailsOpen}
          desktopHost={desktopHost}
          diagnostic={settingsDiagnostic}
          dimensions={activeTab?.dimensions ?? '80 × 24'}
          onChange={updateSettings}
          onClose={() => setDetailsOpen(false)}
          onRestart={restart}
          onSave={() => void saveSettings()}
          saveState={saveState}
          settings={settings}
          settingsPath={settingsPath}
        />
      </section>
    </main>
  )
}
