// UI theme presets (web-term pattern): full app chrome palettes applied as
// CSS variables on <html>. Each preset also names the terminal theme (from
// ./terminal-themes.ts) that matches it, so the terminal follows the app theme
// unless the user picks a terminal theme explicitly.
//
// Curated subset of the reference app's themes: ~9 palettes × dark/light.
// Hex values come from the reference (themes.ts); the first entry is the
// Default dark theme, matching the app's current dark look.

import { themeLuminance } from './terminal-themes'

export interface UiThemePreset {
  name: string // stable key, persisted in settingsStore.uiTheme
  label: string // display name in the settings grid
  // Terminal preset (terminal-themes.ts) that matches this UI theme.
  terminalTheme: string
  colors: {
    background: string
    foreground: string
    card: string
    cardForeground: string
    primary: string
    primaryForeground: string
    secondary: string
    secondaryForeground: string
    muted: string
    mutedForeground: string
    accent: string
    accentForeground: string
    destructive: string
    destructiveForeground: string
    border: string
    input: string
    ring: string
  }
}

export const uiThemes: UiThemePreset[] = [
  {
    name: 'default-dark',
    label: 'Default',
    terminalTheme: 'vscode-dark',
    colors: {
      background: '#1e1e1e',
      foreground: '#d4d4d4',
      card: '#1e1e1e',
      cardForeground: '#d4d4d4',
      primary: '#d4d4d4',
      primaryForeground: '#1e1e1e',
      secondary: '#2d2d2d',
      secondaryForeground: '#d4d4d4',
      muted: '#2d2d2d',
      mutedForeground: '#808080',
      accent: '#2d2d2d',
      accentForeground: '#d4d4d4',
      destructive: '#7f1d1d',
      destructiveForeground: '#ffffff',
      border: '#3c3c3c',
      input: '#3c3c3c',
      ring: '#808080',
    },
  },
  {
    name: 'default-light',
    label: 'Default',
    terminalTheme: 'light',
    colors: {
      background: '#fafafa',
      foreground: '#383a42',
      card: '#ffffff',
      cardForeground: '#383a42',
      primary: '#383a42',
      primaryForeground: '#ffffff',
      secondary: '#f0f0f0',
      secondaryForeground: '#383a42',
      muted: '#f0f0f0',
      mutedForeground: '#a0a1a7',
      accent: '#f0f0f0',
      accentForeground: '#383a42',
      destructive: '#ef4444',
      destructiveForeground: '#ffffff',
      border: '#dbdbdc',
      input: '#dbdbdc',
      ring: '#a0a1a7',
    },
  },
  {
    name: 'ocean-dark',
    label: 'Ocean',
    terminalTheme: 'ocean',
    colors: {
      background: '#0c1929',
      foreground: '#ffffff',
      card: '#162940',
      cardForeground: '#ffffff',
      primary: '#2563eb',
      primaryForeground: '#ffffff',
      secondary: '#162940',
      secondaryForeground: '#ffffff',
      muted: '#1e2b3b',
      mutedForeground: '#94a3b8',
      accent: '#1e3a5f',
      accentForeground: '#ffffff',
      destructive: '#7f1d1d',
      destructiveForeground: '#ffffff',
      border: '#274768',
      input: '#274768',
      ring: '#2563eb',
    },
  },
  {
    name: 'ocean-light',
    label: 'Ocean',
    terminalTheme: 'ocean-light',
    colors: {
      background: '#f0f9ff',
      foreground: '#0f172a',
      card: '#ffffff',
      cardForeground: '#0f172a',
      primary: '#2563eb',
      primaryForeground: '#ffffff',
      secondary: '#eff6ff',
      secondaryForeground: '#0f172a',
      muted: '#e3ecf2',
      mutedForeground: '#64748b',
      accent: '#eff6ff',
      accentForeground: '#0f172a',
      destructive: '#ef4444',
      destructiveForeground: '#ffffff',
      border: '#bfdbfe',
      input: '#bfdbfe',
      ring: '#3b82f6',
    },
  },
  {
    name: 'forest-dark',
    label: 'Forest',
    terminalTheme: 'forest',
    colors: {
      background: '#0c1f17',
      foreground: '#ffffff',
      card: '#173025',
      cardForeground: '#ffffff',
      primary: '#059669',
      primaryForeground: '#ffffff',
      secondary: '#173025',
      secondaryForeground: '#ffffff',
      muted: '#1e3129',
      mutedForeground: '#94a3b8',
      accent: '#064e3b',
      accentForeground: '#ffffff',
      destructive: '#7f1d1d',
      destructiveForeground: '#ffffff',
      border: '#1f4a32',
      input: '#1f4a32',
      ring: '#059669',
    },
  },
  {
    name: 'forest-light',
    label: 'Forest',
    terminalTheme: 'forest-light',
    colors: {
      background: '#f0fdf4',
      foreground: '#0f172a',
      card: '#ffffff',
      cardForeground: '#0f172a',
      primary: '#059669',
      primaryForeground: '#ffffff',
      secondary: '#ecfdf5',
      secondaryForeground: '#0f172a',
      muted: '#e3f0e7',
      mutedForeground: '#64748b',
      accent: '#ecfdf5',
      accentForeground: '#0f172a',
      destructive: '#ef4444',
      destructiveForeground: '#ffffff',
      border: '#a7f3d0',
      input: '#a7f3d0',
      ring: '#10b981',
    },
  },
  {
    name: 'nord-dark',
    label: 'Nord',
    terminalTheme: 'nord',
    colors: {
      background: '#2e3440',
      foreground: '#eceff4',
      card: '#3b4252',
      cardForeground: '#eceff4',
      primary: '#88c0d0',
      primaryForeground: '#2e3440',
      secondary: '#434c5e',
      secondaryForeground: '#eceff4',
      muted: '#404652',
      mutedForeground: '#d8dee9',
      accent: '#5e81ac',
      accentForeground: '#eceff4',
      destructive: '#bf616a',
      destructiveForeground: '#eceff4',
      border: '#4c566a',
      input: '#4c566a',
      ring: '#88c0d0',
    },
  },
  {
    name: 'nord-light',
    label: 'Nord',
    terminalTheme: 'nord',
    colors: {
      background: '#eceff4',
      foreground: '#2e3440',
      card: '#ffffff',
      cardForeground: '#2e3440',
      primary: '#5e81ac',
      primaryForeground: '#ffffff',
      secondary: '#e5e9f0',
      secondaryForeground: '#2e3440',
      muted: '#dfe2e7',
      mutedForeground: '#4c566a',
      accent: '#d8dee9',
      accentForeground: '#2e3440',
      destructive: '#bf616a',
      destructiveForeground: '#ffffff',
      border: '#d8dee9',
      input: '#d8dee9',
      ring: '#5e81ac',
    },
  },
  {
    name: 'dracula-dark',
    label: 'Dracula',
    terminalTheme: 'dracula',
    colors: {
      background: '#282a36',
      foreground: '#f8f8f2',
      card: '#343746',
      cardForeground: '#f8f8f2',
      primary: '#bd93f9',
      primaryForeground: '#282a36',
      secondary: '#343746',
      secondaryForeground: '#f8f8f2',
      muted: '#3a3c48',
      mutedForeground: '#6272a4',
      accent: '#ff79c6',
      accentForeground: '#282a36',
      destructive: '#ff5555',
      destructiveForeground: '#f8f8f2',
      border: '#44475a',
      input: '#44475a',
      ring: '#bd93f9',
    },
  },
  {
    name: 'gruvbox-dark',
    label: 'Gruvbox',
    terminalTheme: 'gruvbox-dark',
    colors: {
      background: '#282828',
      foreground: '#ebdbb2',
      card: '#3c3836',
      cardForeground: '#ebdbb2',
      primary: '#fabd2f',
      primaryForeground: '#282828',
      secondary: '#3c3836',
      secondaryForeground: '#ebdbb2',
      muted: '#3a3a3a',
      mutedForeground: '#a89984',
      accent: '#83a598',
      accentForeground: '#282828',
      destructive: '#fb4934',
      destructiveForeground: '#ebdbb2',
      border: '#504945',
      input: '#504945',
      ring: '#fabd2f',
    },
  },
  {
    name: 'gruvbox-light',
    label: 'Gruvbox',
    terminalTheme: 'gruvbox-light',
    colors: {
      background: '#fbf1c7',
      foreground: '#3c3836',
      card: '#ebdbb2',
      cardForeground: '#3c3836',
      primary: '#d79921',
      primaryForeground: '#fbf1c7',
      secondary: '#ebdbb2',
      secondaryForeground: '#3c3836',
      muted: '#eee4ba',
      mutedForeground: '#7c6f64',
      accent: '#076678',
      accentForeground: '#fbf1c7',
      destructive: '#cc241d',
      destructiveForeground: '#fbf1c7',
      border: '#d5c4a1',
      input: '#d5c4a1',
      ring: '#d79921',
    },
  },
  {
    name: 'tokyo-night-dark',
    label: 'Tokyo Night',
    terminalTheme: 'tokyo-night',
    colors: {
      background: '#1a1b26',
      foreground: '#c0caf5',
      card: '#24283b',
      cardForeground: '#c0caf5',
      primary: '#7aa2f7',
      primaryForeground: '#1a1b26',
      secondary: '#24283b',
      secondaryForeground: '#c0caf5',
      muted: '#2c2d38',
      mutedForeground: '#565f89',
      accent: '#bb9af7',
      accentForeground: '#1a1b26',
      destructive: '#f7768e',
      destructiveForeground: '#1a1b26',
      border: '#292e42',
      input: '#292e42',
      ring: '#7aa2f7',
    },
  },
  {
    name: 'rose-pine-dark',
    label: 'Rosé Pine',
    terminalTheme: 'rose-pine',
    colors: {
      background: '#191724',
      foreground: '#e0def4',
      card: '#26233a',
      cardForeground: '#e0def4',
      primary: '#ebbcba',
      primaryForeground: '#191724',
      secondary: '#26233a',
      secondaryForeground: '#e0def4',
      muted: '#2b2936',
      mutedForeground: '#908caa',
      accent: '#31748f',
      accentForeground: '#191724',
      destructive: '#eb6f92',
      destructiveForeground: '#e0def4',
      border: '#312e42',
      input: '#312e42',
      ring: '#ebbcba',
    },
  },
  {
    name: 'one-dark',
    label: 'One Dark',
    terminalTheme: 'one-dark',
    colors: {
      background: '#282c34',
      foreground: '#abb2bf',
      card: '#353a44',
      cardForeground: '#abb2bf',
      primary: '#61afef',
      primaryForeground: '#282c34',
      secondary: '#353a44',
      secondaryForeground: '#abb2bf',
      muted: '#3a3e46',
      mutedForeground: '#5c6370',
      accent: '#e5c07b',
      accentForeground: '#282c34',
      destructive: '#e06c75',
      destructiveForeground: '#282c34',
      border: '#3e4451',
      input: '#3e4451',
      ring: '#61afef',
    },
  },
  {
    name: 'one-light',
    label: 'One Light',
    terminalTheme: 'light',
    colors: {
      background: '#fafafa',
      foreground: '#383a42',
      card: '#ffffff',
      cardForeground: '#383a42',
      primary: '#4078f2',
      primaryForeground: '#ffffff',
      secondary: '#f0f0f0',
      secondaryForeground: '#383a42',
      muted: '#ededed',
      mutedForeground: '#a0a1a7',
      accent: '#e45649',
      accentForeground: '#ffffff',
      destructive: '#ca1243',
      destructiveForeground: '#ffffff',
      border: '#dbdbdc',
      input: '#dbdbdc',
      ring: '#4078f2',
    },
  },
  {
    name: 'solarized-light',
    label: 'Solarized',
    terminalTheme: 'solarized-light',
    colors: {
      background: '#fdf6e3',
      foreground: '#657b83',
      card: '#eee8d5',
      cardForeground: '#657b83',
      primary: '#268bd2',
      primaryForeground: '#fdf6e3',
      secondary: '#eee8d5',
      secondaryForeground: '#657b83',
      muted: '#f0e9d6',
      mutedForeground: '#93a1a1',
      accent: '#2aa198',
      accentForeground: '#fdf6e3',
      destructive: '#dc322f',
      destructiveForeground: '#fdf6e3',
      border: '#d3cbb4',
      input: '#d3cbb4',
      ring: '#268bd2',
    },
  },
  {
    name: 'everforest-dark',
    label: 'Everforest',
    terminalTheme: 'everforest',
    colors: {
      background: '#2b3339',
      foreground: '#d3c6aa',
      card: '#343f44',
      cardForeground: '#d3c6aa',
      primary: '#a7c080',
      primaryForeground: '#2b3339',
      secondary: '#343f44',
      secondaryForeground: '#d3c6aa',
      muted: '#3d454b',
      mutedForeground: '#859289',
      accent: '#84a598',
      accentForeground: '#2b3339',
      destructive: '#e67e80',
      destructiveForeground: '#d3c6aa',
      border: '#3d484d',
      input: '#3d484d',
      ring: '#a7c080',
    },
  },
  {
    name: 'ayu-light',
    label: 'Ayu Light',
    terminalTheme: 'ayu-light',
    colors: {
      background: '#fafafa',
      foreground: '#575f66',
      card: '#ffffff',
      cardForeground: '#575f66',
      primary: '#ff6a00',
      primaryForeground: '#ffffff',
      secondary: '#f0f0f0',
      secondaryForeground: '#575f66',
      muted: '#ededed',
      mutedForeground: '#8a9199',
      accent: '#55b4d4',
      accentForeground: '#ffffff',
      destructive: '#f27983',
      destructiveForeground: '#ffffff',
      border: '#d9d8d7',
      input: '#d9d8d7',
      ring: '#ff6a00',
    },
  },
]

// Look up a UI theme by its persisted name. Falls back to the default dark
// theme for unknown/missing names (old persisted values).
export function getUiTheme(name: string | null | undefined): UiThemePreset {
  return uiThemes.find((t) => t.name === name) ?? uiThemes[0]
}

export function isLightUiTheme(preset: UiThemePreset): boolean {
  return themeLuminance(preset.colors.background) > 0.5
}

// Terminal preset name mapped from the UI theme (the terminal follows the app
// theme when no explicit terminal theme is chosen). null when unknown.
export function getMappedTerminalTheme(
  uiThemeName: string | null | undefined,
): string | null {
  return getUiTheme(uiThemeName).terminalTheme ?? null
}

// Resolve the terminal theme to use: an explicit terminalTheme wins; otherwise
// fall back to the UI theme's mapped terminal preset. null = CSS variables.
export function resolvedTerminalTheme(
  uiThemeName: string | null | undefined,
  terminalTheme: string | null | undefined,
): string | null {
  if (terminalTheme) return terminalTheme
  return getMappedTerminalTheme(uiThemeName)
}
