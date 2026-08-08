// Live tmux state (PRD §46: Zustand). Holds the snapshot for the active
// session, command-result correlation, and transport state. Terminal output
// bypasses React entirely — the terminal registry writes directly to xterm.

import { create } from 'zustand'
import type { TmuxSnapshot } from '@/lib/tmux-types'

export type TransportState = 'connecting' | 'connected' | 'reconnecting' | 'disconnected'

interface CommandCallbacks {
  ok: (message?: string) => void
  error: (message: string) => void
}

interface TmuxState {
  snapshot: TmuxSnapshot | null
  transport: TransportState
  tmuxVersion: string
  reconnecting: boolean

  // pending command correlations (requestId → callbacks)
  pending: Record<string, CommandCallbacks>

  setSnapshot: (snap: TmuxSnapshot) => void
  setTransport: (s: TransportState) => void
  setReconnecting: (v: boolean) => void
  setTmuxVersion: (v: string) => void

  trackCommand: (requestId: string, cbs: CommandCallbacks) => void
  resolveCommand: (requestId: string, ok: boolean, message?: string) => void
  forgetCommand: (requestId: string) => void

  clear: () => void
}

export const useTmuxStore = create<TmuxState>()((set, get) => ({
  snapshot: null,
  transport: 'disconnected',
  tmuxVersion: '',
  reconnecting: false,
  pending: {},

  setSnapshot: (snap) => set({ snapshot: snap }),

  setTransport: (transport) => {
    set({ transport, reconnecting: transport === 'reconnecting' })
  },

  setReconnecting: (reconnecting) => set({ reconnecting }),

  setTmuxVersion: (tmuxVersion) => set({ tmuxVersion }),

  trackCommand: (requestId, cbs) =>
    set((s) => ({ pending: { ...s.pending, [requestId]: cbs } })),

  resolveCommand: (requestId, ok, message) => {
    const cb = get().pending[requestId]
    if (!cb) return
    set((s) => {
      const next = { ...s.pending }
      delete next[requestId]
      return { pending: next }
    })
    if (ok) cb.ok(message)
    else cb.error(message ?? 'tmux command failed')
  },

  // forgetCommand drops a pending correlation without resolving it — used by
  // the runCommand timeout so an abandoned command can never leave UI state
  // stuck, and a late response is silently ignored.
  forgetCommand: (requestId) => {
    if (!get().pending[requestId]) return
    set((s) => {
      const next = { ...s.pending }
      delete next[requestId]
      return { pending: next }
    })
  },

  clear: () =>
    set({
      snapshot: null,
      transport: 'disconnected',
      reconnecting: false,
      pending: {},
    }),
}))

// Selector helpers used by components.
// IMPORTANT: these must return stable references when there is no data —
// Zustand v5 compares selector results with Object.is; a fresh []/filtered
// array every render causes "Maximum update depth exceeded" (#185).

import type { TmuxPane, TmuxWindow } from '@/lib/tmux-types'

const NO_WINDOWS: TmuxWindow[] = []
const NO_PANES: TmuxPane[] = []

export const selectSessionName = (s: TmuxState): string | null =>
  s.snapshot?.session.name ?? null

export const selectWindows = (s: TmuxState): TmuxWindow[] =>
  s.snapshot?.windows ?? NO_WINDOWS

export const selectActiveWindowId = (s: TmuxState): string | null =>
  s.snapshot?.activeWindow ?? null

export const selectActivePaneId = (s: TmuxState): string | null =>
  s.snapshot?.activePane ?? null

// Returns the panes of a window. Callers must wrap in useShallow, or pass a
// stable selector key, because the filtered array is a new reference.
export const selectPanesForWindow = (s: TmuxState, windowId: string): TmuxPane[] => {
  const panes = s.snapshot?.panes ?? NO_PANES
  if (!s.snapshot) return NO_PANES
  return panes.filter((p) => p.windowId === windowId)
}
