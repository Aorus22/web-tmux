//! Phase 3 tracer: triple generation-guard apply-function tests
//! (stale / current / unknown-session / envelope-mismatch drops).

use webtmux::app_state::AppState;
use webtmux_backend_client::{SessionSnapshot, TmuxSession, WsOutgoing};
use webtmux_settings::DesktopSettings;

fn empty_snapshot(name: &str) -> SessionSnapshot {
    SessionSnapshot {
        session: TmuxSession {
            name: name.to_string(),
            windows: 0,
            attached: 0,
            created_at: 0,
            width: 80,
            height: 24,
        },
        windows: Vec::new(),
        panes: Vec::new(),
        active_window: String::new(),
        active_pane: String::new(),
        replace: false,
        seq: 0,
    }
}

fn snapshot_msg(envelope_session: Option<&str>, snap: SessionSnapshot) -> WsOutgoing {
    WsOutgoing {
        msg_type: "state.snapshot".to_string(),
        session: envelope_session.map(|s| s.to_string()),
        snapshot: Some(snap),
        ..Default::default()
    }
}

#[test]
fn test_ws_generation_guard() {
    let mut app = AppState::new(DesktopSettings::default(), None);
    app.open_session("dev");
    // Simulate two connects: current generation is 2.
    app.sessions.get_mut("dev").unwrap().generation = 2;

    // Stale (session, old-generation) events are dropped.
    assert!(
        !app.apply_event("dev", 1, &snapshot_msg(Some("dev"), empty_snapshot("dev"))),
        "stale generation must drop"
    );
    assert!(
        app.sessions.get("dev").unwrap().snapshot.is_none(),
        "stale event must not commit"
    );

    // Future generations are dropped too.
    assert!(!app.apply_event("dev", 3, &snapshot_msg(Some("dev"), empty_snapshot("dev"))));

    // Current generation applies.
    assert!(app.apply_event("dev", 2, &snapshot_msg(Some("dev"), empty_snapshot("dev"))));
    assert_eq!(
        app.sessions.get("dev").unwrap().snapshot.as_ref().unwrap().session.name,
        "dev"
    );

    // Unknown session: dropped (tab-liveness).
    assert!(!app.apply_event("ghost", 2, &snapshot_msg(Some("ghost"), empty_snapshot("ghost"))));

    // Mismatched envelope session: dropped (cross-session leak).
    assert!(
        !app.apply_event("dev", 2, &snapshot_msg(Some("other"), empty_snapshot("other"))),
        "envelope session mismatch must drop"
    );
    assert_eq!(
        app.sessions.get("dev").unwrap().snapshot.as_ref().unwrap().session.name,
        "dev",
        "mismatched event must not overwrite the committed snapshot"
    );

    // Absent envelope session treated as belonging (FE parity): applies.
    assert!(app.apply_event("dev", 2, &snapshot_msg(None, empty_snapshot("dev"))));

    // Closing the tab drops later events for it (tab-liveness).
    app.close_session("dev");
    assert!(!app.apply_event("dev", 2, &snapshot_msg(Some("dev"), empty_snapshot("dev"))));
}

#[test]
fn test_ws_terminal_frames_ignored() {
    let mut app = AppState::new(DesktopSettings::default(), None);
    app.open_session("dev");
    let gen = app.sessions.get("dev").unwrap().generation;

    // terminal.snapshot / terminal.output parse into the DTO and are accepted
    // but committed nowhere (Phase 4 owns them).
    for msg_type in ["terminal.snapshot", "terminal.output"] {
        let frame: WsOutgoing = serde_json::from_str(&format!(
            r#"{{"type":"{}","session":"dev","paneId":"%0","data":"hello"}}"#,
            msg_type
        ))
        .expect("terminal frame must parse without panic");
        assert!(app.apply_event("dev", gen, &frame), "{} must not drop", msg_type);
    }
    assert!(
        app.sessions.get("dev").unwrap().snapshot.is_none(),
        "terminal frames must commit no state"
    );
}
