//! Tracer headless geometry contracts (PANE-01/PANE-04 per D1).
//!
//! Verbatim port expectations from `fe/src/lib/geometry.ts:23-146`:
//! `CELL_W = 8`, `CELL_H = 18`, `pixel_rect` scale formula, `close_pane_gaps`
//! 1-cell absorption, divider adjacency in cell space, incremental drag steps
//! with FLIP. Pure — no GPUI imports.

use webtmux::pane_geometry::{
    close_pane_gaps, divider_layout, flip_direction, pixel_rect, px_to_cells,
    resize_drag_step, CellPane, CELL_H, CELL_W,
};

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
    assert_eq!(CELL_W, 8.0);
    assert_eq!(CELL_H, 18.0);

    // scaleX = containerWidth / windowWidth with rounding.
    let p = cell("%1", 10, 5, 20, 10);
    let r = pixel_rect(&p, 800.0, 600.0, 100, 50);
    assert_eq!(r.left, 80.0);
    assert_eq!(r.top, 60.0);
    assert_eq!(r.w, 160.0);
    assert_eq!(r.h, 120.0);

    // Rounding: 1 cell at 10px per cell over 3 cells -> 3.333... rounds.
    let p = cell("%2", 1, 1, 1, 1);
    let r = pixel_rect(&p, 100.0, 100.0, 3, 3);
    assert_eq!(r.left, 33.0);
    assert_eq!(r.w, 33.0);

    // Zero/negative window dims return the zero rect guard (T-05-02).
    let z = pixel_rect(&p, 800.0, 600.0, 0, 50);
    assert_eq!((z.left, z.top, z.w, z.h), (0.0, 0.0, 0.0, 0.0));
    let z = pixel_rect(&p, 800.0, 600.0, 100, 0);
    assert_eq!((z.left, z.top, z.w, z.h), (0.0, 0.0, 0.0, 0.0));
    let z = pixel_rect(&p, 800.0, 600.0, -10, 50);
    assert_eq!((z.left, z.top, z.w, z.h), (0.0, 0.0, 0.0, 0.0));
}

#[test]
fn test_close_pane_gaps() {
    // 1-cell left strip absorbed: q.left + q.width + 1 == p.left with
    // vertical overlap absorbs left.
    let panes = vec![cell("%0", 0, 0, 50, 30), cell("%1", 51, 0, 49, 30)];
    let out = close_pane_gaps(&panes);
    let right = out.iter().find(|p| p.id == "%1").unwrap();
    assert_eq!(right.left, 50);
    assert_eq!(right.w, 50);
    // Edge pane invariant: leftmost never moves.
    let left = out.iter().find(|p| p.id == "%0").unwrap();
    assert_eq!((left.left, left.w), (0, 50));

    // 1-cell top strip absorbed with horizontal overlap.
    let panes = vec![cell("%0", 0, 0, 100, 10), cell("%1", 0, 11, 100, 19)];
    let out = close_pane_gaps(&panes);
    let below = out.iter().find(|p| p.id == "%1").unwrap();
    assert_eq!(below.top, 10);
    assert_eq!(below.h, 20);

    // Order-independence: reversed input yields the same geometry.
    let panes = vec![cell("%0", 0, 0, 50, 30), cell("%1", 51, 0, 49, 30)];
    let fwd = close_pane_gaps(&panes);
    let mut rev = panes.clone();
    rev.reverse();
    let bwd = close_pane_gaps(&rev);
    for p in &fwd {
        let q = bwd.iter().find(|x| x.id == p.id).unwrap();
        assert_eq!((q.left, q.top, q.w, q.h), (p.left, p.top, p.w, p.h));
    }

    // No vertical overlap -> no absorption.
    let panes = vec![cell("%0", 0, 0, 50, 10), cell("%1", 51, 20, 49, 10)];
    let out = close_pane_gaps(&panes);
    let right = out.iter().find(|p| p.id == "%1").unwrap();
    assert_eq!((right.left, right.w), (51, 49));
}

#[test]
fn test_divider_adjacency() {
    use webtmux::pane_geometry::PxRect;
    // Two side-by-side panes: vertical divider, direction R.
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
    assert!(divs[0].is_vertical);
    assert_eq!(divs[0].rect.left, 400.0 - 2.0);
    assert_eq!(divs[0].rect.w, 4.0);

    // T-layout overlap: full-height left pane next to a split right side
    // still yields dividers (overlap, not equal-height bands).
    let cells = vec![
        cell("%0", 0, 0, 40, 30),
        cell("%1", 40, 0, 60, 15),
        cell("%2", 40, 15, 60, 15),
    ];
    let rects = vec![
        PxRect {
            left: 0.0,
            top: 0.0,
            w: 320.0,
            h: 540.0,
        },
        PxRect {
            left: 320.0,
            top: 0.0,
            w: 480.0,
            h: 270.0,
        },
        PxRect {
            left: 320.0,
            top: 270.0,
            w: 480.0,
            h: 270.0,
        },
    ];
    let divs = divider_layout(&cells, &rects);
    let verticals: Vec<_> = divs.iter().filter(|d| d.is_vertical).collect();
    assert_eq!(
        verticals.len(),
        2,
        "T-layout needs one vertical handle per right pane"
    );

    // Horizontal divider: direction D, h=4 at top-2.
    let cells = vec![cell("%0", 0, 0, 100, 15), cell("%1", 0, 15, 100, 15)];
    let rects = vec![
        PxRect {
            left: 0.0,
            top: 0.0,
            w: 800.0,
            h: 270.0,
        },
        PxRect {
            left: 0.0,
            top: 270.0,
            w: 800.0,
            h: 270.0,
        },
    ];
    let divs = divider_layout(&cells, &rects);
    assert_eq!(divs.len(), 1);
    assert_eq!(divs[0].direction, 'D');
    assert!(!divs[0].is_vertical);
    assert_eq!(divs[0].rect.top, 270.0 - 2.0);
    assert_eq!(divs[0].rect.h, 4.0);

    // Non-adjacent pairs excluded: three in a row, %0 and %2 share no edge.
    let cells = vec![
        cell("%0", 0, 0, 30, 30),
        cell("%1", 30, 0, 30, 30),
        cell("%2", 60, 0, 40, 30),
    ];
    let rects = vec![
        PxRect {
            left: 0.0,
            top: 0.0,
            w: 240.0,
            h: 540.0,
        },
        PxRect {
            left: 240.0,
            top: 0.0,
            w: 240.0,
            h: 540.0,
        },
        PxRect {
            left: 480.0,
            top: 0.0,
            w: 320.0,
            h: 540.0,
        },
    ];
    let divs = divider_layout(&cells, &rects);
    assert_eq!(divs.len(), 2, "only immediate neighbors get dividers");
    for d in &divs {
        assert_ne!(d.key, "v-%0-%2");
    }
}

#[test]
fn test_drag_step_incremental() {
    // Cumulative displacement minus last_cells yields incremental steps.
    assert_eq!(resize_drag_step(100.0, 0.0, 0, 8.0), 13); // round(100/8)=13
    assert_eq!(resize_drag_step(100.0, 0.0, 13, 8.0), 0); // already sent -> drop
    assert_eq!(resize_drag_step(120.0, 0.0, 13, 8.0), 2); // round(120/8)=15-13

    // step==0 drops (sub-cell movement).
    assert_eq!(px_to_cells(3.0, 'R', Some(8.0)), 0);
    assert_eq!(resize_drag_step(3.0, 0.0, 0, 8.0), 0);

    // Negative steps return the FLIP direction with positive amount.
    assert_eq!(flip_direction('L'), 'R');
    assert_eq!(flip_direction('R'), 'L');
    assert_eq!(flip_direction('U'), 'D');
    assert_eq!(flip_direction('D'), 'U');
    let step = resize_drag_step(-50.0, 0.0, 0, 8.0); // round(-50/8) = -6
    assert_eq!(step, -6);
    let (dir, amount) = if step < 0 {
        (flip_direction('R'), -step)
    } else {
        ('R', step)
    };
    assert_eq!((dir, amount), ('L', 6));

    // Vertical cell size defaults to CELL_H when absent.
    assert_eq!(px_to_cells(36.0, 'D', None), 2);
    assert_eq!(px_to_cells(16.0, 'R', None), 2);
    // Non-positive cell size guards to zero (T-05-02).
    assert_eq!(px_to_cells(100.0, 'R', Some(0.0)), 0);
}
