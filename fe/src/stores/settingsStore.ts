// Frontend UI settings persisted in localStorage (PRD §42). Backend config
// stays environment/CLI — this store is UI-only.

import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export interface Settings {
  // Selected UI theme preset name (see features/settings/data/ui-themes.ts).
  // Unknown/missing values fall back to the default dark theme.
  uiTheme: string
  // Legacy field, kept for persisted-value compatibility. The terminal now
  // always follows the UI theme; this value is no longer read.
  terminalTheme: string | null
  fontFamily: string
  fontSize: number
  lineHeight: number
  scrollbackLines: number
  // Per-tmux-pane wheel behavior: true sends PageUp/PageDown to TUIs.
  tuiScrollPanes: Record<string, boolean>
  confirmKillPane: boolean
  confirmKillWindow: boolean
  confirmKillSession: boolean
}

interface SettingsState extends Settings {
  set: (patch: Partial<Settings>) => void
  setTuiScrollPane: (paneId: string, enabled: boolean) => void
  reset: () => void
}

const DEFAULTS: Settings = {
  uiTheme: 'default-dark',
  terminalTheme: null,
  fontFamily: 'JetBrains Mono, Menlo, Consolas, monospace',
  fontSize: 14,
  lineHeight: 1.35,
  scrollbackLines: 2000,
  tuiScrollPanes: {},
  confirmKillPane: true,
  confirmKillWindow: true,
  confirmKillSession: true,
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      ...DEFAULTS,
      set: (patch) => set(patch),
      setTuiScrollPane: (paneId, enabled) =>
        set((state) => ({
          tuiScrollPanes: { ...state.tuiScrollPanes, [paneId]: enabled },
        })),
      reset: () => set(DEFAULTS),
    }),
    {
      name: 'tmux-gui-settings',
    },
  ),
)
