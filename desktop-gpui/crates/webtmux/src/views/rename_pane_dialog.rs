//! DLG1 Rename Pane Dialog (PANE-05 + DLG-02 per D6).
//!
//! Modal rename built on `gpui-component` Dialog + InputState, cloning the
//! DLG1 lifetime from `rename_session_dialog.rs`: a fresh `RenamePaneForm`
//! entity per open, `None` on dismiss, `is_submitting` double-submit guard
//! (T-03-07), inline `#7F1D1D` error.
//!
//! Prefill is `title || current_command || ''` (FE `PaneContextMenu.tsx:131`
//! parity); the gate is trim + non-empty only (no session-name validation —
//! tmux titles accept anything). Submit sends correlated `pane.rename`
//! (`title` travels as a JSON field into argv — no shell, T-05-04) with
//! dialog feedback: success clears the form + closes, `command.error`/
//! timeout renders inline with the dialog open.

use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::{
    dialog::{DialogFooter, DialogTitle},
    input::{Input, InputEvent, InputState},
    WindowExt as _,
};

use crate::app_state::AppState;

/// Modal form state entity for the rename dialog. Owned by `AppState` for the
/// lifetime of the open dialog; replaced (and dropped, releasing its
/// InputState/subscription) on every open, which guarantees a fresh prefill
/// on re-open.
pub struct RenamePaneForm {
    /// Pane being renamed (`%N`).
    pub target: String,
    pub name_input: Entity<InputState>,
    pub is_submitting: bool,
    pub error_message: Option<String>,
    app: WeakEntity<AppState>,
    window_handle: AnyWindowHandle,
    _subscriptions: Vec<Subscription>,
}

/// Open the rename dialog for `target`. Guards against double-open; a fresh
/// (prefilled) form entity is stored on `AppState` each time the dialog opens.
pub fn open_rename_pane_dialog(
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
    let target_id = target.to_string();
    let window_handle = window.window_handle();

    let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("New name"));

    // Prefill with `title || current_command || ''` + autofocus (FE parity).
    let prefill = app_entity
        .read(cx)
        .pane_for_rename(target)
        .map(|(title, cmd)| AppState::pane_rename_prefill(&title, &cmd))
        .unwrap_or_default();
    name_input.update(cx, |state, cx| {
        state.set_value(prefill, window, cx);
        state.focus(window, cx);
    });

    let form = cx.new(|_cx| RenamePaneForm {
        target: target_id,
        name_input: name_input.clone(),
        is_submitting: false,
        error_message: None,
        app: app_weak.clone(),
        window_handle,
        _subscriptions: Vec::new(),
    });

    // Pressing Enter submits the form (FE parity).
    let sub = {
        let form_weak = form.downgrade();
        cx.subscribe(&name_input, move |_: Entity<InputState>, event, cx| match event {
            InputEvent::PressEnter { .. } => request_rename_pane_submit(&form_weak, cx),
            _ => {}
        })
    };
    form.update(cx, |f, _cx| {
        f._subscriptions = vec![sub];
    });

    // Reset-on-dismiss: clear the AppState-held form so Escape/backdrop/X leave
    // no stale state; the old entity (and its InputState) is dropped.
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
                    .child("Rename Pane"),
            )
            .child(render_rename_pane_body(&build_form, cx))
            .footer(render_rename_pane_footer(&build_form, cx))
            .on_close({
                // Fresh clone per build invocation (the closure re-runs per frame).
                let on_close_app = on_close_app.clone();
                move |_, _, cx| {
                    if let Some(app) = on_close_app.upgrade() {
                        app.update(cx, |this, cx| {
                            this.rename_pane_form = None;
                            cx.notify();
                        });
                    }
                }
            })
    });

    // Keep the form entity + subscriptions alive across the dialog lifetime.
    app_entity.update(cx, |app, cx| {
        app.rename_pane_form = Some(form);
        cx.notify();
    });
}

// ---------------------------------------------------------------------------
// Dialog composition
// ---------------------------------------------------------------------------

/// Dialog body: the prefilled name field, the inline error line, and the
/// in-flight hint while submitting.
fn render_rename_pane_body(form: &Entity<RenamePaneForm>, cx: &mut App) -> impl IntoElement {
    let muted_text = rgb(0x808080);
    let foreground_text = rgb(0xd4d4d4);

    let (target, is_submitting, error) = {
        let f = form.read(cx);
        (f.target.clone(), f.is_submitting, f.error_message.clone())
    };

    let mut body = div().flex().flex_col().gap(px(12.0)).w_full();

    body = body.child(
        div()
            .text_xs()
            .text_color(muted_text)
            .child(format!("Rename pane \"{target}\" to a new title.")),
    );

    // New-name field (required, prefilled, gated trim + non-empty only).
    let name_input = form.read(cx).name_input.clone();
    body = body.child(
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(foreground_text)
                    .child("New name"),
            )
            .child(Input::new(&name_input).w_full()),
    );

    // Inline destructive error line (command.error / timeout).
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
                .child("Renaming pane..."),
        );
    }

    body
}

/// Footer: Cancel (outline) and Rename (primary `#d4d4d4` fill). Rename is
/// disabled while the name is empty/trim-empty or the request is in flight.
fn render_rename_pane_footer(form: &Entity<RenamePaneForm>, cx: &mut App) -> DialogFooter {
    let foreground_text = rgb(0xd4d4d4);
    let border_color = rgb(0x3c3c3c);
    let hover_bg = rgb(0x262626);

    let state = form.read(cx);
    let name_empty = state.name_input.read(cx).value().trim().is_empty();
    let is_submitting = state.is_submitting;

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
                .bg(rgb(0xd4d4d4))
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(0x1e1e1e))
                .when(is_submitting || name_empty, |s| {
                    s.opacity(0.4).cursor_default()
                })
                .when(!is_submitting && !name_empty, |s| {
                    s.cursor_pointer()
                        .hover(|h| h.opacity(0.9))
                        .on_mouse_down(MouseButton::Left, {
                            let form_weak = form.downgrade();
                            move |_, _, cx| {
                                request_rename_pane_submit(&form_weak, cx);
                            }
                        })
                })
                .child("Rename"),
        )
}

/// Trim + non-empty gate, then a correlated `pane.rename` with dialog
/// feedback via `AppState::await_command_feedback`.
///
/// Strict double-submit guard: the form's `is_submitting` flag is checked
/// synchronously, so a second click or Enter while in-flight cannot fire a
/// second rename (T-03-07).
pub fn request_rename_pane_submit(form: &WeakEntity<RenamePaneForm>, cx: &mut App) {
    // Owned weak handle for the AppState handoff below.
    let form_weak = form.clone();
    let Some(form_entity) = form_weak.upgrade() else { return };

    let state = form_entity.read(cx);
    if state.is_submitting {
        return; // Throttle guard T-03-07 (short-circuit, no state change).
    }

    let target = state.target.clone();
    let raw = state.name_input.read(cx).value().trim().to_string();
    let app = state.app.clone();
    let window_handle = state.window_handle;

    if !AppState::rename_name_allowed(&raw) {
        return; // Rename button disabled in this state; ignore the event.
    }

    form_entity.update(cx, |f, cx| {
        f.is_submitting = true;
        f.error_message = None;
        cx.notify();
    });

    let Some(app_entity) = app.upgrade() else {
        form_entity.update(cx, |f, cx| {
            f.is_submitting = false;
            f.error_message = Some("Application state is gone.".to_string());
            cx.notify();
        });
        return;
    };

    let title = raw.trim().to_string();
    let msg = AppState::build_pane_rename(&target, &title);
    let (session, rx) = match app_entity.read(cx).try_send_pane_command(&target, msg) {
        Ok(v) => v,
        Err(e) => {
            let message = e.clone();
            app_entity.update(cx, |this, cx| {
                if let Some(sess) = this.owning_session(&target) {
                    this.note_session_error(&sess, e);
                }
                cx.notify();
            });
            form_entity.update(cx, |f, cx| {
                f.is_submitting = false;
                f.error_message = Some(message);
                cx.notify();
            });
            return;
        }
    };

    let form_weak = form.clone();
    let session_err = session.clone();
    app_entity.update(cx, |this, cx| {
        this.await_command_feedback(
            rx,
            move |this, outcome, cx| {
                match outcome {
                    Ok(()) => {
                        this.rename_pane_form = None;
                        let _ = window_handle.update(cx, |_, window, cx| {
                            window.close_dialog(cx);
                        });
                    }
                    Err(e) => {
                        this.note_session_error(&session_err, e.clone());
                        if let Some(f) = form_weak.upgrade() {
                            f.update(cx, |f, cx| {
                                f.is_submitting = false;
                                f.error_message = Some(e);
                                cx.notify();
                            });
                        }
                    }
                }
                cx.notify();
            },
            cx,
        );
    });
}
