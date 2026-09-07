//! Single pane: h-7 header + re-hosted Phase-4 `TerminalView` (Phase 5 tracer).
//!
//! FE parity (`PaneView.tsx`, `PaneHeader.tsx:41-167`): `currentPath` +
//! mono pane id, TUI-scroll switch bound to the `tui_scroll` map (default
//! ON), Split right (`horizontal`) / Split down (`vertical`) / Zoom (hidden
//! when single-pane, `canZoom = otherPanes.len() > 0`) / Kill actions, header
//! double-click zooms, active pane border, `!pane.active` click sends
//! `pane.select`. Body re-hosts the Phase-4 `TerminalView` unchanged.
//!
//! Tooltip fallback (D10/A3, verified): `gpui-pre =0.3.3` exposes no tooltip
//! API on raw divs and `gpui-component =0.6.0`'s tooltip applicator
//! (`ManagedTooltipExt`) is crate-internal — only its own themed components
//! get `.tooltip()`. Header actions therefore carry stable ids
//! (`pane-split-right/%N`, …) plus hover affordance; hover tooltips land with
//! the 05-02 menu wave / Phase-7 parity audit.

use gpui::*;
use gpui::prelude::{FluentBuilder, InteractiveElement};
use webtmux_backend_client::TmuxPane;

use crate::app_state::AppState;
use crate::icons::{
    MAXIMIZE2_SVG, SPLIT_SQUARE_HORIZONTAL_SVG, SPLIT_SQUARE_VERTICAL_SVG, X_SVG,
};
use crate::pane_geometry::PxRect;

/// Render one positioned pane at its `pixel_rect` rect.
pub fn render_pane_view(
    app: &AppState,
    pane: &TmuxPane,
    rect: PxRect,
    can_zoom: bool,
    cx: &mut Context<AppState>,
) -> impl IntoElement {
    let pane_id = pane.id.clone();
    let is_active = pane.active;
    let tui_on = app.tui_scroll(&pane.id);

    let border_color = if is_active {
        rgb(0x5a5a5a)
    } else {
        rgb(0x3c3c3c)
    };
    let header_bg = if is_active {
        rgb(0x2d2d2d)
    } else {
        rgb(0x242424)
    };
    let muted_text = rgb(0x808080);

    // Terminal host: the store-owned view re-hosted unchanged (Phase 4
    // ownership — views never create terminals).
    let body = match app.terminal_views.get(&pane.id) {
        Some(view) => div()
            .relative()
            .min_h(px(0.0))
            .min_w(px(0.0))
            .flex_1()
            .size_full()
            .child(view.clone())
            .into_any_element(),
        None => div()
            .flex()
            .items_center()
            .justify_center()
            .flex_1()
            .size_full()
            .text_xs()
            .text_color(muted_text)
            .child("Connecting...")
            .into_any_element(),
    };

    let pid_select = pane_id.clone();
    let pid_zoom = pane_id.clone();

    div()
        .absolute()
        .left(px(rect.left))
        .top(px(rect.top))
        .w(px(rect.w.max(0.0)))
        .h(px(rect.h.max(0.0)))
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(px(4.0))
        .border_1()
        .border_color(border_color)
        .bg(rgb(0x1e1e1e))
        // Inactive click selects (FE `PaneView.tsx:57-59`); header-button
        // clicks bubbling here only add a harmless select alongside the action.
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                if !is_active {
                    this.send_pane_select(&pid_select);
                    cx.notify();
                }
            }),
        )
        .child(
            div()
                .id(format!("pane-header/{}", pane.id))
                .flex()
                .flex_row()
                .items_center()
                .flex_none()
                .h(px(28.0))
                .gap(px(6.0))
                .px(px(8.0))
                .border_b_1()
                .border_color(border_color)
                .bg(header_bg)
                // Header double-click zooms (FE `PaneHeader.tsx:67`).
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        if event.click_count >= 2 {
                            this.submit_pane_zoom(&pid_zoom, cx);
                        }
                    }),
                )
                .child(
                    div()
                        .flex_1()
                        .truncate()
                        .text_xs()
                        .text_color(muted_text)
                        .child(pane.current_path.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .font_family("JetBrains Mono")
                        .text_color(muted_text)
                        .child(pane.id.clone()),
                )
                .child(render_tui_switch(app, &pane.id, tui_on, cx))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_shrink_0()
                        .items_center()
                        .gap(px(2.0))
                        .child(header_action_button(
                            format!("pane-split-right/{}", pane.id),
                            SPLIT_SQUARE_HORIZONTAL_SVG,
                            0xd4d4d4,
                            pane_id.clone(),
                            |this, pid, cx| {
                                this.submit_pane_split(pid, "horizontal", cx);
                            },
                            cx,
                        ))
                        .child(header_action_button(
                            format!("pane-split-down/{}", pane.id),
                            SPLIT_SQUARE_VERTICAL_SVG,
                            0xd4d4d4,
                            pane_id.clone(),
                            |this, pid, cx| {
                                this.submit_pane_split(pid, "vertical", cx);
                            },
                            cx,
                        ))
                        .when(can_zoom, |s| {
                            s.child(header_action_button(
                                format!("pane-zoom/{}", pane.id),
                                MAXIMIZE2_SVG,
                                0xd4d4d4,
                                pane_id.clone(),
                                |this, pid, cx| {
                                    this.submit_pane_zoom(pid, cx);
                                },
                                cx,
                            ))
                        })
                        .child(header_action_button(
                            format!("pane-kill/{}", pane.id),
                            X_SVG,
                            0xf87171,
                            pane_id.clone(),
                            |this, pid, cx| {
                                // Tracer direct kill; the confirm dialog
                                // honoring `kill_requires_confirm_pane`
                                // lands with the 05-02 menu wave.
                                this.submit_pane_kill(pid, cx);
                            },
                            cx,
                        )),
                )
                .into_any_element(),
        )
        .child(body)
        .into_any_element()
}

/// Small square header action button (20px box, 12px icon). Free function —
/// not a closure — so each call reborrows `cx` independently.
fn header_action_button(
    id: String,
    icon: &'static [u8],
    hover_fg: u32,
    pid: String,
    on_click: fn(&mut AppState, &str, &mut Context<AppState>),
    cx: &mut Context<AppState>,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(20.0))
        .h(px(20.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .text_color(rgb(0x808080))
        .hover(move |s| s.bg(rgb(0x2d2d2d)).text_color(rgb(hover_fg)))
        .child(svg().data(icon).size(px(12.0)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                on_click(this, &pid, cx);
            }),
        )
        .into_any_element()
}

/// TUI-scroll switch bound to the `AppState::tui_scroll` map (default ON).
/// Tracer uses a compact text toggle; the full Switch control arrives with
/// Phase-6 settings (D7 — in-memory map only in Phase 5).
fn render_tui_switch(
    app: &AppState,
    pane_id: &str,
    tui_on: bool,
    cx: &mut Context<AppState>,
) -> impl IntoElement {
    let muted_text = rgb(0x808080);
    let foreground_text = rgb(0xd4d4d4);
    let pid = pane_id.to_string();
    let _ = app;
    div()
        .id(format!("pane-tui/{}", pane_id))
        .flex()
        .flex_row()
        .flex_shrink_0()
        .items_center()
        .gap(px(4.0))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                let cur = this.tui_scroll(&pid);
                this.set_tui_scroll(&pid, !cur);
                cx.notify();
            }),
        )
        .child(
            div()
                .text_xs()
                .text_color(muted_text)
                .child("TUI"),
        )
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(foreground_text)
                .child(if tui_on { "On" } else { "Off" }),
        )
        .into_any_element()
}
