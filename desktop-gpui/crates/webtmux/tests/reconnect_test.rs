//! Phase 7 plan 07-01 Task 1: transport-lost vs tmux-event mapping + ladder.
//!
//! Split-taksonomy contracts (D2/D3): the pump-local `transport.lost` signal
//! (a string the server can never send) maps to `Disconnected + Local` WITH
//! an armed BACKOFF retry, while server-sent `tmux.disconnected` maps to
//! `Disconnected + Server` WITHOUT a client retry and `tmux.reconnecting`
//! maps to the transient `Reconnecting` state (monitor owns that retry).

use webtmux::app_state::{
    banner_copy_for, backoff_delay, backoff_delay_ms, should_arm_retry, should_run_retry,
    AppState, TransportOrigin, RECONNECT_BACKOFF_MS,
};
use webtmux_backend_client::{
    TransportState, WsOutgoing, EV_TMUX_DISCONNECTED, EV_TMUX_RECONNECTING, EV_TRANSPORT_LOST,
};
use webtmux_settings::DesktopSettings;

fn event_msg(msg_type: &str, session: Option<&str>) -> WsOutgoing {
    WsOutgoing {
        msg_type: msg_type.to_string(),
        session: session.map(|s| s.to_string()),
        ..Default::default()
    }
}

fn dev_app_at_gen(gen: u64) -> AppState {
    let mut app = AppState::new(DesktopSettings::default(), None);
    app.open_session("dev");
    app.sessions.get_mut("dev").unwrap().generation = gen;
    app
}

#[test]
fn test_transport_lost_distinct_from_tmux_disconnected() {
    // Pitfall 1 gate (headless half): the pump synthesis constant differs
    // from the server `tmux.disconnected` constant the backend relays
    // (`be/internal/realtime/hub.go:115-118`). The grep half (zero
    // EV_TMUX_DISCONNECTED in the pump close/error arms) is a review gate.
    assert_ne!(
        EV_TRANSPORT_LOST, EV_TMUX_DISCONNECTED,
        "pump-local signal must differ from the server event"
    );
    assert_ne!(
        EV_TRANSPORT_LOST, EV_TMUX_RECONNECTING,
        "pump-local signal must differ from tmux.reconnecting"
    );

    let mut app = dev_app_at_gen(2);

    // Pump close-synthesis → Disconnected + Local origin + first attempt.
    assert!(app.apply_event("dev", 2, &event_msg(EV_TRANSPORT_LOST, Some("dev"))));
    let entry = app.sessions.get("dev").unwrap();
    assert_eq!(entry.transport, TransportState::Disconnected);
    assert_eq!(entry.transport_origin, TransportOrigin::Local);
    assert_eq!(entry.reconnect_attempt, 1);
    assert!(
        should_arm_retry(EV_TRANSPORT_LOST),
        "transport-lost is the SOLE retry-arming signal"
    );

    // Server-sent tmux loss → same Disconnected pixels, distinct origin, NO
    // client retry (the backend monitor owns that ladder).
    assert!(app.apply_event("dev", 2, &event_msg(EV_TMUX_DISCONNECTED, Some("dev"))));
    let entry = app.sessions.get("dev").unwrap();
    assert_eq!(entry.transport, TransportState::Disconnected);
    assert_eq!(entry.transport_origin, TransportOrigin::Server);
    assert_eq!(
        entry.reconnect_attempt, 0,
        "server-sent tmux loss must not carry a client attempt"
    );
    assert!(
        !should_arm_retry(EV_TMUX_DISCONNECTED),
        "client must never retry server-sent tmux.disconnected"
    );

    // Banner copy keys off the origin bit: the two Disconnected states
    // render distinct fixed strings (D2 copy table).
    let (lost_title, lost_button) = banner_copy_for(
        TransportState::Disconnected,
        TransportOrigin::Local,
        1,
    )
    .expect("transport-lost must render a banner");
    assert_eq!(lost_title, "Connection lost — retrying… (attempt 1)");
    assert!(lost_button, "WS-drop banner offers manual Reconnect");

    let (tmux_title, tmux_button) = banner_copy_for(
        TransportState::Disconnected,
        TransportOrigin::Server,
        0,
    )
    .expect("tmux.disconnected must render a banner");
    assert_eq!(tmux_title, "tmux disconnected — waiting for tmux");
    assert!(tmux_button, "tmux banner offers manual Reconnect");
    assert_ne!(lost_title, tmux_title, "origins must stay distinguishable");
}

#[test]
fn test_tmux_reconnecting_maps_transient() {
    let mut app = dev_app_at_gen(2);

    assert!(app.apply_event("dev", 2, &event_msg(EV_TMUX_RECONNECTING, Some("dev"))));
    let entry = app.sessions.get("dev").unwrap();
    assert_eq!(entry.transport, TransportState::Reconnecting);
    assert_eq!(entry.transport_origin, TransportOrigin::Server);
    assert!(
        !should_arm_retry(EV_TMUX_RECONNECTING),
        "tmux.reconnecting must not arm a client retry (monitor owns it)"
    );

    let (title, show_button) = banner_copy_for(
        TransportState::Reconnecting,
        TransportOrigin::Server,
        0,
    )
    .expect("tmux.reconnecting must render the transient banner");
    assert_eq!(title, "Reconnecting to tmux…");
    assert!(!show_button, "transient banner has no Reconnect button");
}

#[test]
fn test_backoff_ladder_verbatim() {
    // FE `BACKOFF = [250,500,1000,2000,5000,10000]` verbatim
    // (`fe/src/lib/websocket.ts:27`); backend monitor parity
    // (`be/internal/tmux/monitor.go:332-333`).
    assert_eq!(RECONNECT_BACKOFF_MS, [250, 500, 1000, 2000, 5000, 10_000]);
    assert_eq!(backoff_delay_ms(1), 250);
    assert_eq!(backoff_delay_ms(2), 500);
    assert_eq!(backoff_delay_ms(3), 1000);
    assert_eq!(backoff_delay_ms(4), 2000);
    assert_eq!(backoff_delay_ms(5), 5000);
    assert_eq!(backoff_delay_ms(6), 10_000);
    // Tails at 10s forever like FE (BACKOFF[5]).
    assert_eq!(backoff_delay_ms(7), 10_000);
    assert_eq!(backoff_delay_ms(100), 10_000);
    assert_eq!(backoff_delay(1), std::time::Duration::from_millis(250));
    assert_eq!(backoff_delay(9), std::time::Duration::from_millis(10_000));

    // Generation capture/drop predicate (T-07-03): rename/close/re-resolve
    // bumps the generation, retiring stale timers.
    assert!(should_run_retry(3, 3));
    assert!(!should_run_retry(3, 4));
}

