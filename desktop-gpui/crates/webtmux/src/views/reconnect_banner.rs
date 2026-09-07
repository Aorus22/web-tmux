//! Reconnect banner (Phase 7 STATE-03, D2 per UI-SPEC BANNER-1).
//!
//! Full-width bar docked directly above the workspace (below the title bar /
//! tab strip), persistent while its state holds (never autohides). Renders
//! the strict 4-way taxonomy from `(transport, origin, attempt)` — NEVER from
//! bare `TransportState` (that reintroduces the pump conflation).
//!
//! Copy table (fixed strings, tmux- prefix marks server-originated states):
//! - WS-drop (`transport.lost`): `"Connection lost — retrying… (attempt N)"`
//!   + `[Reconnect]` (auto-retry armed).
//! - `tmux.reconnecting`: `"Reconnecting to tmux…"` (amber, transient, no
//!   button — the backend monitor owns that retry).
//! - `tmux.disconnected`: `"tmux disconnected — waiting for tmux"` +
//!   `[Reconnect]` (manual only, no client retry).
//! - Connected/recovered: banner unmounts + `"Reconnected"` toast fires
//!   (see `apply_event`).
//!
//! `[Reconnect]` = manual close+ensure path (FE `reconnectSession` parity).

use gpui::*;

use webtmux_backend_client::TransportState;

use crate::app_state::{banner_copy_for, AppState, TransportOrigin};
use crate::icons::{ALERT_TRIANGLE_SVG, REFRESH_CW_SVG};

/// Render the reconnect banner for the active session, or an empty element
/// when no banner state holds (connected / connecting-fresh / no tab).
pub fn render_reconnect_banner(
    app: &mut AppState,
    cx: &mut Context<AppState>,
) -> impl IntoElement {
    let preset_name = app.settings.theme_preset.clone();
    let card_bg = crate::theme::preset_card(&preset_name);
    let border_color = crate::theme::preset_border(&preset_name);
    let fg = crate::theme::preset_fg(&preset_name);
    let destructive = crate::theme::preset_destructive(&preset_name);
    let primary_bg = crate::theme::preset_primary(&preset_name);
    let primary_fg = crate::theme::preset_primary_fg(&preset_name);
    // FE `text-amber-500` verbatim for the transient reconnecting state (the
    // generated presets carry no amber token).
    let amber = rgb(0xf59e0b);

    let Some(active) = app.active_session.clone() else {
        return div().into_any_element();
    };
    let Some(entry) = app.sessions.get(&active) else {
        return div().into_any_element();
    };
    let (transport, origin, attempt) =
        (entry.transport, entry.transport_origin, entry.reconnect_attempt);
    let Some((title, show_button)) = banner_copy_for(transport, origin, attempt) else {
        return div().into_any_element();
    };

    let is_reconnecting = transport == TransportState::Reconnecting;
    let icon_color = if is_reconnecting { amber } else { destructive };
    let (icon, icon_size) = if is_reconnecting {
        (REFRESH_CW_SVG, px(14.0))
    } else {
        (ALERT_TRIANGLE_SVG, px(14.0))
    };
    // Silence the unused-variant warning for future origins: the banner key
    // below already distinguishes Local vs Server via the title text.
    let _ = TransportOrigin::None;

    let mut bar = div()
        .flex()
        .flex_row()
        .items_center()
        .flex_none()
        .w_full()
        .px(px(12.0))
        .py(px(6.0))
        .gap(px(8.0))
        .bg(card_bg)
        .border_b_1()
        .border_color(border_color)
        .child(
            svg()
                .data(icon)
                .size(icon_size)
                .text_color(icon_color),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg)
                // Backend-verbatim/fixed copy as plain text only (T-07-01).
                .child(title),
        );

    if show_button {
        let session = active.clone();
        bar = bar.child(
            div()
                .id("reconnect-banner/reconnect")
                .px(px(12.0))
                .py(px(4.0))
                .rounded(px(6.0))
                .bg(primary_bg)
                .text_color(primary_fg)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .cursor_pointer()
                .hover(|s| s.opacity(0.9))
                .child("Reconnect")
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _window, cx| {
                        this.manual_reconnect(&session, cx);
                        cx.notify();
                    }),
                ),
        );
    }

    bar.into_any_element()
}
