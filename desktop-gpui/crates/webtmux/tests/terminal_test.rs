//! Phase 4 tracer (Task 2 RED): AppState TerminalStore contracts.
//!
//! Exactly-once replay, hidden-session ingest under the triple guard,
//! D7 stale-pane retirement, and terminal.input owning-socket resolution.
//! All headless: sends assert on envelope construction + socket resolution
//! (no live socket needed).

use webtmux::app_state::AppState;
use webtmux_backend_client::{
    SessionSnapshot, TmuxPane, TmuxSession, TmuxWindow, WsOutgoing, EV_STATE_SNAPSHOT,
    EV_TERMINAL_OUTPUT, EV_TERMINAL_SNAPSHOT, MSG_TERMINAL_INPUT,
};
use webtmux_settings::DesktopSettings;

fn tmux_session(name: &str) -> TmuxSession {
    TmuxSession {
        name: name.to_string(),
        windows: 1,
        attached: 0,
        created_at: 1717200000,
        width: 80,
        height: 24,
    }
}

fn tmux_window(id: &str) -> TmuxWindow {
    TmuxWindow {
        id: id.to_string(),
        index: 0,
        name: "main".to_string(),
        active: true,
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

fn snapshot_with_panes(name: &str, panes: Vec<(&str, &str)>) -> SessionSnapshot {
    SessionSnapshot {
        session: tmux_session(name),
        windows: vec![tmux_window("@0")],
        panes: panes
            .into_iter()
            .map(|(id, win)| tmux_pane(id, win))
            .collect(),
        active_window: "@0".to_string(),
        active_pane: String::new(),
        replace: false,
        seq: 0,
    }
}

fn state_msg(session: &str, snap: SessionSnapshot) -> WsOutgoing {
    WsOutgoing {
        msg_type: EV_STATE_SNAPSHOT.to_string(),
        session: Some(session.to_string()),
        snapshot: Some(snap),
        ..Default::default()
    }
}

fn terminal_frame(
    msg_type: &str,
    session: &str,
    pane: &str,
    data: &str,
    replace: bool,
    screen_rows: Option<i32>,
) -> WsOutgoing {
    WsOutgoing {
        msg_type: msg_type.to_string(),
        session: Some(session.to_string()),
        pane_id: Some(pane.to_string()),
        data: Some(data.to_string()),
        replace,
        screen_rows,
        ..Default::default()
    }
}

fn fresh_app() -> AppState {
    AppState::new(DesktopSettings::default(), None)
}

/// Commit a state snapshot for `session` and return its generation.
fn commit_state(app: &mut AppState, session: &str, panes: Vec<(&str, &str)>) {
    let gen = app.sessions.get(session).unwrap().generation;
    assert!(app.apply_event(session, gen, &state_msg(session, snapshot_with_panes(session, panes))));
}

#[test]
fn test_snapshot_exactly_once() {
    let mut app = fresh_app();
    app.open_session("dev");
    commit_state(&mut app, "dev", vec![("%0", "@0")]);
    let gen = app.sessions.get("dev").unwrap().generation;

    // Fresh pane: empty grid.
    assert_eq!(app.pane_grid_text("%0").unwrap()[0], "");

    // First snapshot replays history exactly once then paints the screen.
    let blob1 = "h1\nh2\ns1\ns2";
    assert!(app.apply_event(
        "dev",
        gen,
        &terminal_frame(EV_TERMINAL_SNAPSHOT, "dev", "%0", blob1, true, Some(2)),
    ));
    let grid = app.pane_grid_text("%0").unwrap();
    assert_eq!(grid[0], "s1");
    assert_eq!(grid[1], "s2");
    assert_eq!(app.pane_ingested_history("%0"), Some(2));

    // Second identical snapshot drops: no content doubling, counter still.
    assert!(app.apply_event(
        "dev",
        gen,
        &terminal_frame(EV_TERMINAL_SNAPSHOT, "dev", "%0", blob1, true, Some(2)),
    ));
    let grid = app.pane_grid_text("%0").unwrap();
    assert_eq!(grid[0], "s1");
    assert_eq!(grid[1], "s2");
    assert_eq!(app.pane_ingested_history("%0"), Some(2));

    // replace=true output bypasses the gate: grown history accumulates.
    let blob2 = "h1\nh2\nh3\ns1\ns2";
    assert!(app.apply_event(
        "dev",
        gen,
        &terminal_frame(EV_TERMINAL_OUTPUT, "dev", "%0", blob2, true, Some(2)),
    ));
    assert_eq!(app.pane_ingested_history("%0"), Some(3));
    let grid = app.pane_grid_text("%0").unwrap();
    assert_eq!(grid[0], "s1");
    assert_eq!(grid[1], "s2");

    // Post-invalidate snapshot applies again.
    app.invalidate_pane_snapshot("%0");
    assert!(app.apply_event(
        "dev",
        gen,
        &terminal_frame(EV_TERMINAL_SNAPSHOT, "dev", "%0", blob1, true, Some(2)),
    ));
    let grid = app.pane_grid_text("%0").unwrap();
    assert_eq!(grid[0], "s1");

    // Live output continues seamlessly after replay.
    assert!(app.apply_event(
        "dev",
        gen,
        &terminal_frame(EV_TERMINAL_OUTPUT, "dev", "%0", "LIVE", false, None),
    ));
    let grid = app.pane_grid_text("%0").unwrap();
    assert!(
        grid.iter().any(|row| row.contains("LIVE")),
        "live bytes land on the grid: {grid:?}"
    );
}

#[test]
fn test_hidden_session_ingest() {
    let mut app = fresh_app();
    app.open_session("a");
    app.open_session("b");
    app.set_active_session(Some("a".to_string()));
    commit_state(&mut app, "a", vec![("%a0", "@0")]);
    commit_state(&mut app, "b", vec![("%b0", "@0")]);
    let gen_b = app.sessions.get("b").unwrap().generation;

    // Terminal events for the non-active session commit to the store.
    assert!(app.apply_event(
        "b",
        gen_b,
        &terminal_frame(EV_TERMINAL_OUTPUT, "b", "%b0", "hello-b", false, None),
    ));
    let grid = app.pane_grid_text("%b0").unwrap();
    assert!(grid.iter().any(|row| row.contains("hello-b")));

    // Stale generation drops.
    app.sessions.get_mut("b").unwrap().generation += 1;
    let new_gen = app.sessions.get("b").unwrap().generation;
    assert!(!app.apply_event(
        "b",
        gen_b,
        &terminal_frame(EV_TERMINAL_OUTPUT, "b", "%b0", "stale", false, None),
    ));
    let grid = app.pane_grid_text("%b0").unwrap();
    assert!(!grid.iter().any(|row| row.contains("stale")));

    // Envelope session mismatch drops.
    let mismatched = WsOutgoing {
        session: Some("a".to_string()),
        ..terminal_frame(EV_TERMINAL_OUTPUT, "a", "%b0", "leak", false, None)
    };
    assert!(!app.apply_event("b", new_gen, &mismatched));

    // Unknown session drops (tab-liveness).
    assert!(!app.apply_event(
        "ghost",
        new_gen,
        &terminal_frame(EV_TERMINAL_OUTPUT, "ghost", "%b0", "ghost", false, None),
    ));
}

#[test]
fn test_stale_pane_retirement() {
    let mut app = fresh_app();
    app.open_session("dev");
    commit_state(&mut app, "dev", vec![("%0", "@0"), ("%1", "@0")]);
    let gen = app.sessions.get("dev").unwrap().generation;

    for pane in ["%0", "%1"] {
        assert!(app.apply_event(
            "dev",
            gen,
            &terminal_frame(EV_TERMINAL_SNAPSHOT, "dev", pane, "h\ns", true, Some(1)),
        ));
        // D8 title commit via OSC.
        assert!(app.apply_event(
            "dev",
            gen,
            &terminal_frame(
                EV_TERMINAL_OUTPUT,
                "dev",
                pane,
                "\x1b]0;title-pane\x07",
                false,
                None
            ),
        ));
    }
    assert!(app.pane_grid_text("%1").is_some());
    assert_eq!(app.pane_title("%1").as_deref(), Some("title-pane"));

    // New snapshot omits %1: entry (and counter + title) retires.
    commit_state(&mut app, "dev", vec![("%0", "@0")]);
    assert!(app.pane_grid_text("%1").is_none());
    assert_eq!(app.pane_ingested_history("%1"), None);
    assert_eq!(app.pane_title("%1"), None);
    // Survivor untouched.
    assert!(app.pane_grid_text("%0").is_some());

    // A later snapshot re-creates the pane fresh (gate open, counter reset).
    commit_state(&mut app, "dev", vec![("%0", "@0"), ("%1", "@0")]);
    let gen = app.sessions.get("dev").unwrap().generation;
    assert!(app.apply_event(
        "dev",
        gen,
        &terminal_frame(EV_TERMINAL_SNAPSHOT, "dev", "%1", "h\ns", true, Some(1)),
    ));
    assert_eq!(app.pane_ingested_history("%1"), Some(1));
}

#[test]
fn test_terminal_input_ownership() {
    let mut app = fresh_app();
    app.open_session("a");
    app.open_session("b");
    app.set_active_session(Some("a".to_string()));
    commit_state(&mut app, "a", vec![("%a0", "@0")]);
    commit_state(&mut app, "b", vec![("%b1", "@0")]);

    // Byte-safe payload incl. CJK built through from_utf8 semantics.
    let bytes = "\x1b[A中".as_bytes().to_vec();
    let data = String::from_utf8(bytes).expect("input bytes are UTF-8");

    // Resolves the OWNING session socket — never the active-session proxy.
    let (session, envelope) = app
        .build_terminal_input("%b1", data.clone())
        .expect("known pane resolves");
    assert_eq!(session, "b");
    assert_eq!(envelope.msg_type, MSG_TERMINAL_INPUT);
    assert_eq!(envelope.pane_id.as_deref(), Some("%b1"));
    assert_eq!(envelope.data.as_deref(), Some(data.as_str()));

    // Special-key bytes ride the same path untouched.
    let (_, arrow) = app
        .build_terminal_input("%a0", "\x1b[5~".to_string())
        .expect("pane of active session resolves to its own socket");
    assert_eq!(arrow.data.as_deref(), Some("\x1b[5~"));

    // Unknown pane: typed miss, nothing sent, no panic.
    assert!(app.build_terminal_input("%ghost", "x".to_string()).is_none());
    assert!(!app.send_terminal_input("%ghost", "x".to_string()));
    assert!(!app.request_pane_capture("%ghost"));

    // tui_scroll defaults ON for unknown panes (D3).
    assert!(app.tui_scroll("%fresh-pane"));
}
