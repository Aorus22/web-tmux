// Frontend UI settings persisted in localStorage (PRD §42). Backend config
// stays environment/CLI — this store is UI-only.

import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export interface Settings {
  theme: 'dark' | 'light'
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
  theme: 'dark',
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
