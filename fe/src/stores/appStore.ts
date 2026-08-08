// App-level state (PRD §46): open session tabs, active tab, sidebar and
// palette visibility. The sidebar tree (all sessions) is fetched via TanStack
// Query, not stored here.

import { create } from 'zustand'

export type SidebarPage = 'sessions' | 'settings'

interface AppState {
  // Open session tabs (ordered). Each open session keeps its own WebSocket
  // connection and live snapshot; the active one is displayed.
  activeSession: string | null
  openSessions: string[]

  // Sidebar page: 'settings' shows the dedicated Settings page in the main
  // area (while tabs stay open underneath).
  sidebarPage: SidebarPage
  sidebarOpen: boolean
  paletteOpen: boolean

  // openSession opens a session as a tab (if not already open) and activates
  // it. Clicking a session in the sidebar / picking one from the palette.
  openSession: (name: string) => void
  // closeSession closes a tab (disconnects its socket; the tmux session
  // itself persists). If the closed tab was active, the neighbor tab
  // becomes active.
  closeSession: (name: string) => void
  // setActiveSession switches the active tab view (null = no tab focused,
  // e.g. while viewing a sidebar page).
  setActiveSession: (name: string | null) => void
  setSidebarPage: (page: SidebarPage) => void

  toggleSidebar: () => void
  setSidebarOpen: (open: boolean) => void
  setPaletteOpen: (open: boolean) => void
}

export const useAppStore = create<AppState>()((set) => ({
  activeSession: null,
  openSessions: [],
  sidebarPage: 'sessions',
  sidebarOpen: true,
  paletteOpen: false,

  openSession: (name) =>
    set((s) => ({
      openSessions: s.openSessions.includes(name)
        ? s.openSessions
        : [...s.openSessions, name],
      activeSession: name,
      sidebarPage: 'sessions',
    })),

  closeSession: (name) =>
    set((s) => {
      const idx = s.openSessions.indexOf(name)
      if (idx === -1) return s
      const openSessions = s.openSessions.filter((n) => n !== name)
      // If the active tab was closed, activate a neighbor.
      let activeSession = s.activeSession
      if (activeSession === name) {
        const neighbor = openSessions[Math.min(idx, openSessions.length - 1)] ?? null
        activeSession = neighbor
      }
      return { openSessions, activeSession }
    }),

  setActiveSession: (activeSession) =>
    set((s) => ({
      activeSession,
      // Only tabs that are actually open can be displayed.
      sidebarPage: activeSession ? 'sessions' : s.sidebarPage,
    })),

  setSidebarPage: (sidebarPage) => set({ sidebarPage }),

  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),

  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),

  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
}))
