//! Theme management for web-tmux.
//!
//! Two layers live here:
//!
//! * the **preset layer** — the 102 FE-derived `UiThemePreset` rows in
//!   [`crate::themes_generated`], read through the `preset_*` helpers that every
//!   view calls per render; and
//! * the **desktop (GTK) layer** — an optional [`GtkPalette`] installed from the
//!   active GTK theme ([`crate::gtk_theme`]). While one is installed every
//!   `preset_*` helper returns the GTK value, so the whole app follows the
//!   desktop without any view knowing about GTK (mirrors the reference
//!   implementation's token override, adapted to a preset table).
//!
//! [`apply_theme`] is the single mutation point: it pushes the setting into
//! `gpui_component::Theme` (so dialogs, menus, inputs and the command palette
//! follow) and installs/clears the GTK palette for the app's own helpers.
//! `preset_*` reads are pure — GTK is only ever touched from
//! [`crate::gtk_theme`] on the main thread, never from a render path.

use gpui::{rgb, App, Hsla, Rgba};
use gpui_component::{Theme, ThemeMode};
use parking_lot::RwLock;
use webtmux_settings::Theme as SettingsTheme;

// --- Color helpers (pure; unit-tested) ------------------------------------

/// Same color with the given float alpha.
pub fn with_alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba {
        a: alpha.clamp(0.0, 1.0),
        ..color
    }
}

/// Same color with the given 0-255 alpha byte.
pub fn with_alpha_byte(color: Rgba, alpha: u8) -> Rgba {
    with_alpha(color, alpha as f32 / 255.0)
}

/// Linear interpolation between two colors (`t = 0` → `a`, `t = 1` → `b`).
pub fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

pub fn lighten(color: Rgba, amount: f32) -> Rgba {
    mix(color, rgb(0xffffff), amount)
}

pub fn darken(color: Rgba, amount: f32) -> Rgba {
    mix(color, rgb(0x000000), amount)
}

fn srgb_to_linear(component: f32) -> f32 {
    if component <= 0.04045 {
        component / 12.92
    } else {
        ((component + 0.055) / 1.055).powf(2.4)
    }
}

/// Rec. 709 relative luminance (0 = black, 1 = white).
pub fn relative_luminance(color: Rgba) -> f32 {
    0.2126 * srgb_to_linear(color.r)
        + 0.7152 * srgb_to_linear(color.g)
        + 0.0722 * srgb_to_linear(color.b)
}

/// Readable text color on top of `background` (black on light, white on dark).
pub fn contrast_text(background: Rgba) -> Rgba {
    if relative_luminance(background) > 0.45 {
        rgb(0x000000)
    } else {
        rgb(0xffffff)
    }
}

/// Opaque color from 0-255 components.
pub fn rgb8(r: u8, g: u8, b: u8) -> Rgba {
    rgb(((r as u32) << 16) | ((g as u32) << 8) | b as u32)
}

/// `0xRRGGBB` for a UI-preset token (the preset layer is byte-typed).
fn to_u32(color: Rgba) -> u32 {
    let channel = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
    (channel(color.r) << 16) | (channel(color.g) << 8) | channel(color.b)
}

// --- Desktop (GTK) palette ------------------------------------------------

/// Every token the GTK mode can override, resolved from the active GTK theme.
/// Plain data (`Copy` + `Send`) so render paths never touch GTK themselves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GtkPalette {
    pub is_dark: bool,
    pub bg_primary: Rgba,
    pub bg_secondary: Rgba,
    pub bg_tertiary: Rgba,
    pub bg_card: Rgba,
    pub bg_card_hover: Rgba,
    pub text_primary: Rgba,
    pub text_secondary: Rgba,
    pub text_tertiary: Rgba,
    pub text_disabled: Rgba,
    pub border: Rgba,
    pub border_subtle: Rgba,
    pub accent: Rgba,
    pub accent_hover: Rgba,
    pub on_accent: Rgba,
    pub error: Rgba,
    pub warning: Rgba,
    pub success: Rgba,
    pub placeholder: Rgba,
    pub close_hover: Rgba,
    pub destructive_foreground: Rgba,
    pub selection: Rgba,
    pub muted: Rgba,
    pub input: Rgba,
}

static GTK: RwLock<Option<GtkPalette>> = RwLock::new(None);

/// Install the desktop palette; every override-aware helper switches over.
pub fn install_gtk(palette: GtkPalette) {
    *GTK.write() = Some(palette);
}

/// Drop the desktop palette; helpers fall back to the preset table.
pub fn clear_gtk() {
    *GTK.write() = None;
}

pub fn gtk_active() -> bool {
    GTK.read().is_some()
}

/// Copy of the installed palette (`RwLock` keeps this allocation-free).
pub fn gtk_palette() -> Option<GtkPalette> {
    *GTK.read()
}

/// Polarity of whatever chrome is active (GTK palette when installed, else the
/// persisted preset's `is_dark`).
pub fn active_is_dark(preset_name: &str) -> bool {
    match gtk_palette() {
        Some(palette) => palette.is_dark,
        None => crate::themes_generated::ui_preset_by_name(preset_name).is_dark,
    }
}

// --- Theme application ----------------------------------------------------

/// Apply theme setting to the GPUI application context.
///
/// `Dark`/`Light`/`System` clear any desktop palette (System keeps the
/// long-standing dark fallback); `Gtk` resolves the desktop palette, installs it
/// for the app's own helpers, and pushes it into `gpui_component::Theme` so
/// dialogs, menus, inputs and the command palette follow too.
pub fn apply_theme(theme: SettingsTheme, cx: &mut App) {
    match theme {
        SettingsTheme::Dark => {
            clear_gtk();
            Theme::change(ThemeMode::Dark, None, cx);
        }
        SettingsTheme::Light => {
            clear_gtk();
            Theme::change(ThemeMode::Light, None, cx);
        }
        SettingsTheme::System => {
            // No runtime system-appearance query on this platform; the previous
            // behaviour (dark) is preserved verbatim.
            clear_gtk();
            Theme::change(ThemeMode::Dark, None, cx);
        }
        SettingsTheme::Gtk => {
            apply_gtk_theme(cx);
        }
    }
}

/// Resolve + install the desktop palette. Returns whether GTK was detected; on
/// failure the app falls back to the dark component theme and the settings page
/// shows the "not detected" note.
pub fn apply_gtk_theme(cx: &mut App) -> bool {
    match crate::gtk_theme::palette() {
        Some(palette) => {
            install_gtk(palette);
            // The mode follows the palette's *polarity*, never the theme name.
            let mode = if palette.is_dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            };
            Theme::change(mode, None, cx);
            override_component_theme(&palette, cx);
            // Writes to public theme fields only reach the scrollbar and resize
            // handles after this (gpui-component's own doc).
            Theme::sync_base(cx);
            true
        }
        None => {
            clear_gtk();
            Theme::change(ThemeMode::Dark, None, cx);
            false
        }
    }
}

/// Map the desktop palette onto `gpui_component::ThemeColor` so the component
/// widgets read the same colors as the app's own chrome.
fn override_component_theme(palette: &GtkPalette, cx: &mut App) {
    let p = palette;
    let theme = Theme::global_mut(cx);
    let colors = &mut theme.colors;

    let to_hsla = |c: Rgba| -> Hsla { c.into() };
    let px = |c: Rgba| to_hsla(c);

    colors.background = px(p.bg_primary);
    colors.foreground = px(p.text_primary);
    colors.border = px(p.border);
    colors.accent = px(p.accent);
    colors.accent_foreground = px(p.on_accent);
    colors.primary = px(p.accent);
    colors.primary_foreground = px(p.on_accent);
    colors.primary_hover = px(p.accent_hover);
    colors.primary_active = px(darken(p.accent, 0.12));
    colors.secondary = px(p.bg_secondary);
    colors.secondary_foreground = px(p.text_primary);
    colors.secondary_hover = px(p.bg_tertiary);
    colors.secondary_active = px(p.bg_card);
    colors.muted = px(p.muted);
    colors.muted_foreground = px(p.text_secondary);
    colors.input = px(p.input);
    colors.popover = px(p.bg_card);
    colors.popover_foreground = px(p.text_primary);
    colors.overlay = px(with_alpha_byte(p.bg_primary, 0xcc));
    colors.ring = px(p.accent);
    colors.selection = px(p.selection);
    colors.caret = px(p.accent);
    colors.drag_border = px(p.accent);
    colors.window_border = px(p.border);

    colors.danger = px(p.error);
    colors.danger_hover = px(lighten(p.error, 0.10));
    colors.danger_active = px(darken(p.error, 0.12));
    colors.danger_foreground = px(p.destructive_foreground);
    colors.success = px(p.success);
    colors.success_hover = px(lighten(p.success, 0.10));
    colors.success_active = px(darken(p.success, 0.12));
    colors.success_foreground = px(contrast_text(p.success));
    colors.warning = px(p.warning);
    colors.warning_hover = px(lighten(p.warning, 0.10));
    colors.warning_active = px(darken(p.warning, 0.12));
    colors.warning_foreground = px(contrast_text(p.warning));
    colors.info = px(p.accent);
    colors.info_hover = px(p.accent_hover);
    colors.info_active = px(darken(p.accent, 0.12));
    colors.info_foreground = px(p.on_accent);

    colors.button = px(p.bg_secondary);
    colors.button_hover = px(p.bg_tertiary);
    colors.button_active = px(p.bg_card);
    colors.button_foreground = px(p.text_primary);
    colors.button_primary = px(p.accent);
    colors.button_primary_hover = px(p.accent_hover);
    colors.button_primary_active = px(darken(p.accent, 0.12));
    colors.button_primary_foreground = px(p.on_accent);
    colors.button_secondary = px(p.bg_secondary);
    colors.button_secondary_hover = px(p.bg_tertiary);
    colors.button_secondary_active = px(p.bg_card);
    colors.button_secondary_foreground = px(p.text_primary);
    colors.button_danger = px(p.error);
    colors.button_danger_hover = px(lighten(p.error, 0.10));
    colors.button_danger_active = px(darken(p.error, 0.12));
    colors.button_danger_foreground = px(p.destructive_foreground);
    colors.button_success = px(p.success);
    colors.button_success_hover = px(lighten(p.success, 0.10));
    colors.button_success_active = px(darken(p.success, 0.12));
    colors.button_success_foreground = px(contrast_text(p.success));
    colors.button_warning = px(p.warning);
    colors.button_warning_hover = px(lighten(p.warning, 0.10));
    colors.button_warning_active = px(darken(p.warning, 0.12));
    colors.button_warning_foreground = px(contrast_text(p.warning));
    colors.button_info = px(p.accent);
    colors.button_info_hover = px(p.accent_hover);
    colors.button_info_active = px(darken(p.accent, 0.12));
    colors.button_info_foreground = px(p.on_accent);

    colors.sidebar = px(p.bg_primary);
    colors.sidebar_foreground = px(p.text_primary);
    colors.sidebar_border = px(p.border);
    colors.sidebar_accent = px(p.accent);
    colors.sidebar_accent_foreground = px(p.on_accent);
    colors.sidebar_primary = px(p.accent);
    colors.sidebar_primary_foreground = px(p.on_accent);
    colors.title_bar = px(p.bg_primary);
    colors.title_bar_border = px(p.border);
    colors.status_bar = px(p.bg_primary);
    colors.status_bar_border = px(p.border);
    colors.tiles = px(p.bg_card);
    colors.group_box = px(p.bg_card);
    colors.group_box_foreground = px(p.text_primary);

    colors.tab = px(p.bg_primary);
    colors.tab_bar = px(p.bg_primary);
    colors.tab_bar_segmented = px(p.bg_secondary);
    colors.tab_active = px(p.bg_card);
    colors.tab_active_foreground = px(p.text_primary);
    colors.tab_foreground = px(p.text_secondary);

    colors.list = px(p.bg_primary);
    colors.list_even = px(mix(p.bg_primary, p.text_primary, 0.03));
    colors.list_head = px(p.bg_secondary);
    colors.list_hover = px(p.bg_secondary);
    colors.list_active = px(p.bg_tertiary);
    colors.list_active_border = px(p.accent);

    colors.table = px(p.bg_primary);
    colors.table_even = px(mix(p.bg_primary, p.text_primary, 0.03));
    colors.table_head = px(p.bg_secondary);
    colors.table_head_foreground = px(p.text_secondary);
    colors.table_foot = px(p.bg_secondary);
    colors.table_foot_foreground = px(p.text_secondary);
    colors.table_hover = px(p.bg_secondary);
    colors.table_active = px(p.bg_tertiary);
    colors.table_active_border = px(p.accent);
    colors.table_row_border = px(p.border_subtle);

    colors.scrollbar = px(mix(p.bg_primary, p.text_primary, 0.12));
    colors.scrollbar_thumb = px(mix(p.bg_primary, p.text_primary, 0.28));
    colors.scrollbar_thumb_hover = px(mix(p.bg_primary, p.text_primary, 0.40));
    colors.slider_bar = px(p.accent);
    colors.slider_thumb = px(p.accent);
    colors.switch = px(p.accent);
    colors.switch_thumb = px(p.text_primary);
    colors.progress_bar = px(p.accent);
    colors.skeleton = px(p.bg_tertiary);
    colors.description_list_label = px(p.bg_secondary);
    colors.description_list_label_foreground = px(p.text_secondary);
    colors.accordion = px(p.bg_card);
    colors.drop_target = px(p.selection);
    colors.link = px(p.accent);
    colors.link_hover = px(p.accent_hover);
    colors.link_active = px(darken(p.accent, 0.12));

    colors.red = px(p.error);
    colors.red_light = px(lighten(p.error, 0.25));
    colors.green = px(p.success);
    colors.green_light = px(lighten(p.success, 0.25));
    colors.blue = px(p.accent);
    colors.blue_light = px(lighten(p.accent, 0.25));
    colors.yellow = px(p.warning);
    colors.yellow_light = px(lighten(p.warning, 0.25));
    colors.magenta = px(p.accent);
    colors.magenta_light = px(lighten(p.accent, 0.25));
    colors.cyan = px(p.accent);
    colors.cyan_light = px(lighten(p.accent, 0.25));
}

// --- Fallback palette helpers for direct styling ---------------------------

pub fn bg_color() -> Rgba {
    gtk_palette().map_or_else(|| rgb(0x1e1e1e), |p| p.bg_primary)
}
pub fn card_bg() -> Rgba {
    gtk_palette().map_or_else(|| rgb(0x2d2d2d), |p| p.bg_card)
}
pub fn fg_color() -> Rgba {
    gtk_palette().map_or_else(|| rgb(0xd4d4d4), |p| p.text_primary)
}
pub fn muted_fg() -> Rgba {
    gtk_palette().map_or_else(|| rgb(0x808080), |p| p.text_secondary)
}
pub fn border_color() -> Rgba {
    gtk_palette().map_or_else(|| rgb(0x3c3c3c), |p| p.border)
}
pub fn destructive_color() -> Rgba {
    gtk_palette().map_or_else(|| rgb(0x7f1d1d), |p| p.error)
}

// --- Phase 6 preset-driven chrome helpers (D3) ---------------------------
//
// Every helper reads the active preset per render with `UI_THEMES[0]`
// fallback (`getUiTheme` parity via `ui_preset_by_name`). No cached colors
// in views: callers invoke one helper per token per render so `cx.notify()`
// repaints everything on theme pick.
//
// When a desktop (GTK) palette is installed, each of these returns the mapped
// GTK token instead — one source of truth for GTK mode, and the preset literal
// stays byte-identical for the preset layer and its lock tests (see
// `theme::preset_raw` and the tests in `tests/gtk_theme_test.rs`).
use crate::themes_generated::{ui_preset_by_name, UiThemePreset};

/// Active preset for a persisted preset name (fallback `UI_THEMES[0]`).
pub fn preset(name: &str) -> &'static UiThemePreset {
    ui_preset_by_name(name)
}

/// The preset layer's unconditional value for one token, ignoring any installed
/// GTK palette. Used by the appearance grid swatches and the parity lock tests.
pub fn preset_raw(name: &str, token: PresetToken) -> Rgba {
    let p = ui_preset_by_name(name);
    let hex = match token {
        PresetToken::Background => p.background,
        PresetToken::Foreground => p.foreground,
        PresetToken::Card => p.card,
        PresetToken::CardForeground => p.card_foreground,
        PresetToken::Primary => p.primary,
        PresetToken::Muted => p.muted,
        PresetToken::MutedForeground => p.muted_foreground,
        PresetToken::Accent => p.accent,
        PresetToken::Destructive => p.destructive,
        PresetToken::Border => p.border,
        PresetToken::Input => p.input,
        PresetToken::Ring => p.ring,
    };
    rgb(hex)
}

/// One of the preset table's color tokens (for [`preset_raw`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetToken {
    Background,
    Foreground,
    Card,
    CardForeground,
    Primary,
    Muted,
    MutedForeground,
    Accent,
    Destructive,
    Border,
    Input,
    Ring,
}

pub fn preset_bg(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.bg_primary,
        None => rgb(ui_preset_by_name(name).background),
    }
}
pub fn preset_fg(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.text_primary,
        None => rgb(ui_preset_by_name(name).foreground),
    }
}
pub fn preset_card(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.bg_card,
        None => rgb(ui_preset_by_name(name).card),
    }
}
pub fn preset_card_fg(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.text_primary,
        None => rgb(ui_preset_by_name(name).card_foreground),
    }
}
pub fn preset_primary(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.accent,
        None => rgb(ui_preset_by_name(name).primary),
    }
}
pub fn preset_primary_fg(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.on_accent,
        None => rgb(ui_preset_by_name(name).primary_foreground),
    }
}
pub fn preset_secondary(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.bg_secondary,
        None => rgb(ui_preset_by_name(name).secondary),
    }
}
pub fn preset_secondary_fg(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.text_primary,
        None => rgb(ui_preset_by_name(name).secondary_foreground),
    }
}
pub fn preset_muted(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.muted,
        None => rgb(ui_preset_by_name(name).muted),
    }
}
pub fn preset_muted_fg(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.text_secondary,
        None => rgb(ui_preset_by_name(name).muted_foreground),
    }
}
pub fn preset_accent(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.accent,
        None => rgb(ui_preset_by_name(name).accent),
    }
}
pub fn preset_accent_fg(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.on_accent,
        None => rgb(ui_preset_by_name(name).accent_foreground),
    }
}
pub fn preset_destructive(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.error,
        None => rgb(ui_preset_by_name(name).destructive),
    }
}
pub fn preset_destructive_fg(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.destructive_foreground,
        None => rgb(ui_preset_by_name(name).destructive_foreground),
    }
}
pub fn preset_border(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.border,
        None => rgb(ui_preset_by_name(name).border),
    }
}
pub fn preset_input(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.input,
        None => rgb(ui_preset_by_name(name).input),
    }
}
pub fn preset_ring(name: &str) -> Rgba {
    match gtk_palette() {
        Some(p) => p.accent,
        None => rgb(ui_preset_by_name(name).ring),
    }
}

/// Dark bucket for the active chrome (GTK polarity when installed, else the
/// persisted preset's computed bucket — FE `isLightUiTheme` parity).
pub fn preset_is_dark(name: &str) -> bool {
    active_is_dark(name)
}

/// Danger/red text readable on the card surface in both modes: bright red
/// carries on dark surfaces, the deep destructive tone on light ones.
pub fn preset_danger_text(name: &str) -> Rgba {
    if let Some(p) = gtk_palette() {
        return p.error;
    }
    if ui_preset_by_name(name).is_dark {
        rgb(0xf87171)
    } else {
        rgb(ui_preset_by_name(name).destructive)
    }
}

/// FE-parity success accent (`0x22c55e`), lifted to the desktop success color
/// while a GTK palette is installed. The generated presets carry no green
/// token, so this is the one place that literal lives.
pub fn success_accent() -> Rgba {
    gtk_palette().map_or_else(|| rgb(0x22c55e), |p| p.success)
}

/// FE-parity amber accent (`0xf59e0b`), lifted to the desktop warning color
/// while a GTK palette is installed.
pub fn warning_accent() -> Rgba {
    gtk_palette().map_or_else(|| rgb(0xf59e0b), |p| p.warning)
}

/// Helper used by the GTK probe mapping to keep the preset byte conversion in
/// one place while allowing pure unit tests.
pub(crate) fn rgba_to_u32(color: Rgba) -> u32 {
    to_u32(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(hex: u32) -> Rgba {
        rgb(hex)
    }

    #[test]
    fn preset_layer_is_untouched_without_gtk() {
        let _serial = crate::test_util::serial_lock();
        clear_gtk();
        // The literal values every lock test depends on.
        assert_eq!(preset_bg("default-dark"), h(0x1e1e1e));
        assert_eq!(preset_fg("default-dark"), h(0xd4d4d4));
        assert_eq!(preset_card("default-dark"), h(0x1e1e1e));
        assert_eq!(preset_muted_fg("default-dark"), h(0x808080));
        assert_eq!(preset_border("default-dark"), h(0x3c3c3c));
        assert_eq!(preset_raw("default-dark", PresetToken::Background), h(0x1e1e1e));
    }

    #[test]
    fn gtk_palette_overrides_every_preset_helper() {
        let _serial = crate::test_util::serial_lock();
        clear_gtk();
        let palette = GtkPalette {
            is_dark: true,
            bg_primary: h(0x1a1b26),
            bg_secondary: h(0x16161e),
            bg_tertiary: h(0x1f2335),
            bg_card: h(0x282a38),
            bg_card_hover: h(0x2f334d),
            text_primary: h(0xa9b1d6),
            text_secondary: h(0x9aa5ce),
            text_tertiary: h(0x737aa2),
            text_disabled: h(0x484c5f),
            border: h(0x373949),
            border_subtle: h(0x2a2c3a),
            accent: h(0xf7768e),
            accent_hover: h(0xff8fa3),
            on_accent: h(0x222222),
            error: h(0xf7768e),
            warning: h(0xe0af68),
            success: h(0x9ece6a),
            placeholder: h(0x6a6f8a),
            close_hover: h(0xd85f74),
            destructive_foreground: h(0x222222),
            selection: h(0x7aa2f7),
            muted: h(0x1f2335),
            input: h(0x16161e),
        };
        install_gtk(palette);

        assert!(gtk_active());
        assert!(active_is_dark("default-light"));
        assert_eq!(preset_bg("default-dark"), h(0x1a1b26));
        assert_eq!(preset_card("default-light"), h(0x282a38));
        assert_eq!(preset_fg("default-light"), h(0xa9b1d6));
        assert_eq!(preset_muted_fg("default-dark"), h(0x9aa5ce));
        assert_eq!(preset_accent("default-dark"), h(0xf7768e));
        assert_eq!(preset_primary_fg("default-dark"), h(0x222222));
        assert_eq!(preset_border("default-dark"), h(0x373949));
        assert_eq!(preset_destructive("default-dark"), h(0xf7768e));
        assert_eq!(preset_danger_text("default-light"), h(0xf7768e));
        // The raw layer stays preset-literal even while GTK is installed.
        assert_eq!(preset_raw("default-dark", PresetToken::Background), h(0x1e1e1e));

        clear_gtk();
        assert_eq!(preset_bg("default-dark"), h(0x1e1e1e));
    }

    #[test]
    fn color_helpers_behave() {
        assert_eq!(with_alpha_byte(h(0x123456), 0x80).a, 128.0 / 255.0);
        let mid = mix(h(0x000000), h(0xffffff), 0.5);
        assert!((mid.r - 0.5).abs() < 1e-6 && (mid.g - 0.5).abs() < 1e-6);
        assert!(relative_luminance(h(0xffffff)) > 0.99);
        assert!(relative_luminance(h(0x000000)) < 0.01);
        assert_eq!(contrast_text(h(0xffffff)), h(0x000000));
        assert_eq!(contrast_text(h(0x000000)), h(0xffffff));
    }
}
