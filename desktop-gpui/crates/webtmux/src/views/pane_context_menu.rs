//! Pane context menu + kill-confirm dialog (PANE-05 per D7).
//!
//! Rows copy the FE item list verbatim (`PaneContextMenu.tsx:122-147`):
//! Split Right / Split Down / Rename Pane / separator / Zoom / Swap
//! (same-window picker submenu, disabled when empty) / Break To Window /
//! separator / Kill (destructive red). Wrapped rows carry the stable
//! `pane-menu/%N` id (poll-tick state-loss pitfall — never embed counters or
//! indices); `ContextMenuExt` derives its open-state id from that element id.
//! Left-click select stays `MouseButton::Left`-only, so right-click never
//! conflicts.
//!
//! `pane.join` is ABSENT by design: no `pane.join` WS route exists
//! server-side (`protocol.go`, `handler.go`) and no Electron surface renders
//! it (zero grep hits across `fe/src`) — D8 descopes it to a logged backend
//! gap instead of inventing a message type.
//!
//! Kill honors `DesktopSettings.confirm_kill_pane` (D7): confirm dialog with
//! DLG1 tokens and a destructive `#7F1D1D` Kill button, or a direct kill.
//! Both ride correlated `pane.kill` with dialog/session feedback.

use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::{
    dialog::{DialogFooter, DialogTitle},
    menu::{ContextMenuExt, PopupMenuItem},
    WindowExt as _,
};

use crate::app_state::{AppState, SwapCandidate};
use crate::views::rename_pane_dialog::open_rename_pane_dialog;

/// Wrap a pane root in its right-click menu.
///
/// `row` is the already-composed pane element carrying the stable
/// `pane-menu/%N` id; `ContextMenuExt` derives its open-state id from that
/// element id. `candidates` are the same-window swap targets with FE-verbatim
/// labels; `app_weak` routes menu actions back to `AppState` (dialogs need
/// the entity + window at click time).
pub fn with_pane_context_menu(
    row: impl InteractiveElement + ParentElement + Styled + IntoElement + 'static,
    pane_id: String,
    candidates: Vec<SwapCandidate>,
    app_weak: WeakEntity<AppState>,
) -> impl IntoElement {
    row.context_menu(move |menu, window, cx| {
        // Split directions verbatim (PANE-02 pitfall lock): Split right →
        // "horizontal" (tmux -h), Split down → "vertical" (tmux -v).
        let split_right_weak = app_weak.clone();
        let split_right_pane = pane_id.clone();
        let split_down_weak = app_weak.clone();
        let split_down_pane = pane_id.clone();
        let rename_weak = app_weak.clone();
        let rename_target = pane_id.clone();
        let zoom_weak = app_weak.clone();
        let zoom_target = pane_id.clone();
        let break_weak = app_weak.clone();
        let break_target = pane_id.clone();
        let kill_weak = app_weak.clone();
        let kill_target = pane_id.clone();

        let menu = menu
            .item(PopupMenuItem::new("Split Right").on_click(
                move |_, _, cx| {
                    if let Some(app) = split_right_weak.upgrade() {
                        let pane = split_right_pane.clone();
                        app.update(cx, |this, cx| {
                            this.submit_pane_split(
                                &pane,
                                AppState::pane_split_direction(true),
                                cx,
                            );
                        });
                    }
                },
            ))
            .item(PopupMenuItem::new("Split Down").on_click(
                move |_, _, cx| {
                    if let Some(app) = split_down_weak.upgrade() {
                        let pane = split_down_pane.clone();
                        app.update(cx, |this, cx| {
                            this.submit_pane_split(
                                &pane,
                                AppState::pane_split_direction(false),
                                cx,
                            );
                        });
                    }
                },
            ))
            .item(PopupMenuItem::new("Rename Pane").on_click(
                move |_, window, cx| {
                    if let Some(app) = rename_weak.upgrade() {
                        open_rename_pane_dialog(&app, &rename_target, window, cx);
                    }
                },
            ))
            .separator()
            .item(PopupMenuItem::new("Zoom").on_click(move |_, _, cx| {
                if let Some(app) = zoom_weak.upgrade() {
                    let pane = zoom_target.clone();
                    app.update(cx, |this, cx| {
                        this.submit_pane_zoom(&pane, cx);
                    });
                }
            }));

        // Swap picker submenu (pitfall 6): same-window panes only, disabled
        // when empty.
        let menu = if candidates.is_empty() {
            menu.item(PopupMenuItem::new("Swap").disabled(true))
        } else {
            let swap_cands = candidates.clone();
            let swap_weak = app_weak.clone();
            let swap_pane = pane_id.clone();
            menu.submenu("Swap", window, cx, move |sub, _, _| {
                swap_cands
                    .iter()
                    .fold(sub, |m, cand| {
                        let other = cand.id.clone();
                        let label = if cand.label.is_empty() {
                            cand.id.clone()
                        } else {
                            format!("{}  {}", cand.id, cand.label)
                        };
                        let item_weak = swap_weak.clone();
                        let item_pane = swap_pane.clone();
                        m.item(PopupMenuItem::new(label).on_click(
                            move |_, _, cx| {
                                if let Some(app) = item_weak.upgrade() {
                                    let (pane, other) =
                                        (item_pane.clone(), other.clone());
                                    app.update(cx, |this, cx| {
                                        this.submit_pane_swap(&pane, &other, cx);
                                    });
                                }
                            },
                        ))
                    })
            })
        };

        menu.item(PopupMenuItem::new("Break To Window").on_click(
            move |_, _, cx| {
                if let Some(app) = break_weak.upgrade() {
                    let pane = break_target.clone();
                    app.update(cx, |this, cx| {
                        this.submit_pane_break(&pane, cx);
                    });
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
                    .child("Kill")
            })
            .on_click(move |_, window, cx| {
                if let Some(app) = kill_weak.upgrade() {
                    request_kill_pane(&app, &kill_target, window, cx);
                }
            }),
        )
    })
}

/// Kill entry from the context menu: confirm dialog when
/// `confirm_kill_pane` says so, direct kill otherwise.
pub fn request_kill_pane(
    app_entity: &Entity<AppState>,
    target: &str,
    window: &mut Window,
    cx: &mut App,
) {
    if app_entity.read(cx).kill_requires_confirm_pane() {
        open_kill_pane_dialog(app_entity, target, window, cx);
    } else {
        let target_name = target.to_string();
        app_entity.update(cx, |this, cx| {
            this.submit_pane_kill(&target_name, cx);
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
pub struct KillPaneForm {
    /// Pane awaiting the kill decision.
    pub target: String,
    pub is_submitting: bool,
    pub error_message: Option<String>,
    pub app: WeakEntity<AppState>,
    pub window_handle: AnyWindowHandle,
}

/// Open the kill-confirm dialog for `target`. Guards against double-open.
pub fn open_kill_pane_dialog(
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

    let form = cx.new(|_cx| KillPaneForm {
        target: target_id,
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
                    // FE verbatim (`PaneContextMenu.tsx:211` /
                    // `PaneHeader.tsx:172`): no quotes around the pane id.
                    .child(format!("Kill pane {}?", build_form.read(cx).target)),
            )
            .child(render_kill_pane_body(&build_form, cx))
            .footer(render_kill_pane_footer(&build_form, cx))
            .on_close({
                // Fresh clone per build invocation (the closure re-runs per frame).
                let on_close_app = on_close_app.clone();
                move |_, _, cx| {
                    if let Some(app) = on_close_app.upgrade() {
                        app.update(cx, |this, cx| {
                            this.kill_pane_form = None;
                            cx.notify();
                        });
                    }
                }
            })
    });

    // Keep the form entity alive across the dialog lifetime.
    app_entity.update(cx, |app, cx| {
        app.kill_pane_form = Some(form);
        cx.notify();
    });
}

/// Dialog body: the destructive description, the inline error line, and the
/// in-flight hint while the correlated kill is outstanding.
fn render_kill_pane_body(form: &Entity<KillPaneForm>, cx: &mut App) -> impl IntoElement {
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
            .child("This terminates the shell running in this pane."),
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
                .child("Killing pane..."),
        );
    }

    body
}

/// Footer: Cancel (outline) and Kill (destructive `#7F1D1D` fill, white text —
/// matches the S1 close-button hover). Kill is disabled while a request is in
/// flight.
fn render_kill_pane_footer(form: &Entity<KillPaneForm>, cx: &mut App) -> DialogFooter {
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
                                request_kill_pane_submit(&form_weak, cx);
                            }
                        })
                })
                .child("Kill"),
        )
}

/// Fire the correlated kill with dialog feedback.
///
/// Strict double-submit guard: the form's `is_submitting` flag is checked
/// synchronously, so a second click while in-flight cannot fire a second kill
/// (T-03-07).
pub fn request_kill_pane_submit(form: &WeakEntity<KillPaneForm>, cx: &mut App) {
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

    let msg = AppState::build_pane_kill(&target);
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
                        this.kill_pane_form = None;
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
