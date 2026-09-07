//! Phase 7 plan 07-01 Task 2: bounded toast overlay queue + taxonomy copy.
//!
//! Flap-safety contracts (D1/D2, T-07-02): hand-rolled `VecDeque` (cap 5,
//! drop-oldest, dedupe key per kind+session, autohide 5s errors / 3s
//! success+info). Every push path funnels through `push_toast` so no caller
//! can grow the queue unbounded; reconnect flaps (250ms ladder) collapse
//! instead of covering the workspace.

use std::time::{Duration, Instant};

use webtmux::app_state::{
    banner_copy_for, AppState, ToastKind, BANNER_TMUX_DISCONNECTED,
    BANNER_TMUX_RECONNECTING, TOAST_AUTOHIDE_ERROR_MS, TOAST_AUTOHIDE_SUCCESS_MS,
    TOAST_QUEUE_CAP, TOAST_RECONNECTED, TOAST_AUTOHIDE_INFO_MS,
};
use webtmux_backend_client::{TransportState, WsOutgoing, EV_SERVER_ERROR};
use webtmux::app_state::TransportOrigin;
use webtmux_settings::DesktopSettings;

fn dev_app() -> AppState {
    let mut app = AppState::new(DesktopSettings::default(), None);
    app.open_session("dev");
    app
}

#[test]
fn test_toast_queue_cap_and_dedupe() {
    let mut app = dev_app();

    // Push 7 distinct-session error toasts: newest 5 survive (drop-oldest).
    for i in 0..7 {
        app.push_toast(
            ToastKind::Error,
            &format!("s{i}"),
            &format!("boom {i}"),
        );
    }
    assert_eq!(
        app.visible_toasts().len(),
        TOAST_QUEUE_CAP,
        "queue must cap at 5"
    );
    assert_eq!(TOAST_QUEUE_CAP, 5, "cap is a fixed D1 contract");
    let sessions: Vec<String> = app
        .visible_toasts()
        .iter()
        .map(|t| t.session.clone())
        .collect();
    // Oldest two (s0, s1) evicted; newest five (s2..s6) retained in order.
    assert_eq!(sessions, vec!["s2", "s3", "s4", "s5", "s6"]);

    // Same (kind+session) key collapses instead of duplicating (Pitfall 3):
    // re-pushing the s6 error refreshes newest-wins without growing.
    app.push_toast(ToastKind::Error, "s6", "boom 6 updated");
    assert_eq!(app.visible_toasts().len(), 5);
    let s6 = app
        .visible_toasts()
        .iter()
        .find(|t| t.session == "s6")
        .unwrap()
        .clone();
    assert_eq!(s6.message, "boom 6 updated");
    // Newest-wins refresh moves it to the back.
    assert_eq!(app.visible_toasts().last().unwrap().message, "boom 6 updated");

    // Different kind, same session does NOT dedupe (key is kind+session).
    app.push_toast(ToastKind::Success, "s6", TOAST_RECONNECTED);
    assert_eq!(app.visible_toasts().len(), 5, "cap still holds");
    assert!(app
        .visible_toasts()
        .iter()
        .any(|t| t.kind == ToastKind::Success && t.session == "s6"));

    // Cap invariant holds for review tooling.
    assert!(app.visible_toasts().len() <= TOAST_QUEUE_CAP);
}

#[test]
fn test_toast_copy_keys() {
    // Each D2 taxonomy arm resolves its fixed copy string.
    let (lost, _) = banner_copy_for(
        TransportState::Disconnected,
        TransportOrigin::Local,
        2,
    )
    .expect("transport-lost must render");
    assert_eq!(lost, "Connection lost — retrying… (attempt 2)");

    let (disc, _) = banner_copy_for(
        TransportState::Disconnected,
        TransportOrigin::Server,
        0,
    )
    .expect("tmux.disconnected must render");
    assert_eq!(disc, BANNER_TMUX_DISCONNECTED);

    let (recon, _) = banner_copy_for(TransportState::Reconnecting, TransportOrigin::Server, 0)
        .expect("tmux.reconnecting must render");
    assert_eq!(recon, BANNER_TMUX_RECONNECTING);

    // Command-error toast carries the backend-verbatim message (D2d):
    // `note_session_error` preserves the exact string (no prefix/rewrite).
    let mut app = dev_app();
    let verbatim = "tmux: unknown command: frobnicate (exit 1)";
    app.note_session_error("dev", verbatim.to_string());
    let toasts = app.visible_toasts();
    assert_eq!(toasts.len(), 1);
    assert_eq!(toasts[0].kind, ToastKind::Error);
    assert_eq!(toasts[0].message, verbatim, "backend-verbatim, no rewrite");

    // Autohide lifetimes are the A1-tunable guesses (5s errors / 3s success+info).
    assert_eq!(TOAST_AUTOHIDE_ERROR_MS, 5_000);
    assert_eq!(TOAST_AUTOHIDE_SUCCESS_MS, 3_000);
    assert_eq!(TOAST_AUTOHIDE_INFO_MS, 3_000);
}

#[test]
fn test_command_error_surfaces_toast() {
    let mut app = dev_app();

    // Correlated `command.error` / 10s timeout path (`note_session_error`)
    // enqueues an error toast while keeping the inline `last_error` line
    // untouched (toasts are additive, never a replacement).
    app.note_session_error("dev", "pane.kill failed: no such pane".to_string());
    assert_eq!(
        app.sessions.get("dev").unwrap().last_error.as_deref(),
        Some("pane.kill failed: no such pane"),
        "inline last_error stays"
    );
    assert_eq!(app.visible_toasts().len(), 1);
    assert_eq!(app.visible_toasts()[0].kind, ToastKind::Error);

    // `server.error` (Open Q2) also surfaces as an error toast via apply_event.
    let gen = app.sessions.get("dev").unwrap().generation;
    let msg = WsOutgoing {
        msg_type: EV_SERVER_ERROR.to_string(),
        session: Some("dev".to_string()),
        message: Some("backend exploded".to_string()),
        ..Default::default()
    };
    assert!(app.apply_event("dev", gen, &msg));
    let errors: Vec<_> = app
        .visible_toasts()
        .iter()
        .filter(|t| t.kind == ToastKind::Error)
        .cloned()
        .collect();
    assert!(
        errors.iter().any(|t| t.message == "backend exploded"),
        "server.error must toast backend-verbatim"
    );

    // Recovery pushes the `Reconnected` success toast tested here at the
    // model level: a snapshot closing a failure episode toasts. Drive it
    // through the real path — transport-lost then a snapshot.
    let mut app2 = dev_app();
    let gen2 = app2.sessions.get("dev").unwrap().generation;
    app2.apply_event(
        "dev",
        gen2,
        &WsOutgoing {
            msg_type: webtmux_backend_client::EV_TRANSPORT_LOST.to_string(),
            session: Some("dev".to_string()),
            ..Default::default()
        },
    );
    assert!(app2
        .visible_toasts()
        .iter()
        .all(|t| t.message != TOAST_RECONNECTED));
    // A snapshot with empty panes closes the episode → Reconnected toast.
    let snap = webtmux_backend_client::SessionSnapshot {
        session: webtmux_backend_client::TmuxSession {
            name: "dev".to_string(),
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
    };
    app2.apply_event(
        "dev",
        gen2,
        &WsOutgoing {
            msg_type: webtmux_backend_client::EV_STATE_SNAPSHOT.to_string(),
            session: Some("dev".to_string()),
            snapshot: Some(snap),
            ..Default::default()
        },
    );
    assert!(
        app2
            .visible_toasts()
            .iter()
            .any(|t| t.kind == ToastKind::Success && t.message == TOAST_RECONNECTED),
        "recovery must fire the Reconnected toast"
    );

    // Autohide model: errors expire after 5s, success after 3s (no sleeping —
    // drive `expire_toasts` with a fake `now`).
    let now = Instant::now();
    let mut app3 = dev_app();
    app3.push_toast(ToastKind::Error, "dev", "old boom");
    app3.push_toast(ToastKind::Success, "dev", TOAST_RECONNECTED);
    // Backdate: error 6s old (expired), success 1s old (live).
    for t in app3.toasts.iter_mut() {
        if t.kind == ToastKind::Error {
            t.created = now - Duration::from_secs(6);
        } else {
            t.created = now - Duration::from_secs(1);
        }
    }
    app3.expire_toasts(now);
    assert_eq!(app3.visible_toasts().len(), 1);
    assert_eq!(app3.visible_toasts()[0].kind, ToastKind::Success);
}
