// Geometry tests (PRD §13, §19): proportional pane rendering + px→cell math.

import { describe, expect, it } from 'vitest'
import { pixelRect, pxToCells, pxToColsRows } from '@/lib/geometry'

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
