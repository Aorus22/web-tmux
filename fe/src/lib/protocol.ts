// WebSocket protocol (PRD §26, §27, §28). Every GUI action carries a requestId;
// the backend replies command.success / command.error with the same requestId.

import type { TmuxSnapshot } from './tmux-types'

// --- client → server ---

export interface WsIncoming {
  type: string
  requestId?: string
  cols?: number
  rows?: number
  paneId?: string
  data?: string
  direction?: string
  amount?: number
  session?: string
  name?: string
  cwd?: string
  command?: string
  newName?: string
  title?: string
  otherPaneId?: string
  layout?: string
}

// --- server → client ---

export interface WsOutgoing {
  type: string
  requestId?: string
  session?: string
  paneId?: string
  data?: string
  message?: string
  seq?: number
  snapshot?: TmuxSnapshot
}

export const MSG = {
  hello: 'hello',
  terminalInput: 'terminal.input',
  terminalResize: 'terminal.resize',
  terminalCapture: 'terminal.capture',
  paneSelect: 'pane.select',
  paneSplit: 'pane.split',
  paneResize: 'pane.resize',
  paneKill: 'pane.kill',
  paneRename: 'pane.rename',
  paneZoom: 'pane.zoom',
  paneBreak: 'pane.break',
  paneSwap: 'pane.swap',
  windowSelect: 'window.select',
  windowCreate: 'window.create',
  windowRename: 'window.rename',
  windowKill: 'window.kill',
  windowLayout: 'window.layout',
  windowMove: 'window.move',
  windowBreakActive: 'window.break-active',
  sessionCreate: 'session.create',
  sessionRename: 'session.rename',
  sessionKill: 'session.kill',
  stateResync: 'state.resync',
} as const

export const EV = {
  connectionReady: 'connection.ready',
  stateSnapshot: 'state.snapshot',
  stateDelta: 'state.delta',
  terminalSnapshot: 'terminal.snapshot',
  terminalOutput: 'terminal.output',
  commandSuccess: 'command.success',
  commandError: 'command.error',
  tmuxDisconnected: 'tmux.disconnected',
  tmuxReconnecting: 'tmux.reconnecting',
  serverError: 'server.error',
} as const
