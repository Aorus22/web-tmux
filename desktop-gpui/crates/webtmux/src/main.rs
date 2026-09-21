//! webtmux desktop client (GPUI) application binary entry point.

use std::borrow::Cow;
use std::sync::Arc;
use gpui::*;
use parking_lot::Mutex;
use webtmux::app_state::{AppState, TOKIO_RT};
use webtmux::bundle::resolve_backend_path;
use webtmux::glass;
use webtmux::theme;
use webtmux::window_state;
use webtmux_settings::DesktopSettings;
use webtmux_supervisor::SpawnOptions;

fn fit_bounds_to_display(bounds: WindowBounds, cx: &mut App) -> WindowBounds {
    let WindowBounds::Windowed(b) = bounds else {
        return bounds;
    };
    let Some(display) = cx.primary_display() else {
        return WindowBounds::Windowed(b);
    };
    let db = display.bounds();
    // Reserve room for the OS top bar + dock.
    let avail_w = (f32::from(db.size.width) - 64.0).max(800.0);
    let avail_h = (f32::from(db.size.height) - 96.0).max(540.0);
    let w: f32 = f32::from(b.size.width).min(avail_w);
    let h: f32 = f32::from(b.size.height).min(avail_h);
    let max_x = (f32::from(db.origin.x) + f32::from(db.size.width) - w - 16.0)
        .max(f32::from(db.origin.x));
    let max_y = (f32::from(db.origin.y) + f32::from(db.size.height) - h - 16.0)
        .max(f32::from(db.origin.y));
    let x = f32::from(b.origin.x).clamp(f32::from(db.origin.x), max_x);
    let y = f32::from(b.origin.y).clamp(f32::from(db.origin.y), max_y);
    WindowBounds::Windowed(Bounds {
        origin: Point { x: px(x), y: px(y) },
        size: size(px(w), px(h)),
    })
}

fn main() {
    // 0. Enter Tokio runtime context so all Tokio primitives (timers, channels, reqwest)
    // work across the main GPUI thread and foreground async tasks.
    let _tokio_guard = TOKIO_RT.enter();

    // 1. Headless bootstrap: load settings, resolve backend executable path
    let settings = DesktopSettings::load().unwrap_or_default();
    let backend_path = resolve_backend_path(&settings);
    let spawn_opts = Some(SpawnOptions::new(backend_path));

    // 2. Initial window geometry restored via window_state module (default 1200x800)
    let restored_bounds = window_state::restore(&settings).unwrap_or_else(|| {
        WindowBounds::Windowed(Bounds {
            origin: Point {
                x: px(window_state::DEFAULT_ORIGIN_X),
                y: px(window_state::DEFAULT_ORIGIN_Y),
            },
            size: size(
                px(window_state::DEFAULT_WIDTH as f32),
                px(window_state::DEFAULT_HEIGHT as f32),
            ),
        })
    });

    let settings_arc = Arc::new(Mutex::new(settings.clone()));

    // 3. Launch GPUI application
    Application::with_platform(gpui_platform::current_platform(false)).run(move |cx: &mut App| {
        // Register bundled monospace font asset
        let font_bytes: &'static [u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");
        let _ = cx.text_system().add_fonts(vec![Cow::Borrowed(font_bytes)]);

        // Initialize gpui-component subsystem and apply theme
        gpui_component::init(cx);
        theme::apply_theme(settings.theme, cx);

        // Liquid Glass: publish the persisted preferences once, so surfaces
        // that render without an AppState in hand (the gpui-component dialogs)
        // can paint themselves as glass too. `AppState::set_glass_*` refreshes
        // this global whenever the settings page writes a new value.
        glass::GlassPrefs::register(cx, glass::GlassPrefs::from_settings(&settings));

        // Clamp the initial windowed bounds to the primary display so a
        // first-run window never opens taller than the screen (which hides
        // the sidebar footer behind the dock). Maximized restores pass
        // through untouched.
        let initial_bounds = fit_bounds_to_display(restored_bounds, cx);

        let window_options = WindowOptions {
            window_bounds: Some(initial_bounds),
            window_min_size: Some(size(px(800.0), px(540.0))),
            // Must match StartupWMClass/Icon in dist/webtmux-gpui.desktop so
            // docks (GNOME/KDE) group the window and show the app icon.
            app_id: Some("webtmux-gpui".to_string()),
            // Transparent + client decorations: the root view draws its own
            // CSD frame (see AppState::render), giving compositor shadow on
            // Linux instead of a fused rectangle. Without client decorations
            // the WM owns the frame and a custom titlebar breaks move/resize.
            window_background: WindowBackgroundAppearance::Transparent,
            window_decorations: Some(WindowDecorations::Client),
            titlebar: Some(TitlebarOptions {
                title: Some("Tmux GUI".into()),
                appears_transparent: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let settings_for_observe = settings_arc.clone();
        let _ = cx.open_window(window_options, move |window, cx| {
            // Field diagnostics: startup geometry for move/resize/footer
            // reports. Visible in the tmux-pane stderr log.
            eprintln!(
                "[webtmux] window opened viewport={:?} maximized={} decorations={:?}",
                window.viewport_size(),
                window.is_maximized(),
                window.window_decorations(),
            );
            eprintln!(
                "[webtmux] displays={} primary={:?}",
                cx.displays().len(),
                cx.primary_display().map(|d| d.bounds()),
            );
            #[cfg(target_os = "windows")]
            {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                if let Ok(handle) = HasWindowHandle::window_handle(window) {
                    if let RawWindowHandle::Win32(h) = handle.as_raw() {
                        let hwnd = h.hwnd.get();
                        extern "system" {
                            fn DwmSetWindowAttribute(
                                hwnd: isize,
                                dwAttribute: u32,
                                pvAttribute: *const std::ffi::c_void,
                                cbAttribute: u32,
                            ) -> i32;
                        }
                        let dark: i32 = 1;
                        unsafe {
                            DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as _, 4);
                            DwmSetWindowAttribute(hwnd, 19, &dark as *const _ as _, 4);
                        }
                    }
                }
            }

            // Liquid Glass: keep the transparent backdrop the CSD frame relies
            // on, upgraded to a real frost where the compositor implements one
            // (KWin/Hyprland on Wayland). A no-op everywhere else.
            glass::apply_backdrop_material(window, settings.glass_enabled);

            window_state::observe(window, settings_for_observe, cx);

            let app_state = cx.new(|_cx| AppState::new(settings, spawn_opts));
            app_state.update(cx, |this, cx| {
                this.start_supervisor(cx);
            });
            // Wrap in a gpui-component Root so the dialog/sheet APIs
            // (`Root::read`/`update` behind `open_dialog`, `close_dialog`,
            // `has_active_dialog`) resolve: they require the window root to
            // be a `Root`, otherwise they panic on the downcast. Transparent
            // bg + no border keeps our own CSD frame intact.
            cx.new(|cx| {
                gpui_component::Root::new(app_state, window, cx)
                    .bordered(false)
                    .bg(gpui::transparent_black())
            })
        });
    });
}
