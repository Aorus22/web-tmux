//! S1 minimal title bar rendering with Phase-3 WindowTabs + Settings gear +
//! Phase-5 window ops (PANE-08 per D7/D8).
//!
//! Shell: `h(px(44.0))`, bg `#1e1e1e`, 1px `#3c3c3c` bottom border (S1 kept).
//! Order: sidebar toggle + identity → divider + WindowTabs flex-1 middle (ONLY
//! when `active_session` is set, fed by the active snapshot's windows plus
//! `active_window`) → Settings gear → window controls. Chip click is a
//! fire-and-forget correlated `window.select` on the active socket (no await,
//! no optimistic flip — state follows via delta). Trailing Plus sends
//! `window.create` (no args, correlated); per-chip right-click menus offer
//! Rename Window / Move Left (−1) / Move Right (+1) / Break Active Pane /
//! Kill Window (gated by `confirm_kill_window`).
//!
//! All window targets ride `pane_id: Some("@N")` (no `windowId` field exists
//! server-side). Chip/menu ids stay `window-tab/@N` (poll-tick stable — no
//! counters or indices); the Plus button is `window-create`.

use gpui::*;
use gpui::prelude::{FluentBuilder, StatefulInteractiveElement};
use gpui_component::{
    dialog::{DialogFooter, DialogTitle},
    menu::{ContextMenuExt, PopupMenuItem},
    WindowExt as _,
};
use crate::app_state::AppState;
use crate::icons::{
    COPY_SVG, MINUS_SVG, PANEL_LEFT_SVG, PLUS_SVG, SETTINGS_SVG, SQUARE_SVG, TERMINAL_SQUARE_SVG,
    X_SVG,
};
use crate::views::rename_window_dialog::open_rename_window_dialog;

/// Render the minimal S1 title bar with app identity, drag area, WindowTabs of
/// the active session, Settings gear, and custom window controls.
pub fn render_title_bar(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    // Phase 6: chrome reads the active preset per render (no cached colors).
    let preset_name = app.settings.theme_preset.clone();
    let bar_bg = crate::theme::preset_bg(&preset_name);
    let border_color = crate::theme::preset_border(&preset_name);
    let muted_text = crate::theme::preset_muted_fg(&preset_name);
    let icon_idle_color = crate::theme::preset_muted_fg(&preset_name);
    let text_color = crate::theme::preset_fg(&preset_name);
    let hover_bg = crate::theme::preset_muted(&preset_name);
    let destructive = crate::theme::preset_destructive(&preset_name);
    let destructive_fg = crate::theme::preset_destructive_fg(&preset_name);

    let has_active = app.active_session.is_some();

    div()
        .flex()
        .flex_row()
        .items_center()
        .w_full()
        .h(px(44.0))
        .bg(bar_bg)
        .border_b_1()
        .border_color(border_color)
        .px_3()
        // Left: Sidebar Toggle button (PanelLeft 16px, 40x32 button) + Identity icon + "Tmux GUI"
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                // Sidebar toggle button (SHELL-03)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(32.0))
                        .h(px(32.0))
                        .rounded_md()
                        .cursor_pointer()
                        .hover(|s| s.bg(hover_bg).text_color(text_color))
                        .child(
                            svg()
                                .data(PANEL_LEFT_SVG)
                                .size(px(16.0))
                                .text_color(if app.sidebar_open {
                                    text_color
                                } else {
                                    muted_text
                                }),
                        )
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, cx| {
                            this.sidebar_open = !this.sidebar_open;
                            cx.notify();
                        })),
                )
                .child(
                    svg()
                        .data(TERMINAL_SQUARE_SVG)
                        .size(px(16.0))
                        .text_color(muted_text),
                )
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(text_color)
                        .child("Tmux GUI"),
                ),
        )
        // WindowTabs middle (SHELL-01): divider + flex-1 chips, ONLY when a
        // session tab is open. Empty-windows snapshots render an empty middle
        // with no crash.
        .when(has_active, |s| {
            s.child(
                div()
                    .flex_shrink_0()
                    .w(px(1.0))
                    .h(px(20.0))
                    .bg(border_color)
                    .ml(px(8.0))
                    .mr(px(4.0)),
            )
            .child(render_window_tabs(app, cx))
        })
        // Center: Native Drag control area (full flex when no tabs, spacer
        // while tabs take the middle so the bar stays draggable).
        .child(
            div()
                .h_full()
                .window_control_area(WindowControlArea::Drag)
                .when(has_active, |s| s.flex_none().w(px(8.0)))
                .when(!has_active, |s| s.flex_1()),
        )
        // Settings gear (SHELL-01): unfocus the tab and show the honest
        // Phase-6 placeholder; tabs stay open underneath.
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .w(px(32.0))
                .h(px(32.0))
                .rounded_md()
                .cursor_pointer()
                .hover(|s| s.bg(hover_bg).text_color(text_color))
                .child(
                    svg()
                        .data(SETTINGS_SVG)
                        .size(px(16.0))
                        .text_color(muted_text),
                )
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, cx| {
                    this.active_session = None;
                    this.showing_settings = true;
                    cx.notify();
                })),
        )
        // Right: 1px separator line + 3 control buttons (40x32 each, 6px radius)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .h_full()
                .ml_1()
                .pl_1()
                // Separator
                .child(
                    div()
                        .w(px(1.0))
                        .h(px(16.0))
                        .bg(border_color)
                        .mr_2(),
                )
                // Minimize Button
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(40.0))
                        .h(px(32.0))
                        .rounded_md()
                        .cursor_pointer()
                        .hover(|s| s.bg(hover_bg).text_color(text_color))
                        .child(
                            svg()
                                .data(MINUS_SVG)
                                .size(px(16.0))
                                .text_color(icon_idle_color),
                        )
                        .on_mouse_down(MouseButton::Left, |_, window, _| {
                            window.minimize_window();
                        }),
                )
                // Maximize / Restore Button
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(40.0))
                        .h(px(32.0))
                        .rounded_md()
                        .cursor_pointer()
                        .hover(|s| s.bg(hover_bg).text_color(text_color))
                        .child(
                            svg()
                                .data(if app.is_maximized {
                                    COPY_SVG
                                } else {
                                    SQUARE_SVG
                                })
                                .size(px(14.0))
                                .text_color(icon_idle_color),
                        )
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                            let new_max = crate::window_state::toggle_maximize(window);
                            this.is_maximized = new_max;
                            cx.notify();
                        })),
                )
                // Close Button (Destructive hover #7F1D1D / white icon)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(40.0))
                        .h(px(32.0))
                        .rounded_md()
                        .cursor_pointer()
                        .hover(|s| s.bg(destructive).text_color(destructive_fg))
                        .child(
                            svg()
                                .data(X_SVG)
                                .size(px(16.0))
                                .text_color(icon_idle_color),
                        )
                        .on_mouse_down(MouseButton::Left, |_, window, _| {
                            window.remove_window();
                        }),
                ),
        )
}

/// WindowTabs middle: flex-1 horizontal scroll of `h-7 max-w-44` chips showing
/// `{index}: {name}` truncated (active bg `#2d2d2d`/text `#d4d4d4`, idle text
/// `#808080` hover `#262626`). Click = fire-and-forget `window.select`;
/// right-click = chip menu (rename/move/break/kill); trailing Plus creates a
/// window. FE parity (`WindowTabs.tsx:109-190`).
fn render_window_tabs(app: &AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    // Phase 6: chrome reads the active preset per render (no cached colors).
    let preset_name = app.settings.theme_preset.clone();
    let active_bg = crate::theme::preset_muted(&preset_name);
    let active_text = crate::theme::preset_fg(&preset_name);
    let idle_text = crate::theme::preset_muted_fg(&preset_name);
    let hover_bg = crate::theme::preset_muted(&preset_name);
    let app_weak = cx.entity().downgrade();

    div()
        .id("window-tabs")
        .flex()
        .flex_row()
        .items_center()
        .flex_1()
        .h_full()
        .gap(px(2.0))
        .overflow_x_scroll()
        .px(px(4.0))
        .children(app.window_tabs().into_iter().map(|tab| {
            let win_id = tab.id.clone();
            let label = format!("{}: {}", tab.index, tab.name);
            let is_active = tab.active;
            let chip = div()
                .id(format!("window-tab/{}", tab.id))
                .flex()
                .flex_row()
                .items_center()
                .flex_none()
                .h(px(28.0))
                .max_w(px(176.0))
                .rounded_md()
                .px(px(10.0))
                .text_sm()
                .truncate()
                .cursor_pointer()
                .when(is_active, |s| s.bg(active_bg).text_color(active_text))
                .when(!is_active, |s| {
                    s.text_color(idle_text).hover(|h| h.bg(hover_bg))
                })
                .child(label)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _window, _cx| {
                        this.send_window_select(&win_id);
                    }),
                );
            with_window_tab_menu(chip, tab.id.clone(), app_weak.clone()).into_any_element()
        }))
        // Trailing Plus: `window.create` with no args, correlated
        // (`WindowTabs.tsx:182-190` parity).
        .child(
            div()
                .id("window-create")
                .flex()
                .items_center()
                .justify_center()
                .flex_none()
                .w(px(28.0))
                .h(px(28.0))
                .rounded_md()
                .cursor_pointer()
                .text_color(idle_text)
                .hover(|s| s.bg(hover_bg).text_color(active_text))
                .child(svg().data(PLUS_SVG).size(px(14.0)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.submit_window_create(None, cx);
                    }),
                )
                .into_any_element(),
        )
}

/// Wrap a window-tab chip in its right-click menu.
///
/// Rows copy the FE item list verbatim (`WindowContextMenu.tsx:34-43`):
/// Rename Window / Move Left / Move Right / Break Active Pane / separator /
/// Kill Window (destructive red). Move offsets are exactly ±1 (FE
/// `WindowTabs.tsx:159-164` parity — tmux `move-window %+d`). `row` carries
/// the stable `window-tab/@N` id; `ContextMenuExt` derives its open-state id
/// from it. Left-click select stays `MouseButton::Left`-only, so right-click
/// never conflicts.
pub fn with_window_tab_menu(
    row: impl InteractiveElement + ParentElement + Styled + IntoElement + 'static,
    window_id: String,
    app_weak: WeakEntity<AppState>,
) -> impl IntoElement {
    row.context_menu(move |menu, _window, _cx| {
        let rename_weak = app_weak.clone();
        let rename_target = window_id.clone();
        let left_weak = app_weak.clone();
        let left_target = window_id.clone();
        let right_weak = app_weak.clone();
        let right_target = window_id.clone();
        let break_weak = app_weak.clone();
        let break_target = window_id.clone();
        let kill_weak = app_weak.clone();
        let kill_target = window_id.clone();

        menu.item(PopupMenuItem::new("Rename Window").on_click(
            move |_, window, cx| {
                if let Some(app) = rename_weak.upgrade() {
                    open_rename_window_dialog(&app, &rename_target, window, cx);
                }
            },
        ))
        .item(PopupMenuItem::new("Move Left").on_click(
            move |_, _, cx| {
                if let Some(app) = left_weak.upgrade() {
                    let target = left_target.clone();
                    app.update(cx, |this, cx| {
                        this.submit_window_move(&target, -1, cx);
                    });
                }
            },
        ))
        .item(PopupMenuItem::new("Move Right").on_click(
            move |_, _, cx| {
                if let Some(app) = right_weak.upgrade() {
                    let target = right_target.clone();
                    app.update(cx, |this, cx| {
                        this.submit_window_move(&target, 1, cx);
                    });
                }
            },
        ))
        .item(PopupMenuItem::new("Break Active Pane").on_click(
            move |_, _, cx| {
                if let Some(app) = break_weak.upgrade() {
                    let target = break_target.clone();
                    app.update(cx, |this, cx| {
                        this.submit_window_break_active(&target, cx);
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
                    .child("Kill Window")
            })
            .on_click(move |_, window, cx| {
                if let Some(app) = kill_weak.upgrade() {
                    request_kill_window(&app, &kill_target, window, cx);
                }
            }),
        )
    })
}

/// Kill entry from the chip menu: confirm dialog when
/// `confirm_kill_window` says so, direct kill otherwise.
pub fn request_kill_window(
    app_entity: &Entity<AppState>,
    target: &str,
    window: &mut Window,
    cx: &mut App,
) {
    if app_entity.read(cx).kill_requires_confirm_window() {
        open_kill_window_dialog(app_entity, target, window, cx);
    } else {
        let target_id = target.to_string();
        app_entity.update(cx, |this, cx| {
            this.submit_window_kill(&target_id, cx);
        });
    }
}

// ---------------------------------------------------------------------------
// Kill-confirm dialog (DLG1 tokens, destructive Close)
// ---------------------------------------------------------------------------

/// Modal form state entity for the kill-confirm dialog. Owned by `AppState`
/// for the lifetime of the open dialog; replaced on every open, `None` on
/// dismiss. No text input — a plain confirm with an `is_submitting` guard
/// (T-03-07) and an inline error line.
pub struct KillWindowForm {
    /// Window awaiting the kill decision.
    pub target: String,
    pub is_submitting: bool,
    pub error_message: Option<String>,
    pub app: WeakEntity<AppState>,
    pub window_handle: AnyWindowHandle,
}

/// Open the kill-confirm dialog for `target`. Guards against double-open.
pub fn open_kill_window_dialog(
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

    let form = cx.new(|_cx| KillWindowForm {
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
                    .child(format!("Kill window \"{}\"?", build_form.read(cx).target)),
            )
            .child(render_kill_window_body(&build_form, cx))
            .footer(render_kill_window_footer(&build_form, cx))
            .on_close({
                // Fresh clone per build invocation (the closure re-runs per frame).
                let on_close_app = on_close_app.clone();
                move |_, _, cx| {
                    if let Some(app) = on_close_app.upgrade() {
                        app.update(cx, |this, cx| {
                            this.kill_window_form = None;
                            cx.notify();
                        });
                    }
                }
            })
    });

    // Keep the form entity alive across the dialog lifetime.
    app_entity.update(cx, |app, cx| {
        app.kill_window_form = Some(form);
        cx.notify();
    });
}

/// Dialog body: the destructive description, the inline error line, and the
/// in-flight hint while the correlated kill is outstanding.
fn render_kill_window_body(form: &Entity<KillWindowForm>, cx: &mut App) -> impl IntoElement {
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
            .child("This closes the tmux window and all its panes."),
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
                .child("Closing window..."),
        );
    }

    body
}

/// Footer: Cancel (outline) and Close (destructive `#7F1D1D` fill, white text —
/// FE `WindowTabs.tsx:230` parity). Close is disabled while a request is in
/// flight.
fn render_kill_window_footer(form: &Entity<KillWindowForm>, cx: &mut App) -> DialogFooter {
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
                                request_kill_window_submit(&form_weak, cx);
                            }
                        })
                })
                .child("Close"),
        )
}

/// Fire the correlated kill with dialog feedback.
///
/// Strict double-submit guard: the form's `is_submitting` flag is checked
/// synchronously, so a second click while in-flight cannot fire a second kill
/// (T-03-07).
pub fn request_kill_window_submit(form: &WeakEntity<KillWindowForm>, cx: &mut App) {
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

    let msg = AppState::build_window_kill(&target);
    let rx = match app_entity.read(cx).try_send_window_command(msg) {
        Ok((_, rx)) => rx,
        Err(e) => {
            let message = e.clone();
            app_entity.update(cx, |this, cx| {
                if let Some(active) = this.active_session.clone() {
                    this.note_session_error(&active, e);
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
    app_entity.update(cx, |this, cx| {
        let active = this.active_session.clone();
        this.await_command_feedback(
            rx,
            move |this, outcome, cx| {
                match outcome {
                    Ok(()) => {
                        this.kill_window_form = None;
                        let _ = window_handle.update(cx, |_, window, cx| {
                            window.close_dialog(cx);
                        });
                    }
                    Err(e) => {
                        if let Some(sess) = active.clone() {
                            this.note_session_error(&sess, e.clone());
                        }
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
