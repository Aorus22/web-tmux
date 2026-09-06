//! Phase 3 tracer: pure tab-lifecycle, rename re-resolution, and
//! title-bar window-tabs model tests (no GPUI runtime needed).

use webtmux::app_state::AppState;
use webtmux_backend_client::{
    SessionSnapshot, TmuxPane, TmuxSession, TmuxWindow, WsOutgoing,
};
use webtmux_settings::DesktopSettings;

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

fn tmux_window(id: &str, index: usize, name: &str, active: bool) -> TmuxWindow {
    TmuxWindow {
        id: id.to_string(),
        index,
        name: name.to_string(),
        active,
        panes: 1,
        width: 80,
        height: 24,
        layout: "tiled".to_string(),
    }
}

fn tmux_pane(id: &str, window_id: &str) -> TmuxPane {
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
        current_command: "zsh".to_string(),
        current_path: "/home/dev".to_string(),
        title: "shell".to_string(),
    }
}

pub fn test_snapshot(name: &str) -> SessionSnapshot {
    SessionSnapshot {
        session: tmux_session(name),
        windows: vec![
            tmux_window("@0", 0, "editor", false),
            tmux_window("@1", 1, "server", true),
        ],
        panes: vec![tmux_pane("%0", "@1")],
        active_window: "@1".to_string(),
        active_pane: "%0".to_string(),
        replace: false,
        seq: 0,
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

fn fresh_app() -> AppState {
    AppState::new(DesktopSettings::default(), None)
}

#[test]
fn test_tab_lifecycle() {
    let mut app = fresh_app();

    // Opening appends to open_sessions and activates.
    app.open_session("a");
    assert_eq!(app.open_sessions, vec!["a".to_string()]);
    assert_eq!(app.active_session.as_deref(), Some("a"));
    assert!(app.sessions.contains_key("a"));

    app.open_session("b");
    assert_eq!(app.open_sessions, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(app.active_session.as_deref(), Some("b"));

    // Re-opening an open tab only activates (no duplicate).
    app.open_session("a");
    assert_eq!(app.open_sessions, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(app.active_session.as_deref(), Some("a"));

    // Switching sets active_session only and never touches sockets.
    app.set_active_session(Some("b".to_string()));
    assert_eq!(app.active_session.as_deref(), Some("b"));
    for entry in app.sessions.values() {
        assert!(entry.handle.is_none(), "switch must never connect a socket");
    }

    // Closing a non-active tab leaves the active tab alone.
    app.open_session("c");
    assert_eq!(app.active_session.as_deref(), Some("c"));
    app.close_session("b");
    assert_eq!(app.open_sessions, vec!["a".to_string(), "c".to_string()]);
    assert_eq!(app.active_session.as_deref(), Some("c"));
    assert!(!app.sessions.contains_key("b"));

    // Closing the active tab activates the neighbor at min(idx, len-1).
    app.close_session("c"); // idx 1, remaining ["a"] -> "a"
    assert_eq!(app.open_sessions, vec!["a".to_string()]);
    assert_eq!(app.active_session.as_deref(), Some("a"));

    // Closing the last tab clears active.
    app.close_session("a");
    assert!(app.open_sessions.is_empty());
    assert!(app.sessions.is_empty());
    assert_eq!(app.active_session, None);

    // Closing an unknown tab is a no-op.
    app.close_session("ghost");
    assert_eq!(app.active_session, None);
}

#[test]
fn test_tab_close_neighbor_middle() {
    let mut app = fresh_app();
    for name in ["a", "b", "c"] {
        app.open_session(name);
    }
    app.set_active_session(Some("b".to_string()));
    // Closing active middle tab "b" (idx 1): neighbor min(1, 1) -> "c".
    app.close_session("b");
    assert_eq!(app.open_sessions, vec!["a".to_string(), "c".to_string()]);
    assert_eq!(app.active_session.as_deref(), Some("c"));
}

#[test]
fn test_rename_reresolution() {
    let mut app = fresh_app();
    app.open_session("old");
    app.expanded_sessions.insert("old".to_string());
    let gen_before = app.sessions.get("old").unwrap().generation;

    assert!(app.rename_session_entry("old", "new"));
    assert!(!app.sessions.contains_key("old"));
    assert!(app.sessions.contains_key("new"));
    assert_eq!(app.open_sessions, vec!["new".to_string()]);
    assert_eq!(app.active_session.as_deref(), Some("new"));
    assert!(app.expanded_sessions.contains("new"));
    assert!(!app.expanded_sessions.contains("old"));

    // Generation bumped exactly once.
    let gen_after = app.sessions.get("new").unwrap().generation;
    assert_eq!(gen_after, gen_before + 1);

    // Stale old-generation events drop post-migration.
    let snap = test_snapshot("new");
    assert!(!app.apply_event("new", gen_before, &snapshot_msg("new", snap.clone())));
    // Current generation applies.
    assert!(app.apply_event("new", gen_after, &snapshot_msg("new", snap)));
    // Old name is now unknown: dropped.
    assert!(!app.apply_event(
        "old",
        gen_after,
        &snapshot_msg("old", test_snapshot("old"))
    ));

    // Rejections: unknown old, existing new, empty new, identity.
    assert!(!app.rename_session_entry("ghost", "x"));
    app.open_session("other");
    assert!(!app.rename_session_entry("new", "other"));
    assert!(!app.rename_session_entry("new", ""));
    assert!(!app.rename_session_entry("new", "new"));
}

#[test]
fn test_window_tabs_model() {
    let mut app = fresh_app();

    // No active session -> empty middle, no panic.
    assert!(app.window_tabs().is_empty());

    // Active session open but no snapshot yet -> empty.
    app.open_session("dev");
    assert!(app.window_tabs().is_empty());

    // Snapshot committed -> tabs derived, activeness from active_window.
    let gen = app.sessions.get("dev").unwrap().generation;
    assert!(app.apply_event("dev", gen, &snapshot_msg("dev", test_snapshot("dev"))));
    let tabs = app.window_tabs();
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs[0].id, "@0");
    assert_eq!(tabs[0].index, 0);
    assert_eq!(tabs[0].name, "editor");
    assert!(!tabs[0].active);
    assert_eq!(tabs[1].id, "@1");
    assert!(tabs[1].active);

    // Empty-windows snapshot -> empty middle, no panic.
    let mut empty = test_snapshot("dev");
    empty.windows.clear();
    let gen = app.sessions.get("dev").unwrap().generation;
    assert!(app.apply_event("dev", gen, &snapshot_msg("dev", empty)));
    assert!(app.window_tabs().is_empty());
}
