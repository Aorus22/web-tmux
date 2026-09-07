//! Theme management for web-tmux.

use gpui::{rgb, App, Rgba};
use gpui_component::{Theme, ThemeMode};
use webtmux_settings::Theme as SettingsTheme;

/// Apply theme setting to the GPUI application context.
pub fn apply_theme(theme: SettingsTheme, cx: &mut App) {
    let mode = match theme {
        SettingsTheme::Dark => ThemeMode::Dark,
        SettingsTheme::Light => ThemeMode::Light,
        SettingsTheme::System => ThemeMode::Dark,
    };
    Theme::change(mode, None, cx);
}

/// Fallback palette helpers for direct styling in Phase 1 views.
/// Kept for pre-Phase-6 call sites; new chrome reads the `preset_*` helpers
/// below (one preset read per render, no caching — presets are `&'static`).
pub fn bg_color() -> Rgba { rgb(0x1e1e1e) }
pub fn card_bg() -> Rgba { rgb(0x2d2d2d) }
pub fn fg_color() -> Rgba { rgb(0xd4d4d4) }
pub fn muted_fg() -> Rgba { rgb(0x808080) }
pub fn border_color() -> Rgba { rgb(0x3c3c3c) }
pub fn destructive_color() -> Rgba { rgb(0x7f1d1d) }

// --- Phase 6 preset-driven chrome helpers (D3) ---------------------------
//
// Every helper reads the active preset per render with `UI_THEMES[0]`
// fallback (`getUiTheme` parity via `ui_preset_by_name`). No cached colors
// in views: callers invoke one helper per token per render so `cx.notify()`
// repaints everything on theme pick.
use crate::themes_generated::{ui_preset_by_name, UiThemePreset};

/// Active preset for a persisted preset name (fallback `UI_THEMES[0]`).
pub fn preset(name: &str) -> &'static UiThemePreset {
    ui_preset_by_name(name)
}

pub fn preset_bg(name: &str) -> Rgba { rgb(ui_preset_by_name(name).background) }
pub fn preset_fg(name: &str) -> Rgba { rgb(ui_preset_by_name(name).foreground) }
pub fn preset_card(name: &str) -> Rgba { rgb(ui_preset_by_name(name).card) }
pub fn preset_card_fg(name: &str) -> Rgba {
    rgb(ui_preset_by_name(name).card_foreground)
}
pub fn preset_primary(name: &str) -> Rgba { rgb(ui_preset_by_name(name).primary) }
pub fn preset_primary_fg(name: &str) -> Rgba {
    rgb(ui_preset_by_name(name).primary_foreground)
}
pub fn preset_secondary(name: &str) -> Rgba {
    rgb(ui_preset_by_name(name).secondary)
}
pub fn preset_secondary_fg(name: &str) -> Rgba {
    rgb(ui_preset_by_name(name).secondary_foreground)
}
pub fn preset_muted(name: &str) -> Rgba { rgb(ui_preset_by_name(name).muted) }
pub fn preset_muted_fg(name: &str) -> Rgba {
    rgb(ui_preset_by_name(name).muted_foreground)
}
pub fn preset_accent(name: &str) -> Rgba { rgb(ui_preset_by_name(name).accent) }
pub fn preset_accent_fg(name: &str) -> Rgba {
    rgb(ui_preset_by_name(name).accent_foreground)
}
pub fn preset_destructive(name: &str) -> Rgba {
    rgb(ui_preset_by_name(name).destructive)
}
pub fn preset_destructive_fg(name: &str) -> Rgba {
    rgb(ui_preset_by_name(name).destructive_foreground)
}
pub fn preset_border(name: &str) -> Rgba { rgb(ui_preset_by_name(name).border) }
pub fn preset_input(name: &str) -> Rgba { rgb(ui_preset_by_name(name).input) }
pub fn preset_ring(name: &str) -> Rgba { rgb(ui_preset_by_name(name).ring) }

/// Dark bucket for a persisted preset name (FE `isLightUiTheme` parity).
pub fn preset_is_dark(name: &str) -> bool {
    ui_preset_by_name(name).is_dark
}
