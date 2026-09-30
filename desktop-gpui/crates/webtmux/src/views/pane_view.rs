//! Single pane: h-7 header + re-hosted Phase-4 `TerminalView` (Phase 5).
//!
//! FE parity (`PaneView.tsx`, `PaneHeader.tsx:41-167`): `currentPath` +
//! mono pane id, TUI-scroll switch bound to the `tui_scroll` map (default
//! ON), Split right (`horizontal`) / Split down (`vertical`) / Zoom (hidden
//! when single-pane, `canZoom = otherPanes.len() > 0`) / Kill actions, header
//! double-click zooms, active pane border, `!pane.active` click sends
//! `pane.select`. Body re-hosts the Phase-4 `TerminalView` unchanged.
//!
//! The pane HEADER (`pane-header/%N`) carries the right-click menu
//! (`pane_context_menu`: Split right/down, Rename, Zoom, Swap picker, Break,
//! Kill through the `confirm_kill_pane` gate); the terminal body owns the
//! terminal's Copy/Cut/Paste menu, so the two never stack. Header Kill routes
//! through the same gate.
//!
//! Tooltip fallback (D10/A3, verified): `gpui-pre =0.3.3` exposes no tooltip
//! API on raw divs and `gpui-component =0.6.0`'s tooltip applicator
//! (`ManagedTooltipExt`) is crate-internal — only its own themed components
//! get `.tooltip()`. Header actions therefore carry stable ids
//! (`pane-split-right/%N`, …) plus hover affordance; hover tooltips land with
//! the Phase-7 parity audit.

use gpui::*;
use gpui::prelude::{FluentBuilder, InteractiveElement};
use webtmux_backend_client::TmuxPane;

use crate::app_state::AppState;
use crate::icons::{
    MAXIMIZE2_SVG, SPLIT_SQUARE_HORIZONTAL_SVG, SPLIT_SQUARE_VERTICAL_SVG, X_SVG,
};
use crate::pane_geometry::PxRect;
use crate::views::pane_context_menu::with_pane_context_menu;

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

    // Phase 6: pane chrome follows the active UI preset (not hardcoded
    // dark) so panes match light themes too. GTK mode is already folded into
    // the preset accessors, so this stays one source of truth.
    let preset_name = app.settings.theme_preset.clone();
    let pane_border = crate::theme::preset_border(&preset_name);
    let border_color = if is_active {
        // Active pane edge: the theme border lifted towards the foreground.
        crate::theme::mix(pane_border, crate::theme::preset_fg(&preset_name), 0.15)
    } else {
        pane_border
    };
    let pane_bg = crate::theme::preset_bg(&preset_name);
    let header_bg = if is_active {
        crate::theme::preset_muted(&preset_name)
    } else {
        crate::theme::preset_card(&preset_name)
    };
    let muted_text = crate::theme::preset_muted_fg(&preset_name);
    let hover_bg = crate::theme::preset_muted(&preset_name);

    let pid_select = pane_id.clone();
    let pid_zoom = pane_id.clone();

    // Same-window swap targets for the right-click picker (pitfall 6).
    let candidates = app.swap_candidates(&pane.id);
    let app_weak = cx.entity().downgrade();
    let menu_pane_id = pane.id.clone();

    // Terminal host: the store-owned view re-hosted unchanged (Phase 4
    // ownership — views never create terminals). The hosted terminal carries
    // its own Copy/Cut/Paste menu, so only the pre-terminal placeholder borrows
    // the pane menu here.
    let body = match app.terminal_views.get(&pane.id) {
        Some(view) => div()
            .relative()
            .min_h(px(0.0))
            .min_w(px(0.0))
            .flex_1()
            .size_full()
            .child(view.clone())
            .into_any_element(),
        None => with_pane_context_menu(
            div()
                .id(format!("pane-body/{}", pane.id))
                .flex()
                .items_center()
                .justify_center()
                .flex_1()
                .size_full()
                .text_xs()
                .text_color(muted_text)
                .child("Connecting..."),
            pane.id.clone(),
            candidates.clone(),
            app_weak.clone(),
        )
        .into_any_element(),
    };

    // Pane actions (Split / Rename / Zoom / Break / Swap / Kill) hang off the
    // pane HEADER's right-click, not the whole pane: the terminal body owns its
    // own Copy/Cut/Paste menu, and two nested context menus would both open on
    // one right-click (both are window-level listeners keyed on hitbox hover).
    // Every pane action is also reachable from the header buttons.
    let header = div()
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
                    crate::theme::preset_fg(&preset_name),
                    hover_bg,
                    muted_text,
                    pane_id.clone(),
                    |this, pid, _window, cx| {
                        this.submit_pane_split(pid, "horizontal", cx);
                    },
                    cx,
                ))
                .child(header_action_button(
                    format!("pane-split-down/{}", pane.id),
                    SPLIT_SQUARE_VERTICAL_SVG,
                    crate::theme::preset_fg(&preset_name),
                    hover_bg,
                    muted_text,
                    pane_id.clone(),
                    |this, pid, _window, cx| {
                        this.submit_pane_split(pid, "vertical", cx);
                    },
                    cx,
                ))
                .when(can_zoom, |s| {
                    s.child(header_action_button(
                        format!("pane-zoom/{}", pane.id),
                        MAXIMIZE2_SVG,
                        crate::theme::preset_fg(&preset_name),
                        hover_bg,
                        muted_text,
                        pane_id.clone(),
                        |this, pid, _window, cx| {
                            this.submit_pane_zoom(pid, cx);
                        },
                        cx,
                    ))
                })
                .child(header_action_button(
                    format!("pane-kill/{}", pane.id),
                    X_SVG,
                    crate::theme::preset_danger_text(&preset_name),
                    hover_bg,
                    muted_text,
                    pane_id.clone(),
                    |_this, pid, window, cx| {
                        // Kill-confirm gate (PANE-05 per D7): dialog
                        // when `confirm_kill_pane`, direct kill else.
                        // Lease-safe variant: this runs inside an
                        // AppState update.
                        crate::views::pane_context_menu::request_kill_pane_from_state(
                            _this, pid, window, cx,
                        );
                    },
                    cx,
                )),
        );

    let header = with_pane_context_menu(header, menu_pane_id, candidates, app_weak);

    let root = div()
        // Positioned root + element state. The context menu sits on the header
        // above, so a right-click in the terminal body reaches the terminal's
        // own menu instead of stacking two popups.
        .id(format!("pane-menu/{}", pane.id))
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
        .bg(pane_bg)
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
        .child(header)
        .child(body);

    root.into_any_element()
}

/// Small square header action button (20px box, 12px icon). Free function —
/// not a closure — so each call reborrows `cx` independently. The click
/// handler receives the window so gated actions (kill-confirm) can open
/// dialogs.
fn header_action_button(
    id: String,
    icon: &'static [u8],
    hover_fg: Rgba,
    hover_bg: Rgba,
    icon_color: Rgba,
    pid: String,
    on_click: fn(&mut AppState, &str, &mut Window, &mut Context<AppState>),
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
        .text_color(crate::theme::muted_fg())
        .hover(move |s| s.bg(hover_bg).text_color(hover_fg))
        .child(svg().data(icon).size(px(12.0)).text_color(icon_color))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, window, cx| {
                on_click(this, &pid, window, cx);
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
    let preset_name = app.settings.theme_preset.clone();
    let muted_text = crate::theme::preset_muted_fg(&preset_name);
    let foreground_text = crate::theme::preset_fg(&preset_name);
    let pid = pane_id.to_string();
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
