//! Bounded toast overlay stack (Phase 7 STATE-03, D1/D2 per UI-SPEC TOAST-1).
//!
//! Bottom-right overlay stack above the workspace (below the palette modal in
//! z-order), max 5 visible (drop-oldest), dedupe key per (kind+session).
//! Kinds: error (`command.error` / timeout / `server.error`, backend-verbatim
//! message, autohide 5s), success (`Reconnected` + FE success copy, autohide
//! 3s), info (transient notes, autohide 3s). Timings are UAT-tunable guesses
//! (A1); the 07-03 audit may adjust them.
//!
//! Security (T-07-01): backend strings render as plain text elements only
//! (never markup/commands); every toast carries its kind icon prefix so the
//! origin is visually typed. Flap safety (T-07-02): cap + dedupe + autohide
//! so 250ms-ladder flaps never cover the workspace.

use std::time::Instant;

use gpui::*;

use crate::app_state::{toast_timeout, AppState, Toast, ToastKind};
use crate::icons::{ALERT_TRIANGLE_SVG, CHECK_SVG, REFRESH_CW_SVG};

/// Render the toast overlay stack (empty element when the queue is empty).
///
/// Lazily expires stale toasts on each render and arms a single wake-up
/// notify for the earliest expiry so autohide fires without render spam:
/// the timer only notifies (expiry itself happens at the top of the next
/// render), so duplicate timers are harmless extra frames, never state
/// corruption.
pub fn render_toasts(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    app.expire_toasts(Instant::now());
    if app.toasts.is_empty() {
        return div().into_any_element();
    }

    // Wake up at the earliest expiry while toasts are visible.
    if let Some(min_remaining) = app
        .toasts
        .iter()
        .map(|t| toast_timeout(t.kind).saturating_sub(t.created.elapsed()))
        .min()
    {
        if !min_remaining.is_zero() {
            cx.spawn(move |view_weak: WeakEntity<AppState>, cx: &mut AsyncApp| {
                let cx_handle = cx.clone();
                async move {
                    tokio::time::sleep(min_remaining).await;
                    let _ = cx_handle.update(|cx: &mut App| {
                        if let Some(entity) = view_weak.upgrade() {
                            entity.update(cx, |_, cx| cx.notify());
                        }
                    });
                }
            })
            .detach();
        }
    }

    let preset_name = app.settings.theme_preset.clone();
    let card_bg = crate::theme::preset_card(&preset_name);
    let border_color = crate::theme::preset_border(&preset_name);
    let fg = crate::theme::preset_fg(&preset_name);
    let muted = crate::theme::preset_muted_fg(&preset_name);
    let destructive = crate::theme::preset_destructive(&preset_name);
    // FE-parity accents (no green/amber tokens in the generated presets).
    let success_green = rgb(0x22c55e);
    let info_amber = rgb(0xf59e0b);

    let toasts: Vec<Toast> = app.visible_toasts();
    div()
        .absolute()
        .bottom(px(16.0))
        .right(px(16.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .children(toasts.into_iter().enumerate().map(|(i, toast)| {
            let (icon, icon_color) = match toast.kind {
                ToastKind::Error => (ALERT_TRIANGLE_SVG, destructive),
                ToastKind::Success => (CHECK_SVG, success_green),
                ToastKind::Info => (REFRESH_CW_SVG, info_amber),
            };
            let kind_label = match toast.kind {
                ToastKind::Error => "Error",
                ToastKind::Success => "Success",
                ToastKind::Info => "Note",
            };
            let (kind, session, message) =
                (toast.kind, toast.session.clone(), toast.message.clone());
            div()
                .id(format!("toast/{i}"))
                .flex()
                .flex_row()
                .items_start()
                .gap(px(8.0))
                .w(px(320.0))
                .max_w(px(320.0))
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(8.0))
                .border_1()
                .border_color(border_color)
                .bg(card_bg)
                .child(
                    svg()
                        .data(icon)
                        .size(px(14.0))
                        .mt(px(2.0))
                        .text_color(icon_color),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(muted)
                                // Kind icon prefix types the origin (T-07-01).
                                .child(format!("{kind_label} · {session}")),
                        )
                        // Backend-verbatim message as plain text (T-07-01).
                        .child(div().text_sm().text_color(fg).child(message.clone())),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .cursor_pointer()
                        .hover(|s| s.opacity(0.7))
                        .child("✕")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _window, cx| {
                                this.toasts.retain(|t| {
                                    !(t.kind == kind
                                        && t.session == session
                                        && t.message == message)
                                });
                                cx.notify();
                            }),
                        ),
                )
        }))
        .into_any_element()
}
