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

    // Fresh pane: no store entry until the first terminal frame lands.
    assert_eq!(app.pane_grid_text("%0"), None);

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

// --- Phase 4 plan 04-02 Task 1 RED: wheel / selection / copy-paste depth ---

use webtmux::app_state::{
    decide_key_route, decide_wheel_action, wheel_delta_to_lines, wheel_delta_to_px, KeyRoute,
    WheelAction, WheelDelta,
};
use webtmux_terminal::{AlacPoint, Column, Line, TermMode};

fn alac_origin() -> AlacPoint {
    AlacPoint::new(Line(0), Column(0))
}

#[test]
fn test_wheel_policy() {
    // Mouse-reporting mode → SGR 64 (up) / 65 (down), no paging.
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let (up, _) = decide_wheel_action(
        mode,
        true,
        WheelDelta::Pixels(120.0),
        0.0,
        18.9,
        alac_origin(),
        0,
    );
    match up {
        WheelAction::Sgr(bytes) => {
            assert_eq!(String::from_utf8(bytes).unwrap(), "\x1b[<64;1;1M")
        }
        other => panic!("mouse mode must emit SGR up, got {other:?}"),
    }
    let (down, _) = decide_wheel_action(
        mode,
        true,
        WheelDelta::Pixels(-120.0),
        0.0,
        18.9,
        alac_origin(),
        0,
    );
    match down {
        WheelAction::Sgr(bytes) => {
            assert_eq!(String::from_utf8(bytes).unwrap(), "\x1b[<65;1;1M")
        }
        other => panic!("mouse mode must emit SGR down, got {other:?}"),
    }

    // TUI-on (incl. absent map entry = ON per D3) → 100px/notch PageUp/Down.
    let plain = TermMode::empty();
    // Sub-notch accumulates without emitting.
    let (ignored, accum) = decide_wheel_action(
        plain,
        true,
        WheelDelta::Pixels(40.0),
        0.0,
        18.9,
        alac_origin(),
        0,
    );
    assert_eq!(ignored, WheelAction::Ignored);
    assert!((accum - 40.0).abs() < f32::EPSILON);
    // Crossing the notch emits one PageUp (GPUI positive = wheel-up, matching
    // the SGR/scrollback sign above and the reference view).
    let (paged, accum2) = decide_wheel_action(
        plain,
        true,
        WheelDelta::Pixels(70.0),
        accum,
        18.9,
        alac_origin(),
        0,
    );
    match paged {
        WheelAction::Pages(bytes) => {
            assert_eq!(String::from_utf8(bytes).unwrap(), "\x1b[5~")
        }
        other => panic!("TUI-on must page, got {other:?}"),
    }
    assert!(
        (accum2 - 10.0).abs() < 1.0,
        "leftover must be 110-100=10, got {accum2}"
    );

    // Line deltas scale ×16 into px (FE parity): 7 lines = 112px = 1 notch.
    assert!((wheel_delta_to_px(WheelDelta::Lines(7.0)) - 112.0).abs() < f32::EPSILON);
    let (line_page, _) = decide_wheel_action(
        plain,
        true,
        WheelDelta::Lines(7.0),
        0.0,
        18.9,
        alac_origin(),
        0,
    );
    match line_page {
        WheelAction::Pages(bytes) => {
            assert_eq!(String::from_utf8(bytes).unwrap(), "\x1b[5~")
        }
        other => panic!("line-delta TUI paging failed, got {other:?}"),
    }

    // Burst clamp 3: a fast fling never emits more than 3 repeats.
    let (burst, _) = decide_wheel_action(
        plain,
        true,
        WheelDelta::Pixels(1000.0),
        0.0,
        18.9,
        alac_origin(),
        0,
    );
    match burst {
        WheelAction::Pages(bytes) => {
            assert_eq!(String::from_utf8(bytes).unwrap(), "\x1b[5~\x1b[5~\x1b[5~")
        }
        other => panic!("burst must clamp to 3, got {other:?}"),
    }
    // Wheel down (negative px) pages down.
    let (down_page, _) = decide_wheel_action(
        plain,
        true,
        WheelDelta::Pixels(-100.0),
        0.0,
        18.9,
        alac_origin(),
        0,
    );
    match down_page {
        WheelAction::Pages(bytes) => {
            assert_eq!(String::from_utf8(bytes).unwrap(), "\x1b[6~")
        }
        other => panic!("wheel down must send PageDown, got {other:?}"),
    }

    // TUI-off → scrollback delta lines via the reference pixel→lines conversion.
    assert_eq!(wheel_delta_to_lines(WheelDelta::Pixels(37.8), 18.9), 2);
    let (sb, _) = decide_wheel_action(
        plain,
        false,
        WheelDelta::Pixels(37.8),
        0.0,
        18.9,
        alac_origin(),
        0,
    );
    assert_eq!(sb, WheelAction::Scrollback(2));

    // AppState-side accumulation + default-ON switch (absent entry = ON).
    let mut app = fresh_app();
    app.open_session("dev");
    commit_state(&mut app, "dev", vec![("%0", "@0")]);
    assert!(app.tui_scroll("%0"));
    let a1 = app.apply_wheel("%0", WheelDelta::Pixels(40.0), 18.9, alac_origin(), 0);
    assert_eq!(a1, WheelAction::Ignored);
    let a2 = app.apply_wheel("%0", WheelDelta::Pixels(70.0), 18.9, alac_origin(), 0);
    match a2 {
        WheelAction::Pages(bytes) => {
            assert_eq!(String::from_utf8(bytes).unwrap(), "\x1b[5~")
        }
        other => panic!("AppState accum must page on notch, got {other:?}"),
    }
}

#[test]
fn test_selection_text() {
    let mut app = fresh_app();
    app.open_session("dev");
    commit_state(&mut app, "dev", vec![("%0", "@0")]);
    let gen = app.sessions.get("dev").unwrap().generation;
    assert!(app.apply_event(
        "dev",
        gen,
        &terminal_frame(EV_TERMINAL_OUTPUT, "dev", "%0", "hello selection", false, None),
    ));
    // Programmatic selection across the first row yields the fed text.
    {
        let entry = app.terminals.get("%0").expect("store entry must exist");
        let mut term = entry.terminal.lock();
        term.start_selection(
            AlacPoint::new(Line(0), Column(0)),
            webtmux_terminal::selection_type_from_clicks(1),
        );
        term.update_selection(AlacPoint::new(Line(0), Column(14)));
    }
    let text = app
        .pane_selection_text("%0")
        .expect("selection text must exist");
    assert!(
        text.contains("hello selection"),
        "selection must return fed grid text, got {text:?}"
    );
}

#[test]
fn test_copy_paste_keys() {
    // Copy keys with a selection route to copy (not to terminal.input).
    assert_eq!(
        decide_key_route(true, true, true, false, "c"),
        KeyRoute::Copy
    );
    assert_eq!(
        decide_key_route(true, false, false, true, "c"),
        KeyRoute::Copy
    );
    // Paste keys route clipboard content to terminal.input.
    assert_eq!(
        decide_key_route(false, true, true, false, "v"),
        KeyRoute::Paste
    );
    assert_eq!(
        decide_key_route(false, false, false, true, "v"),
        KeyRoute::Paste
    );
    // Everything else falls through to the terminal byte path.
    assert_eq!(
        decide_key_route(false, false, false, false, "c"),
        KeyRoute::Terminal
    );
    // With EMPTY selection Ctrl+C falls through to interrupt bytes (\x03).
    assert_eq!(
        decide_key_route(false, true, false, false, "c"),
        KeyRoute::Terminal
    );
    let ctrl_c = webtmux_terminal::keystroke_to_bytes(
        &gpui::Keystroke {
            modifiers: gpui::Modifiers {
                control: true,
                ..Default::default()
            },
            key: "c".into(),
            key_char: None,
        },
        TermMode::empty(),
    )
    .expect("Ctrl+C must map to bytes");
    assert_eq!(ctrl_c, vec![0x03]);
    // Paste byte path rides terminal.input untouched (no client chunking).
    let mut app = fresh_app();
    app.open_session("dev");
    commit_state(&mut app, "dev", vec![("%0", "@0")]);
    let (_, envelope) = app
        .build_terminal_input("%0", "pasted text 中".to_string())
        .expect("paste builds terminal.input");
    assert_eq!(envelope.msg_type, MSG_TERMINAL_INPUT);
    assert_eq!(envelope.data.as_deref(), Some("pasted text 中"));
}
