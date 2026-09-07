//! Pure tmux pane geometry (Phase 5, D1).
//!
//! Verbatim port of `fe/src/lib/geometry.ts:23-146` operating on `i64` cell
//! coordinates — snapshot `usize` cells convert at the module boundary
//! (`cell_pane`), so `p.left - 1` never underflows `usize` in debug. No GPUI
//! types inside; headless `cargo test` coverage ships in the same wave.
//!
//! Threat mitigations (T-05-02): `ww<=0||wh<=0 → zero rect` guard, `i64`
//! internals, `cols.max(2)/rows.max(1)` clamps; zero-dim windows render empty
//! without panic.

/// Nominal terminal cell size (`geometry.ts:23-24`).
pub const CELL_W: f32 = 8.0;
/// Nominal terminal cell height (`geometry.ts:24`).
pub const CELL_H: f32 = 18.0;

/// Cell-space pane geometry (tmux `pane_left/top/width/height`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellPane {
    pub id: String,
    pub left: i64,
    pub top: i64,
    pub w: i64,
    pub h: i64,
}

/// Pixel-space rect within the workspace container.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PxRect {
    pub left: f32,
    pub top: f32,
    pub w: f32,
    pub h: f32,
}

/// Divider handle derived from cell adjacency + pixel rects.
#[derive(Debug, Clone, PartialEq)]
pub struct DividerHandle {
    pub key: String,
    pub pane_id: String,
    pub direction: char,
    pub rect: PxRect,
    pub is_vertical: bool,
    pub cell_w: f32,
    pub cell_h: f32,
}

/// Convert snapshot `usize` cells to `i64` at the module boundary.
pub fn cell_pane(id: String, left: usize, top: usize, w: usize, h: usize) -> CellPane {
    CellPane {
        id,
        left: left as i64,
        top: top as i64,
        w: w as i64,
        h: h as i64,
    }
}

/// `pixelRect` (`geometry.ts:35-56`): cell geometry → container pixels via
/// `scaleX = containerWidth / windowWidth` with rounding. Zero/negative
/// window dims return the zero rect guard.
pub fn pixel_rect(
    p: &CellPane,
    container_w: f32,
    container_h: f32,
    window_w: i64,
    window_h: i64,
) -> PxRect {
    if window_w <= 0 || window_h <= 0 {
        return PxRect {
            left: 0.0,
            top: 0.0,
            w: 0.0,
            h: 0.0,
        };
    }
    let sx = container_w / window_w as f32;
    let sy = container_h / window_h as f32;
    PxRect {
        left: (p.left as f32 * sx).round(),
        top: (p.top as f32 * sy).round(),
        w: (p.w as f32 * sx).round(),
        h: (p.h as f32 * sy).round(),
    }
}

/// `closePaneGaps` (`geometry.ts:88-114`): absorb tmux's 1-cell border strips
/// so panes tile edge-to-edge. A pane with a left-neighbor shifts left by 1
/// cell; a pane with a top-neighbor shifts up by 1 cell. `left+width` and
/// `top+height` are invariant under absorption, so the result is
/// order-independent and every shared edge stays exactly aligned (window
/// edges — where no border exists — never move).
pub fn close_pane_gaps(panes: &[CellPane]) -> Vec<CellPane> {
    let mut out: Vec<CellPane> = panes.to_vec();
    let n = out.len();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let (ql, qt, qw, qh) = (out[j].left, out[j].top, out[j].w, out[j].h);
            // Left strip: q immediately left of p, vertically overlapping.
            {
                let (pl, pt, _pw, ph) = (out[i].left, out[i].top, out[i].w, out[i].h);
                if ql + qw + 1 == pl && pt < qt + qh && qt < pt + ph {
                    out[i].left -= 1;
                    out[i].w += 1;
                }
            }
            // Top strip: q immediately above p, horizontally overlapping.
            {
                let (pl, pt, pw, _ph) = (out[i].left, out[i].top, out[i].w, out[i].h);
                if qt + qh + 1 == pt && pl < ql + qw && ql < pl + pw {
                    out[i].top -= 1;
                    out[i].h += 1;
                }
            }
        }
    }
    out
}

/// `pxToCells` (`geometry.ts:120-131`): pixel delta → cell count for
/// `resize-pane`. Horizontal (L/R) uses cell width; vertical (U/D) uses cell
/// height. `cell_px` overrides the nominal size with the pane's actual
/// rendered cell size so the divider tracks the pointer exactly.
pub fn px_to_cells(delta_px: f32, direction: char, cell_px: Option<f32>) -> i64 {
    let cell = cell_px.unwrap_or_else(|| {
        if direction == 'U' || direction == 'D' {
            CELL_H
        } else {
            CELL_W
        }
    });
    if cell <= 0.0 {
        return 0;
    }
    let cells = (delta_px / cell).round() as i64;
    if cells == 0 {
        return 0;
    }
    cells
}

/// `resizeDragStep` (`geometry.ts:139-146`): the INCREMENTAL cell step for a
/// resize drag — deltas relative to what was already sent, not the whole
/// accumulated delta from the drag start (tmux applies every resize on top of
/// the current size, so cumulative amounts overshoot and stick at limits).
pub fn resize_drag_step(pos: f32, start: f32, last_cells: i64, cell: f32) -> i64 {
    px_to_cells(pos - start, 'R', Some(cell)) - last_cells
}

/// Negative-step flip (`PaneResizeHandle.tsx:30-35`): tmux rejects negative
/// resize adjustments, so a negative step flips the direction and keeps the
/// amount positive. `FLIP = { L:R, R:L, U:D, D:U }`.
pub fn flip_direction(dir: char) -> char {
    match dir {
        'L' => 'R',
        'R' => 'L',
        'U' => 'D',
        'D' => 'U',
        other => other,
    }
}

/// Divider handles between panes sharing an edge (`PaneWorkspace.tsx:276-345`).
///
/// Adjacency is checked in CELL space (exact integers after `close_pane_gaps`):
/// only immediate neighbors get a divider. Each handle spans the overlapping
/// portion of the edge — not just same-height/same-width bands — so T layouts
/// (full-height left pane next to a split right side) get working handles.
/// Vertical handles: `left = a.rect.left + a.rect.width - 2`, `w = 4`,
/// cursor col-resize, direction `R` on the trailing pane. Horizontal:
/// `top = a.rect.top + a.rect.height - 2`, `h = 4`, cursor row-resize,
/// direction `D`.
pub fn divider_layout(cells: &[CellPane], rects: &[PxRect]) -> Vec<DividerHandle> {
    debug_assert_eq!(cells.len(), rects.len());
    let mut dividers = Vec::new();
    for (ai, a) in cells.iter().enumerate() {
        for (bi, b) in cells.iter().enumerate() {
            if ai == bi {
                continue;
            }
            let ar = rects[ai];
            let br = rects[bi];
            // Vertical divider: b immediately right of a, vertical overlap.
            if b.left == a.left + a.w && a.top < b.top + b.h && b.top < a.top + a.h {
                let top = ar.top.max(br.top);
                let bottom = (ar.top + ar.h).min(br.top + br.h);
                dividers.push(DividerHandle {
                    key: format!("v-{}-{}", a.id, b.id),
                    pane_id: a.id.clone(),
                    direction: 'R',
                    rect: PxRect {
                        left: ar.left + ar.w - 2.0,
                        top,
                        w: 4.0,
                        h: bottom - top,
                    },
                    is_vertical: true,
                    cell_w: if a.w > 0 { ar.w / a.w as f32 } else { CELL_W },
                    cell_h: if a.h > 0 { ar.h / a.h as f32 } else { CELL_H },
                });
            }
            // Horizontal divider: b immediately below a, horizontal overlap.
            if b.top == a.top + a.h && a.left < b.left + b.w && b.left < a.left + a.w {
                let left = ar.left.max(br.left);
                let right = (ar.left + ar.w).min(br.left + br.w);
                dividers.push(DividerHandle {
                    key: format!("h-{}-{}", a.id, b.id),
                    pane_id: a.id.clone(),
                    direction: 'D',
                    rect: PxRect {
                        left,
                        top: ar.top + ar.h - 2.0,
                        w: right - left,
                        h: 4.0,
                    },
                    is_vertical: false,
                    cell_w: if a.w > 0 { ar.w / a.w as f32 } else { CELL_W },
                    cell_h: if a.h > 0 { ar.h / a.h as f32 } else { CELL_H },
                });
            }
        }
    }
    dividers
}

/// `pxToColsRows` (`geometry.ts:28-32`): container pixels → tmux viewport
/// with `cols.max(2)/rows.max(1)` clamps.
pub fn px_to_cols_rows(width_px: f32, height_px: f32) -> (usize, usize) {
    (
        (width_px / CELL_W).round().max(2.0) as usize,
        (height_px / CELL_H).round().max(1.0) as usize,
    )
}

/// Drag send throttle window (`PaneResizeHandle.tsx:87-90`, ~40ms per PRD §19).
pub const DRAG_THROTTLE_MS: u64 = 40;

/// Throttled incremental drag decision: the pure half of the D3 divider-drag
/// state machine (`PaneResizeHandle.tsx:80-94`).
///
/// `pos_px` is the current pointer position on the drag axis, `start_px` the
/// position at `mouse_down`, `last_cells` the cumulative cells already sent,
/// `cell_px` the dragged pane's axis cell size, and `elapsed_ms` the time
/// since the last sent step. Returns `(send_direction, amount, new_last_cells)`
/// when a `pane.resize` step should fire, `None` otherwise:
/// - `step == 0` drops (sub-cell jitter);
/// - sends inside the 40ms window drop WITHOUT advancing `last_cells`, so the
///   next command still carries the full pending difference;
/// - negative steps flip the direction with a positive amount (tmux rejects
///   negative adjustments — `FLIP = { L:R, R:L, U:D, D:U }`).
pub fn drag_step_throttled(
    direction: char,
    pos_px: f32,
    start_px: f32,
    last_cells: i64,
    cell_px: f32,
    elapsed_ms: u64,
) -> Option<(char, i32, i64)> {
    let step = resize_drag_step(pos_px, start_px, last_cells, cell_px);
    if step == 0 {
        return None;
    }
    if elapsed_ms < DRAG_THROTTLE_MS {
        return None;
    }
    let new_last_cells = last_cells + step;
    let (send_dir, amount) = if step < 0 {
        (flip_direction(direction), step.saturating_abs())
    } else {
        (direction, step)
    };
    let amount = i32::try_from(amount).unwrap_or(i32::MAX);
    Some((send_dir, amount, new_last_cells))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(id: &str, left: i64, top: i64, w: i64, h: i64) -> CellPane {
        CellPane {
            id: id.to_string(),
            left,
            top,
            w,
            h,
        }
    }

    #[test]
    fn test_pixel_rect_scale() {
        let p = cell("%1", 10, 5, 20, 10);
        let r = pixel_rect(&p, 800.0, 600.0, 100, 50);
        assert_eq!((r.left, r.top, r.w, r.h), (80.0, 60.0, 160.0, 120.0));
        let z = pixel_rect(&p, 800.0, 600.0, 0, 50);
        assert_eq!((z.left, z.top, z.w, z.h), (0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn test_close_pane_gaps() {
        let panes = vec![cell("%0", 0, 0, 50, 30), cell("%1", 51, 0, 49, 30)];
        let out = close_pane_gaps(&panes);
        let right = out.iter().find(|p| p.id == "%1").unwrap();
        assert_eq!((right.left, right.w), (50, 50));
    }

    #[test]
    fn test_divider_adjacency() {
        let cells = vec![cell("%0", 0, 0, 50, 30), cell("%1", 50, 0, 50, 30)];
        let rects = vec![
            PxRect {
                left: 0.0,
                top: 0.0,
                w: 400.0,
                h: 540.0,
            },
            PxRect {
                left: 400.0,
                top: 0.0,
                w: 400.0,
                h: 540.0,
            },
        ];
        let divs = divider_layout(&cells, &rects);
        assert_eq!(divs.len(), 1);
        assert_eq!(divs[0].direction, 'R');
    }

    #[test]
    fn test_drag_step_incremental() {
        assert_eq!(resize_drag_step(100.0, 0.0, 0, 8.0), 13);
        assert_eq!(resize_drag_step(100.0, 0.0, 13, 8.0), 0);
        assert_eq!(flip_direction('R'), 'L');
    }
}
