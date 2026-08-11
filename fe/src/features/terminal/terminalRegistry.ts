// Terminal registry (PRD §46: terminal registry in Zustand store).
//
// Terminal output must reach the right xterm instance without re-rendering
// React for every keystroke of output. Each TerminalView registers itself here
// keyed by paneId; the WebSocket handler writes output straight to the
// registered terminal.

import type { Terminal } from '@xterm/xterm'

type Registered = {
  term: Terminal
  cols: number
  rows: number
  // A capture-pane snapshot is a full-screen replacement. It must be written
  // exactly once per terminal instance: React StrictMode (dev) double-mounts
  // and can queue TWO terminal.capture requests on the one WebSocket, and both
  // responses would otherwise append the same screen → every line doubled.
  snapshotWritten?: boolean
}

const registry = new Map<string, Registered>()

export const terminalRegistry = {
  register(paneId: string, term: Terminal, cols: number, rows: number) {
    registry.set(paneId, { term, cols, rows, snapshotWritten: false })
  },

  unregister(paneId: string) {
    registry.delete(paneId)
  },

  get(paneId: string): Registered | undefined {
    return registry.get(paneId)
  },

  write(paneId: string, data: string) {
    const r = registry.get(paneId)
    r?.term.write(data)
  },

  // writeSnapshot applies the initial capture-pane screen. Idempotent per
  // terminal instance: the first snapshot clears stale pre-snapshot content
  // (e.g. the control-mode attach redraw) and replaces the buffer; later
  // duplicates are dropped so live output is never clobbered or doubled.
  writeSnapshot(paneId: string, data: string) {
    const r = registry.get(paneId)
    if (!r || r.snapshotWritten) return
    r.snapshotWritten = true
    r.term.clear()
    r.term.write(data)
  },

  // invalidateSnapshot re-arms the snapshot guard so the next capture-pane
  // snapshot replaces the buffer instead of being dropped as a duplicate.
  // Used to force a full-screen resync after zoom/layout/window changes, where
  // only incremental output arrives and stale pixels can linger or overlap.
  invalidateSnapshot(paneId: string) {
    const r = registry.get(paneId)
    if (r) r.snapshotWritten = false
  },

  has(paneId: string): boolean {
    return registry.has(paneId)
  },

  // updateSize keeps the stored viewport in sync (used by fit).
  setSize(paneId: string, cols: number, rows: number) {
    const r = registry.get(paneId)
    if (r) {
      r.cols = cols
      r.rows = rows
    }
  },

  size(paneId: string): { cols: number; rows: number } | null {
    const r = registry.get(paneId)
    return r ? { cols: r.cols, rows: r.rows } : null
  },
}
