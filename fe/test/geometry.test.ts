// Geometry tests (PRD §13, §19): proportional pane rendering + px→cell math.

import { describe, expect, it } from 'vitest'
import {
  closePaneGaps,
  pixelRect,
  pxToCells,
  pxToColsRows,
  resizeDragStep,
} from '@/lib/geometry'

describe('pixelRect', () => {
  it('maps cell geometry proportionally to container pixels', () => {
    const r = pixelRect(0, 0, 40, 12, 800, 480, 80, 24)
    expect(r.left).toBe(0)
    expect(r.top).toBe(0)
    expect(r.width).toBe(400) // 40/80 of 800
    expect(r.height).toBe(240) // 12/24 of 480
  })

  it('handles offset panes', () => {
    const r = pixelRect(40, 12, 40, 12, 800, 480, 80, 24)
    expect(r.left).toBe(400)
    expect(r.top).toBe(240)
    expect(r.width).toBe(400)
    expect(r.height).toBe(240)
  })

  it('guards against zero window size', () => {
    const r = pixelRect(0, 0, 40, 12, 800, 480, 0, 0)
    expect(r.width).toBe(0)
    expect(r.height).toBe(0)
  })
})

describe('pxToCells', () => {
  it('converts horizontal pixels using cell width', () => {
    expect(pxToCells(40, 'R')).toBe(5) // 40px / 8px cell
  })

  it('converts vertical pixels using cell height', () => {
    expect(pxToCells(36, 'D')).toBe(2) // 36px / 18px cell
  })

  it('rounds to nearest cell and clamps zero', () => {
    expect(pxToCells(3, 'L')).toBe(0)
    expect(pxToCells(20, 'R')).toBe(3) // 20/8 = 2.5 → 3
  })

  it('accepts an explicit rendered cell size', () => {
    expect(pxToCells(40, 'R', 4)).toBe(10)
    expect(pxToCells(18, 'D', 9)).toBe(2)
  })
})

describe('resizeDragStep', () => {
  // Incremental steps: each command carries only the difference from what was
  // already sent, so the pane tracks the pointer (tmux applies every resize
  // on top of the current size — cumulative amounts would overshoot).
  it('returns the difference from the last sent cells', () => {
    expect(resizeDragStep(100, 0, 0, 8)).toBe(13) // 100px = 12.5 → 13
    expect(resizeDragStep(100, 0, 13, 8)).toBe(0) // unchanged
    expect(resizeDragStep(50, 0, 13, 8)).toBe(-7) // back left of 50px
  })

  it('reverses direction once the pointer crosses back, even past a limit', () => {
    // Drag to +40 cells (hitting a no-op limit in tmux), then back 100px:
    // the step is negative immediately — no accumulated positive offset to
    // overcome first.
    expect(resizeDragStep(200, 0, 40, 8)).toBe(-15) // 200px = 25 → 25 - 40
  })

  it('uses the actual rendered cell size when provided', () => {
    expect(resizeDragStep(36, 0, 0, 9)).toBe(4)
  })
})

describe('pxToColsRows', () => {
  it('converts container pixels to a tmux viewport size', () => {
    const vp = pxToColsRows(1280, 720)
    expect(vp.cols).toBe(160) // 1280 / 8
    expect(vp.rows).toBe(40) // 720 / 18
  })

  it('clamps minimums to keep the viewport usable', () => {
    const vp = pxToColsRows(4, 2)
    expect(vp.cols).toBe(2)
    expect(vp.rows).toBe(1)
  })

  it('rounds partial cells', () => {
    expect(pxToColsRows(100, 100)).toEqual({ cols: 13, rows: 6 })
  })
})

describe('closePaneGaps', () => {
  // Real tmux geometry for a 100x30 window split -h then -v (T layout): the
  // vertical border strip (column 50) and the horizontal strip (row 15) belong
  // to no pane — A spans [0,50), B/C start at left=51, C starts at top=16.
  const tmuxGeometry = [
    { left: 0, top: 0, width: 50, height: 30 }, // A: full-height left
    { left: 51, top: 0, width: 49, height: 15 }, // B: top right
    { left: 51, top: 16, width: 49, height: 14 }, // C: bottom right
  ]

  it('absorbs border strips so panes tile edge-to-edge (T layout)', () => {
    const out = closePaneGaps(tmuxGeometry)
    // A stays at the window edges; B/C absorb the vertical strip (shift left),
    // C also absorbs the horizontal strip (shift up).
    expect(out[0]).toEqual({ left: 0, top: 0, width: 50, height: 30 })
    expect(out[1]).toEqual({ left: 50, top: 0, width: 50, height: 15 })
    expect(out[2]).toEqual({ left: 50, top: 15, width: 50, height: 15 })
    // Every shared edge lines up exactly.
    expect(out[0].left + out[0].width).toBe(out[1].left)
    expect(out[0].left + out[0].width).toBe(out[2].left)
    expect(out[1].top + out[1].height).toBe(out[2].top)
    expect(out[0].top + out[0].height).toBe(out[2].top + out[2].height)
  })

  it('tiles a plain side-by-side split', () => {
    const out = closePaneGaps([
      { left: 0, top: 0, width: 50, height: 30 },
      { left: 51, top: 0, width: 49, height: 30 },
    ])
    expect(out[0]).toEqual({ left: 0, top: 0, width: 50, height: 30 })
    expect(out[1]).toEqual({ left: 50, top: 0, width: 50, height: 30 })
    expect(out[0].left + out[0].width).toBe(out[1].left)
  })

  it('tiles a 2x2 grid', () => {
    const out = closePaneGaps([
      { left: 0, top: 0, width: 50, height: 15 },
      { left: 51, top: 0, width: 49, height: 15 },
      { left: 0, top: 16, width: 50, height: 14 },
      { left: 51, top: 16, width: 49, height: 14 },
    ])
    // All four panes become exactly 50x15.
    expect(out.map((p) => `${p.left},${p.top},${p.width},${p.height}`)).toEqual([
      '0,0,50,15',
      '50,0,50,15',
      '0,15,50,15',
      '50,15,50,15',
    ])
  })

  it('leaves already-tiled panes untouched', () => {
    const out = closePaneGaps([
      { left: 0, top: 0, width: 50, height: 30 },
      { left: 50, top: 0, width: 50, height: 30 },
    ])
    expect(out[0]).toEqual({ left: 0, top: 0, width: 50, height: 30 })
    expect(out[1]).toEqual({ left: 50, top: 0, width: 50, height: 30 })
  })

  it('does not mutate the input panes', () => {
    const input = [{ left: 0, top: 0, width: 50, height: 30 }]
    closePaneGaps(input)
    expect(input[0]).toEqual({ left: 0, top: 0, width: 50, height: 30 })
  })
})
