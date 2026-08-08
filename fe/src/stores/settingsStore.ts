// Frontend UI settings persisted in localStorage (PRD §42). Backend config
// stays environment/CLI — this store is UI-only.

import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export interface Settings {
  // Selected UI theme preset name (see features/settings/data/ui-themes.ts).
  // Unknown/missing values fall back to the default dark theme.
  uiTheme: string
  // Selected terminal theme preset name (see features/settings/data/
  // terminal-themes.ts). null = follow the app UI theme's terminal colors.
  terminalTheme: string | null
  fontFamily: string
  fontSize: number
  lineHeight: number
  scrollbackLines: number
  confirmKillPane: boolean
  confirmKillWindow: boolean
  confirmKillSession: boolean
}

interface SettingsState extends Settings {
  set: (patch: Partial<Settings>) => void
  reset: () => void
}

const DEFAULTS: Settings = {
  uiTheme: 'default-dark',
  terminalTheme: null,
  fontFamily: 'JetBrains Mono, Menlo, Consolas, monospace',
  fontSize: 14,
  lineHeight: 1.35,
  scrollbackLines: 2000,
  confirmKillPane: true,
  confirmKillWindow: true,
  confirmKillSession: true,
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      ...DEFAULTS,
      set: (patch) => set(patch),
      reset: () => set(DEFAULTS),
    }),
    {
      name: 'tmux-gui-settings',
    },
  ),
)
