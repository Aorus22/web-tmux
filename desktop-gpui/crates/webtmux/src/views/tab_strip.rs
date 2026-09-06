//! S1 minimal title bar rendering.

use gpui::*;
use crate::app_state::AppState;
use crate::icons::{
    COPY_SVG, MINUS_SVG, PANEL_LEFT_SVG, SQUARE_SVG, TERMINAL_SQUARE_SVG, X_SVG,
};

/// Render the minimal S1 title bar with app identity, drag area, and custom window controls.
pub fn render_title_bar(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let bar_bg = rgb(0x1e1e1e);
    let border_color = rgb(0x3c3c3c);
    let muted_text = rgb(0x808080);
    let icon_idle_color = rgb(0x9d9d9d);
    let text_color = rgb(0xd4d4d4);
    let hover_bg = rgb(0x2d2d2d);

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
        // Center: Native Drag control area
        .child(
            div()
                .flex_1()
                .h_full()
                .window_control_area(WindowControlArea::Drag),
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
