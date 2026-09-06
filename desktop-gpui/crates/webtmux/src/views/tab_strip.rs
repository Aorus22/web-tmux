//! S1 minimal title bar rendering with Phase-3 WindowTabs + Settings gear.
//!
//! Shell: `h(px(44.0))`, bg `#1e1e1e`, 1px `#3c3c3c` bottom border (S1 kept).
//! Order: sidebar toggle + identity → divider + WindowTabs flex-1 middle (ONLY
//! when `active_session` is set, fed by the active snapshot's windows plus
//! `active_window`) → Settings gear → window controls. Chip click is a
//! fire-and-forget correlated `window.select` on the active socket (no await,
//! no optimistic flip — state follows via delta). No Plus new-window button
//! and no chip context menu (D8 — Phase 5 owns window ops).

use gpui::*;
use gpui::prelude::{FluentBuilder, StatefulInteractiveElement};
use crate::app_state::AppState;
use crate::icons::{
    COPY_SVG, MINUS_SVG, PANEL_LEFT_SVG, SETTINGS_SVG, SQUARE_SVG, TERMINAL_SQUARE_SVG, X_SVG,
};

/// Render the minimal S1 title bar with app identity, drag area, WindowTabs of
/// the active session, Settings gear, and custom window controls.
pub fn render_title_bar(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let bar_bg = rgb(0x1e1e1e);
    let border_color = rgb(0x3c3c3c);
    let muted_text = rgb(0x808080);
    let icon_idle_color = rgb(0x9d9d9d);
    let text_color = rgb(0xd4d4d4);
    let hover_bg = rgb(0x2d2d2d);

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
                        .hover(|s| s.bg(rgb(0x7F1D1D)).text_color(rgb(0xffffff)))
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
/// `#808080` hover `#262626`). Click = fire-and-forget `window.select`.
fn render_window_tabs(app: &AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let active_bg = rgb(0x2d2d2d);
    let active_text = rgb(0xd4d4d4);
    let idle_text = rgb(0x808080);
    let hover_bg = rgb(0x262626);

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
            div()
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
                )
                .into_any_element()
        }))
}
