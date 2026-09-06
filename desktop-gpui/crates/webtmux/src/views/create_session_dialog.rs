//! DLG1 Create Session Dialog.
//!
//! Modal session creation built on `gpui-component` Dialog + InputState with native
//! directory browsing via `App::prompt_for_paths`. Matches
//! `fe/src/features/sessions/CreateSessionDialog.tsx` and 02-UI-SPEC DLG1:
//! - Modal: 440px wide, radius 8px, `#1e1e1e` background, 1px `#3c3c3c` border,
//!   20px padding, dark backdrop overlay, Escape/backdrop dismissal.
//! - Header: "Create Session" (16px/500) + "Start a new tmux session." (12px/400).
//! - Fields: Name (required, client-side pre-validated and server-side validated),
//!   Working directory (optional, native folder picker), Initial command (optional).
//! - Inline destructive error line when the backend rejects the POST (400/409);
//!   double-submit guarded via `is_submitting` (threat T-02-06).
//! - Footer: Cancel (outline) and Create (primary `#d4d4d4` fill, `#1e1e1e` text),
//!   disabled while the name is empty or the request is in-flight.

use gpui::*;
use gpui::prelude::FluentBuilder as _;
use gpui_component::{
    dialog::{DialogFooter, DialogTitle},
    input::{Input, InputEvent, InputState},
    WindowExt as _,
};
use webtmux_backend_client::{
    RestError, RestClient, validate_session_name, CreateSessionRequest,
};

use crate::app_state::AppState;
use crate::icons::FOLDER_OPEN_SVG;

/// Modal form state entity for DLG1. Owned by `AppState` for the lifetime of the
/// open dialog; replaced (and dropped, releasing its InputStates/subscription)
/// on every open, which is what guarantees cleared fields on re-open.
pub struct CreateSessionForm {
    pub name_input: Entity<InputState>,
    pub cwd_input: Entity<InputState>,
    pub cmd_input: Entity<InputState>,
    pub is_submitting: bool,
    pub error_message: Option<String>,
    rest_client: Option<RestClient>,
    app: WeakEntity<AppState>,
    window_handle: AnyWindowHandle,
    _subscriptions: Vec<Subscription>,
}

/// Open the DLG1 Create Session dialog. Guards against double-open; a fresh
/// (cleared) form entity is stored on `AppState` each time the dialog opens.
pub fn open_create_session_dialog(
    app_entity: &Entity<AppState>,
    window: &mut Window,
    cx: &mut App,
) {
    // No stacked DLG1: a second click on Plus must not open a second dialog.
    if window.has_active_dialog(cx) {
        return;
    }

    let app_weak = app_entity.downgrade();
    let rest_client = app_entity.read(cx).rest_client.clone();
    let window_handle = window.window_handle();

    let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("dev"));
    let cwd_input = cx.new(|cx| InputState::new(window, cx).placeholder("~/projects (optional)"));
    let cmd_input = cx.new(|cx| InputState::new(window, cx).placeholder("e.g. nvim (optional)"));

    let form = cx.new(|_cx| CreateSessionForm {
        name_input: name_input.clone(),
        cwd_input: cwd_input.clone(),
        cmd_input: cmd_input.clone(),
        is_submitting: false,
        error_message: None,
        rest_client,
        app: app_weak.clone(),
        window_handle,
        _subscriptions: Vec::new(),
    });

    // Pressing Enter inside the Initial command field submits the form (fe parity).
    let sub_cmd = {
        let form_weak = form.downgrade();
        cx.subscribe(&cmd_input, move |_: Entity<InputState>, event, cx| match event {
            InputEvent::PressEnter { .. } => request_submit(&form_weak, cx),
            _ => {}
        })
    };
    form.update(cx, |f, _cx| {
        f._subscriptions = vec![sub_cmd];
    });

    // Reset-on-dismiss: clear the AppState-held form so Escape/backdrop/X leave
    // no stale state; the old entity (and its InputStates) is dropped.
    let on_close_app = app_weak;
    let build_form = form.clone();
    window.open_dialog(cx, move |dialog, window, cx| {
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
                    .child("Create Session"),
            )
            .child(render_dialog_body(&build_form, window, cx))
            .footer(render_dialog_footer(&build_form, cx))
            .on_close({
                // Fresh clone per build invocation (the closure re-runs per frame).
                let on_close_app = on_close_app.clone();
                move |_, _, cx| {
                    if let Some(app) = on_close_app.upgrade() {
                        app.update(cx, |this, cx| {
                            this.create_session_form = None;
                            cx.notify();
                        });
                    }
                }
            })
    });

    // Autofocus the Name field on dialog open (fe parity).
    name_input.update(cx, |state, cx| state.focus(window, cx));

    // Keep the form entity + subscriptions alive across the dialog lifetime.
    app_entity.update(cx, |app, cx| {
        app.create_session_form = Some(form);
        cx.notify();
    });
}

/// Listener-friendly trigger used by the sidebar Plus button and EmptyState CTA.
pub fn open_create_session_dialog_from_state(
    app: &mut AppState,
    window: &mut Window,
    cx: &mut Context<AppState>,
) {
    let _ = app;
    let entity = cx.entity();
    open_create_session_dialog(&entity, window, cx);
}

// ---------------------------------------------------------------------------
// Dialog composition
// ---------------------------------------------------------------------------

/// Dialog body: description, the three form fields, the optional inline error
/// line, and the in-flight hint while submitting.
fn render_dialog_body(form: &Entity<CreateSessionForm>, _window: &mut Window, cx: &mut App) -> impl IntoElement {
    let muted_text = rgb(0x808080);
    let foreground_text = rgb(0xd4d4d4);

    let (is_submitting, error) = {
        let f = form.read(cx);
        (f.is_submitting, f.error_message.clone())
    };

    let mut body = div().flex().flex_col().gap(px(12.0)).w_full();

    // Description: "Start a new tmux session."
    body = body.child(
        div()
            .text_xs()
            .text_color(muted_text)
            .child("Start a new tmux session."),
    );

    // Name field (required, validated client-side + server-side).
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
                    .child("Name *"),
            )
            .child(Input::new(&name_input).w_full()),
    );

    // Working directory field: Input + 32x32 native picker button (FolderOpen).
    let cwd_input = form.read(cx).cwd_input.clone();
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
                    .child("Working directory"),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(Input::new(&cwd_input).flex_1())
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(32.0))
                            .h(px(32.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(rgb(0x3c3c3c))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x262626)))
                            .child(
                                svg()
                                    .data(FOLDER_OPEN_SVG)
                                    .size(px(16.0))
                                    .text_color(muted_text),
                            )
                            .on_mouse_down(MouseButton::Left, {
                                let form_weak = form.downgrade();
                                move |_, window, cx| {
                                    request_browse_directory(&form_weak, window, cx);
                                }
                            }),
                    ),
            ),
    );

    // Initial command field: pressing Enter submits the form.
    let cmd_input = form.read(cx).cmd_input.clone();
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
                    .child("Initial command"),
            )
            .child(Input::new(&cmd_input).w_full()),
    );

    // Inline destructive error line (backend 400/409 rejection, D-01).
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
                .child("Creating session..."),
        );
    }

    body
}

/// Footer: Cancel (outline) and Create (primary `#d4d4d4` fill). Create is
/// disabled while the name is empty or the request is in flight.
fn render_dialog_footer(form: &Entity<CreateSessionForm>, cx: &mut App) -> DialogFooter {
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
                            return; // cancellable only when not busy (T-02-06)
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
                                request_submit(&form_weak, cx);
                            }
                        })
                })
                .child("Create"),
        )
}

/// Open the native directory picker (Windows IFileOpenDialog with FOS_PICKFOLDERS,
/// Linux XDG portal — gpui `App::prompt_for_paths`). On cancel the existing cwd
/// input is left untouched; on selection the chosen path populates the input.
fn request_browse_directory(form: &WeakEntity<CreateSessionForm>, _window: &mut Window, cx: &mut App) {
    let Some(form_entity) = form.upgrade() else { return };

    let cwd_input = form_entity.read(cx).cwd_input.clone();
    let window_handle = form_entity.read(cx).window_handle;

    let rx = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some("Select Working Directory".into()),
    });

    cx.spawn(async move |cx: &mut AsyncApp| {
        if let Ok(Ok(Some(paths))) = rx.await {
            if let Some(path) = paths.first() {
                let path = path.to_string_lossy().to_string();
                let _ = window_handle.update(cx, |_, window, cx| {
                    cwd_input.update(cx, |state, cx| {
                        state.replace_all(path, window, cx);
                    });
                });
            }
        }
        // Ok(None) = user cancelled; Err = portal unavailable. Either way the
        // existing cwd input is left untouched.
    })
    .detach();
}

/// Validate the name locally (tmux rules mirror the Go backend), then
/// POST /api/sessions. On success: close the dialog, select the created
/// session, and refresh the tree. On failure: inline destructive error while
/// the dialog stays open (D-01).
///
/// Strict double-submit guard: the form's `is_submitting` flag (rendered as a
/// disabled Create button) is checked synchronously, so a second click or
/// Enter while in-flight cannot fire a second POST.
pub fn request_submit(form: &WeakEntity<CreateSessionForm>, cx: &mut App) {
    // Owned weak handle for the 'static async continuation below.
    let form_weak = form.clone();
    let Some(form_entity) = form_weak.upgrade() else { return };

    let state = form_entity.read(cx);
    if state.is_submitting {
        return; // Throttle guard T-02-06 (short-circuit, no state change).
    }

    let name = state.name_input.read(cx).value().trim().to_string();
    let cwd = state.cwd_input.read(cx).value().trim().to_string();
    let initial_command = state.cmd_input.read(cx).value().trim().to_string();
    let rest_client = state.rest_client.clone();
    let app = state.app.clone();
    let window_handle = state.window_handle;

    if name.is_empty() {
        return; // Create button disabled in this state; ignore the event.
    }

    // Client-side validation mirrors the Go backend before any network call.
    if let Err(e) = validate_session_name(&name) {
        let message = e.to_string();
        form_entity.update(cx, |f, cx| {
            f.error_message = Some(message);
            cx.notify();
        });
        return;
    }

    let request = CreateSessionRequest {
        name: name.clone(),
        cwd: (!cwd.is_empty()).then_some(cwd),
        initial_command: (!initial_command.is_empty()).then_some(initial_command),
    };

    form_entity.update(cx, |f, cx| {
        f.is_submitting = true;
        f.error_message = None;
        cx.notify();
    });

    cx.spawn(async move |cx: &mut AsyncApp| {
        let result = match &rest_client {
            Some(client) => client.create_session(&request).await,
            None => Err(RestError::Api {
                url: String::new(),
                status: 503,
                message: "Backend is not ready yet. Try again in a moment.".to_string(),
            }),
        };

        let _ = cx.update(|cx: &mut App| {
            match result {
                Ok(created) => {
                    // Select the new session, refresh the tree, close the modal.
                    if let Some(app) = app.upgrade() {
                        app.update(cx, |this, cx| {
                            this.create_session_form = None;
                            this.active_session = Some(created.name);
                            this.trigger_poll(cx);
                        });
                    }
                    let _ = window_handle.update(cx, |_, window, cx| {
                        window.close_dialog(cx);
                    });
                }
                Err(e) => {
                    // Inline destructive error; the dialog stays open (D-01).
                    if let Some(form) = form_weak.upgrade() {
                        form.update(cx, |f, cx| {
                            f.is_submitting = false;
                            f.error_message = Some(e.to_string());
                            cx.notify();
                        });
                    }
                }
            }
        });
    })
    .detach();
}
