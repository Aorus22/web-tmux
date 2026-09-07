//! Phase 5 wave-2 pane-ops headless tests (05-02, PANE-04).
//!
//! Divider-drag state machine: 40ms throttle accounting without advancing
//! `last_cells`, negative-step FLIP with positive amounts, zero-step drops,
//! and the drag path constructing only `pane.resize` envelopes (never
//! `terminal.resize`, never `hello`). All pure `AppState`/`pane_geometry`
//! helpers — no GPUI runtime needed.

use std::time::{Duration, Instant};

use webtmux::app_state::AppState;
use webtmux::pane_geometry::drag_step_throttled;
use webtmux_backend_client::{MSG_HELLO, MSG_PANE_RESIZE, MSG_TERMINAL_RESIZE};
use webtmux_settings::DesktopSettings;

fn fresh_app() -> AppState {
    AppState::new(DesktopSettings::default(), None)
}

#[test]
fn test_drag_throttle() {
    // First move inside the 40ms window sends nothing and does NOT advance
    // `last_cells` (PANE-04 per D3 — throttled steps are not marked sent).
    let mut app = fresh_app();
    let t0 = Instant::now();
    app.begin_pane_drag_at("%1", 'R', 100.0, 8.0, t0);
    // 24px at 8px/cell = 3 cells pending, but 10ms < 40ms → dropped.
    assert_eq!(
        app.poll_pane_drag(t0 + Duration::from_millis(10), 124.0),
        None
    );
    assert_eq!(app.pane_drag.as_ref().map(|d| d.last_cells), Some(0));

    // The move after the window sends the FULL accumulated step.
    assert_eq!(
        app.poll_pane_drag(t0 + Duration::from_millis(50), 124.0),
        Some(("%1".to_string(), 'R', 3))
    );
    assert_eq!(app.pane_drag.as_ref().map(|d| d.last_cells), Some(3));
}

#[test]
fn test_drag_flip() {
    // Negative displacement returns the FLIP direction with positive amount
    // (PANE-04 per D3 — tmux rejects negative adjustments).
    let mut app = fresh_app();
    let t0 = Instant::now();
    app.begin_pane_drag_at("%1", 'R', 100.0, 8.0, t0);
    // -16px at 8px/cell = -2 cells → flip R→L, amount 2.
    assert_eq!(
        app.poll_pane_drag(t0 + Duration::from_millis(50), 84.0),
        Some(("%1".to_string(), 'L', 2))
    );
    // Sub-cell jitter netting zero after accounting drops.
    assert_eq!(
        app.poll_pane_drag(t0 + Duration::from_millis(100), 86.0),
        None
    );

    // Pure helper agrees: negative flips, zero drops, throttle gates.
    assert_eq!(
        drag_step_throttled('D', 164.0, 200.0, 0, 18.0, 50),
        Some(('U', 2, -2))
    );
    assert_eq!(drag_step_throttled('R', 102.0, 100.0, 0, 8.0, 100), None);
    assert_eq!(
        drag_step_throttled('R', 124.0, 100.0, 0, 8.0, 10),
        None
    );
}

#[test]
fn test_drag_no_viewport_send() {
    // The drag path constructs only `pane.resize` envelopes — never
    // `terminal.resize` (Phase-4 layout-key timers own viewport resync),
    // never `hello` (PANE-04 pitfall lock).
    let msg = AppState::build_pane_resize("%1", 'R', 2);
    assert_eq!(msg.msg_type, MSG_PANE_RESIZE);
    assert_ne!(msg.msg_type, MSG_TERMINAL_RESIZE);
    assert_ne!(msg.msg_type, MSG_HELLO);

    // Without a live socket the drag step sends nothing anywhere
    // (no viewport fallback, no panic).
    let app = fresh_app();
    assert!(!app.submit_pane_resize("%9", 'R', 2));
    assert!(!app.submit_pane_resize("%9", 'R', 0));
}
