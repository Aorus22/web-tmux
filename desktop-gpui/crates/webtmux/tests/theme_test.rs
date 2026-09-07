//! Phase 6 tracer headless theme contracts (THEME-01/02/03, SET-01 per D1/D3/D4).
//!
//! - `test_theme_table_lengths`: 102 UI + 78 terminal rows (FE drift alarm).
//! - `test_terminal_link_resolution`: every UI `terminal_theme` resolves.
//! - `test_get_ui_theme_fallback`: unknown id falls back to `UI_THEMES[0]`
//!   (`getUiTheme` parity) + `default-dark.background == 0x1e1e1e` spot-check.
//! - `test_luminance_buckets`: ported `themeLuminance` (WCAG, `> 0.5`) buckets
//!   known presets like FE (`default-dark` dark, `default-light` light).
//! - `test_theme_filter_counts`: all/dark/light filter counts sum to 102.

use webtmux::themes_generated::{
    terminal_preset_by_name, ui_preset_by_name, TERMINAL_THEMES, UI_THEMES,
};

#[test]
fn test_theme_table_lengths() {
    assert_eq!(
        UI_THEMES.len(),
        102,
        "UI theme table must hold 102 presets (FE ui-themes.ts parity); regen via node scripts/generate-gpui-themes.mjs"
    );
    assert_eq!(
        TERMINAL_THEMES.len(),
        78,
        "terminal theme table must hold 78 presets (FE terminal-themes.ts parity)"
    );
}

#[test]
fn test_terminal_link_resolution() {
    for ui in UI_THEMES {
        let found = TERMINAL_THEMES
            .iter()
            .any(|t| t.name == ui.terminal_theme);
        assert!(
            found,
            "UI preset '{}' links terminal_theme '{}' which must resolve in TERMINAL_THEMES",
            ui.name, ui.terminal_theme
        );
    }
    // Spot-check the canonical link.
    let default_dark = ui_preset_by_name("default-dark");
    let linked = terminal_preset_by_name(default_dark.terminal_theme);
    assert_eq!(linked.name, "default");
}

#[test]
fn test_get_ui_theme_fallback() {
    let fallback = ui_preset_by_name("does-not-exist");
    assert_eq!(
        fallback.name, UI_THEMES[0].name,
        "unknown id must fall back to UI_THEMES[0] (getUiTheme parity)"
    );
    let default_dark = ui_preset_by_name("default-dark");
    assert_eq!(
        default_dark.background, 0x1e1e1e,
        "default-dark.background spot-check must hold"
    );
}

#[test]
fn test_luminance_buckets() {
    let dark = ui_preset_by_name("default-dark");
    let light = ui_preset_by_name("default-light");
    assert!(
        dark.is_dark,
        "default-dark must bucket dark (themeLuminance <= 0.5)"
    );
    assert!(
        !light.is_dark,
        "default-light must bucket light (themeLuminance > 0.5)"
    );
    // Ported rule is WCAG-weighted; backgrounds at the extremes agree.
    assert!(webtmux::themes_generated::theme_luminance(0x000000) < 0.5);
    assert!(webtmux::themes_generated::theme_luminance(0xffffff) > 0.5);
}

#[test]
fn test_theme_filter_counts() {
    let dark = UI_THEMES.iter().filter(|t| t.is_dark).count();
    let light = UI_THEMES.iter().filter(|t| !t.is_dark).count();
    assert_eq!(dark + light, 102, "dark + light must sum to all (102)");
    assert_eq!(UI_THEMES.len(), 102);
    assert!(dark > 0 && light > 0, "both buckets must be non-empty");
}
