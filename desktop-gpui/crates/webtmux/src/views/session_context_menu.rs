//! Sidebar session context menu + kill-confirm dialog (SESS-04, SESS-05).
//!
//! Rows are wrapped in `gpui-component` `ContextMenuExt::context_menu` with a
//! STABLE per-session id (poll-tick state-loss pitfall — the id never embeds
//! counters or indices). Menu = `Rename` + separator + `Kill Session`
//! (destructive red text) only — no dead Phase-5 entries. Left-click select
//! stays `MouseButton::Left`-only, so right-click never conflicts.
//!
//! Kill honors `DesktopSettings.confirm_kill_session` (D2): confirm dialog
//! with DLG1 tokens and a destructive `#7F1D1D` Kill button, or a direct kill.
//! Both ride `AppState::execute_kill` (D6 transport order); correlated success
//! closes the socket + tab with neighbor activation + poll, errors render
//! inline destructive with the dialog open and the tab untouched.

use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::{
    dialog::{DialogFooter, DialogTitle},
    menu::{ContextMenuExt, PopupMenuItem},
    WindowExt as _,
};

use crate::app_state::AppState;
use crate::views::rename_session_dialog::open_rename_session_dialog;

/// Wrap a sidebar session row in its right-click menu.
///
/// `row` is the already-composed row element carrying a STABLE per-session id
/// (poll-tick state-loss pitfall — never embed counters or indices in it);
/// `ContextMenuExt` derives its open-state id from that element id.
/// `app_weak` routes menu actions back to `AppState` (dialogs need the entity
/// + window at click time).
pub fn with_session_context_menu(
    row: impl InteractiveElement + ParentElement + Styled + IntoElement + 'static,
    session_name: String,
    app_weak: WeakEntity<AppState>,
) -> impl IntoElement {
    row.context_menu(move |menu, _window, _cx| {
            let rename_weak = app_weak.clone();
            let rename_target = session_name.clone();
            let kill_weak = app_weak.clone();
            let kill_target = session_name.clone();
            menu.item(PopupMenuItem::new("Rename").on_click(
                move |_, window, cx| {
                    if let Some(app) = rename_weak.upgrade() {
                        open_rename_session_dialog(&app, &rename_target, window, cx);
                    }
                },
            ))
            .separator()
            .item(
                PopupMenuItem::element(|_, _| {
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(rgb(0xf87171))
                        .child("Kill Session")
                })
                .on_click(move |_, window, cx| {
                    if let Some(app) = kill_weak.upgrade() {
                        request_kill_session(&app, &kill_target, window, cx);
                    }
                }),
            )
        })
}

/// Kill entry from the context menu: confirm dialog when
/// `confirm_kill_session` says so, direct kill otherwise.
pub fn request_kill_session(
    app_entity: &Entity<AppState>,
    target: &str,
    window: &mut Window,
    cx: &mut App,
) {
    if app_entity.read(cx).kill_requires_confirm() {
        open_kill_confirm_dialog(app_entity, target, window, cx);
    } else {
        app_entity.update(cx, |this, cx| {
            this.execute_kill(target, None, cx);
        });
    }
}

// ---------------------------------------------------------------------------
// Kill-confirm dialog (DLG1 tokens, destructive Kill)
// ---------------------------------------------------------------------------

/// Modal form state entity for the kill-confirm dialog. Owned by `AppState`
/// for the lifetime of the open dialog; replaced on every open, `None` on
/// dismiss. No text input — a plain confirm with an `is_submitting` guard
/// (T-03-07) and an inline error line.
pub struct KillSessionForm {
    /// Session awaiting the kill decision.
    pub target: String,
    pub is_submitting: bool,
    pub error_message: Option<String>,
    pub app: WeakEntity<AppState>,
    pub window_handle: AnyWindowHandle,
}

/// Open the kill-confirm dialog for `target`. Guards against double-open.
pub fn open_kill_confirm_dialog(
    app_entity: &Entity<AppState>,
    target: &str,
    window: &mut Window,
    cx: &mut App,
) {
    // No stacked dialogs: a second menu click must not open a second dialog.
    if window.has_active_dialog(cx) {
        return;
    }

    let app_weak = app_entity.downgrade();
    let target_name = target.to_string();
    let window_handle = window.window_handle();

    let form = cx.new(|_cx| KillSessionForm {
        target: target_name,
        is_submitting: false,
        error_message: None,
        app: app_weak.clone(),
        window_handle,
    });

    // Reset-on-dismiss: clear the AppState-held form so Escape/backdrop/X leave
    // no stale state.
    let on_close_app = app_weak;
    let build_form = form.clone();
    window.open_dialog(cx, move |dialog, _window, cx| {
        dialog
            .w(px(440.0))
            .p(px(20.0))
            .rounded(px(8.0))
            .bg(rgb(0x1e1e1e))
            .border_color(rgb(0x3c3c3c))
            .border_1()
            .title(
                DialogTitle::new()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(0xd4d4d4))
                    .child(format!("Kill session \"{}\"?", build_form.read(cx).target)),
            )
            .child(render_kill_body(&build_form, cx))
            .footer(render_kill_footer(&build_form, cx))
            .on_close({
                // Fresh clone per build invocation (the closure re-runs per frame).
                let on_close_app = on_close_app.clone();
                move |_, _, cx| {
                    if let Some(app) = on_close_app.upgrade() {
                        app.update(cx, |this, cx| {
                            this.kill_session_form = None;
                            cx.notify();
                        });
                    }
                }
            })
    });

    // Keep the form entity alive across the dialog lifetime.
    app_entity.update(cx, |app, cx| {
        app.kill_session_form = Some(form);
        cx.notify();
    });
}

/// Dialog body: the destructive description, the inline error line, and the
/// in-flight hint while the correlated kill is outstanding.
fn render_kill_body(form: &Entity<KillSessionForm>, cx: &mut App) -> impl IntoElement {
    let muted_text = rgb(0x808080);

    let (is_submitting, error) = {
        let f = form.read(cx);
        (f.is_submitting, f.error_message.clone())
    };

    let mut body = div().flex().flex_col().gap(px(12.0)).w_full();

    body = body.child(
        div()
            .text_xs()
            .text_color(muted_text)
            .child("This terminates the tmux session and all processes inside it."),
    );

    // Inline destructive error line (command.error / timeout — dialog stays open).
    if let Some(error) = &error {
        body = body.child(
            div()
                .rounded(px(4.0))
                .bg(rgb(0x7F1D1D))
                .px(px(8.0))
                .py(px(6.0))
                .text_xs()
                .text_color(rgb(0xffffff))
                .child(error.clone()),
        );
    }

    if is_submitting {
        body = body.child(
            div()
                .text_xs()
                .text_color(muted_text)
                .child("Killing session..."),
        );
    }

    body
}

/// Footer: Cancel (outline) and Kill (destructive `#7F1D1D` fill, white text —
/// matches the S1 close-button hover). Kill is disabled while a request is in
/// flight.
fn render_kill_footer(form: &Entity<KillSessionForm>, cx: &mut App) -> DialogFooter {
    let foreground_text = rgb(0xd4d4d4);
    let border_color = rgb(0x3c3c3c);
    let hover_bg = rgb(0x262626);

    let is_submitting = form.read(cx).is_submitting;

    DialogFooter::new()
        .child(
            div()
                .px(px(14.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(border_color)
                .cursor_pointer()
                .hover(|s| s.bg(hover_bg))
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(foreground_text)
                .child("Cancel")
                .on_mouse_down(MouseButton::Left, {
                    let busy_form = form.clone();
                    move |_, window, cx| {
                        if busy_form.read(cx).is_submitting {
                            return; // cancellable only when not busy (T-03-07)
                        }
                        window.close_dialog(cx);
                    }
                }),
        )
        .child(
            div()
                .px(px(14.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .bg(rgb(0x7F1D1D))
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(0xffffff))
                .when(is_submitting, |s| s.opacity(0.4).cursor_default())
                .when(!is_submitting, |s| {
                    s.cursor_pointer()
                        .hover(|h| h.opacity(0.9))
                        .on_mouse_down(MouseButton::Left, {
                            let form_weak = form.downgrade();
                            move |_, _, cx| {
                                request_kill_submit(&form_weak, cx);
                            }
                        })
                })
                .child("Kill"),
        )
}

/// Fire the correlated kill via `AppState::execute_kill` (D6 transport order).
///
/// Strict double-submit guard: the form's `is_submitting` flag is checked
/// synchronously, so a second click while in-flight cannot fire a second kill
/// (T-03-07).
pub fn request_kill_submit(form: &WeakEntity<KillSessionForm>, cx: &mut App) {
    // Owned weak handle for the AppState handoff below.
    let form_weak = form.clone();
    let Some(form_entity) = form_weak.upgrade() else { return };

    let state = form_entity.read(cx);
    if state.is_submitting {
        return; // Throttle guard T-03-07 (short-circuit, no state change).
    }

    let target = state.target.clone();
    let app = state.app.clone();
    let window_handle = state.window_handle;

    form_entity.update(cx, |f, cx| {
        f.is_submitting = true;
        f.error_message = None;
        cx.notify();
    });

    let Some(app_entity) = app.upgrade() else {
        form_entity.update(cx, |f, cx| {
            f.is_submitting = false;
            cx.notify();
        });
        return;
    };

    app_entity.update(cx, |this, cx| {
        this.execute_kill(&target, Some(window_handle), cx);
    });
}
