//! Workspace pane grid (Phase 5, PANE-01/02/03/04).
//!
//! FE parity (`PaneWorkspace.tsx:116-120,261-370`): filter `snapshot.panes`
//! by `activeWindow`, zoomed window renders only the zoomed pane full-size
//! with dividers hidden, otherwise `close_pane_gaps` then `pixel_rect` per
//! pane with absolutely positioned children. Divider handles render from
//! `divider_layout` and arm the D3 drag state machine on `mouse_down`
//! (40ms throttle + FLIP, incremental `pane.resize` steps, receiver dropped);
//! grid-level `mouse_move` streams the drag so fast drags past the 4px handle
//! keep resizing (the pointer-capture equivalent — div listeners cannot reach
//! `Window::capture_pointer`, which needs a `HitboxId`; see research A2).
//!
//! Container measurement: GPUI lays out before paint, so the grid measures
//! itself with a full-size `canvas` probe (the `TerminalView` measure
//! pattern) into `AppState::workspace_size` and positions from the last
//! known size (`800x600` before the first measure). The Phase-4 150/325ms
//! layout-key resync keeps self-debouncing under the new layout via the
//! existing `observe_layout_key_and_schedule` (first mount skips — each
//! `TerminalView` already captures initially); drag steps never arm it
//! directly, so no drag storms. This module adds no timers.

use gpui::*;
use gpui::prelude::FluentBuilder;
use std::sync::Arc;
use webtmux_backend_client::TmuxPane;

use crate::app_state::AppState;
use crate::pane_geometry::{
    DividerHandle, cell_pane, close_pane_gaps, divider_layout, pixel_rect,
};
use crate::views::pane_view::render_pane_view;
use crate::views::terminal_view::TerminalView;
use crate::views::window_toolbar::render_window_toolbar;

/// Fallback container size before the first canvas measure (px).
const FALLBACK_W: f32 = 800.0;
const FALLBACK_H: f32 = 600.0;

/// Active window's panes plus window dims in tmux cells.
fn active_window_geometry(app: &AppState) -> Option<(Vec<TmuxPane>, usize, usize, String)> {
    let name = app.active_session.as_deref()?;
    let snap = app.sessions.get(name)?.snapshot.as_ref()?;
    let mut panes: Vec<TmuxPane> = snap
        .panes
        .iter()
        .filter(|p| p.window_id == snap.active_window)
        .cloned()
        .collect();
    if panes.is_empty() {
        return None;
    }
    panes.sort_by(|a, b| a.index.cmp(&b.index));
    let window_obj = snap.windows.iter().find(|w| w.id == snap.active_window);
    let ww = window_obj
        .map(|w| w.width)
        .unwrap_or_else(|| panes[0].width);
    let wh = window_obj
        .map(|w| w.height)
        .unwrap_or_else(|| panes[0].height);
    Some((panes, ww, wh, name.to_string()))
}

/// Render the workspace grid for the active session's active window:
/// the window toolbar (layout presets) above the geometry grid.
pub fn render_pane_grid(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let toolbar = render_window_toolbar(app, cx).into_any_element();
    let body = render_grid_body(app, cx).into_any_element();
    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(rgb(0x1e1e1e))
        .child(toolbar)
        .child(body)
        .into_any_element()
}

/// Render the geometry grid body (panes + dividers) inside the flex-1 space
/// below the toolbar. The canvas probe measures this inner container, so the
/// h-8 toolbar never enters pane geometry (FE parity — the toolbar lives
/// outside the workspace div).
fn render_grid_body(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let muted_text = rgb(0x808080);

    let Some((panes, ww, wh, _session)) = active_window_geometry(app) else {
        return div()
            .flex()
            .flex_1()
            .w_full()
            .min_h(px(0.0))
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(muted_text)
            .child(match &app.active_session {
                Some(name) => format!("Active session: {}", name),
                None => "Select a session".to_string(),
            })
            .into_any_element();
    };

    // Zoom: only the zoomed pane is visible, full-size (FE `:119-120`).
    let zoomed = panes.iter().find(|p| p.zoomed).cloned();
    let visible: Vec<TmuxPane> = match zoomed.clone() {
        Some(z) => vec![z],
        None => panes.clone(),
    };
    // Zoom affordance follows the full same-window count, not the visible
    // count — a zoomed pane still offers unzoom (FE `canZoom`).
    let can_zoom = panes.len() > 1;

    let (cw, ch) = app.workspace_size.unwrap_or((FALLBACK_W, FALLBACK_H));

    // tmux leaves a 1-cell border strip between panes — absorb those cells so
    // panes tile edge-to-edge instead of showing workspace background.
    let cells: Vec<_> = visible
        .iter()
        .map(|p| cell_pane(p.id.clone(), p.left, p.top, p.width, p.height))
        .collect();
    let tiled = close_pane_gaps(&cells);
    let rects: Vec<_> = tiled
        .iter()
        .map(|c| pixel_rect(c, cw, ch, ww as i64, wh as i64))
        .collect();

    // Dividers between immediate neighbors only (T-layout overlap included,
    // non-adjacent pairs excluded); hidden while zoomed.
    let dividers = if zoomed.is_some() {
        Vec::new()
    } else {
        divider_layout(&tiled, &rects)
    };

    // Retain Phase-4 store ownership: views hold only a shared terminal
    // clone — the store stays the owner, so hidden sessions keep ingesting.
    // Only visible panes get views here; zoom-hidden views stay retained by
    // the prune below (capture in flight).
    app.prune_terminal_views();
    let app_weak = cx.entity().downgrade();
    for pane in &visible {
        let pane_id = pane.id.clone();
        app.pane_entry(&pane_id);
        if !app.terminal_views.contains_key(&pane_id) {
            let term_arc = Arc::clone(
                &app.terminals
                    .get(&pane_id)
                    .expect("entry ensured above")
                    .terminal,
            );
            let weak = app_weak.clone();
            let pid = pane_id.clone();
            let view = cx.new(|cx| TerminalView::new(pid, term_arc, weak, cx));
            app.terminal_views.insert(pane_id.clone(), view);
            // FE initial-capture parity: the blank grid becomes replayed
            // history as soon as the capture reply lands.
            app.request_pane_capture(&pane_id);
        }
    }

    // Render phase (shared borrow only — entities already ensured above).
    let snapshot: Vec<(TmuxPane, crate::pane_geometry::PxRect)> =
        visible.into_iter().zip(rects.into_iter()).collect();
    let shared = &*app;
    let mut grid = div()
        .relative()
        .flex_1()
        .w_full()
        .min_h(px(0.0))
        .bg(rgb(0x1e1e1e))
        .child(render_workspace_probe(cx))
        // Grid-level drag streaming (D3): the divider `mouse_down` arms
        // `AppState::pane_drag`; moves anywhere inside the workspace keep
        // firing on this container (bubble phase over the full-size hitbox),
        // so fast drags past the 4px handle keep streaming steps. Paint stays
        // arm-only — steps send on the owning socket, truth follows via
        // snapshot deltas, and no notify fires until the snapshot lands.
        .on_mouse_move(cx.listener(
            move |this, event: &MouseMoveEvent, _window, _cx| {
                if event.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                if this.pane_drag.is_none() {
                    return;
                }
                let x: f32 = event.position.x.into();
                let y: f32 = event.position.y.into();
                this.stream_pane_drag(x, y);
            },
        ))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |this, _, _, _| {
                this.end_pane_drag();
            }),
        )
        // Releases outside the workspace still end the drag.
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(move |this, _, _, _| {
                this.end_pane_drag();
            }),
        );
    for (pane, rect) in &snapshot {
        grid = grid.child(render_pane_view(shared, pane, *rect, can_zoom, cx));
    }
    for d in &dividers {
        grid = grid.child(render_divider(d, cx));
    }
    grid.into_any_element()
}

/// Divider handle between adjacent panes (D3): `mouse_down` arms the drag
/// state machine on `AppState`; the grid-level `mouse_move` above streams it.
/// Stable `pane-divider/{v|h}-a-b` id (no counters — poll-tick stable),
/// col-resize / row-resize cursor per axis, hover affordance.
fn render_divider(d: &DividerHandle, cx: &mut Context<AppState>) -> impl IntoElement {
    let pane_id = d.pane_id.clone();
    let direction = d.direction;
    let cell_px = if d.is_vertical {
        d.cell_w
    } else {
        d.cell_h
    };
    let is_vertical = d.is_vertical;
    div()
        .id(format!("pane-divider/{}", d.key))
        .absolute()
        .left(px(d.rect.left))
        .top(px(d.rect.top))
        .w(px(d.rect.w.max(0.0)))
        .h(px(d.rect.h.max(0.0)))
        .rounded_full()
        .bg(rgb(0x3c3c3c))
        .opacity(0.4)
        .hover(|s| s.opacity(1.0))
        .when(is_vertical, |s| s.cursor_col_resize())
        .when(!is_vertical, |s| s.cursor_row_resize())
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(
                move |this, event: &MouseDownEvent, _window, _cx| {
                    let pos: f32 = if is_vertical {
                        event.position.x.into()
                    } else {
                        event.position.y.into()
                    };
                    this.begin_pane_drag(&pane_id, direction, pos, cell_px);
                },
            ),
        )
        .into_any_element()
}

/// Full-size canvas probe measuring the grid container into
/// `AppState::workspace_size` (the `TerminalView` measure pattern). Notifies
/// only on change, so paint converges after one extra frame.
fn render_workspace_probe(cx: &mut Context<AppState>) -> impl IntoElement {
    let app_weak = cx.entity().downgrade();
    canvas(
        move |bounds, _, _| bounds,
        move |bounds, _, _, cx| {
            let w: f32 = bounds.size.width.into();
            let h: f32 = bounds.size.height.into();
            let _ = app_weak.update(cx, |app, cx| {
                let next = Some((w, h));
                if app.workspace_size != next {
                    app.workspace_size = next;
                    cx.notify();
                }
            });
        },
    )
    .absolute()
    .size_full()
    .into_any_element()
}
