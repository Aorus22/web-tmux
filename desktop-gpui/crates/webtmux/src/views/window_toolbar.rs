//! Window layout toolbar (Phase 5, PANE-06 per D7).
//!
//! FE parity (`WindowToolbar.tsx:19-77` + `LayoutSelector.tsx`): an h-8 bar
//! above the workspace with 5 preset icon buttons + separator + Next. Each
//! button sends `window.layout { paneId: @N (active window), layout }`
//! correlated on the active socket; layout strings pass through verbatim
//! incl. `next-layout` (opaque passthrough — never validated client-side).
//! `command.error`/timeout records inline on the session entry (the Phase-3
//! inline-error convention); the bar additionally renders the active
//! session's `last_error` as an inline destructive line when present.

use gpui::*;
use gpui::prelude::FluentBuilder;

use crate::app_state::AppState;
use crate::icons::{
    ARROW_RIGHT_LEFT_SVG, COLUMNS2_SVG, LAYOUT_GRID_SVG, ROWS2_SVG,
    SQUARE_SPLIT_HORIZONTAL_SVG, SQUARE_SPLIT_VERTICAL_SVG,
};

/// One layout preset button: verbatim layout id, human label, lucide icon.
pub struct LayoutPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static [u8],
}

/// The five preset buttons, FE `LAYOUTS` order verbatim
/// (`WindowToolbar.tsx:19-25`).
pub const WINDOW_LAYOUT_PRESETS: [LayoutPreset; 5] = [
    LayoutPreset {
        id: "even-horizontal",
        label: "Even Horizontal",
        icon: COLUMNS2_SVG,
    },
    LayoutPreset {
        id: "even-vertical",
        label: "Even Vertical",
        icon: ROWS2_SVG,
    },
    LayoutPreset {
        id: "main-horizontal",
        label: "Main Horizontal",
        icon: SQUARE_SPLIT_HORIZONTAL_SVG,
    },
    LayoutPreset {
        id: "main-vertical",
        label: "Main Vertical",
        icon: SQUARE_SPLIT_VERTICAL_SVG,
    },
    LayoutPreset {
        id: "tiled",
        label: "Tiled",
        icon: LAYOUT_GRID_SVG,
    },
];

/// Trailing Next button target (`LayoutSelector.tsx:47` parity).
pub const NEXT_LAYOUT_ID: &str = "next-layout";
/// Trailing Next button label (UI-SPEC copywriting contract).
pub const NEXT_LAYOUT_LABEL: &str = "Next Layout";

/// Render the h-8 preset bar for the active window. Buttons disable (muted,
/// no handler) when no active window is committed yet.
pub fn render_window_toolbar(app: &AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let border_color = rgb(0x3c3c3c);
    let muted_text = rgb(0x808080);
    let idle_icon = rgb(0x9d9d9d);

    let active_window = app.active_window_id();
    let last_error = app
        .active_session
        .as_deref()
        .and_then(|s| app.sessions.get(s))
        .and_then(|e| e.last_error.clone());

    let mut bar = div()
        .flex()
        .flex_row()
        .items_center()
        .flex_none()
        .w_full()
        .h(px(32.0))
        .gap(px(2.0))
        .px(px(8.0))
        .bg(rgb(0x1e1e1e))
        .border_b_1()
        .border_color(border_color);

    for preset in &WINDOW_LAYOUT_PRESETS {
        let layout = preset.id.to_string();
        let target = active_window.clone();
        let enabled = target.is_some();
        bar = bar.child(
            div()
                .id(format!("window-layout/{}", preset.id))
                .flex()
                .items_center()
                .justify_center()
                .w(px(24.0))
                .h(px(24.0))
                .rounded(px(4.0))
                .text_color(if enabled { idle_icon } else { muted_text })
                .when(enabled, |s| {
                    s.cursor_pointer().hover(|h| h.bg(rgb(0x2d2d2d)))
                })
                .when(!enabled, |s| s.opacity(0.4))
                .child(svg().data(preset.icon).size(px(14.0)))
                .when(enabled, |s| {
                    s.on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            if let Some(win) = target.clone() {
                                this.submit_window_layout(&win, &layout, cx);
                            }
                        }),
                    )
                }),
        );
    }

    // Separator + Next (FE `mx-1 h-4 w-px bg-border` parity).
    bar = bar.child(
        div()
            .flex_shrink_0()
            .w(px(1.0))
            .h(px(16.0))
            .bg(border_color)
            .mx(px(4.0)),
    );
    {
        let target = active_window.clone();
        let enabled = target.is_some();
        bar = bar.child(
            div()
                .id("window-layout/next-layout")
                .flex()
                .items_center()
                .justify_center()
                .w(px(24.0))
                .h(px(24.0))
                .rounded(px(4.0))
                .text_color(if enabled { idle_icon } else { muted_text })
                .when(enabled, |s| {
                    s.cursor_pointer().hover(|h| h.bg(rgb(0x2d2d2d)))
                })
                .when(!enabled, |s| s.opacity(0.4))
                .child(svg().data(ARROW_RIGHT_LEFT_SVG).size(px(14.0)))
                .when(enabled, |s| {
                    s.on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            if let Some(win) = target.clone() {
                                this.submit_window_layout(&win, NEXT_LAYOUT_ID, cx);
                            }
                        }),
                    )
                }),
        );
    }

    // Inline error line (command.error / timeout on the last preset apply —
    // the dialog-free mutations surface here instead of a toast).
    if let Some(error) = last_error {
        bar = bar.child(
            div()
                .flex_1()
                .truncate()
                .text_xs()
                .text_color(rgb(0xf87171))
                .child(error),
        );
    }

    bar.into_any_element()
}
