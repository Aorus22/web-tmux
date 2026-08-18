// Pane geometry (PRD §13): panes render proportionally from tmux-provided
// coordinates (pane_left/top/width/height) relative to window_width/height.
// No manual grid translation needed — geometry comes from tmux itself.

export interface Rect {
  left: number // CSS px
  top: number
  width: number // CSS px
  height: number
}

// Cell geometry shared by closePaneGaps inputs (tmux pane coordinates).
export interface CellGeometry {
  left: number
  top: number
  width: number
  height: number
}

// Nominal terminal cell size used for container → tmux-viewport conversion
// (PRD §26 terminal.resize / hello). xterm fit handles per-pane rendering; the
// window-level viewport only needs an accurate cols×rows for tmux layout.
export const CELL_W = 8
export const CELL_H = 18

// pxToColsRows converts a container's pixel size into a tmux viewport size.
// Used when reporting window resize to the backend (refresh-client -C).
export function pxToColsRows(widthPx: number, heightPx: number): { cols: number; rows: number } {
  const cols = Math.max(2, Math.round(widthPx / CELL_W))
  const rows = Math.max(1, Math.round(heightPx / CELL_H))
  return { cols, rows }
}

// pixelRect converts a pane's cell geometry to CSS pixels within a container.
export function pixelRect(
  paneLeft: number,
  paneTop: number,
  paneWidth: number,
  paneHeight: number,
  containerWidth: number,
  containerHeight: number,
  windowWidth: number,
  windowHeight: number,
): Rect {
  if (windowWidth <= 0 || windowHeight <= 0) {
    return { left: 0, top: 0, width: 0, height: 0 }
  }
  const scaleX = containerWidth / windowWidth
  const scaleY = containerHeight / windowHeight
  return {
    left: Math.round(paneLeft * scaleX),
    top: Math.round(paneTop * scaleY),
    width: Math.round(paneWidth * scaleX),
    height: Math.round(paneHeight * scaleY),
  }
}

// cellRect is the same calculation but accounting for a known cell size, used
// by the resize divider to convert pixel drags into tmux cell amounts (PRD §19).
export function pixelRectCells(
  pane: { left: number; top: number; width: number; height: number },
  containerWidth: number,
  containerHeight: number,
  windowWidth: number,
  windowHeight: number,
): Rect {
  return pixelRect(
    pane.left,
    pane.top,
    pane.width,
    pane.height,
    containerWidth,
    containerHeight,
    windowWidth,
    windowHeight,
  )
}

// tmux reserves a 1-cell strip for pane borders between adjacent panes: a
// window of 100 cols split in two reports pane A as [0,50) and pane B starting
// at left=51 — the border column belongs to no pane. Rendering those cells
// as-is leaves a visible strip of workspace background between panes, so each
// border strip is absorbed by the panes on its right/below side: a pane with a
// left-neighbor shifts left by 1 cell, a pane with a top-neighbor shifts up by
// 1 cell. left+width and top+height are invariant under absorption, so the
// result is order-independent and every shared edge stays exactly aligned
// (window edges — where no border exists — never move).
export function closePaneGaps<T extends CellGeometry>(panes: T[]): T[] {
  const out = panes.map((p) => ({ ...p }))
  for (const p of out) {
    for (const q of out) {
      if (p === q) continue
      // Left strip: q immediately left of p, vertically overlapping.
      if (
        q.left + q.width + 1 === p.left &&
        p.top < q.top + q.height &&
        q.top < p.top + p.height
      ) {
        p.left -= 1
        p.width += 1
      }
      // Top strip: q immediately above p, horizontally overlapping.
      if (
        q.top + q.height + 1 === p.top &&
        p.left < q.left + q.width &&
        q.left < p.left + p.width
      ) {
        p.top -= 1
        p.height += 1
      }
    }
  }
  return out
}

// pxToCells converts a pixel delta into a cell count for resize-pane (PRD §19).
// horizontal (L/R) uses cell width; vertical (U/D) uses cell height.
// cellPx overrides the nominal cell size with the pane's actual rendered cell
// size so the divider tracks the pointer exactly.
export function pxToCells(
  deltaPx: number,
  direction: 'L' | 'R' | 'U' | 'D',
  cellPx?: number,
): number {
  const cell = cellPx ?? (direction === 'U' || direction === 'D' ? CELL_H : CELL_W)
  if (cell <= 0) return 0
  const cells = Math.round(deltaPx / cell)
  if (cells === 0) return 0
  // tmux positive = expand right/down; our drag handles are mirrored.
  return cells
}

// resizeDragStep returns the INCREMENTAL cell step for a resize drag. The
// divider must send deltas relative to what was already sent, not the whole
// accumulated delta from the drag start: tmux applies every resize-pane on top
// of the current size, so cumulative amounts make the pane overshoot the
// pointer and, once it hits a limit, the accumulated offset keeps sending
// no-op commands in the same direction (dragging back then does nothing).
export function resizeDragStep(
  pos: number,
  start: number,
  lastCells: number,
  cell: number,
): number {
  return pxToCells(pos - start, 'R', cell) - lastCells
}

// isVerticalDivider guesses divider orientation from the two panes it separates.
export function isVerticalDivider(a: { left: number }, b: { left: number }): boolean {
  return Math.abs(a.left - b.left) > 0
}
