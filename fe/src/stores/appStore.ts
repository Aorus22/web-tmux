// App-level state (PRD §46): active session, sidebar visibility, palette open.
// The sidebar tree (all sessions) is fetched via TanStack Query, not stored here.

import { create } from 'zustand'

interface AppState {
  activeSession: string | null
  sidebarOpen: boolean
  paletteOpen: boolean

  setActiveSession: (name: string | null) => void
  toggleSidebar: () => void
  setSidebarOpen: (open: boolean) => void
  setPaletteOpen: (open: boolean) => void
}

export const useAppStore = create<AppState>()((set) => ({
  activeSession: null,
  sidebarOpen: true,
  paletteOpen: false,

  setActiveSession: (activeSession) => set({ activeSession }),
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
}))
