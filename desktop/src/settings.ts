import type { ITheme, Terminal } from '@xterm/xterm'

export type DesktopTheme = 'graphite' | 'bone' | 'nord' | 'amber' | 'moss' | 'dusk'
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
  bone: {
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
  nord: {
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
  amber: {
    background: '#1b1510',
    foreground: '#f2dfbd',
    cursor: '#f0ad4e',
    cursorAccent: '#1b1510',
    selectionBackground: '#f0ad4e50',
    black: '#1b1510', red: '#ed7b72', green: '#a9c47f', yellow: '#f0c36a',
    blue: '#8eb3cf', magenta: '#c89ac7', cyan: '#7fc7b7', white: '#f2dfbd',
    brightBlack: '#756553', brightWhite: '#fff4df',
  },
  moss: {
    background: '#101713',
    foreground: '#dce7d8',
    cursor: '#a8cc75',
    cursorAccent: '#101713',
    selectionBackground: '#a8cc7550',
    black: '#101713', red: '#e1857c', green: '#a8cc75', yellow: '#d8bd70',
    blue: '#86b4b4', magenta: '#b59ac8', cyan: '#79c4aa', white: '#dce7d8',
    brightBlack: '#627064', brightWhite: '#f2f8ef',
  },
  dusk: {
    background: '#17131d',
    foreground: '#e9e0ef',
    cursor: '#d69adf',
    cursorAccent: '#17131d',
    selectionBackground: '#d69adf50',
    black: '#17131d', red: '#ef8794', green: '#9bc58f', yellow: '#dfbd72',
    blue: '#91acf0', magenta: '#d69adf', cyan: '#7bc5c2', white: '#e9e0ef',
    brightBlack: '#71667a', brightWhite: '#fff7ff',
  },
}

export function applyTerminalSettings(terminal: Terminal, settings: DesktopSettings, reduceMotion = false) {
  terminal.options.theme = terminalThemes[settings.theme]
  terminal.options.fontFamily = fontStacks[settings.fontFamily]
  terminal.options.fontSize = settings.fontSize
  terminal.options.cursorStyle = settings.cursorStyle
  terminal.options.cursorBlink = settings.cursorBlink && !reduceMotion
}
