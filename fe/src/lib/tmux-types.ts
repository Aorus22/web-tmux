// Tmux data models (PRD §12, §6). Stable IDs: session = name, window = @N, pane = %N.

export interface TmuxSession {
  name: string
  windows: number
  attached: number
  createdAt: number
  width: number
  height: number
}

export interface TmuxWindow {
  id: string // @N
  index: number // display only
  name: string
  active: boolean
  panes: number
  width: number
  height: number
  layout: string
}

export interface TmuxPane {
  id: string // %N
  index: number // display only
  windowId: string
  active: boolean
  zoomed: boolean
  left: number
  top: number
  width: number
  height: number
  pid: number
  currentCommand: string
  currentPath: string
  title: string
}

export interface TmuxSnapshot {
  session: TmuxSession
  windows: TmuxWindow[]
  panes: TmuxPane[]
  activeWindow: string
  activePane: string
}

export interface WindowTreeNode {
  window: TmuxWindow
  panes: TmuxPane[]
}

export interface SessionTreeNode {
  session: TmuxSession
  windows: WindowTreeNode[]
}

export interface TmuxTree {
  sessions: SessionTreeNode[]
}

export interface HealthResponse {
  status: string
  tmux: {
    installed: boolean
    version: string
  }
}

export interface TmuxInfo {
  version: string
  ok: boolean
}
