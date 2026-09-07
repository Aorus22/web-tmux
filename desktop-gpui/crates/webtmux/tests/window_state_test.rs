//! STATE.md-gap clamp-guard coverage (D9).
//!
//! Covers only the pure `restore` paths that need no `Window`:
//! widths clamp 800..3840, heights 500..2160, -10000 sentinel passes through
//! to the default origin, maximized bounds preserved.

use webtmux::window_state::{DEFAULT_HEIGHT, DEFAULT_WIDTH};
use webtmux_settings::{DesktopSettings, WindowState};

fn settings_with(state: WindowState) -> DesktopSettings {
    DesktopSettings {
        window_state: Some(state),
        ..Default::default()
    }
}

#[test]
fn test_window_state_clamps() {
    // Width clamps 800..3840.
    let s = settings_with(WindowState {
        x: Some(100),
        y: Some(100),
        width: Some(100),
        height: Some(800),
        maximized: false,
    });
    let bounds = webtmux::window_state::restore(&s).expect("bounds");
    let size = match bounds {
        gpui::WindowBounds::Windowed(b) | gpui::WindowBounds::Maximized(b) => b.size,
    };
    assert_eq!(size.width, gpui::px(800.0));
    let s = settings_with(WindowState {
        x: Some(100),
        y: Some(100),
        width: Some(5000),
        height: Some(800),
        maximized: false,
    });
    let bounds = webtmux::window_state::restore(&s).expect("bounds");
    let size = match bounds {
        gpui::WindowBounds::Windowed(b) | gpui::WindowBounds::Maximized(b) => b.size,
    };
    assert_eq!(size.width, gpui::px(3840.0));

    // Height clamps 500..2160.
    let s = settings_with(WindowState {
        x: Some(100),
        y: Some(100),
        width: Some(1200),
        height: Some(100),
        maximized: false,
    });
    let bounds = webtmux::window_state::restore(&s).expect("bounds");
    let size = match bounds {
        gpui::WindowBounds::Windowed(b) | gpui::WindowBounds::Maximized(b) => b.size,
    };
    assert_eq!(size.height, gpui::px(500.0));
    let s = settings_with(WindowState {
        x: Some(100),
        y: Some(100),
        width: Some(1200),
        height: Some(5000),
        maximized: false,
    });
    let bounds = webtmux::window_state::restore(&s).expect("bounds");
    let size = match bounds {
        gpui::WindowBounds::Windowed(b) | gpui::WindowBounds::Maximized(b) => b.size,
    };
    assert_eq!(size.height, gpui::px(2160.0));

    // -10000 sentinel passes through to the default origin.
    let s = settings_with(WindowState {
        x: Some(-10000),
        y: Some(100),
        width: Some(1200),
        height: Some(800),
        maximized: false,
    });
    let bounds = webtmux::window_state::restore(&s).expect("bounds");
    let origin = match bounds {
        gpui::WindowBounds::Windowed(b) | gpui::WindowBounds::Maximized(b) => b.origin,
    };
    assert_eq!(origin.x, gpui::px(180.0));
    assert_eq!(origin.y, gpui::px(60.0));

    // Zero/missing dims fall back to defaults.
    let s = settings_with(WindowState {
        x: Some(100),
        y: Some(100),
        width: Some(0),
        height: Some(0),
        maximized: false,
    });
    let bounds = webtmux::window_state::restore(&s).expect("bounds");
    let size = match bounds {
        gpui::WindowBounds::Windowed(b) | gpui::WindowBounds::Maximized(b) => b.size,
    };
    assert_eq!(size.width, gpui::px(DEFAULT_WIDTH as f32));
    assert_eq!(size.height, gpui::px(DEFAULT_HEIGHT as f32));

    // Maximized bounds preserved.
    let s = settings_with(WindowState {
        x: Some(100),
        y: Some(100),
        width: Some(1200),
        height: Some(800),
        maximized: true,
    });
    let bounds = webtmux::window_state::restore(&s).expect("bounds");
    assert!(matches!(bounds, gpui::WindowBounds::Maximized(_)));
    let s = settings_with(WindowState {
        x: Some(100),
        y: Some(100),
        width: Some(1200),
        height: Some(800),
        maximized: false,
    });
    let bounds = webtmux::window_state::restore(&s).expect("bounds");
    assert!(matches!(bounds, gpui::WindowBounds::Windowed(_)));
}
