import { useRef, type ChangeEvent, type KeyboardEvent, type RefObject } from 'react'

import type { ConnectionState } from './terminal-types'
import type { DesktopSettings } from './settings'

interface SettingsDrawerProps {
  closeButton: RefObject<HTMLButtonElement | null>
  connection: ConnectionState
  detailsOpen: boolean
  desktopHost: boolean
  diagnostic: string | null
  dimensions: string
  onChange: (settings: DesktopSettings) => void
  onClose: () => void
  onRestart: () => void
  onSave: () => void
  saveState: 'idle' | 'saving' | 'saved' | 'error'
  settings: DesktopSettings
  settingsPath: string | null
}

export default function SettingsDrawer({
  closeButton,
  connection,
  detailsOpen,
  desktopHost,
  diagnostic,
  dimensions,
  onChange,
  onClose,
  onRestart,
  onSave,
  saveState,
  settings,
  settingsPath,
}: SettingsDrawerProps) {
  const drawer = useRef<HTMLElement>(null)
  const update = <Key extends keyof DesktopSettings>(key: Key, value: DesktopSettings[Key]) => {
    onChange({ ...settings, [key]: value })
  }
  const updateNumber = (key: 'fontSize') => (event: ChangeEvent<HTMLInputElement>) => {
    update(key, Number(event.target.value))
  }
  const keepFocusInside = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== 'Tab' || !drawer.current) return
    const controls = Array.from(drawer.current.querySelectorAll<HTMLElement>(
      'button:not(:disabled), input:not(:disabled), select:not(:disabled), [tabindex]:not([tabindex="-1"])',
    )).filter((control) => control.offsetParent !== null)
    const first = controls[0]
    const last = controls.at(-1)
    if (!first || !last) return
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault()
      last.focus()
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault()
      first.focus()
    }
  }

  return (
    <aside
      ref={drawer}
      id="session-drawer"
      className={`session-drawer ${detailsOpen ? 'session-drawer--open' : ''}`}
      role="dialog"
      aria-modal={detailsOpen ? true : undefined}
      aria-hidden={!detailsOpen}
      aria-labelledby="settings-heading"
      inert={!detailsOpen}
      onKeyDown={keepFocusInside}
    >
      <div className="drawer-heading">
        <div>
          <h2 id="settings-heading">Réglages</h2>
          <p>Le rendu change immédiatement. L’enregistrement reste validé côté Rust.</p>
        </div>
        <button ref={closeButton} type="button" className="close-button" onClick={onClose} aria-label="Fermer le panneau">
          Fermer
        </button>
      </div>

      {diagnostic && <p className="settings-diagnostic" role="alert">{diagnostic}</p>}

      <fieldset className="settings-group">
        <legend>Apparence</legend>
        <label className="field-row">
          <span><strong>Thème</strong><small>Palette du terminal et de la fenêtre</small></span>
          <select value={settings.theme} onChange={(event) => update('theme', event.target.value as DesktopSettings['theme'])}>
            <option value="graphite">Graphite</option>
            <option value="midnight">Minuit</option>
            <option value="paper">Papier</option>
          </select>
        </label>
        <label className="field-row">
          <span><strong>Police</strong><small>Toujours avec replis système locaux</small></span>
          <select value={settings.fontFamily} onChange={(event) => update('fontFamily', event.target.value as DesktopSettings['fontFamily'])}>
            <option value="system">Système</option>
            <option value="cascadia">Cascadia</option>
            <option value="jetbrains">JetBrains</option>
          </select>
        </label>
        <label className="range-row">
          <span><strong>Taille du texte</strong><output>{settings.fontSize}px</output></span>
          <input type="range" min="12" max="22" step="1" value={settings.fontSize} onChange={updateNumber('fontSize')} />
        </label>
      </fieldset>

      <fieldset className="settings-group">
        <legend>Curseur</legend>
        <div className="choice-grid" role="group" aria-label="Forme du curseur">
          {(['block', 'bar', 'underline'] as const).map((style) => (
            <button
              key={style}
              type="button"
              className={settings.cursorStyle === style ? 'choice-button choice-button--active' : 'choice-button'}
              aria-pressed={settings.cursorStyle === style}
              onClick={() => update('cursorStyle', style)}
            >
              {style === 'block' ? 'Bloc' : style === 'bar' ? 'Barre' : 'Souligné'}
            </button>
          ))}
        </div>
        <label className="toggle-row">
          <span><strong>Clignotement</strong><small>Respecte aussi la réduction des animations</small></span>
          <input type="checkbox" checked={settings.cursorBlink} onChange={(event) => update('cursorBlink', event.target.checked)} />
        </label>
      </fieldset>

      <fieldset className="settings-group">
        <legend>Fenêtre</legend>
        <label className="field-row">
          <span><strong>Densité</strong><small>Marge autour de la surface terminal</small></span>
          <select value={settings.windowDensity} onChange={(event) => update('windowDensity', event.target.value as DesktopSettings['windowDensity'])}>
            <option value="comfortable">Confortable</option>
            <option value="compact">Compacte</option>
          </select>
        </label>
        <label className="toggle-row">
          <span><strong>Barre d’état</strong><small>Taille, encodage et historique</small></span>
          <input type="checkbox" checked={settings.showStatusBar} onChange={(event) => update('showStatusBar', event.target.checked)} />
        </label>
      </fieldset>

      <dl className="session-facts">
        <div><dt>Transport</dt><dd>PTY natif</dd></div>
        <div><dt>Taille</dt><dd>{dimensions}</dd></div>
        <div><dt>État</dt><dd>{connection}</dd></div>
      </dl>

      <div className="drawer-actions">
        <button type="button" className="secondary-button" onClick={onRestart} disabled={!desktopHost || connection === 'connecting'}>
          Redémarrer
        </button>
        <button type="button" className="save-button" onClick={onSave} disabled={!desktopHost || saveState === 'saving'}>
          {saveState === 'saving' ? 'Enregistrement…' : saveState === 'saved' ? 'Enregistré' : 'Enregistrer'}
        </button>
      </div>
      {saveState === 'error' && <p className="save-error" role="alert">Impossible d’enregistrer ces réglages.</p>}
      {settingsPath && <p className="settings-path" title={settingsPath}>Fichier local · {settingsPath}</p>}
    </aside>
  )
}
