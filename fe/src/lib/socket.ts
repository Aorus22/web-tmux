// Facade socket (multi-session): UI components call `tmuxSocket.<action>()`
// and the call is delegated to the ACTIVE session's live socket. Each open
// session tab owns a real TmuxSocket managed by lib/sockets.ts; this proxy
// keeps existing components working unchanged while always targeting the
// session currently displayed.
//
// getOrCreateSocket (not getSocket) so messages sent before the socket
// manager connects — e.g. the initial terminalCapture when a workspace
// mounts — are queued on the socket and flushed once it connects.

import type { TmuxSocket } from './websocket'
import { useAppStore } from '@/stores/appStore'
import { getOrCreateSocket } from './sockets'

const NOOP = (): string => ''

export const tmuxSocket: TmuxSocket = new Proxy({} as TmuxSocket, {
  get(_target, prop: string | symbol) {
    if (typeof prop !== 'string') return undefined
    const active = useAppStore.getState().activeSession
    const socket = active ? getOrCreateSocket(active) : undefined
    if (!socket) return NOOP
    const value = (socket as unknown as Record<string, unknown>)[prop]
    return typeof value === 'function'
      ? (value as (...args: unknown[]) => unknown).bind(socket)
      : value
  },
})
