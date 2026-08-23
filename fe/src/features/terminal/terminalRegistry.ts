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
  pendingScreen?: string
  writingScreen?: boolean
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

  // Native Windows polling produces complete screen frames. xterm parses
  // writes asynchronously, so clearing immediately before every write can
  // interleave an older frame with a newer one during resize. Keep only the
  // newest pending frame and start the next one after the previous write's
  // callback, making clear + write atomic at the frame level.
  replaceScreen(paneId: string, data: string) {
    const r = registry.get(paneId)
    if (!r) return
    r.pendingScreen = data
    if (r.writingScreen) return
    r.writingScreen = true

    const flush = () => {
      if (registry.get(paneId) !== r) return
      const next = r.pendingScreen
      r.pendingScreen = undefined
      if (next === undefined) {
        r.writingScreen = false
        return
      }
      // `Terminal.clear()` only clears the buffer; it does not move xterm's
      // cursor back to the origin. Native Windows frames are complete grids,
      // so writing the next frame from the previous cursor position shifts
      // rows/columns and produces artifacts such as `Se-`/`PadaPadanan`.
      // Keep clear + frame in one parser write so it cannot interleave.
      // capture-pane includes trailing spaces up to the exact pane width. A
      // full-width row can trigger xterm's auto-wrap before its CRLF is
      // consumed, so replay each row at an explicit origin instead of relying
      // on newline/wrap behavior.
      const rows = next.replace(/\r\n?/g, '\n').split('\n')
      const positioned = rows
        .map((row, index) => `\x1b[${index + 1};1H${row}`)
        .join('')
      r.term.write(`\x1b[2J\x1b[H${positioned}`, () => queueMicrotask(flush))
    }
    flush()
  },

  // writeSnapshot applies the initial capture-pane screen. Idempotent per
  // terminal instance: the first snapshot clears stale pre-snapshot content
  // (e.g. the control-mode attach redraw) and replaces the buffer; later
  // duplicates are dropped so live output is never clobbered or doubled.
  writeSnapshot(paneId: string, data: string) {
    const r = registry.get(paneId)
    if (!r || r.snapshotWritten) return
    r.snapshotWritten = true
    terminalRegistry.replaceScreen(paneId, data)
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
