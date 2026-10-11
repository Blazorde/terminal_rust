import type { ITheme, Terminal } from '@xterm/xterm'

export type DesktopTheme = 'graphite' | 'midnight' | 'paper'
export type FontFamily = 'system' | 'cascadia' | 'jetbrains'
export type CursorStyle = 'block' | 'bar' | 'underline'
export type WindowDensity = 'compact' | 'comfortable'

export interface DesktopSettings {
  version: 1
  theme: DesktopTheme
  fontFamily: FontFamily
  fontSize: number
  cursorStyle: CursorStyle
  cursorBlink: boolean
  windowDensity: WindowDensity
  showStatusBar: boolean
}

export interface SettingsLoad {
  settings: DesktopSettings
  diagnostic: string | null
  path: string
}

export const defaultSettings: DesktopSettings = {
  version: 1,
  theme: 'graphite',
  fontFamily: 'system',
  fontSize: 14,
  cursorStyle: 'block',
  cursorBlink: true,
  windowDensity: 'comfortable',
  showStatusBar: true,
}

export const fontStacks: Record<FontFamily, string> = {
  system: "'Cascadia Mono', 'SFMono-Regular', 'DejaVu Sans Mono', monospace",
  cascadia: "'Cascadia Mono', 'Cascadia Code', 'SFMono-Regular', 'DejaVu Sans Mono', monospace",
  jetbrains: "'JetBrains Mono', 'Cascadia Mono', 'SFMono-Regular', 'DejaVu Sans Mono', monospace",
}

const terminalThemes: Record<DesktopTheme, ITheme> = {
  graphite: {
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
  midnight: {
    background: '#0d1320',
    foreground: '#dce7f5',
    cursor: '#67d7c4',
    cursorAccent: '#0d1320',
    selectionBackground: '#67d7c455',
    black: '#0d1320',
    red: '#ff8290',
    green: '#8ad6a3',
    yellow: '#e4c878',
    blue: '#7db7ff',
    magenta: '#bc9cff',
    cyan: '#67d7c4',
    white: '#dce7f5',
    brightBlack: '#627089',
    brightWhite: '#f6fbff',
  },
  paper: {
    background: '#f2eee5',
    foreground: '#25231f',
    cursor: '#b94725',
    cursorAccent: '#f2eee5',
    selectionBackground: '#b9472540',
    black: '#25231f',
    red: '#b13d45',
    green: '#3d7650',
    yellow: '#8b6918',
    blue: '#365f91',
    magenta: '#765087',
    cyan: '#2e7370',
    white: '#e6dfd2',
    brightBlack: '#756f65',
    brightWhite: '#fffdf8',
  },
}

export function applyTerminalSettings(terminal: Terminal, settings: DesktopSettings) {
  terminal.options.theme = terminalThemes[settings.theme]
  terminal.options.fontFamily = fontStacks[settings.fontFamily]
  terminal.options.fontSize = settings.fontSize
  terminal.options.cursorStyle = settings.cursorStyle
  terminal.options.cursorBlink = settings.cursorBlink
}
