//! SB1 Collapsible Sidebar View.
//!
//! Matches fe/src/components/layout/AppSidebar.tsx geometry and styling:
//! - Container: w-60 (240px) when open, w-0 (0px) when closed, overflow_hidden, border-r 1px #3c3c3c, bg #1e1e1e.
//! - Header row: h-9 (36px), px-3 (12px), title "Sessions" (Label 12px/500 #808080), RefreshCw (20x20 ghost), Plus (20x20 ghost).
//! - Tree area: scrollable list, "No sessions" when empty, session row (14px/500, ChevronRight 14px, windows badge 10px #808080 bg #2d2d2d, active highlight bg #2d2d2d / text #d4d4d4).
//! - Window sub-tree (when expanded): ml-4 (16px), border-l 1px #3c3c3c, pl-2 (8px), window row 12px/400 #aaaaaa "{index}: {name}", pane row 11px/400 #808080 with 4px circular dot.
//! - Footer: border-t 1px #3c3c3c, p-1.5, Settings ghost button with 14px icon + text.

use gpui::*;
use gpui::prelude::{FluentBuilder, InteractiveElement, StatefulInteractiveElement};
use crate::app_state::AppState;
use crate::icons::{CHEVRON_DOWN_SVG, CHEVRON_RIGHT_SVG, PLUS_SVG, REFRESH_CW_SVG, SETTINGS_SVG};

pub fn render_sidebar(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let sidebar_open = app.sidebar_open;
    let bg_color = rgb(0x1e1e1e);
    let border_color = rgb(0x3c3c3c);
    let muted_text = rgb(0x808080);
    let foreground_text = rgb(0xd4d4d4);
    let active_bg = rgb(0x2d2d2d);
    let hover_bg = rgb(0x262626);
    let window_text = rgb(0xaaaaaa);
    let pane_dot_bg = rgb(0x4f4f4f);

    let width_val = if sidebar_open { px(240.0) } else { px(0.0) };

    div()
        .id("app-sidebar")
        .flex_shrink_0()
        .overflow_hidden()
        .w(width_val)
        .h_full()
        .bg(bg_color)
        .border_r_1()
        .border_color(border_color)
        .child(
            div()
                .flex()
                .flex_col()
                .w(px(240.0))
                .h_full()
                // Header row (36px / h-9)
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .h(px(36.0))
                        .px(px(12.0))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(muted_text)
                                .child("Sessions"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(2.0))
                                // Refresh Button (20x20 ghost)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(20.0))
                                        .h(px(20.0))
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(active_bg).text_color(foreground_text))
                                        .child(
                                            svg()
                                                .data(REFRESH_CW_SVG)
                                                .size(px(12.0))
                                                .text_color(muted_text),
                                        )
                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, cx| {
                                            this.trigger_poll(cx);
                                        })),
                                )
                                // Plus Button (20x20 ghost)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(20.0))
                                        .h(px(20.0))
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(active_bg).text_color(foreground_text))
                                        .child(
                                            svg()
                                                .data(PLUS_SVG)
                                                .size(px(12.0))
                                                .text_color(muted_text),
                                        )
                                        .on_mouse_down(MouseButton::Left, cx.listener(|_this, _, _window, _cx| {
                                            // Dialog creation hook will be expanded in Plan 02-02
                                        })),
                                ),
                        ),
                )
                // Tree Area (scrollable vertical list)
                .child(
                    div()
                        .id("sessions-scroll-area")
                        .flex_1()
                        .overflow_y_scroll()
                        .px(px(6.0))
                        .pb(px(12.0))
                        .children(if app.tree.sessions.is_empty() {
                            vec![
                                div()
                                    .px(px(8.0))
                                    .py(px(16.0))
                                    .text_xs()
                                    .text_color(muted_text)
                                    .child("No sessions")
                                    .into_any_element(),
                            ]
                        } else {
                            app.tree.sessions.iter().map(|node| {
                                let session_name = node.session.name.clone();
                                let is_active = app.active_session.as_ref() == Some(&session_name);
                                let is_expanded = app.expanded_sessions.contains(&session_name);
                                let windows_count = node.session.windows;

                                let session_name_for_click = session_name.clone();
                                let session_name_for_toggle = session_name.clone();

                                div()
                                    .mb(px(2.0))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .w_full()
                                            .gap(px(4.0))
                                            .rounded(px(6.0))
                                            .px(px(8.0))
                                            .py(px(6.0))
                                            .cursor_pointer()
                                            .when(is_active, |s| s.bg(active_bg).text_color(foreground_text))
                                            .when(!is_active, |s| s.text_color(foreground_text).hover(|h| h.bg(hover_bg)))
                                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                                                this.active_session = Some(session_name_for_click.clone());
                                                cx.notify();
                                            }))
                                            // Chevron expand icon
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .w(px(14.0))
                                                    .h(px(14.0))
                                                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                                                        if this.expanded_sessions.contains(&session_name_for_toggle) {
                                                            this.expanded_sessions.remove(&session_name_for_toggle);
                                                        } else {
                                                            this.expanded_sessions.insert(session_name_for_toggle.clone());
                                                        }
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        svg()
                                                            .data(if is_expanded {
                                                                CHEVRON_DOWN_SVG
                                                            } else {
                                                                CHEVRON_RIGHT_SVG
                                                            })
                                                            .size(px(14.0))
                                                            .text_color(muted_text),
                                                    ),
                                            )
                                            // Session Name
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .text_sm()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .overflow_hidden()
                                                    .child(session_name.clone()),
                                            )
                                            // Window count badge
                                            .child(
                                                div()
                                                    .ml_auto()
                                                    .rounded(px(4.0))
                                                    .bg(active_bg)
                                                    .px(px(4.0))
                                                    .text_color(muted_text)
                                                    .text_xs()
                                                    .child(format!("{}", windows_count)),
                                            ),
                                    )
                                    // Window sub-tree when expanded
                                    .when(is_expanded, |s| {
                                        s.child(
                                            div()
                                                .ml(px(16.0))
                                                .border_l_1()
                                                .border_color(border_color)
                                                .pl(px(8.0))
                                                .children(node.windows.iter().map(|wn| {
                                                    div()
                                                        .py(px(2.0))
                                                        .child(
                                                            div()
                                                                .flex()
                                                                .flex_row()
                                                                .items_center()
                                                                .gap(px(4.0))
                                                                .px(px(4.0))
                                                                .text_xs()
                                                                .text_color(window_text)
                                                                .child(format!("{}: {}", wn.window.index, wn.window.name)),
                                                        )
                                                        .children(wn.panes.iter().map(|p| {
                                                            let cmd = if !p.current_command.is_empty() {
                                                                &p.current_command
                                                            } else if !p.title.is_empty() {
                                                                &p.title
                                                            } else {
                                                                &p.id
                                                            };

                                                            div()
                                                                .flex()
                                                                .flex_row()
                                                                .items_center()
                                                                .gap(px(4.0))
                                                                .px(px(4.0))
                                                                .py(px(1.0))
                                                                .pl(px(12.0))
                                                                .text_color(muted_text)
                                                                .child(
                                                                    div()
                                                                        .w(px(4.0))
                                                                        .h(px(4.0))
                                                                        .rounded_full()
                                                                        .bg(pane_dot_bg),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_xs()
                                                                        .overflow_hidden()
                                                                        .child(cmd.to_string()),
                                                                )
                                                        }))
                                                })),
                                        )
                                    })
                                    .into_any_element()
                            }).collect()
                        }),
                )
                // Footer (1px top border, Settings ghost button)
                .child(
                    div()
                        .border_t_1()
                        .border_color(border_color)
                        .p(px(6.0))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(8.0))
                                .w_full()
                                .rounded(px(6.0))
                                .px(px(8.0))
                                .py(px(6.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(hover_bg))
                                .child(
                                    svg()
                                        .data(SETTINGS_SVG)
                                        .size(px(14.0))
                                        .text_color(muted_text),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(foreground_text)
                                        .child("Settings"),
                                ),
                        ),
                ),
        )
}
