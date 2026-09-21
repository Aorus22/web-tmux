//! Liquid Glass material tests: tier alphas, the pass-through rendering when
//! the material is off, the compositor gate for the backdrop, and the
//! persistence of the two settings that drive it.
//!
//! Everything here is headless — `glass` is deliberately free of `Window` and
//! `App` state except for the `GlassPrefs` global, so the alphas and the
//! pass-through path can be asserted without a renderer. The one thing these
//! tests cannot see is the pixels themselves (a glass fill over the desktop),
//! which is what the manual UAT run covers.

use webtmux::app_state::AppState;
use webtmux::glass::{self, Elevation, GlassPrefs, GlassTier};
use webtmux::theme;
use webtmux_settings::{DesktopSettings, Theme as SettingsTheme};

/// A path under the OS temp dir, unique per test run.
fn temp_base(tag: &str) -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("webtmux_glass_test_{tag}_{now}"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_clamping_is_readable_and_finite() {
    assert_eq!(glass::clamp_opacity(0.85), 0.85);
    // Below the floor a translucent panel over terminal output stops being
    // readable.
    assert_eq!(glass::clamp_opacity(0.1), glass::MIN_OPACITY);
    // Above it there is nothing left to hide: the range tops out at opaque.
    assert_eq!(glass::clamp_opacity(5.0), glass::MAX_OPACITY);
    // A hand-edited or corrupted settings file must never produce NaN alpha.
    assert_eq!(glass::clamp_opacity(f32::NAN), glass::DEFAULT_OPACITY);
    assert_eq!(glass::clamp_opacity(-0.0), glass::MIN_OPACITY);
}

#[test]
fn test_intensity_presets_are_reachable_and_ordered() {
    // The settings row highlights the preset whose value equals the current
    // alpha, so a preset outside the clamp window (or one that rounds on the
    // way to the settings store) would render three buttons with none active
    // and leave the user unable to get back out of it.
    let mut previous = f32::INFINITY;
    for (label, value) in glass::INTENSITY_STEPS {
        assert_eq!(
            glass::clamp_opacity(value),
            value,
            "preset {label} must survive the clamp unchanged"
        );
        assert!(
            value < previous,
            "presets run from most to least opaque ({label} out of order)"
        );
        previous = value;
    }
}

#[test]
fn test_tier_alphas_keep_chrome_more_transparent_than_overlays() {
    // Overlay surfaces sit over the app's own output, so the setting *is*
    // their alpha.
    assert_eq!(glass::tier_alpha(GlassTier::Overlay, 0.85), 0.85);
    assert_eq!(glass::tier_alpha(GlassTier::Overlay, 1.0), 1.0);

    // Chrome has the desktop behind it and always stays lighter than an
    // overlay at the same setting.
    let chrome = glass::tier_alpha(GlassTier::Chrome, 0.85);
    assert!((chrome - 0.85 * 0.78).abs() < 1e-6, "chrome alpha was {chrome}");
    assert!(chrome < glass::tier_alpha(GlassTier::Overlay, 0.85));

    // …but never dissolves, even at the most transparent setting.
    assert_eq!(
        glass::tier_alpha(GlassTier::Chrome, glass::MIN_OPACITY),
        0.45
    );

    // Out-of-range values are clamped before they are scaled.
    assert_eq!(
        glass::tier_alpha(GlassTier::Chrome, 9.0),
        glass::tier_alpha(GlassTier::Chrome, 1.0)
    );
}

#[test]
fn test_disabled_material_returns_the_theme_pixels() {
    let preset = "default-dark";
    let base = theme::preset_card(preset);
    let border = theme::preset_border(preset);
    let prefs = GlassPrefs {
        enabled: false,
        opacity: 0.85,
    };

    for tier in [GlassTier::Chrome, GlassTier::Overlay] {
        let style = glass::style_for(
            prefs,
            base,
            border,
            theme::preset_is_dark(preset),
            tier,
            Elevation::Lg,
        );
        assert!(!style.is_glassy());
        // Exactly the colours the leaf painted before the material existed.
        assert_eq!(style.fill, base);
        assert_eq!(style.fill.a, 1.0);
        assert_eq!(style.border, border);
        // …and the plain elevation stack, with no rim bolted on.
        assert_eq!(style.shadows, glass::elevation_stack(Elevation::Lg));
        assert!(style.shadows.iter().all(|s| !s.inset));
    }
}

#[test]
fn test_enabled_material_fades_the_theme_colour_and_adds_the_rim() {
    let preset = "default-dark";
    let base = theme::preset_card(preset);
    let chrome = glass::style_for(
        GlassPrefs {
            enabled: true,
            opacity: 0.85,
        },
        base,
        theme::preset_border(preset),
        true,
        GlassTier::Chrome,
        Elevation::None,
    );
    assert!(chrome.is_glassy());
    assert!((chrome.fill.a - glass::tier_alpha(GlassTier::Chrome, 0.85)).abs() < 1e-6);
    // Two inset layers on top of the (empty) elevation: top rim + bottom shade.
    assert_eq!(chrome.shadows.len(), 2);
    assert!(chrome.shadows.iter().all(|s| s.inset));
    assert_eq!(chrome.shadows[0].offset.y, gpui::px(1.0));
    assert_eq!(chrome.shadows[1].offset.y, gpui::px(-1.0));
    // The red/green/blue channels are untouched: the fill is the preset's own
    // colour, only faded — never a fixed grey tint.
    assert_eq!((chrome.fill.r, chrome.fill.g, chrome.fill.b), (base.r, base.g, base.b));

    // A surface that had elevation keeps it, plus the rim, because `.shadow()`
    // replaces the stack rather than appending to it.
    let elevated = glass::style_for(
        GlassPrefs {
            enabled: true,
            opacity: 0.85,
        },
        base,
        theme::preset_border(preset),
        true,
        GlassTier::Overlay,
        Elevation::Lg,
    );
    assert_eq!(
        elevated.shadows.len(),
        glass::elevation_stack(Elevation::Lg).len() + 2
    );
    assert_eq!(elevated.shadows.iter().filter(|s| !s.inset).count(), 2);
}

#[test]
fn test_border_and_rim_ink_follow_theme_polarity() {
    // A white hairline reads as a lifted edge on dark fills, a black one as
    // definition on light fills.
    let dark_border = glass::border_ink(true);
    let light_border = glass::border_ink(false);
    assert_eq!(dark_border.a, 0.10);
    assert_eq!(light_border.a, 0.08);
    assert!(dark_border.r > light_border.r);

    // The top rim is a sheen: barely there on dark fills, strong on light ones
    // (where the fill is nearly white anyway). The bottom shade is the
    // opposite, so the slab reads as having thickness.
    assert!(glass::rim_ink(false).a > glass::rim_ink(true).a);
    assert!(glass::shade_ink(true).a > glass::shade_ink(false).a);
}

#[test]
fn test_app_state_glass_style_tracks_the_setting_and_the_preset() {
    // Default settings ship the material on, so the chrome starts translucent.
    let mut settings = DesktopSettings::default();
    settings.theme = SettingsTheme::Dark;
    settings.theme_preset = "default-dark".to_string();
    let app = AppState::new(settings.clone(), None);
    assert!(app.glass_enabled());
    assert_eq!(app.glass_opacity(), 0.85);

    let style = app.glass_style(app_card_bg(&app), GlassTier::Chrome, Elevation::None);
    assert!(style.is_glassy());
    assert!(style.fill.a < 1.0);
    assert_eq!(style.fill.r, app_card_bg(&app).r);

    // Switching the material off returns the preset's own opaque colours…
    let mut off = settings.clone();
    off.glass_enabled = false;
    let app_off = AppState::new(off, None);
    let plain = app_off.glass_style(app_card_bg(&app_off), GlassTier::Chrome, Elevation::None);
    assert!(!plain.is_glassy());
    assert_eq!(plain.fill, app_card_bg(&app_off));
    assert_eq!(plain.fill.a, 1.0);
    assert_eq!(
        plain.border,
        theme::preset_border(&app_off.settings.theme_preset)
    );

    // …and the material follows the active preset rather than a fixed tint: a
    // light preset keeps a light fill, with only its alpha changed.
    let mut light = DesktopSettings::default();
    light.theme = SettingsTheme::Light;
    light.theme_preset = "default-light".to_string();
    let app_light = AppState::new(light, None);
    let light_style = app_light.glass_style(
        app_card_bg(&app_light),
        GlassTier::Chrome,
        Elevation::None,
    );
    assert!(
        light_style.fill.r > 0.9,
        "light preset fill was {}",
        light_style.fill.r
    );
    assert!(light_style.fill.a < 1.0);
}

/// The preset colour a chrome leaf passes in, read the way the views read it.
fn app_card_bg(app: &AppState) -> gpui::Rgba {
    theme::preset_card(&app.settings.theme_preset)
}

#[test]
fn test_backdrop_blur_is_opt_in_per_compositor() {
    // `backdrop_blur_available` only reports true for the Wayland compositors
    // that implement org_kde_kwin_blur; everywhere else the window keeps the
    // plain transparent backdrop the CSD frame already relies on, so the
    // upgrade can never make a session worse.
    let wayland = std::env::var_os("WAYLAND_DISPLAY");
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();

    std::env::remove_var("WAYLAND_DISPLAY");
    std::env::set_var("XDG_CURRENT_DESKTOP", "KDE");
    assert!(
        !glass::backdrop_blur_available(),
        "X11/KDE must not claim a backdrop blur"
    );

    if wayland.is_some() {
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        std::env::set_var("XDG_CURRENT_DESKTOP", "GNOME");
        assert!(
            !glass::backdrop_blur_available(),
            "GNOME exposes no blur protocol"
        );
        std::env::set_var("XDG_CURRENT_DESKTOP", "Hyprland");
        assert!(glass::backdrop_blur_available());
        std::env::set_var("XDG_CURRENT_DESKTOP", "KDE");
        assert!(glass::backdrop_blur_available());
    }

    // Restore whatever the test runner had.
    match wayland {
        Some(v) => std::env::set_var("WAYLAND_DISPLAY", v),
        None => std::env::remove_var("WAYLAND_DISPLAY"),
    }
    match desktop {
        Some(v) => std::env::set_var("XDG_CURRENT_DESKTOP", v),
        None => std::env::remove_var("XDG_CURRENT_DESKTOP"),
    }
}

#[test]
fn test_glass_prefs_mirror_the_settings_store() {
    let mut settings = DesktopSettings::default();
    assert!(settings.glass_enabled, "the material is on by default");
    assert_eq!(settings.glass_opacity, glass::DEFAULT_OPACITY);
    // The copies the dialogs read (they are handed no `AppState`) start out
    // agreeing with the store.
    let prefs = GlassPrefs::from_settings(&settings);
    assert!(prefs.enabled);
    assert_eq!(prefs.opacity, glass::DEFAULT_OPACITY);

    // A corrupted alpha is clamped on the way in, so the global can never hold
    // a value the renderer would refuse.
    settings.glass_enabled = false;
    settings.glass_opacity = f32::NAN;
    let prefs = GlassPrefs::from_settings(&settings);
    assert!(!prefs.enabled);
    assert_eq!(prefs.opacity, glass::DEFAULT_OPACITY);

    // The fallback used before anything is registered (early frames, headless
    // tests) is the same shape as the store default.
    let fallback = GlassPrefs::default();
    assert!(fallback.enabled);
    assert_eq!(fallback.opacity, glass::DEFAULT_OPACITY);
}

#[test]
fn test_glass_settings_survive_a_save_and_a_legacy_file() {
    let base = temp_base("roundtrip");

    let mut settings = DesktopSettings::load_from(&base).expect("should load defaults");
    assert!(settings.glass_enabled);
    assert_eq!(settings.glass_opacity, glass::DEFAULT_OPACITY);

    settings.glass_enabled = false;
    settings.glass_opacity = 0.68;
    settings.save_to(&base).expect("should save");

    let reloaded = DesktopSettings::load_from(&base).expect("should reload");
    assert!(!reloaded.glass_enabled);
    assert_eq!(reloaded.glass_opacity, 0.68);

    // A settings file written before the material existed has no glass keys at
    // all and must still load — with the defaults, not with a deserialization
    // error (which would file the whole settings file away as .bak).
    let path = webtmux_settings::paths::settings_path_with_base(&base);
    let raw = std::fs::read_to_string(&path).expect("settings file exists");
    let mut json: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
    let obj = json.as_object_mut().expect("object");
    obj.remove("glass_enabled");
    obj.remove("glass_opacity");
    std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap()).expect("write legacy");

    let legacy = DesktopSettings::load_from(&base).expect("legacy file must load");
    assert!(legacy.glass_enabled);
    assert_eq!(legacy.glass_opacity, glass::DEFAULT_OPACITY);
    // The unrelated fields survived the round trip, i.e. nothing was reset.
    assert_eq!(legacy.theme, SettingsTheme::Dark);
    assert_eq!(legacy.theme_preset, "default-dark");

    let _ = std::fs::remove_dir_all(&base);
}
