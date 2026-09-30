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
//! - tmux-binary validation rows ship in 06-02 Task 1 (SET-03 per D6);
//!   the three kill-confirm `Switch` rows ship in Task 3 (SET-04 per D7).
//! - One preset read per render, no cached colors: every token goes through
//!   the `theme::preset_*` helpers so `cx.notify()` repaints everything.
//! - FE copy preserved verbatim: `"Filter by dark or light appearance"`,
//!   `"{n} themes available"`,
//!   `"The selected theme is applied to every pane and app surface."`

use gpui::*;
use gpui::prelude::{InteractiveElement, StatefulInteractiveElement};
use gpui_component::input::Input;
use gpui_component::scroll::{Scrollbar, ScrollbarMode};
use gpui_component::switch::Switch;
use webtmux_backend_client::binary_status_copy;
use crate::app_state::{AppState, KillConfirmKind};
use crate::glass::{Elevation, GlassTier, INTENSITY_STEPS};
use crate::icons::{CHECK_SVG, CHEVRON_DOWN_SVG, PAINTBRUSH_SVG, TERMINAL_SQUARE_SVG};
use crate::themes_generated::{UI_THEMES, UiThemePreset};

/// Theme grid layout: fixed-width columns and fixed-height rows so every row
/// measures identically for uniform_list virtualization — only visible rows
/// are built, no matter the preset count (web-term parity).
const THEME_GRID_COLS: usize = 3;
const THEME_CARD_W: f32 = 204.0;
const THEME_CARD_H: f32 = 92.0;
/// Card height + 12px row gap.
const THEME_ROW_H: f32 = 104.0;
/// Four rows visible; the rest scrolls inside the grid region.
const THEME_GRID_H: f32 = 416.0;

/// Monospace families offered by the terminal font selector (web-term
/// parity). The chosen name is applied verbatim; anything not installed
/// falls back per the platform font stack.
const MONO_FONTS: &[&str] = &[
    "Geist Mono",
    "JetBrains Mono",
    "Fira Code",
    "Source Code Pro",
    "IBM Plex Mono",
    "Cascadia Code",
    "Inconsolata",
    "Ubuntu Mono",
    "Menlo",
    "Consolas",
    "Monaco",
    "monospace",
];

/// Render the full settings page behind the `showing_settings` route.
pub fn render_settings(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let preset_name = app.settings.theme_preset.clone();
    let bg = crate::theme::preset_bg(&preset_name);
    let fg = crate::theme::preset_fg(&preset_name);
    let border = crate::theme::preset_border(&preset_name);
    let muted = crate::theme::preset_muted(&preset_name);
    let primary = crate::theme::preset_primary(&preset_name);

    // Persistent scroll state driving the visual scrollbar overlay below.
    let scroll_handle = app.settings_scroll.clone();

    // Outer relative container: scroll area + floating scrollbar overlay
    // (web-term parity).
    div()
        .relative()
        .size_full()
        .overflow_hidden()
        .child(
            div()
                .id("settings-scroll")
                .flex()
                .flex_col()
                .items_center()
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&scroll_handle)
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
                        .child(
                            div()
                                .text_2xl()
                                .font_weight(FontWeight::BOLD)
                                .text_color(fg)
                                .child("Settings"),
                        )
                        .child(render_appearance(app, cx))
                        .child(render_window(app, cx))
                        .child(render_terminal(app, cx))
                        .child(render_kill_switches(app, cx))
                        .into_any_element(),
                )
                .text_color(fg)
                .border_color(border),
        )
        .child(
            Scrollbar::vertical(&scroll_handle)
                .mode(ScrollbarMode::Always)
                .styles(|s| {
                    s.track(|t| t.bg(Hsla::from(rgba(0x00000000))))
                        .thumb(|th| {
                            th.bg(Hsla::from(muted.opacity(0.35)))
                                .radius(px(3.0))
                                .width(px(6.0))
                        })
                        .thumb_hover(|th| {
                            th.bg(Hsla::from(muted.opacity(0.65)))
                                .radius(px(4.0))
                                .width(px(8.0))
                        })
                        .thumb_active(|th| {
                            th.bg(Hsla::from(primary.opacity(0.8)))
                                .radius(px(4.0))
                                .width(px(8.0))
                        })
                }),
        )
        .into_any_element()
}

/// Window section: the Liquid Glass material switch + fill-intensity presets +
/// what the compositor can actually do with it.
///
/// The material runs along one axis only — on/off and how opaque the fill is —
/// because the alpha table in `crate::glass` already fixes the relationship
/// between the chrome tier (over the desktop) and the overlay tier (over this
/// app's own output). Exposing those two alphas separately would let a caller
/// put a see-through panel over terminal text, which is exactly the failure the
/// split exists to prevent.
fn render_window(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let preset_name = app.settings.theme_preset.clone();
    let card = crate::theme::preset_card(&preset_name);
    let fg = crate::theme::preset_fg(&preset_name);
    let muted = crate::theme::preset_muted(&preset_name);
    let muted_fg = crate::theme::preset_muted_fg(&preset_name);
    let border = crate::theme::preset_border(&preset_name);
    let primary = crate::theme::preset_primary(&preset_name);
    let primary_fg = crate::theme::preset_primary_fg(&preset_name);

    let enabled = app.glass_enabled();
    let opacity = app.glass_opacity();
    let app_weak = cx.weak_entity();

    // Row wording follows the two settings that exist, not the mechanism: the
    // switch is the material, the presets are its alpha.
    let backdrop_hint = if crate::glass::backdrop_blur_available() {
        "This compositor frosts the desktop behind the window."
    } else {
        "No backdrop blur on this compositor, so the glass is a translucent tint."
    };

    let intensity = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .children(INTENSITY_STEPS.iter().map(|(label, value)| {
            let value = *value;
            let active = (value - opacity).abs() < 0.001;
            div()
                .id(ElementId::Name(
                    format!("settings/glass/{label}").into(),
                ))
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(6.0))
                .bg(if active { primary } else { muted })
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(if active { primary_fg } else { fg })
                .cursor_pointer()
                .hover(|s| s.opacity(0.9))
                .child(*label)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _window, cx| {
                        this.set_glass_opacity(value, cx);
                    }),
                )
        }));

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .w_full()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(muted_fg)
                .child("WINDOW"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .rounded(px(8.0))
                .border_1()
                .border_color(border)
                // The card is opaque like every other row on this page: the
                // material applies to the window chrome, never to content.
                .bg(card)
                // Row 1: the material itself.
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .px(px(16.0))
                        .py(px(12.0))
                        .border_b_1()
                        .border_color(border)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(fg)
                                        .child("Liquid Glass"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child("Translucent title bar, sidebar, dialogs and menus"),
                                ),
                        )
                        .child(
                            Switch::new("glass-enabled")
                                .checked(enabled)
                                .on_click(move |_, window, cx: &mut App| {
                                    if let Some(app) = app_weak.upgrade() {
                                        app.update(cx, |this, cx| {
                                            // Flip the CURRENT value rather than
                                            // trusting the passed bool, so either
                                            // `on_click` semantic converges (same
                                            // as the kill rows below).
                                            let next = !this.glass_enabled();
                                            this.set_glass_enabled(next, window, cx);
                                        });
                                    }
                                }),
                        ),
                )
                // Row 2: how opaque the fill is.
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .px(px(16.0))
                        .py(px(12.0))
                        .border_b_1()
                        .border_color(border)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(fg)
                                        .child("Glass intensity"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child("Higher keeps more of the window opaque"),
                                ),
                        )
                        .child(intensity),
                )
                // Row 3: what this session can actually do with it, so the gap
                // between "translucent" and "frosted" is visible where the
                // setting lives instead of in a commit message.
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .px(px(16.0))
                        .py(px(12.0))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(fg)
                                        .child("Backdrop"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child(backdrop_hint),
                                ),
                        ),
                ),
        )
        .into_any_element()
}

/// "Desktop (GTK)" card — the fourth theme mode.
///
/// The preview and caption read the **cached** desktop palette
/// (`gtk_theme::cached_palette` / `cached_name`), never GTK itself: render paths
/// must stay I/O-free and GTK is not thread-safe. Picking the card installs the
/// probed palette app-wide; picking a preset card below leaves the mode.
fn render_desktop_theme_card(app: &mut AppState, cx: &mut Context<AppState>) -> AnyElement {
    let preset_name = app.settings.theme_preset.clone();
    let card = crate::theme::preset_card(&preset_name);
    let fg = crate::theme::preset_fg(&preset_name);
    let muted_fg = crate::theme::preset_muted_fg(&preset_name);
    let border = crate::theme::preset_border(&preset_name);
    let primary = crate::theme::preset_primary(&preset_name);
    let muted = crate::theme::preset_muted(&preset_name);

    let is_active = app.settings.theme.is_gtk();
    let available = crate::gtk_theme::is_available();
    let cached = crate::gtk_theme::cached_palette();
    let theme_name = crate::gtk_theme::cached_name();
    let user_css = crate::gtk_theme::user_css_active();

    // Preview swatches: the probed desktop palette when one exists, else the
    // active preset's tokens so the card is never blank.
    let (sw_bg, sw_fg, sw_accent, sw_border) = match cached {
        Some(p) => (p.bg_primary, p.text_primary, p.accent, p.border),
        None => (
            crate::theme::preset_bg(&preset_name),
            crate::theme::preset_fg(&preset_name),
            crate::theme::preset_primary(&preset_name),
            crate::theme::preset_border(&preset_name),
        ),
    };

    let caption = if !available {
        "GTK theme not detected — falls back to the dark preset.".to_string()
    } else if let Some(name) = theme_name {
        let source = if user_css { "user CSS" } else { "GTK theme" };
        format!("Following {name} ({source})")
    } else {
        "GTK theme not detected — falls back to the dark preset.".to_string()
    };

    div()
        .id("settings/theme-gtk")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .w_full()
        .rounded(px(8.0))
        .border_1()
        .border_color(if is_active { primary } else { border })
        .bg(if is_active { muted } else { card })
        .p(px(12.0))
        .cursor_pointer()
        .hover(|s| s.border_color(primary))
        // Desktop preview: window color, foreground bar, accent + border dots.
        .child(
            div()
                .flex()
                .flex_row()
                .items_end()
                .gap(px(4.0))
                .w(px(72.0))
                .h(px(48.0))
                .flex_shrink_0()
                .rounded(px(6.0))
                .border_1()
                .border_color(sw_border)
                .bg(sw_bg)
                .p(px(6.0))
                .child(div().w(px(10.0)).h(px(18.0)).rounded(px(3.0)).bg(sw_fg))
                .child(div().size(px(10.0)).rounded(px(3.0)).bg(sw_accent))
                .child(div().size(px(10.0)).rounded(px(3.0)).bg(sw_border)),
        )
        .child(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(fg)
                        .child("Desktop (GTK)"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(muted_fg)
                        .truncate()
                        .child(caption),
                ),
        )
        .children(if is_active {
            Some(svg().data(CHECK_SVG).size(px(16.0)).text_color(primary))
        } else {
            None
        })
        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
            this.set_theme(webtmux_settings::Theme::Gtk, cx);
        }))
        .into_any_element()
}

/// Appearance section: header + filter row + count + card grid.
fn render_appearance(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let preset_name = app.settings.theme_preset.clone();
    let card = crate::theme::preset_card(&preset_name);
    let fg = crate::theme::preset_fg(&preset_name);
    let muted = crate::theme::preset_muted(&preset_name);
    let muted_fg = crate::theme::preset_muted_fg(&preset_name);
    let border = crate::theme::preset_border(&preset_name);
    let primary = crate::theme::preset_primary(&preset_name);

    let filter = app.settings.theme_mode_filter.clone();

    let filtered: Vec<_> = UI_THEMES
        .iter()
        .filter(|t| match filter.as_str() {
            "dark" => t.is_dark,
            "light" => !t.is_dark,
            _ => true,
        })
        .collect();
    let count = filtered.len();
    let theme_row_count = (filtered.len() + THEME_GRID_COLS - 1) / THEME_GRID_COLS;

    // Cloned up front: the handle is shared into builders below.
    let themes_handle = app.settings_themes_scroll.clone();

    let theme_mode_label = match filter.as_str() {
        "dark" => "Dark",
        "light" => "Light",
        _ => "All themes",
    };
    let show_theme_picker = app.show_theme_mode_picker;
    // Liquid Glass: the dropdown popovers float over the page, so they take the
    // overlay tier. Everything else on this page is content and stays opaque.
    let picker_glass = app.glass_style(card, GlassTier::Overlay, Elevation::Lg);

    let mut col = div().flex().flex_col().gap(px(16.0)).w_full();

    // Section label (FE SettingsPage parity).
    col = col.child(
        div()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .text_color(muted_fg)
            .child("APPEARANCE"),
    );

    // "Desktop (GTK)" — the fourth theme mode, alongside Dark/Light/System.
    // Its preview shows the desktop's own colors.
    col = col.child(render_desktop_theme_card(app, cx));

    // Appearance card (FE UiThemeSettings parity). Row 1 is the Theme-mode
    // row; the filter buttons stand in for the web Select dropdown (no
    // select widget port per D4). Row 2 holds Color theme + grid.
    col = col.child(
        div()
            .flex()
            .flex_col()
            .rounded(px(8.0))
            .border_1()
            .border_color(border)
            .bg(card)
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(12.0))
                            .child(
                                svg()
                                    .data(PAINTBRUSH_SVG)
                                    .size(px(16.0))
                                    .text_color(muted_fg),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(fg)
                                            .child("Theme mode"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted_fg)
                                            .child("Filter by dark or light appearance"),
                                    ),
                            ),
                    )
                    .child(
                div()
                    .id("settings/theme-mode")
                    .w(px(110.0))
                    .h(px(32.0))
                    .px_3()
                    .rounded_md()
                    .border_1()
                    .border_color(border)
                    .bg(muted)
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .hover(|s| s.bg(border))
                    .child(
                        div()
                            .text_xs()
                            .text_color(fg)
                            .child(theme_mode_label),
                    )
                    .child(
                        svg()
                            .data(CHEVRON_DOWN_SVG)
                            .size(px(12.0))
                            .text_color(muted_fg),
                    )
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, cx| {
                        this.show_theme_mode_picker = !this.show_theme_mode_picker;
                        cx.notify();
                    })),
                    )
                    // Dropdown popover (deferred: paints above the rows below,
                    // which would otherwise cover it).
                    .children(if show_theme_picker {
                        Some(deferred(
                            div()
                                .absolute()
                                .top(px(46.0))
                                .right(px(16.0))
                                .w(px(110.0))
                                .rounded_md()
                                .border_1()
                                .border_color(picker_glass.border)
                                .bg(picker_glass.fill)
                                .shadow(picker_glass.shadows)
                                .py_1()
                                .child(
                                    div()
                                        .id("settings/theme-mode/all")
                                        .px_3()
                                        .py_1p5()
                                        .text_xs()
                                        .text_color(if filter == "all" { primary } else { fg })
                                        .cursor_pointer()
                                        .hover(|s| s.bg(muted))
                                        .child("All themes")
                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, cx| {
                                            this.set_theme_mode_filter("all", cx);
                                        })),
                                )
                                .child(
                                    div()
                                        .id("settings/theme-mode/dark")
                                        .px_3()
                                        .py_1p5()
                                        .text_xs()
                                        .text_color(if filter == "dark" { primary } else { fg })
                                        .cursor_pointer()
                                        .hover(|s| s.bg(muted))
                                        .child("Dark")
                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, cx| {
                                            this.set_theme_mode_filter("dark", cx);
                                        })),
                                )
                                .child(
                                    div()
                                        .id("settings/theme-mode/light")
                                        .px_3()
                                        .py_1p5()
                                        .text_xs()
                                        .text_color(if filter == "light" { primary } else { fg })
                                        .cursor_pointer()
                                        .hover(|s| s.bg(muted))
                                        .child("Light")
                                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _window, cx| {
                                            this.set_theme_mode_filter("light", cx);
                                        })),
                                ),
                        ))
                    } else {
                        None
                    }),
            )
            // Row 2: Color theme + count + grid.
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .px(px(16.0))
                    .py(px(12.0))
                    .child(
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
            )
            // Virtualized theme grid: presets are chunked into fixed-height
            // rows of 3 rendered through uniform_list, so only visible rows
            // are built no matter how many presets exist. Four rows visible;
            // the rest scrolls inside the grid region (web-term parity).
            .child(
                div()
                    .relative()
            .w_full()
            .h(px(THEME_GRID_H))
            .overflow_hidden()
            // gpui hands a wheel event to every scrollable under the cursor
            // (its scroll listener never stops propagation), so the page
            // behind would scroll along with this grid. Hold the wheel here
            // while the grid still has room, then hand it back to the page
            // at either end.
            .on_scroll_wheel(cx.listener(
                |this, event: &ScrollWheelEvent, window, cx| {
                    let (offset, max_offset) = {
                        let scroll = this.settings_themes_scroll.0.borrow();
                        let base = &scroll.base_handle;
                        (base.offset(), base.max_offset())
                    };
                    let delta_y = event.delta.pixel_delta(window.line_height()).y;
                    // offset.y runs from 0 (top) to -max_offset.y (bottom).
                    let has_room = (delta_y < Pixels::ZERO && offset.y > -max_offset.y)
                        || (delta_y > Pixels::ZERO && offset.y < Pixels::ZERO);
                    if has_room {
                        cx.stop_propagation();
                    }
                },
            ))
            .child(
                uniform_list(
                    "settings-theme-grid",
                    theme_row_count,
                    cx.processor(
                        |this: &mut AppState,
                         range: std::ops::Range<usize>,
                         _window: &mut Window,
                         cx: &mut Context<AppState>| {
                            let active = this.settings.theme_preset.clone();
                            // In "Desktop (GTK)" mode no preset row is the
                            // active one: the GTK card above owns the check.
                            let gtk_mode = this.settings.theme.is_gtk();
                            let mode = this.settings.theme_mode_filter.clone();
                            let preset_name = active.clone();
                            let card = crate::theme::preset_card(&preset_name);
                            let fg = crate::theme::preset_fg(&preset_name);
                            let muted = crate::theme::preset_muted(&preset_name);
                            let muted_fg = crate::theme::preset_muted_fg(&preset_name);
                            let border = crate::theme::preset_border(&preset_name);
                            let primary = crate::theme::preset_primary(&preset_name);
                            let presets: Vec<_> = UI_THEMES
                                .iter()
                                .filter(|p| match mode.as_str() {
                                    "dark" => p.is_dark,
                                    "light" => !p.is_dark,
                                    _ => true,
                                })
                                .collect();
                            let rows: Vec<&[_]> = presets.chunks(THEME_GRID_COLS).collect();
                            range
                                .map(|row_ix| {
                                    let row: &[&UiThemePreset] =
                                        rows.get(row_ix).copied().unwrap_or(&[]);
                                    div()
                                        .h(px(THEME_ROW_H))
                                        .flex()
                                        .flex_row()
                                        .gap(px(12.0))
                                        .children(row.iter().enumerate().map(
                                            |(col_ix, preset)| {
                                                let is_active = preset.name == active && !gtk_mode;
                                                let preset_id = preset.name;
                                                let global_ix =
                                                    row_ix * THEME_GRID_COLS + col_ix;
                                                let p_bg = rgb(preset.background);
                                                let p_primary = rgb(preset.primary);
                                                let p_accent = rgb(preset.accent);
                                                let p_destructive = rgb(preset.destructive);
                                                let p_muted_fg = rgb(preset.muted_foreground);
                                                let label = preset
                                                    .label
                                                    .trim_end_matches(" Dark")
                                                    .trim_end_matches(" Light")
                                                    .to_string();

                                                // FE ThemeCard parity: h-14 swatch with 3
                                                // accent squares bottom-left; label row
                                                // with inline check when selected.
                                                div()
                                                    .relative()
                                                    .w(px(THEME_CARD_W))
                                                    .h(px(THEME_CARD_H))
                                                    .overflow_hidden()
                                                    .rounded(px(8.0))
                                                    .border_1()
                                                    .border_color(if is_active { primary } else { border })
                                                    .bg(if is_active { muted } else { card })
                                                    .cursor_pointer()
                                                    .id(ElementId::NamedInteger(
                                                        "settings-preset".into(),
                                                        global_ix as u64,
                                                    ))
                                                    .hover(|s| s.border_color(p_muted_fg))
                                                    .child(
                                                        div()
                                                            .h(px(56.0))
                                                            .w_full()
                                                            .flex()
                                                            .flex_row()
                                                            .items_end()
                                                            .gap(px(4.0))
                                                            .p(px(8.0))
                                                            .bg(p_bg)
                                                            .child(div().size(px(12.0)).rounded(px(3.0)).bg(p_primary))
                                                            .child(div().size(px(12.0)).rounded(px(3.0)).bg(p_accent))
                                                            .child(div().size(px(12.0)).rounded(px(3.0)).bg(p_destructive)),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_row()
                                                            .items_center()
                                                            .justify_between()
                                                            .gap(px(4.0))
                                                            .px(px(8.0))
                                                            .py(px(6.0))
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .min_w_0()
                                                                    .truncate()
                                                                    .text_xs()
                                                                    .font_weight(FontWeight::MEDIUM)
                                                                    .text_color(if is_active { fg } else { muted_fg })
                                                                    .child(label),
                                                            )
                                                            .children(if is_active {
                                                                Some(
                                                                    svg()
                                                                        .data(CHECK_SVG)
                                                                        .size(px(14.0))
                                                                        .text_color(primary),
                                                                )
                                                            } else {
                                                                None
                                                            }),
                                                    )
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(move |this, _, _, cx| {
                                                            this.set_theme_preset(preset_id, cx);
                                                        }),
                                                    )
                                                    .into_any_element()
                                            },
                                        ))
                                        // Fillers keep a short last row aligned.
                                        .children((row.len()..THEME_GRID_COLS).map(|_| {
                                            div().w(px(THEME_CARD_W)).h(px(THEME_CARD_H))
                                        }))
                                })
                                .collect()
                        },
                    ),
                )
                .w_full()
                .h_full()
                .track_scroll(&themes_handle),
            )
            .child(
                Scrollbar::vertical(&themes_handle)
                    .mode(ScrollbarMode::Hover)
                    .styles(|s| {
                        s.track(|t| t.bg(Hsla::from(rgba(0x00000000))))
                            .thumb(|th| {
                                th.bg(Hsla::from(muted_fg.opacity(0.35)))
                                    .radius(px(3.0))
                                    .width(px(6.0))
                            })
                            .thumb_hover(|th| {
                                th.bg(Hsla::from(muted_fg.opacity(0.65)))
                                    .radius(px(4.0))
                                    .width(px(8.0))
                            })
                            .thumb_active(|th| {
                                th.bg(Hsla::from(primary.opacity(0.8)))
                                    .radius(px(4.0))
                                    .width(px(8.0))
                            })
                    }),
            ),
                ),
            ),
    );

    col.into_any_element()
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

    let font_family = app.settings.font_family.clone();
    let font_size = app.settings.font_size;
    let line_height = app.settings.line_height;
    let scrollback = app.settings.scrollback_lines;
    let tui_default = app.settings.tui_scroll_default;
    let show_font_picker = app.show_font_picker;
    let app_weak = cx.weak_entity();
    // Liquid Glass: the font dropdown is the second floating popover on this
    // page, so it matches the theme-mode one.
    let picker_glass = app.glass_style(card, GlassTier::Overlay, Elevation::Lg);

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .w_full()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(muted_fg)
                .child("TERMINAL"),
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
                // Card header (FE TerminalSettings parity).
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(12.0))
                        .child(
                            svg()
                                .data(TERMINAL_SQUARE_SVG)
                                .size(px(16.0))
                                .text_color(muted_fg),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(fg)
                                        .child("Terminal preferences"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child("The selected theme is applied to every pane and app surface."),
                                ),
                        ),
                )
                // tmux-binary validation first (FE TerminalSettings order,
                // SET-03 per D6); explicit Check/Enter only.
                .child(render_binary_block(app, cx))
                // Font family selector (web-term parity): dropdown button with
                // the current family + popover list. Replaces the old Reset
                // button.
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
                                .child("Font family"),
                        )
                        .child(
                            div()
                                .relative()
                                .child(
                                    div()
                                        .id("settings/font-family")
                                        .w_full()
                                        .h(px(32.0))
                                        .px_3()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(border)
                                        .bg(muted)
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(border))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(fg)
                                                .child(font_family.clone()),
                                        )
                                        .child(
                                            svg()
                                                .data(CHEVRON_DOWN_SVG)
                                                .size(px(12.0))
                                                .text_color(muted_fg),
                                        )
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, _, _window, cx| {
                                                this.show_font_picker = !this.show_font_picker;
                                                cx.notify();
                                            }),
                                        ),
                                )
                                .children(if show_font_picker {
                                    Some(deferred(
                                        div()
                                            .absolute()
                                            .top(px(36.0))
                                            .left_0()
                                            .right_0()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(picker_glass.border)
                                            .bg(picker_glass.fill)
                                            .shadow(picker_glass.shadows)
                                            .py_1()
                                            .children(MONO_FONTS.iter().map(|font| {
                                                let is_active = *font == font_family;
                                                let font_id = font.to_string();
                                                div()
                                                    .id(format!("settings/font/{}", font))
                                                    .px_3()
                                                    .py_1p5()
                                                    .text_xs()
                                                    .text_color(if is_active { primary } else { fg })
                                                    .cursor_pointer()
                                                    .hover(|s| s.bg(muted))
                                                    .child(font_id.clone())
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(move |this, _, _, cx| {
                                                            this.set_terminal_font_family(&font_id, cx);
                                                        }),
                                                    )
                                                    .into_any_element()
                                            })),
                                    ))
                                } else {
                                    None
                                }),
                        ),
                )
                .child(render_stepper_row(
                    "Font size",
                    &format!("{:.0}", font_size),
                    muted,
                    fg,
                    border,
                    "settings-step/font-size",
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
                    "settings-step/line-height",
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
                    "settings-step/scrollback",
                    cx,
                    {
                        let down = scrollback.saturating_sub(100).max(100);
                        let up = (scrollback + 100).min(50_000);
                        (down as f32, up as f32, StepKind::Scrollback)
                    },
                ))
                // TUI-scroll default toggle: the same functional `Switch`
                // as the Safety rows below.
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
                            Switch::new("tui-scroll-default")
                                .checked(tui_default)
                                .on_click(move |_, _, cx: &mut App| {
                                    if let Some(app) = app_weak.upgrade() {
                                        app.update(cx, |this, cx| {
                                            let next = !this.settings.tui_scroll_default;
                                            this.set_tui_scroll_default(next, cx);
                                        });
                                    }
                                }),
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
    id_base: &'static str,
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
                .child(step_button("-", muted, fg, border, kind, down, format!("{id_base}/down"), cx))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(fg)
                        .child(value.to_string()),
                )
                .child(step_button("+", muted, fg, border, kind, up, format!("{id_base}/up"), cx)),
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
    id: String,
    cx: &mut Context<AppState>,
) -> impl IntoElement {
    div()
        .id(id)
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

/// tmux-binary validation block (SET-03 per D6, 06-02): path input +
/// explicit Check button + status line. Typing only edits the draft held by
/// the `InputState` editor — validation POSTs on Check/Enter only (never
/// per-keystroke). Status shows `Using {binary} ({version})` or the raw
/// backend error via `binary_status_copy`.
fn render_binary_block(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let preset_name = app.settings.theme_preset.clone();
    let fg = crate::theme::preset_fg(&preset_name);
    let muted_fg = crate::theme::preset_muted_fg(&preset_name);
    let border = crate::theme::preset_border(&preset_name);
    let primary = crate::theme::preset_primary(&preset_name);
    let primary_fg = crate::theme::preset_primary_fg(&preset_name);

    let status: Option<String> = app
        .tmux_binary_status
        .as_ref()
        .map(binary_status_copy);
    let editor = app.tmux_binary_input.clone();
    let editor_for_check = editor.clone();

    let mut col = div().flex().flex_col().gap(px(6.0)).w_full();

    col = col.child(
        div()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .text_color(fg)
            .child("tmux binary (Windows)"),
    );

    // Input row: editor (flex-1) + explicit Check button. The editor entity
    // is ensured on the settings render path; the fallback text below only
    // covers the impossible first-frame gap.
    col = col.child(
        div().flex().flex_row().items_center().gap(px(8.0)).w_full()
            .child(
                div().flex_1().children(editor.clone().map(|state| {
                    Input::new(&state).w_full().into_any_element()
                })),
            )
            .child(
                div()
                    .id("settings/check-binary")
                    .px(px(14.0))
                    .py(px(6.0))
                    .rounded(px(6.0))
                    .bg(primary)
                    .text_color(primary_fg)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.9))
                    .child("Check")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            let Some(state) = editor_for_check.clone() else {
                                return;
                            };
                            let value = state.read(cx).value().to_string();
                            this.set_tmux_binary_path(value, cx);
                            this.check_tmux_binary(cx);
                        }),
                    ),
            ),
    );

    // FE copy preserved verbatim (TerminalSettings.tsx).
    col = col.child(
        div()
            .text_xs()
            .text_color(muted_fg)
            .child("Choose the same tmux installation that contains your sessions. You can paste a full path or use tmux for PATH lookup."),
    );

    if editor.is_none() {
        col = col.child(
            div()
                .text_xs()
                .text_color(muted_fg)
                .child(format!("Current: {}", app.settings.tmux_binary)),
        );
    }

    if let Some(line) = status {
        col = col.child(
            div()
                .text_xs()
                .text_color(muted_fg)
                .child(line)
                .border_color(border),
        );
    }

    col.into_any_element()
}

/// Kill-confirmation switches (SET-04 per D7, 06-02 Task 3): the three
/// `Switch` rows with FE labels verbatim (`TerminalSettings.tsx:91-108`),
/// persisting through the existing `save()` path. Toggling flips the next
/// kill flow immediately (dialog vs direct) — the gates read settings live,
/// no restart, no dialog-behavior change.
fn render_kill_switches(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let preset_name = app.settings.theme_preset.clone();
    let card = crate::theme::preset_card(&preset_name);
    let fg = crate::theme::preset_fg(&preset_name);
    let border = crate::theme::preset_border(&preset_name);

    let pane = app.settings.confirm_kill_pane;
    let window = app.settings.confirm_kill_window;
    let session = app.settings.confirm_kill_session;

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
                .child("Safety"),
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
                .child(render_kill_row(
                    "Confirm before killing pane",
                    "kill-confirm-pane",
                    KillConfirmKind::Pane,
                    pane,
                    fg,
                    cx,
                ))
                .child(render_kill_row(
                    "Confirm before killing window",
                    "kill-confirm-window",
                    KillConfirmKind::Window,
                    window,
                    fg,
                    cx,
                ))
                .child(render_kill_row(
                    "Confirm before killing session",
                    "kill-confirm-session",
                    KillConfirmKind::Session,
                    session,
                    fg,
                    cx,
                )),
        )
        .into_any_element()
}

/// One kill-confirm row: FE label left, functional `Switch` right. The click
/// handler flips the CURRENT settings value (rather than trusting the passed
/// bool) so either `on_click` semantic converges, then apply-then-saves
/// synchronously in the same handler.
fn render_kill_row(
    label: &str,
    id: &'static str,
    kind: KillConfirmKind,
    checked: bool,
    fg: Rgba,
    cx: &mut Context<AppState>,
) -> impl IntoElement {
    let app_weak = cx.weak_entity();
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .child(div().text_sm().text_color(fg).child(label.to_string()))
        .child(
            Switch::new(id)
                .checked(checked)
                .on_click(move |_, _, cx: &mut App| {
                    if let Some(app) = app_weak.upgrade() {
                        app.update(cx, |this, cx| {
                            let next = match kind {
                                KillConfirmKind::Pane => !this.settings.confirm_kill_pane,
                                KillConfirmKind::Window => !this.settings.confirm_kill_window,
                                KillConfirmKind::Session => !this.settings.confirm_kill_session,
                            };
                            this.set_confirm_kill(kind, next, cx);
                        });
                    }
                }),
        )
        .into_any_element()
}
