//! ST1 Workspace Zero, Error, and Selection State Views.
//!
//! Matches UI-SPEC contracts and fe/src/features/sessions/ {EmptyState, ErrorState, SelectSessionView}.

use gpui::*;
use gpui::prelude::InteractiveElement;
use crate::app_state::AppState;
use crate::icons::{ALERT_TRIANGLE_SVG, SQUARE_TERMINAL_SVG, TERMINAL_SQUARE_SVG};

/// Represents the active state of the central workspace body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceState {
    Error,
    Empty,
    SelectSession,
    ActiveSession,
}

/// Determine workspace state from tree error, session existence, and active session selection.
pub fn determine_workspace_state(
    tree_error: Option<&str>,
    has_sessions: bool,
    has_active_session: bool,
) -> WorkspaceState {
    if tree_error.is_some() {
        WorkspaceState::Error
    } else if !has_sessions {
        WorkspaceState::Empty
    } else if !has_active_session {
        WorkspaceState::SelectSession
    } else {
        WorkspaceState::ActiveSession
    }
}

/// Render the central workspace body based on current AppState.
pub fn render_workspace_body(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    if app.showing_settings {
        return crate::views::settings::render_settings(app, cx).into_any_element();
    }
    match app.workspace_state() {
        WorkspaceState::Error => render_error_state(app, cx).into_any_element(),
        WorkspaceState::Empty => render_empty_state(app, cx).into_any_element(),
        WorkspaceState::SelectSession => render_select_session_view(app, cx).into_any_element(),
        WorkspaceState::ActiveSession => render_active_session_placeholder(app, cx).into_any_element(),
    }
}

/// Renders EmptyState: when tree.sessions is empty and backend is alive.
pub fn render_empty_state(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    // Phase 6: chrome reads the active preset per render (no cached colors).
    let preset_name = app.settings.theme_preset.clone();
    let muted_text = crate::theme::preset_muted_fg(&preset_name);
    let foreground_text = crate::theme::preset_fg(&preset_name);
    let primary_bg = crate::theme::preset_primary(&preset_name);
    let primary_fg = crate::theme::preset_primary_fg(&preset_name);

    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .size_full()
        .p(px(32.0))
        .gap(px(12.0))
        .text_center()
        // 40x40 TerminalSquare icon (#808080)
        .child(
            svg()
                .data(TERMINAL_SQUARE_SVG)
                .size(px(40.0))
                .text_color(muted_text),
        )
        // Heading and description
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .text_base()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(foreground_text)
                        .child("No tmux sessions"),
                )
                .child(
                    div()
                        .mt(px(4.0))
                        .text_sm()
                        .text_color(muted_text)
                        .child("Create your first session to get started."),
                ),
        )
        // Primary CTA Button: "Create Session"
        .child(
            div()
                .mt(px(4.0))
                .px(px(16.0))
                .py(px(8.0))
                .rounded(px(6.0))
                .bg(primary_bg)
                .text_color(primary_fg)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .cursor_pointer()
                .hover(|s| s.opacity(0.9))
                .child("Create Session")
                .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, window, cx| {
                    crate::views::create_session_dialog::open_create_session_dialog_from_state(
                        _this, window, cx,
                    );
                })),
        )
}

/// Renders ErrorState: when tmux is missing or tree fetch fails.
pub fn render_error_state(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    // Phase 6: chrome reads the active preset per render (no cached colors).
    let preset_name = app.settings.theme_preset.clone();
    let destructive_color = crate::theme::preset_destructive(&preset_name);
    let muted_text = crate::theme::preset_muted_fg(&preset_name);
    let foreground_text = crate::theme::preset_fg(&preset_name);
    let border_color = crate::theme::preset_border(&preset_name);
    let hover_bg = crate::theme::preset_muted(&preset_name);

    let error_desc = if let Some(err) = &app.tree_error {
        if err.contains("not found") || err.contains("installed") {
            "Tmux GUI requires tmux (>= 3.2) on Linux. Install it and try again."
        } else {
            err.as_str()
        }
    } else {
        "Tmux GUI requires tmux (>= 3.2) on Linux. Install it and try again."
    };

    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .size_full()
        .p(px(32.0))
        .gap(px(12.0))
        .text_center()
        // 40x40 AlertTriangle icon (#7F1D1D)
        .child(
            svg()
                .data(ALERT_TRIANGLE_SVG)
                .size(px(40.0))
                .text_color(destructive_color),
        )
        // Heading and description
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .text_base()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(foreground_text)
                        .child("Tmux is not installed"),
                )
                .child(
                    div()
                        .mt(px(4.0))
                        .max_w(px(384.0))
                        .text_sm()
                        .text_color(muted_text)
                        .child(error_desc.to_string()),
                ),
        )
        // Outline CTA Button: "Retry"
        .child(
            div()
                .mt(px(4.0))
                .px(px(16.0))
                .py(px(8.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(border_color)
                .bg(gpui::transparent_black())
                .text_color(foreground_text)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .cursor_pointer()
                .hover(|s| s.bg(hover_bg))
                .child("Retry")
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, cx| {
                    this.trigger_poll(cx);
                })),
        )
}

/// Renders SelectSessionView: when sessions exist in the tree but none is active.
pub fn render_select_session_view(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    // Phase 6: chrome reads the active preset per render (no cached colors).
    let preset_name = app.settings.theme_preset.clone();
    let muted_text = crate::theme::preset_muted_fg(&preset_name);
    let foreground_text = crate::theme::preset_fg(&preset_name);
    let border_color = crate::theme::preset_border(&preset_name);
    let hero_bg = crate::theme::preset_muted(&preset_name);
    let card_bg = crate::theme::preset_card(&preset_name);
    let hover_bg = crate::theme::preset_muted(&preset_name);
    let badge_bg = crate::theme::preset_muted(&preset_name);

    let sessions: Vec<_> = app.tree.sessions.iter().map(|n| {
        (n.session.name.clone(), n.session.windows)
    }).collect();

    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .size_full()
        .px(px(24.0))
        .py(px(40.0))
        .text_center()
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(16.0))
                // Hero Icon Box: 48x48 rounded-xl border bg-muted/50
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(48.0))
                        .h(px(48.0))
                        .rounded(px(12.0))
                        .border_1()
                        .border_color(border_color)
                        .bg(hero_bg)
                        .text_color(muted_text)
                        .child(
                            svg()
                                .data(SQUARE_TERMINAL_SVG)
                                .size(px(24.0))
                                .text_color(muted_text),
                        ),
                )
                // Heading & Description
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(
                            div()
                                .text_base()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(foreground_text)
                                .child("No session open"),
                        )
                        .child(
                            div()
                                .mt(px(4.0))
                                .max_w(px(384.0))
                                .text_sm()
                                .text_color(muted_text)
                                .child("Pick a session to open it as a tab. Closing a tab never kills the tmux session — it keeps running in the background."),
                        ),
                )
                // Session Picker Grid (2-column, max-w-md)
                .children(if !sessions.is_empty() {
                    Some(
                        div()
                            .mt(px(8.0))
                            .w_full()
                            .max_w(px(448.0))
                            .child(
                                div()
                                    .mb(px(8.0))
                                    .text_left()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(muted_text)
                                    .child("Sessions"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.0))
                                    .children(sessions.chunks(2).map(|chunk| {
                                        let mut row = div().flex().flex_row().gap(px(8.0)).w_full();
                                        for (name, windows) in chunk {
                                            let name_str = name.clone();
                                            let name_for_click = name.clone();
                                            row = row.child(
                                                div()
                                                    .flex_1()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(8.0))
                                                    .px(px(12.0))
                                                    .py(px(10.0))
                                                    .rounded(px(8.0))
                                                    .border_1()
                                                    .border_color(border_color)
                                                    .bg(card_bg)
                                                     .cursor_pointer()
                                                     .hover(|s| s.bg(hover_bg))
                                                     .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                                                         // Picker opens-or-activates
                                                         // the tab like a sidebar
                                                         // row click does.
                                                         let base = this.base_url.clone();
                                                         this.open_session(&name_for_click);
                                                         if let Some(base) = base {
                                                             this.ensure_session_socket(&base, &name_for_click, cx);
                                                         }
                                                         cx.notify();
                                                     }))
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .text_sm()
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(foreground_text)
                                                            .overflow_hidden()
                                                            .child(name_str),
                                                    )
                                                    .child(
                                                        div()
                                                            .rounded(px(4.0))
                                                            .bg(badge_bg)
                                                            .px(px(6.0))
                                                            .py(px(2.0))
                                                            .text_xs()
                                                            .text_color(muted_text)
                                                            .child(format!("{}w", windows)),
                                                    ),
                                            );
                                        }
                                        // If odd number in row, add empty flex_1 spacer
                                        if chunk.len() == 1 {
                                            row = row.child(div().flex_1());
                                        }
                                        row
                                    })),
                            ),
                    )
                } else {
                    None
                }),
        )
}

/// Renders the active session workspace through the Phase-5 geometry grid:
/// absolutely positioned panes with headers re-hosting the Phase-4
/// `TerminalView`s (empty/single-pane windows render without dividers/zoom).
/// Empty/error/select routing is untouched.
fn render_active_session_placeholder(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    crate::views::pane_grid::render_pane_grid(app, cx).into_any_element()
}
