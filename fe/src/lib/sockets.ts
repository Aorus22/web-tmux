// Socket manager: one WebSocket per open session tab (multi-session support).
// Each socket owns handlers that route state into the per-session store
// entries and terminal output into the global terminal registry (pane ids are
// unique across sessions on one tmux server, so routing stays unambiguous).
//
// Lifecycle: ensureSocket() when a tab opens, closeSocket() when it closes.
// The backend stops a session's control-mode monitor when its last client
// leaves, so closing a tab frees server resources while the tmux session
// itself stays alive.

import { TmuxSocket, type WsHandlers } from './websocket'
import { useTmuxStore } from '@/stores/tmuxStore'
import { useAppStore } from '@/stores/appStore'
import { terminalRegistry } from '@/features/terminal/terminalRegistry'

interface Entry {
  socket: TmuxSocket
  connected: boolean
}

const sockets = new Map<string, Entry>()

function makeHandlers(session: string): WsHandlers {
  return {
    onReady: () => {
      useTmuxStore.getState().setTransport(session, 'connected')
      // Ask for live state after (re)connect (PRD §48: request full snapshot).
      sockets.get(session)?.socket.requestState()
    },
    onStateSnapshot: (snap) => {
      useTmuxStore.getState().setSnapshot(session, snap as never)
    },
    onStateDelta: (snap) => {
      useTmuxStore.getState().setSnapshot(session, snap as never)
    },
    onTerminalOutput: (paneId, data) => {
      terminalRegistry.write(paneId, data)
    },
    onTerminalSnapshot: (paneId, data) => {
      // Full-screen replacement (idempotent per terminal instance) — never
      // append the capture on top of existing rows (PRD §24).
      terminalRegistry.writeSnapshot(paneId, data)
    },
    onCommandResult: (requestId, ok, message) => {
      useTmuxStore.getState().resolveCommand(requestId, ok, message)
    },
    onReconnecting: () => {
      useTmuxStore.getState().setTransport(session, 'reconnecting')
    },
    onDisconnected: () => {
      useTmuxStore.getState().setTransport(session, 'disconnected')
    },
    onServerError: (message) => {
      console.error('[ws] server error:', message)
    },
    onStateChange: (s) => {
      useTmuxStore.getState().setTransport(session, s)
    },
  }
}

// getSocket returns the live socket for a session, or undefined when the tab
// is not open / already closed.
export function getSocket(session: string): TmuxSocket | undefined {
  return sockets.get(session)?.socket
}

// getOrCreateSocket is used by the facade: creates the socket (without
// connecting) so messages sent before the socket manager runs — e.g. the
// initial terminalCapture when a tab's workspace mounts — are queued and
// flushed once the manager connects the socket.
export function getOrCreateSocket(session: string): TmuxSocket | undefined {
  if (!useAppStore.getState().openSessions.includes(session)) return undefined
  const existing = sockets.get(session)
  if (existing) return existing.socket
  const socket = new TmuxSocket(session, makeHandlers(session))
  sockets.set(session, { socket, connected: false })
  return socket
}

// ensureSocket creates (if needed) and connects the socket for an open tab.
// Callers reset the per-session store state before opening.
export function ensureSocket(session: string): TmuxSocket {
  let entry = sockets.get(session)
  if (!entry) {
    entry = { socket: new TmuxSocket(session, makeHandlers(session)), connected: false }
    sockets.set(session, entry)
  }
  if (!entry.connected) {
    entry.connected = true
    entry.socket.connect()
  }
  return entry.socket
}

// closeSocket disconnects and drops a closed tab's socket.
export function closeSocket(session: string) {
  sockets.get(session)?.socket.close()
  sockets.delete(session)
}
