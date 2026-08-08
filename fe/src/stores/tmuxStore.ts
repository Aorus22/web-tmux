// Live tmux state (PRD §46: Zustand). Holds per-session snapshots and
// transport state for every open session tab, plus command-result
// correlation. Terminal output bypasses React entirely — the terminal
// registry writes directly to xterm.
//
// `viewSession` mirrors appStore.activeSession: `snapshot`/`transport` below
// are kept in sync with it so existing components keep reading the same
// single-session shape while multiple sessions stay connected underneath.

import { create } from 'zustand'
import type { TmuxSnapshot } from '@/lib/tmux-types'

export type TransportState = 'connecting' | 'connected' | 'reconnecting' | 'disconnected'

interface CommandCallbacks {
  ok: (message?: string) => void
  error: (message: string) => void
}

interface TmuxState {
  // Per-session live state (keyed by session name).
  snapshots: Record<string, TmuxSnapshot>
  transports: Record<string, TransportState>

  // View state: the session currently displayed.
  viewSession: string | null
  snapshot: TmuxSnapshot | null
  transport: TransportState
  tmuxVersion: string
  reconnecting: boolean

  // pending command correlations (requestId → callbacks)
  pending: Record<string, CommandCallbacks>

  setSnapshot: (session: string, snap: TmuxSnapshot) => void
  setTransport: (session: string, s: TransportState) => void
  setViewSession: (session: string | null) => void
  clearSession: (session: string) => void
  setReconnecting: (v: boolean) => void
  setTmuxVersion: (v: string) => void

  trackCommand: (requestId: string, cbs: CommandCallbacks) => void
  resolveCommand: (requestId: string, ok: boolean, message?: string) => void
  forgetCommand: (requestId: string) => void

  clear: () => void
}

export const useTmuxStore = create<TmuxState>()((set, get) => ({
  snapshots: {},
  transports: {},
  viewSession: null,
  snapshot: null,
  transport: 'disconnected',
  tmuxVersion: '',
  reconnecting: false,
  pending: {},

  setSnapshot: (session, snap) =>
    set((s) => {
      const snapshots = { ...s.snapshots, [session]: snap }
      return { snapshots, snapshot: session === s.viewSession ? snap : s.snapshot }
    }),

  setTransport: (session, transport) =>
    set((s) => {
      const transports = { ...s.transports, [session]: transport }
      const isView = session === s.viewSession
      return {
        transports,
        transport: isView ? transport : s.transport,
        reconnecting: isView ? transport === 'reconnecting' : s.reconnecting,
      }
    }),

  // setViewSession switches which session the UI displays (tab switch). The
  // store keeps snapshots for all open sessions; only the view is mirrored.
  setViewSession: (viewSession) =>
    set((s) => ({
      viewSession,
      snapshot: viewSession ? (s.snapshots[viewSession] ?? null) : null,
      transport: viewSession ? (s.transports[viewSession] ?? 'disconnected') : 'disconnected',
      reconnecting: viewSession
        ? (s.transports[viewSession] ?? 'disconnected') === 'reconnecting'
        : false,
    })),

  // clearSession drops a closed tab's state (socket already disconnected).
  clearSession: (session) =>
    set((s) => {
      const snapshots = { ...s.snapshots }
      const transports = { ...s.transports }
      delete snapshots[session]
      delete transports[session]
      const view = s.viewSession === session ? null : s.viewSession
      return {
        snapshots,
        transports,
        viewSession: view,
        snapshot: view ? (snapshots[view] ?? null) : null,
        transport: view ? (transports[view] ?? 'disconnected') : 'disconnected',
        reconnecting: view ? (transports[view] ?? 'disconnected') === 'reconnecting' : false,
      }
    }),

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
      snapshots: {},
      transports: {},
      viewSession: null,
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
