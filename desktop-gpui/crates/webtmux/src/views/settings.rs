//! Phase 6 Settings page (SET-01/SET-02, THEME-01/02/03 per D4).
//!
//! Ported from the web-term reference card-grid shape
//! (`views/settings.rs:296-404`) over the existing `showing_settings` route:
//! max-w-672 centered column, Appearance section (filter row +
//! `"{n} themes available"` + flex-wrap 3-column `w(px(204))` cards with
//! 56px swatch / 3 dots / 2 bars / check overlay / click-to-apply), Terminal
//! section (font-size/line-height/scrollback steppers with FE clamps,
//! TUI-scroll-default toggle, honest single-family label).
//!
//! - tmux-binary rows and kill-confirm switches arrive in 06-02; clearly
//!   marked section anchors below hold their place (no dead controls).
//! - One preset read per render, no cached colors: every token goes through
//!   the `theme::preset_*` helpers so `cx.notify()` repaints everything.
//! - FE copy preserved verbatim: `"Filter by dark or light appearance"`,
//!   `"{n} themes available"`,
//!   `"The selected theme is applied to every pane and app surface."`

use gpui::*;
use gpui::prelude::{InteractiveElement, StatefulInteractiveElement};
use crate::app_state::AppState;
use crate::icons::{CHECK_SVG, PAINTBRUSH_SVG};
use crate::themes_generated::UI_THEMES;

/// Render the full settings page behind the `showing_settings` route.
pub fn render_settings(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let preset_name = app.settings.theme_preset.clone();
    let bg = crate::theme::preset_bg(&preset_name);
    let fg = crate::theme::preset_fg(&preset_name);
    let muted_fg = crate::theme::preset_muted_fg(&preset_name);
    let border = crate::theme::preset_border(&preset_name);
    let muted = crate::theme::preset_muted(&preset_name);

    div()
        .id("settings-scroll")
        .flex()
        .flex_col()
        .items_center()
        .size_full()
        .overflow_y_scroll()
        .bg(bg)
        .px(px(24.0))
        .py(px(32.0))
        .child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .max_w(px(672.0))
                .gap(px(24.0))
                .child(render_appearance(app, cx))
                .child(render_terminal(app, cx))
                .child(render_binary_anchor(muted_fg))
                .child(render_kill_anchor(muted_fg))
                .child(render_back_button(fg, border, muted, cx))
                .into_any_element(),
        )
        .text_color(fg)
        .border_color(border)
        .into_any_element()
}

/// Appearance section: header + filter row + count + card grid.
fn render_appearance(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let preset_name = app.settings.theme_preset.clone();
    let card = crate::theme::preset_card(&preset_name);
    let card_fg = crate::theme::preset_card_fg(&preset_name);
    let fg = crate::theme::preset_fg(&preset_name);
    let muted = crate::theme::preset_muted(&preset_name);
    let muted_fg = crate::theme::preset_muted_fg(&preset_name);
    let border = crate::theme::preset_border(&preset_name);
    let primary = crate::theme::preset_primary(&preset_name);

    let filter = app.settings.theme_mode_filter.clone();
    let active_preset = app.settings.theme_preset.clone();

    let filtered: Vec<_> = UI_THEMES
        .iter()
        .filter(|t| match filter.as_str() {
            "dark" => t.is_dark,
            "light" => !t.is_dark,
            _ => true,
        })
        .collect();
    let count = filtered.len();

    let mut col = div().flex().flex_col().gap(px(12.0)).w_full();

    // Header: paintbrush + Appearance + FE sub-copy.
    col = col.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .child(svg().data(PAINTBRUSH_SVG).size(px(16.0)).text_color(muted_fg))
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(fg)
                    .child("Appearance"),
            ),
    );
    col = col.child(
        div()
            .text_sm()
            .text_color(muted_fg)
            .child("The selected theme is applied to every pane and app surface."),
    );

    // Filter row: label + All/Dark/Light toggle (hand-rolled; no select
    // widget port per D4) — persists via settings.theme_mode_filter.
    col = col.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .rounded(px(8.0))
            .border_1()
            .border_color(border)
            .bg(card)
            .px(px(16.0))
            .py(px(12.0))
            .child(
                div()
                    .text_sm()
                    .text_color(muted_fg)
                    .child("Filter by dark or light appearance"),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(4.0))
                    .child(render_filter_button("all", "All", &filter, muted, muted_fg, primary, card_fg, cx))
                    .child(render_filter_button("dark", "Dark", &filter, muted, muted_fg, primary, card_fg, cx))
                    .child(render_filter_button("light", "Light", &filter, muted, muted_fg, primary, card_fg, cx)),
            ),
    );

    // Count line.
    col = col.child(
        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(fg)
                    .child("Color theme"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted_fg)
                    .child(format!("{} themes available", count)),
            ),
    );

    // Card grid: flex-wrap 3-column w(px(204)) cards.
    col = col.child(
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(12.0))
            .children(filtered.into_iter().map(|preset| {
                let is_active = preset.name == active_preset;
                let preset_id = preset.name.to_string();
                let p_bg = rgb(preset.background);
                let p_primary = rgb(preset.primary);
                let p_accent = rgb(preset.accent);
                let p_destructive = rgb(preset.destructive);
                let p_fg = rgb(preset.foreground);
                let p_muted_fg = rgb(preset.muted_foreground);
                let label = preset
                    .label
                    .trim_end_matches(" Dark")
                    .trim_end_matches(" Light")
                    .to_string();

                div()
                    .relative()
                    .w(px(204.0))
                    .p(px(8.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(if is_active { primary } else { border })
                    .bg(if is_active { muted } else { card })
                    .cursor_pointer()
                    .hover(|s| s.border_color(p_muted_fg))
                    // Mini preview swatch (h-14 / 56px): 3 dots + 2 bars.
                    .child(
                        div()
                            .h(px(56.0))
                            .w_full()
                            .rounded(px(6.0))
                            .p(px(8.0))
                            .flex()
                            .flex_col()
                            .justify_between()
                            .bg(p_bg)
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap(px(4.0))
                                    .child(div().size(px(8.0)).rounded_full().bg(p_primary))
                                    .child(div().size(px(8.0)).rounded_full().bg(p_accent))
                                    .child(div().size(px(8.0)).rounded_full().bg(p_destructive)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_end()
                                    .gap(px(4.0))
                                    .child(div().w(px(110.0)).h(px(4.0)).rounded(px(2.0)).bg(p_fg))
                                    .child(div().w(px(40.0)).h(px(4.0)).rounded(px(2.0)).bg(p_muted_fg)),
                            ),
                    )
                    // Label row.
                    .child(
                        div()
                            .mt(px(6.0))
                            .px(px(4.0))
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if is_active { fg } else { muted_fg })
                                    .child(label),
                            ),
                    )
                    // Active check overlay (top-right absolute).
                    .children(if is_active {
                        Some(
                            div()
                                .absolute()
                                .top(px(12.0))
                                .right(px(12.0))
                                .child(
                                    svg()
                                        .data(CHECK_SVG)
                                        .size(px(14.0))
                                        .text_color(primary),
                                ),
                        )
                    } else {
                        None
                    })
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                        this.set_theme_preset(&preset_id, cx);
                    }))
                    .into_any_element()
            })),
    );

    col.into_any_element()
}

/// One filter toggle button; active state follows settings.theme_mode_filter.
#[allow(clippy::too_many_arguments)]
fn render_filter_button(
    value: &str,
    label: &str,
    active: &str,
    muted: Rgba,
    muted_fg: Rgba,
    primary: Rgba,
    card_fg: Rgba,
    cx: &mut Context<AppState>,
) -> impl IntoElement {
    let is_active = active == value;
    let value_owned = value.to_string();
    div()
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .bg(if is_active { primary } else { muted })
        .text_color(if is_active { card_fg } else { muted_fg })
        .child(label.to_string())
        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
            this.set_theme_mode_filter(&value_owned, cx);
        }))
        .into_any_element()
}

/// Terminal section: font steppers + scrollback stepper + TUI default toggle.
fn render_terminal(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let preset_name = app.settings.theme_preset.clone();
    let card = crate::theme::preset_card(&preset_name);
    let fg = crate::theme::preset_fg(&preset_name);
    let muted_fg = crate::theme::preset_muted_fg(&preset_name);
    let border = crate::theme::preset_border(&preset_name);
    let muted = crate::theme::preset_muted(&preset_name);
    let primary = crate::theme::preset_primary(&preset_name);
    let card_fg = crate::theme::preset_card_fg(&preset_name);

    let font_family = app.settings.font_family.clone();
    let font_size = app.settings.font_size;
    let line_height = app.settings.line_height;
    let scrollback = app.settings.scrollback_lines;
    let tui_default = app.settings.tui_scroll_default;

    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .w_full()
        .child(
            div()
                .text_base()
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg)
                .child("Terminal"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .rounded(px(8.0))
                .border_1()
                .border_color(border)
                .bg(card)
                .px(px(16.0))
                .py(px(12.0))
                // Honest single-family label (D8): embedded JetBrains Mono,
                // free text falls back — no font-stack parsing.
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(fg)
                                .child("Font family (embedded: JetBrains Mono)"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(muted_fg)
                                        .child(font_family),
                                )
                                .child(
                                    div()
                                        .px(px(12.0))
                                        .py(px(6.0))
                                        .rounded(px(6.0))
                                        .border_1()
                                        .border_color(border)
                                        .text_xs()
                                        .cursor_pointer()
                                        .text_color(fg)
                                        .hover(|s| s.bg(muted))
                                        .child("Reset to JetBrains Mono")
                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                            this.set_terminal_font_family("JetBrains Mono", cx);
                                        })),
                                ),
                        ),
                )
                .child(render_stepper_row(
                    "Font size",
                    &format!("{:.0}", font_size),
                    muted,
                    fg,
                    border,
                    cx,
                    {
                        let down = clamp_step(font_size - 1.0, 8.0, 32.0);
                        let up = clamp_step(font_size + 1.0, 8.0, 32.0);
                        (down, up, StepKind::FontSize)
                    },
                ))
                .child(render_stepper_row(
                    "Line height",
                    &format!("{:.2}", line_height),
                    muted,
                    fg,
                    border,
                    cx,
                    {
                        let down = clamp_step(line_height - 0.05, 1.0, 2.0);
                        let up = clamp_step(line_height + 0.05, 1.0, 2.0);
                        (down, up, StepKind::LineHeight)
                    },
                ))
                .child(render_stepper_row(
                    "Scrollback lines",
                    &format!("{}", scrollback),
                    muted,
                    fg,
                    border,
                    cx,
                    {
                        let down = scrollback.saturating_sub(100).max(100);
                        let up = (scrollback + 100).min(50_000);
                        (down as f32, up as f32, StepKind::Scrollback)
                    },
                ))
                // TUI-scroll default toggle (hand-rolled; Switch lands with
                // the kill-switch rows in 06-02).
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(fg)
                                        .child("TUI scroll by default"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child("New panes send PageUp/PageDown to full-screen apps."),
                                ),
                        )
                        .child(
                            div()
                                .px(px(12.0))
                                .py(px(6.0))
                                .rounded_full()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .cursor_pointer()
                                .bg(if tui_default { primary } else { muted })
                                .text_color(card_fg)
                                .child(if tui_default { "On" } else { "Off" }.to_string())
                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    let next = !this.settings.tui_scroll_default;
                                    this.set_tui_scroll_default(next, cx);
                                })),
                        ),
                ),
        )
        .into_any_element()
}

fn clamp_step(v: f32, lo: f32, hi: f32) -> f32 {
    v.clamp(lo, hi)
}

#[derive(Clone, Copy)]
enum StepKind {
    FontSize,
    LineHeight,
    Scrollback,
}

/// Numeric stepper row: label + `- value +` with FE clamps applied on click
/// (immediate apply + persist through the AppState setters, D9).
fn render_stepper_row(
    label: &str,
    value: &str,
    muted: Rgba,
    fg: Rgba,
    border: Rgba,
    cx: &mut Context<AppState>,
    step: (f32, f32, StepKind),
) -> impl IntoElement {
    let (down, up, kind) = step;
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .child(div().text_sm().text_color(fg).child(label.to_string()))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(step_button("-", muted, fg, border, kind, down, cx))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(fg)
                        .child(value.to_string()),
                )
                .child(step_button("+", muted, fg, border, kind, up, cx)),
        )
        .into_any_element()
}

fn step_button(
    glyph: &str,
    muted: Rgba,
    fg: Rgba,
    border: Rgba,
    kind: StepKind,
    target: f32,
    cx: &mut Context<AppState>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(28.0))
        .h(px(28.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(border)
        .cursor_pointer()
        .text_color(fg)
        .hover(|s| s.bg(muted))
        .child(glyph.to_string())
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| match kind {
                StepKind::FontSize => this.set_terminal_font_size(target, cx),
                StepKind::LineHeight => this.set_terminal_line_height(target, cx),
                StepKind::Scrollback => this.set_scrollback_lines(target as usize, cx),
            }),
        )
        .into_any_element()
}

/// 06-02 anchor: tmux-binary validation rows land here (D6).
fn render_binary_anchor(muted_fg: Rgba) -> impl IntoElement {
    div()
        .text_xs()
        .text_color(muted_fg)
        .child("tmux binary validation arrives in 06-02 (POST /api/tmux/binary).")
        .into_any_element()
}

/// 06-02 anchor: kill-confirm switch rows land here (D7).
fn render_kill_anchor(muted_fg: Rgba) -> impl IntoElement {
    div()
        .text_xs()
        .text_color(muted_fg)
        .child("Kill-confirmation switches arrive in 06-02.")
        .into_any_element()
}

fn render_back_button(fg: Rgba, border: Rgba, hover: Rgba, cx: &mut Context<AppState>) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .px(px(16.0))
        .py(px(8.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(border)
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .text_color(fg)
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child("Back to sessions")
        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
            this.showing_settings = false;
            cx.notify();
        }))
        .into_any_element()
}
