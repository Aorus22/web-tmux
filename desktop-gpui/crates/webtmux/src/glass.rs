//! Liquid Glass material for the window chrome.
//!
//! The window opens with `WindowBackgroundAppearance::Transparent` and the
//! root view paints no background of its own, so any chrome leaf that stops
//! painting an opaque fill lets the desktop show through. This module turns the
//! active preset's opaque colours into a translucent slab and adds the two
//! layers that make it read as a material rather than as a faded panel:
//!
//! * a hairline border inked from the preset's *polarity* (white on dark, black
//!   on light) instead of the opaque `border` token, and
//! * an inset rim — 1px of light along the top edge, 1px of shade along the
//!   bottom — which is the glass silhouette cue.
//!
//! # What this deliberately is not
//!
//! GPUI 0.3.3 has no `backdrop-filter`: an element cannot blur what is behind
//! it, and `WindowBackgroundAppearance::Blurred` only does anything on Wayland
//! compositors that expose `org_kde_kwin_blur` (KWin, Hyprland) — GNOME ignores
//! it. On GNOME the glass is therefore a genuine translucent tint over the
//! desktop, not a frosted blur. Three rules follow, and all are load-bearing:
//!
//! 1. Only surfaces with the *desktop* behind them get the transparent chrome
//!    tier: the sidebar and the title bar (plus the sidebar's floating footer,
//!    which is the exception that leans opaque because the session tree scrolls
//!    under it). Surfaces that sit over the app's own output — dialogs, the
//!    dropdown popovers, the command palette, toasts — use the denser overlay
//!    tier, because a translucent panel over terminal text is unreadable.
//!
//!    The window toolbar below the title bar is deliberately *not* in either
//!    list: no desktop is behind it (the pane grid is), so it stays opaque like
//!    the rest of the workspace.
//! 2. The alpha lives in the *colour*, never in `.opacity()` on a subtree:
//!    GPUI has no group compositing, so element opacity multiplies into each
//!    primitive separately and would show every child through every other.
//! 3. A leaf whose parent already paints glass must not paint glass itself at
//!    an equal alpha — two translucent layers over each other read as one
//!    denser patch (that is why the floating footer uses the overlay tier and
//!    inactive rows paint nothing).
//!
//! Turning the material off must restore the previous pixels exactly, so the
//! opaque path returns the preset's own colours and the plain elevation stack —
//! no rim, no polarity ink.
//!
//! # gpui-component dialogs
//!
//! The dialog surfaces (`create_session`, `rename_*`) are built by
//! `gpui_component`'s `Dialog`, which assigns its own shadow stack to the panel
//! *after* the caller's builder chain (`dialog.rs`, the open animation's
//! `shadow_xl` equivalent). The caller can therefore set the fill and the
//! border ink but not an inset rim: whatever `.shadow()` the caller sets is
//! overwritten. Those dialogs get the material without the rim, which is a
//! deliberate limit rather than an oversight.
//!
//! The right-click menus (`session_context_menu` / `pane_context_menu`) are the
//! other side of that limit: they are drawn by `gpui_component`'s `PopupMenu`,
//! which styles itself from the component's own `Theme` global rather than from
//! this crate's presets, so its popover surface is opaque and stays that way
//! unless the component is vendored. Same for the entry list inside the command
//! palette — only the palette's shell is this crate's.
//!
//! # Coexistence with the CSD frame outline (B4)
//!
//! Since the window frame gained a 1px outline in the theme border color
//! (`AppState::render`), exactly one element owns the outermost border: that
//! frame panel. The glass "polarity hairline" stays a treatment for surfaces
//! *inside* it — the title bar, the sidebar, dialogs, popovers, toasts — and is
//! not lifted to the frame. When the material is off, the frame outline is
//! unchanged and the inner surfaces fall back to their opaque preset borders,
//! so the frame never changes width or color just because glass toggled.

use gpui::{hsla, px, App, BoxShadow, Global, Hsla, Rgba, Window};
use webtmux_settings::{
    clamp_glass_opacity, DesktopSettings, GLASS_DEFAULT_OPACITY, GLASS_MAX_OPACITY,
    GLASS_MIN_OPACITY,
};

/// Settings presets, strongest glass last. Labels are what the UI shows.
pub const INTENSITY_STEPS: [(&str, f32); 3] = [("Subtle", 0.94), ("Medium", 0.85), ("Bold", 0.68)];

/// Chrome sits this fraction of the overlay alpha, so the desktop still reads
/// through the sidebar and title bar even at the most opaque setting.
const CHROME_FACTOR: f32 = 0.78;
/// …with a floor, so the chrome never dissolves into the desktop at the lowest
/// setting. No ceiling is needed: the factor already keeps chrome below the
/// overlay alpha at every setting.
const CHROME_FLOOR: f32 = 0.45;

/// Which side of the window a glass surface lives on. They do not share an
/// alpha because what shows through them differs: chrome is over the desktop,
/// overlays are over the app's own output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlassTier {
    /// Leaves with the desktop behind them: sidebar, title bar.
    Chrome,
    /// Floating surfaces over app output: dialogs, menus, palette, toasts.
    Overlay,
}

/// The elevation a surface had before glass, so the glass path can keep it.
///
/// Mirrors gpui's `shadow_*` scale rather than only the steps in use today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elevation {
    None,
    Sm,
    Lg,
    Xl,
    /// gpui's `shadow_2xl`.
    Xxl,
}

/// The two persisted values the material needs, copied out of the settings
/// store so surfaces that are rendered without an `AppState` in hand (the
/// dialogs, which are built by `gpui_component`'s `Dialog` closure) can still
/// paint themselves.
///
/// Registered once at startup and refreshed by every setter — the same shape as
/// `gpui_component`'s own `Theme` global.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassPrefs {
    pub enabled: bool,
    pub opacity: f32,
}

impl Default for GlassPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            opacity: GLASS_DEFAULT_OPACITY,
        }
    }
}

impl Global for GlassPrefs {}

impl GlassPrefs {
    pub fn from_settings(settings: &DesktopSettings) -> Self {
        Self {
            enabled: settings.glass_enabled,
            opacity: clamp_glass_opacity(settings.glass_opacity),
        }
    }

    /// Read the registered preferences, falling back to the settings defaults
    /// when nothing has been registered yet (headless tests, early frames).
    pub fn get(cx: &App) -> Self {
        cx.try_global::<GlassPrefs>().copied().unwrap_or_default()
    }

    /// Publish them for the next render (see `AppState::set_glass_*`).
    pub fn register(cx: &mut App, prefs: GlassPrefs) {
        cx.set_global(prefs);
    }
}

/// Everything a surface needs to paint itself as glass.
#[derive(Debug, Clone, PartialEq)]
pub struct GlassStyle {
    pub fill: Rgba,
    pub border: Rgba,
    pub shadows: Vec<BoxShadow>,
    /// False for the pass-through path (material off), which is what the tests
    /// assert on to prove the "off" rendering is the previous one.
    pub glassy: bool,
}

impl GlassStyle {
    pub fn is_glassy(&self) -> bool {
        self.glassy
    }
}

/// Build a glass style, or the material's disabled rendering.
///
/// `base` is the preset colour the leaf paints today (`preset_bg` /
/// `preset_card`), `border` the preset's `border` token, `is_dark` its
/// polarity. With the material off this returns exactly those colours with the
/// plain elevation stack.
pub fn style_for(
    prefs: GlassPrefs,
    base: Rgba,
    border: Rgba,
    is_dark: bool,
    tier: GlassTier,
    elevation: Elevation,
) -> GlassStyle {
    if !prefs.enabled {
        return GlassStyle {
            fill: base,
            border,
            shadows: elevation_stack(elevation),
            glassy: false,
        };
    }
    style(base, border, is_dark, prefs.opacity, tier, elevation)
}

/// Glass style regardless of the setting — what [`style_for`] delegates to once
/// it knows the material is on, and what the tests exercise directly.
pub fn style(
    base: Rgba,
    _border: Rgba,
    is_dark: bool,
    opacity: f32,
    tier: GlassTier,
    elevation: Elevation,
) -> GlassStyle {
    let mut shadows = elevation_stack(elevation);
    // Order matters only in that these paint after the fill; GPUI splits the
    // stack by `inset` on its own (see `paint_inset_shadows`).
    shadows.push(
        BoxShadow::new(px(0.), px(1.), rim_ink(is_dark))
            .blur_radius(px(1.))
            .inset(),
    );
    shadows.push(
        BoxShadow::new(px(0.), px(-1.), shade_ink(is_dark))
            .blur_radius(px(1.))
            .inset(),
    );
    GlassStyle {
        fill: base.alpha(tier_alpha(tier, opacity)),
        border: border_ink(is_dark),
        shadows,
        glassy: true,
    }
}

/// Clamp a persisted alpha into the readable window (see the settings crate for
/// the bounds and the NaN fallback).
pub fn clamp_opacity(opacity: f32) -> f32 {
    clamp_glass_opacity(opacity)
}

/// Fill alpha for a tier.
pub fn tier_alpha(tier: GlassTier, opacity: f32) -> f32 {
    let opacity = clamp_glass_opacity(opacity);
    match tier {
        GlassTier::Overlay => opacity,
        GlassTier::Chrome => (opacity * CHROME_FACTOR).max(CHROME_FLOOR),
    }
}

/// Hairline ink: white reads as a lifted edge on dark fills, black as a
/// definition line on light ones.
pub fn border_ink(is_dark: bool) -> Rgba {
    if is_dark {
        hsla(0., 0., 1., 0.10).into()
    } else {
        hsla(0., 0., 0., 0.08).into()
    }
}

/// 1px of light along the top inner edge.
pub fn rim_ink(is_dark: bool) -> Hsla {
    if is_dark {
        hsla(0., 0., 1., 0.07)
    } else {
        hsla(0., 0., 1., 0.85)
    }
}

/// 1px of shade along the bottom inner edge, so the slab reads as having
/// thickness rather than as a float.
pub fn shade_ink(is_dark: bool) -> Hsla {
    if is_dark {
        hsla(0., 0., 0., 0.18)
    } else {
        hsla(0., 0., 0., 0.04)
    }
}

/// Bounds, re-exported for the settings page and the tests.
pub const MIN_OPACITY: f32 = GLASS_MIN_OPACITY;
pub const MAX_OPACITY: f32 = GLASS_MAX_OPACITY;
pub const DEFAULT_OPACITY: f32 = GLASS_DEFAULT_OPACITY;

/// Whether the compositor can blur what is behind the window.
///
/// Only the KDE blur protocol (`org_kde_kwin_blur`, implemented by KWin and
/// Hyprland) can do this, and gpui's Wayland backend is the only one that binds
/// it — X11 and GNOME sessions get nothing. Asking the platform would be
/// better, but gpui exposes no query, so this reads the desktop session the way
/// the compositor itself identifies it.
pub fn backdrop_blur_available() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return false;
    }
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_ascii_lowercase();
    desktop.contains("kde") || desktop.contains("hyprland")
}

/// Pick the window backdrop for the current glass setting.
///
/// `Transparent` is what the CSD frame already relies on (rounded corners show
/// the desktop); `Blurred` is the same thing plus a backdrop blur where the
/// compositor implements one. On GNOME the two are identical, so this never
/// makes the window worse — it only upgrades sessions that can frost it.
///
/// Only Linux is touched: X11 has no blur protocol to bind (it lands on the
/// `Transparent` the window already had), and macOS/Windows keep whatever
/// backdrop the window was created with.
pub fn apply_backdrop_material(window: &mut Window, glass_enabled: bool) {
    #[cfg(target_os = "linux")]
    {
        let appearance = if glass_enabled && backdrop_blur_available() {
            gpui::WindowBackgroundAppearance::Blurred
        } else {
            gpui::WindowBackgroundAppearance::Transparent
        };
        window.set_background_appearance(appearance);
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (window, glass_enabled);
    }
}

/// The elevation stack a surface had before glass.
///
/// These are gpui's own `shadow_sm`/`shadow_lg`/`shadow_xl`/`shadow_2xl` values
/// copied verbatim (`gpui-pre-macros/src/styles.rs`): a glass leaf needs its
/// outer layers and the rim in *one* vec, because `.shadow()` replaces the
/// stack rather than appending to it, and the styled helpers expose no getter.
pub fn elevation_stack(elevation: Elevation) -> Vec<BoxShadow> {
    match elevation {
        Elevation::None => Vec::new(),
        Elevation::Sm => vec![
            BoxShadow::new(px(0.), px(1.), hsla(0., 0., 0., 0.1)).blur_radius(px(3.)),
            BoxShadow::new(px(0.), px(1.), hsla(0., 0., 0., 0.1))
                .blur_radius(px(2.))
                .spread_radius(px(-1.)),
        ],
        Elevation::Lg => vec![
            BoxShadow::new(px(0.), px(10.), hsla(0., 0., 0., 0.1))
                .blur_radius(px(15.))
                .spread_radius(px(-3.)),
            BoxShadow::new(px(0.), px(4.), hsla(0., 0., 0., 0.1))
                .blur_radius(px(6.))
                .spread_radius(px(-4.)),
        ],
        Elevation::Xl => vec![
            BoxShadow::new(px(0.), px(20.), hsla(0., 0., 0., 0.1))
                .blur_radius(px(25.))
                .spread_radius(px(-5.)),
            BoxShadow::new(px(0.), px(8.), hsla(0., 0., 0., 0.1))
                .blur_radius(px(10.))
                .spread_radius(px(-6.)),
        ],
        Elevation::Xxl => vec![
            BoxShadow::new(px(0.), px(25.), hsla(0., 0., 0., 0.25))
                .blur_radius(px(50.))
                .spread_radius(px(-12.)),
        ],
    }
}
