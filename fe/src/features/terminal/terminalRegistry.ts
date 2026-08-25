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
  // How many capture-history (scrollback) lines have already been pushed to
  // this xterm instance's scrollback. The backend sends history + screen rows
  // together; only the delta above this counter is appended per frame so the
  // cursor stays anchored to the clicked position instead of being shifted by
  // replayed history.
  ingestedHistory: number
}

const registry = new Map<string, Registered>()

// applyCapture turns a captured blob into the single atomic write for one
// frame. With valid screenRows it splits the blob into history lines + the
// visible screen tail: any not-yet-ingested history lines are written first
// (xterm pushes them into scrollback naturally), then a clear + home +
// explicitly positioned repaint of the visible grid. Without screenRows it
// reproduces the legacy absolute-write byte-for-byte.
function applyCapture(r: Registered, data: string, screenRows?: number): string {
  const rows = Number(screenRows)
  const lines = data.replace(/\r\n?/g, '\n').split('\n')

  // capture-pane includes trailing spaces up to the exact pane width. A
  // full-width row can trigger xterm's auto-wrap before its CRLF is consumed,
  // so replay each row at an explicit origin instead of relying on
  // newline/wrap behavior.
  const positioned = (rows_: string[]) => rows_.map((row, index) => `\x1b[${index + 1};1H${row}`).join('')

  if (!Number.isFinite(rows) || rows <= 0) return `\x1b[2J\x1b[H${positioned(lines)}`

  const split = Math.max(0, lines.length - rows)
  const historyLines = lines.slice(0, split)
  const screenLines = lines.slice(split)

  let payload = ''
  if (historyLines.length < r.ingestedHistory) {
    // History shrank (rare): xterm.js has no scrollback-trim API, so reset the
    // counter and accept the cosmetic edge; still write the screen precisely.
    r.ingestedHistory = 0
  } else {
    const delta = historyLines.slice(r.ingestedHistory)
    if (delta.length > 0) payload += `${delta.join('\r\n')}\r\n`
    r.ingestedHistory = historyLines.length
  }

  return payload + `\x1b[2J\x1b[H${positioned(screenLines)}`
}

export const terminalRegistry = {
  register(paneId: string, term: Terminal, cols: number, rows: number) {
    registry.set(paneId, { term, cols, rows, snapshotWritten: false, ingestedHistory: 0 })
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
  replaceScreen(paneId: string, data: string, screenRows?: number) {
    const r = registry.get(paneId)
    if (!r) return
    r.pendingScreen = applyCapture(r, data, screenRows)
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
      // applyCapture has already normalized rows and baked the explicit
      // per-row positioning into the payload, so write it as-is.
      r.term.write(next, () => queueMicrotask(flush))
    }
    flush()
  },

  // writeSnapshot applies the initial capture-pane screen. Idempotent per
  // terminal instance: the first snapshot clears stale pre-snapshot content
  // (e.g. the control-mode attach redraw) and replaces the buffer; later
  // duplicates are dropped so live output is never clobbered or doubled.
  writeSnapshot(paneId: string, data: string, screenRows?: number) {
    const r = registry.get(paneId)
    if (!r || r.snapshotWritten) return
    r.snapshotWritten = true
    terminalRegistry.replaceScreen(paneId, data, screenRows)
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
