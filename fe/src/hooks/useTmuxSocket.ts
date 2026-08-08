// useTmuxSocket — the single WebSocket lifecycle hook (PRD §26-28, §48).
// Configures the shared singleton socket, connects to the active session,
// feeds live state into the tmux store, and routes terminal output/snapshots
// to the terminal registry. Reconnects with exponential backoff; on reconnect
// it requests a full snapshot (PRD §48).

import { useEffect } from 'react'
import { configureSocket, connectSocket, disconnectSocket, tmuxSocket } from '@/lib/socket'
import { useTmuxStore } from '@/stores/tmuxStore'
import { terminalRegistry } from '@/features/terminal/terminalRegistry'

export function useTmuxSocket(session: string | null) {
  useEffect(() => {
    if (!session) {
      disconnectSocket()
      return
    }

    const store = useTmuxStore.getState()
    store.clear()
    store.setTransport('connecting')

    configureSocket({
      onReady: () => {
        // Ask for live state after (re)connect (PRD §48: request full snapshot).
        useTmuxStore.getState().setTransport('connected')
        tmuxSocket.requestState()
      },
      onStateSnapshot: (snap) => {
        useTmuxStore.getState().setSnapshot(snap as never)
      },
      onStateDelta: (snap) => {
        useTmuxStore.getState().setSnapshot(snap as never)
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
        useTmuxStore.getState().setReconnecting(true)
      },
      onDisconnected: () => {
        useTmuxStore.getState().setReconnecting(true)
      },
      onServerError: (message) => {
        console.error('[ws] server error:', message)
      },
      onStateChange: (s) => {
        useTmuxStore.getState().setTransport(s)
      },
    })

    connectSocket(session)

    return () => {
      disconnectSocket()
    }
  }, [session])
}
