// Pane geometry (PRD §13): panes render proportionally from tmux-provided
// coordinates (pane_left/top/width/height) relative to window_width/height.
// No manual grid translation needed — geometry comes from tmux itself.

export interface Rect {
  left: number // CSS px
  top: number
  width: number // CSS px
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

// pxToCells converts a pixel delta into a cell count for resize-pane (PRD §19).
// horizontal (L/R) uses cell width; vertical (U/D) uses cell height.
export function pxToCells(deltaPx: number, direction: 'L' | 'R' | 'U' | 'D'): number {
  const cell = direction === 'U' || direction === 'D' ? CELL_H : CELL_W
  if (cell <= 0) return 0
  const cells = Math.round(deltaPx / cell)
  if (cells === 0) return 0
  // tmux positive = expand right/down; our drag handles are mirrored.
  return cells
}

// isVerticalDivider guesses divider orientation from the two panes it separates.
export function isVerticalDivider(a: { left: number }, b: { left: number }): boolean {
  return Math.abs(a.left - b.left) > 0
}
