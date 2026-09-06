//! Status view rendering Starting (S2), Ready (S4), and Failed (S3) application states.

use std::time::Duration;
use gpui::*;
use gpui::prelude::FluentBuilder;
use webtmux_supervisor::BackendStatus;
use crate::icons::LOADER2_SVG;

/// Redact secret tokens and keys from stderr lines.
///
/// Keeps at most 10 newest lines and replaces matched secret patterns with `[REDACTED]`.
pub fn redact_stderr_tail(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let tail_slice = if lines.len() > 10 {
        &lines[lines.len() - 10..]
    } else {
        &lines[..]
    };

    tail_slice
        .iter()
        .map(|line| {
            let mut sanitized = line.to_string();
            let lower = sanitized.to_lowercase();
            if lower.contains("key") || lower.contains("secret") || lower.contains("token") || lower.contains("password") {
                sanitized = "[REDACTED]".to_string();
            }
            sanitized
        })
        .collect()
}

/// Render the backend status page (Starting S2 or Failed S3 state).
pub fn render_status_page<V: 'static>(
    status: &BackendStatus,
    on_retry: impl Fn(&mut V, &MouseDownEvent, &mut Window, &mut Context<V>) + 'static + Clone,
    on_quit: impl Fn(&mut V, &MouseDownEvent, &mut Window, &mut Context<V>) + 'static + Clone,
    cx: &mut Context<V>,
) -> AnyElement {
    match status {
        BackendStatus::Starting => {
            // S2 Starting page: centered flex col, 20px Loader2 rotating 2s linear infinite (#808080), "Starting backend…" Label 12px/400 (#808080)
            div()
                .flex()
                .flex_col()
                .size_full()
                .items_center()
                .justify_center()
                .bg(rgb(0x1e1e1e))
                .text_color(rgb(0xd4d4d4))
                .gap_3()
                .child(
                    svg()
                        .data(LOADER2_SVG)
                        .size(px(20.0))
                        .text_color(rgb(0x808080))
                        .with_animation(
                            "loader_rotation",
                            Animation::new(Duration::from_secs(2)).repeat(),
                            |svg, delta| svg.with_transformation(Transformation::rotate(percentage(delta))),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::NORMAL)
                        .text_color(rgb(0x808080))
                        .child("Starting backend…"),
                )
                .into_any_element()
        }
        BackendStatus::Failed { reason, stderr_tail } => {
            // S3 Failed page: max-700px card (#2d2d2d), border 1px #7F1D1D, radius 8px, heading 16px/500, reason 14px/400, newest 10 lines stderr tail inset (#1e1e1e, border #3c3c3c), Quit outline + Retry primary
            let redacted_lines = redact_stderr_tail(stderr_tail);
            let has_tail = !redacted_lines.is_empty() && redacted_lines.iter().any(|l| !l.trim().is_empty());
            let retry_handler = on_retry.clone();
            let quit_handler = on_quit.clone();

            div()
                .flex()
                .flex_col()
                .size_full()
                .items_center()
                .justify_center()
                .bg(rgb(0x1e1e1e))
                .text_color(rgb(0xd4d4d4))
                .p_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .w_full()
                        .max_w(px(700.0))
                        .bg(rgb(0x2d2d2d))
                        .border_1()
                        .border_color(rgb(0x7F1D1D))
                        .rounded_lg()
                        .p_6()
                        .shadow_lg()
                        .child(
                            div()
                                .text_base()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(0xd4d4d4))
                                .child("Backend Startup Failed"),
                        )
                        .child(
                            div()
                                .mt_2()
                                .text_sm()
                                .font_weight(FontWeight::NORMAL)
                                .text_color(rgb(0xd4d4d4))
                                .child(format!("Reason: {}", reason)),
                        )
                        .when(has_tail, |card| {
                            card.child(
                                div()
                                    .mt_4()
                                    .p_3()
                                    .bg(rgb(0x1e1e1e))
                                    .border_1()
                                    .border_color(rgb(0x3c3c3c))
                                    .rounded_md()
                                    .text_xs()
                                    .font_family("JetBrains Mono")
                                    .text_color(rgb(0xd4d4d4))
                                    .children(redacted_lines.into_iter().map(|line| {
                                        div().child(line)
                                    })),
                            )
                        })
                        .child(
                            div()
                                .mt_6()
                                .flex()
                                .gap_3()
                                .justify_end()
                                .child(
                                    div()
                                        .px_4()
                                        .py_2()
                                        .bg(rgb(0x1e1e1e))
                                        .border_1()
                                        .border_color(rgb(0x3c3c3c))
                                        .hover(|s| s.bg(rgb(0x3c3c3c)))
                                        .rounded_md()
                                        .cursor_pointer()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0xd4d4d4))
                                        .child("Quit")
                                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev, window, cx| {
                                            quit_handler(this, ev, window, cx);
                                        })),
                                )
                                .child(
                                    div()
                                        .px_4()
                                        .py_2()
                                        .bg(rgb(0xd4d4d4))
                                        .hover(|s| s.bg(rgb(0xc5c5c5)))
                                        .rounded_md()
                                        .cursor_pointer()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0x1e1e1e))
                                        .child("Retry")
                                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev, window, cx| {
                                            retry_handler(this, ev, window, cx);
                                        })),
                                ),
                        ),
                )
                .into_any_element()
        }
        BackendStatus::Ready(_) => {
            // S4 Ready body: bare #1e1e1e surface
            div()
                .size_full()
                .bg(rgb(0x1e1e1e))
                .into_any_element()
        }
    }
}
