//! Window state geometry restoration, toggle maximize/restore, and debounced persistence.

use std::sync::Arc;
use gpui::*;
use parking_lot::Mutex;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use webtmux_settings::{DesktopSettings, WindowState};

/// Default window dimensions for web-tmux (matches Electron desktop/main.js:184-188: 1200x800).
pub const DEFAULT_WIDTH: u32 = 1200;
pub const DEFAULT_HEIGHT: u32 = 800;

/// Default centered window origin for standard displays.
pub const DEFAULT_ORIGIN_X: f32 = 180.0;
pub const DEFAULT_ORIGIN_Y: f32 = 60.0;

/// Check if the window is currently maximized.
pub fn is_window_maximized(window: &Window) -> bool {
    #[cfg(target_os = "windows")]
    {
        if let Ok(handle) = HasWindowHandle::window_handle(window) {
            if let RawWindowHandle::Win32(h) = handle.as_raw() {
                let hwnd = h.hwnd.get();
                extern "system" {
                    fn IsZoomed(hwnd: isize) -> i32;
                }
                return unsafe { IsZoomed(hwnd) != 0 };
            }
        }
    }
    window.is_maximized()
}

/// Toggle maximize / restore.
pub fn toggle_maximize(window: &mut Window) -> bool {
    #[cfg(target_os = "windows")]
    {
        if let Ok(handle) = HasWindowHandle::window_handle(window) {
            if let RawWindowHandle::Win32(h) = handle.as_raw() {
                let hwnd = h.hwnd.get();
                extern "system" {
                    fn IsZoomed(hwnd: isize) -> i32;
                    fn ShowWindowAsync(hwnd: isize, nCmdShow: i32) -> i32;
                    fn DwmSetWindowAttribute(
                        hwnd: isize,
                        dwAttribute: u32,
                        pvAttribute: *const std::ffi::c_void,
                        cbAttribute: u32,
                    ) -> i32;
                    fn RedrawWindow(
                        hwnd: isize,
                        lprcUpdate: *const std::ffi::c_void,
                        hrgnUpdate: isize,
                        flags: u32,
                    ) -> i32;
                }
                const SW_RESTORE: i32 = 9;
                const SW_MAXIMIZE: i32 = 3;
                const RDW_INVALIDATE: u32 = 0x0001;
                const RDW_UPDATENOW: u32 = 0x0100;
                const RDW_ALLCHILDREN: u32 = 0x0080;

                let is_max = unsafe { IsZoomed(hwnd) != 0 };
                let dark: i32 = 1;

                unsafe {
                    // Force dark mode frame so DWM never flashes white
                    DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as _, 4);
                    DwmSetWindowAttribute(hwnd, 19, &dark as *const _ as _, 4);

                    if is_max {
                        ShowWindowAsync(hwnd, SW_RESTORE);
                    } else {
                        ShowWindowAsync(hwnd, SW_MAXIMIZE);
                    }
                    RedrawWindow(
                        hwnd,
                        std::ptr::null(),
                        0,
                        RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN,
                    );
                }

                window.on_next_frame(|window, _cx| {
                    window.refresh();
                });

                return !is_max;
            }
        }
    }
    let was_max = window.is_maximized();
    window.zoom_window();
    window.on_next_frame(|window, _cx| {
        window.refresh();
    });
    !was_max
}

/// Extract current window bounds and maximized state.
pub fn extract_window_state(window: &Window, prev_state: Option<&WindowState>) -> Option<WindowState> {
    let bounds = window.bounds();
    let current_width = (bounds.size.width / px(1.0)) as u32;
    let current_height = (bounds.size.height / px(1.0)) as u32;

    if current_width == 0 || current_height == 0 {
        return None;
    }

    let x = (bounds.origin.x / px(1.0)) as i32;
    let y = (bounds.origin.y / px(1.0)) as i32;

    if x <= -10000 || y <= -10000 {
        return None;
    }

    let maximized = is_window_maximized(window);

    if maximized {
        if let Some(prev) = prev_state {
            let mut p = *prev;
            p.maximized = true;
            return Some(p);
        }
        return Some(WindowState {
            x: None,
            y: None,
            width: Some(DEFAULT_WIDTH),
            height: Some(DEFAULT_HEIGHT),
            maximized: true,
        });
    }

    Some(WindowState {
        x: Some(x),
        y: Some(y),
        width: Some(current_width.clamp(800, 3840)),
        height: Some(current_height.clamp(500, 2160)),
        maximized: false,
    })
}

/// Restore window bounds from saved settings.
pub fn restore(settings: &DesktopSettings) -> Option<WindowBounds> {
    let state = settings.window_state.as_ref()?;

    let (w, h) = match (state.width, state.height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => (w.clamp(800, 3840), h.clamp(500, 2160)),
        _ => (DEFAULT_WIDTH, DEFAULT_HEIGHT),
    };

    let bounds = match (state.x, state.y) {
        (Some(x), Some(y)) if x > -10000 && y > -10000 => Bounds {
            origin: Point {
                x: px(x as f32),
                y: px(y as f32),
            },
            size: size(px(w as f32), px(h as f32)),
        },
        _ => Bounds {
            origin: Point {
                x: px(DEFAULT_ORIGIN_X),
                y: px(DEFAULT_ORIGIN_Y),
            },
            size: size(px(w as f32), px(h as f32)),
        },
    };

    if state.maximized {
        Some(WindowBounds::Maximized(bounds))
    } else {
        Some(WindowBounds::Windowed(bounds))
    }
}

/// Attach window close observer to persist geometry on shutdown.
pub fn observe(window: &mut Window, settings_arc: Arc<Mutex<DesktopSettings>>, cx: &mut App) {
    window.on_window_should_close(cx, move |window, _cx| {
        let prev = { settings_arc.lock().window_state };
        if let Some(new_state) = extract_window_state(window, prev.as_ref()) {
            let mut lock = settings_arc.lock();
            lock.window_state = Some(new_state);
            let _ = lock.save();
        }
        true
    });
}


