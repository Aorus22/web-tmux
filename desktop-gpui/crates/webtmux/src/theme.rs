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
pub fn bg_color() -> Rgba { rgb(0x1e1e1e) }
pub fn card_bg() -> Rgba { rgb(0x2d2d2d) }
pub fn fg_color() -> Rgba { rgb(0xd4d4d4) }
pub fn muted_fg() -> Rgba { rgb(0x808080) }
pub fn border_color() -> Rgba { rgb(0x3c3c3c) }
pub fn destructive_color() -> Rgba { rgb(0x7f1d1d) }
