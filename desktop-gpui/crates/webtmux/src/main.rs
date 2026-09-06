//! webtmux desktop client (GPUI) application binary entry point.

use std::borrow::Cow;
use std::sync::Arc;
use gpui::*;
use parking_lot::Mutex;
use webtmux::app_state::{AppState, TOKIO_RT};
use webtmux::bundle::resolve_backend_path;
use webtmux::theme;
use webtmux::window_state;
use webtmux_settings::DesktopSettings;
use webtmux_supervisor::SpawnOptions;

fn main() {
    // 0. Enter Tokio runtime context so all Tokio primitives (timers, channels, reqwest)
    // work across the main GPUI thread and foreground async tasks.
    let _tokio_guard = TOKIO_RT.enter();

    // 1. Headless bootstrap: load settings, resolve backend executable path
    let settings = DesktopSettings::load().unwrap_or_default();
    let backend_path = resolve_backend_path(&settings);
    let spawn_opts = Some(SpawnOptions::new(backend_path));

    // 2. Initial window geometry restored via window_state module (default 1200x800)
    let initial_bounds = window_state::restore(&settings).unwrap_or_else(|| {
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

        let window_options = WindowOptions {
            window_bounds: Some(initial_bounds),
            window_min_size: Some(size(px(800.0), px(500.0))),
            titlebar: Some(TitlebarOptions {
                title: Some("Tmux GUI".into()),
                appears_transparent: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        let settings_for_observe = settings_arc.clone();
        let _ = cx.open_window(window_options, move |window, cx| {
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

            window_state::observe(window, settings_for_observe, cx);

            let app_state = cx.new(|_cx| AppState::new(settings, spawn_opts));
            app_state.update(cx, |this, cx| {
                this.start_supervisor(cx);
            });
            app_state
        });
    });
}
