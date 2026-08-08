// Singleton socket (PRD §46): exactly one active session connection at a time.
// UI components call tmuxSocket.<action>(...) directly; useTmuxSocket
// configures the handlers, then connects/disconnects.

import { TmuxSocket, type WsHandlers } from './websocket'

export const tmuxSocket = new TmuxSocket('', {})

export function configureSocket(handlers: WsHandlers) {
  tmuxSocket.setHandlers(handlers)
}

export function connectSocket(session: string) {
  tmuxSocket.setSession(session)
  tmuxSocket.connect()
}

export function disconnectSocket() {
  tmuxSocket.close()
}
