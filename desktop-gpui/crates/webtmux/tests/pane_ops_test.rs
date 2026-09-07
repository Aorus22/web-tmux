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
use webtmux::views::window_toolbar::{NEXT_LAYOUT_ID, WINDOW_LAYOUT_PRESETS};
use webtmux_backend_client::{
    MSG_HELLO, MSG_PANE_RESIZE, MSG_TERMINAL_RESIZE, SessionSnapshot, TmuxPane, TmuxSession,
    TmuxWindow, WsOutgoing,
};
use webtmux_settings::DesktopSettings;

fn fresh_app() -> AppState {
    AppState::new(DesktopSettings::default(), None)
}

fn tmux_session(name: &str) -> TmuxSession {
    TmuxSession {
        name: name.to_string(),
        windows: 2,
        attached: 0,
        created_at: 1717200000,
        width: 80,
        height: 24,
    }
}

fn tmux_window(id: &str, index: usize, name: &str) -> TmuxWindow {
    TmuxWindow {
        id: id.to_string(),
        index,
        name: name.to_string(),
        active: false,
        panes: 1,
        width: 80,
        height: 24,
        layout: "tiled".to_string(),
    }
}

fn tmux_pane(id: &str, window_id: &str, command: &str, title: &str) -> TmuxPane {
    TmuxPane {
        id: id.to_string(),
        index: 0,
        window_id: window_id.to_string(),
        active: true,
        zoomed: false,
        left: 0,
        top: 0,
        width: 80,
        height: 24,
        pid: 1234,
        current_command: command.to_string(),
        current_path: "/home/dev".to_string(),
        title: title.to_string(),
    }
}

fn snapshot_msg(session: &str, snap: SessionSnapshot) -> WsOutgoing {
    WsOutgoing {
        msg_type: "state.snapshot".to_string(),
        session: Some(session.to_string()),
        snapshot: Some(snap),
        ..Default::default()
    }
}

/// Two-window snapshot: `%0` + `%1` live in the active `@1`, `%2` sits in
/// the background `@0`.
fn two_window_snapshot(name: &str) -> SessionSnapshot {
    SessionSnapshot {
        session: tmux_session(name),
        windows: vec![tmux_window("@0", 0, "bg"), tmux_window("@1", 1, "main")],
        panes: vec![
            tmux_pane("%0", "@1", "nvim", "editor"),
            tmux_pane("%1", "@1", "zsh", ""),
            tmux_pane("%2", "@0", "htop", ""),
        ],
        active_window: "@1".to_string(),
        active_pane: "%0".to_string(),
        replace: false,
        seq: 0,
    }
}

fn app_with_snapshot() -> AppState {
    let mut app = fresh_app();
    app.open_session("dev");
    let gen = app.sessions.get("dev").unwrap().generation;
    assert!(app.apply_event("dev", gen, &snapshot_msg("dev", two_window_snapshot("dev"))));
    app
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

#[test]
fn test_swap_picker_scope() {
    // Picker lists same-window panes only, excludes the current pane, and
    // labels each `currentCommand || title || currentPath` (PANE-05 pitfall
    // lock — `snapshot.panes` is flat across the session).
    let app = app_with_snapshot();
    let cands = app.swap_candidates("%0");
    assert_eq!(cands.len(), 1, "only %1 shares @1 with %0");
    assert_eq!(cands[0].id, "%1");
    // %1 has no title → falls back to its current command.
    assert_eq!(cands[0].label, "zsh");

    let cands = app.swap_candidates("%1");
    assert_eq!(cands.len(), 1);
    assert_eq!(cands[0].id, "%0");
    // %0 has both → current command wins (FE picker order, unlike prefill).
    assert_eq!(cands[0].label, "nvim");

    // Lone pane in the background window → empty (Swap disables).
    let app = app_with_snapshot();
    assert!(app.swap_candidates("%2").is_empty());
    // Unknown pane → empty, never panics.
    assert!(app.swap_candidates("%9").is_empty());
}

#[test]
fn test_layout_strings() {
    // Toolbar buttons map to the six verbatim layout strings incl.
    // `next-layout` (PANE-06 per D7 — opaque passthrough, never validated).
    let ids: Vec<&str> = WINDOW_LAYOUT_PRESETS.iter().map(|p| p.id).collect();
    assert_eq!(
        ids,
        vec![
            "even-horizontal",
            "even-vertical",
            "main-horizontal",
            "main-vertical",
            "tiled",
        ]
    );
    assert_eq!(NEXT_LAYOUT_ID, "next-layout");
}
