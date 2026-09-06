//! Phase 3 tracer: WS envelope DTOs, snapshot normalization, kill
//! serialization, and URL/request-id helpers (Task 1) plus mock-server
//! interop: bootstrap flow, correlated commands, malformed frames (Task 2).

use webtmux_backend_client::{
    normalize_ws_url, request_id, SessionSnapshot, WsIncoming, WsOutgoing,
};

#[test]
fn test_ws_envelope_roundtrip() {
    // session.rename serializes with requestId/newName and no session field.
    let rename = WsIncoming {
        msg_type: "session.rename".to_string(),
        request_id: Some("abc123".to_string()),
        new_name: Some("newname".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_string(&rename).unwrap();
    assert!(json.contains(r#""type":"session.rename""#));
    assert!(json.contains(r#""requestId":"abc123""#));
    assert!(json.contains(r#""newName":"newname""#));
    assert!(
        !json.contains(r#""session""#),
        "rename must not carry a session field, got: {}",
        json
    );

    // session.kill carries an explicit session field (cross-session kills, D6).
    let kill = WsIncoming {
        msg_type: "session.kill".to_string(),
        request_id: Some("k1".to_string()),
        session: Some("victim".to_string()),
        ..Default::default()
    };
    let kill_json = serde_json::to_string(&kill).unwrap();
    assert!(kill_json.contains(r#""session":"victim""#));

    // WsOutgoing parses connection.ready.
    let ready: WsOutgoing =
        serde_json::from_str(r#"{"type":"connection.ready","session":"dev"}"#).unwrap();
    assert_eq!(ready.msg_type, "connection.ready");
    assert_eq!(ready.session.as_deref(), Some("dev"));

    // WsOutgoing parses state.snapshot with session tag + snapshot payload.
    let snap_json = include_str!("fixtures/ws_snapshot_full.json");
    let snap: WsOutgoing = serde_json::from_str(snap_json).unwrap();
    assert_eq!(snap.msg_type, "state.snapshot");
    assert_eq!(snap.session.as_deref(), Some("dev"));
    let snapshot = snap.snapshot.expect("snapshot payload missing");
    assert_eq!(snapshot.session.name, "dev");
    assert_eq!(snapshot.windows.len(), 2);
    assert_eq!(snapshot.panes.len(), 2);

    // WsOutgoing parses state.delta with session tag.
    let delta_json = include_str!("fixtures/ws_delta.json");
    let delta: WsOutgoing = serde_json::from_str(delta_json).unwrap();
    assert_eq!(delta.msg_type, "state.delta");
    assert_eq!(delta.session.as_deref(), Some("dev"));
    assert_eq!(delta.seq, 7);

    // command.success / command.error echo requestId.
    let ok: WsOutgoing =
        serde_json::from_str(r#"{"type":"command.success","requestId":"abc123"}"#).unwrap();
    assert_eq!(ok.msg_type, "command.success");
    assert_eq!(ok.request_id.as_deref(), Some("abc123"));
    let err: WsOutgoing = serde_json::from_str(
        r#"{"type":"command.error","requestId":"abc123","message":"no such session"}"#,
    )
    .unwrap();
    assert_eq!(err.msg_type, "command.error");
    assert_eq!(err.request_id.as_deref(), Some("abc123"));
    assert_eq!(err.message.as_deref(), Some("no such session"));
}

#[test]
fn test_snapshot_null_normalization() {
    // Full payload parses windows/panes/activeWindow/activePane.
    let full_json = include_str!("fixtures/ws_snapshot_full.json");
    let full: WsOutgoing = serde_json::from_str(full_json).unwrap();
    let snap = full.snapshot.unwrap();
    assert_eq!(snap.session.name, "dev");
    assert_eq!(snap.windows.len(), 2);
    assert_eq!(snap.panes.len(), 2);
    assert_eq!(snap.active_window, "@0");
    assert_eq!(snap.active_pane, "%0");
    assert!(!snap.replace);
    assert_eq!(snap.seq, 0);

    // windows:null / panes:null normalize to empty vecs; missing replace/seq default.
    let null_json = include_str!("fixtures/ws_snapshot_null_slices.json");
    let null_msg: WsOutgoing = serde_json::from_str(null_json).unwrap();
    let null_snap = null_msg.snapshot.unwrap();
    assert!(
        null_snap.windows.is_empty(),
        "windows:null must normalize to empty vec"
    );
    assert!(
        null_snap.panes.is_empty(),
        "panes:null must normalize to empty vec"
    );
    assert!(!null_snap.replace);
    assert_eq!(null_snap.seq, 0);

    // Direct SessionSnapshot parse also normalizes.
    let direct: SessionSnapshot = serde_json::from_str(
        r#"{"session":{"name":"x","windows":0,"attached":0,"createdAt":0,"width":80,"height":24},"windows":null,"panes":null}"#,
    )
    .unwrap();
    assert!(direct.windows.is_empty());
    assert!(direct.panes.is_empty());
}

#[test]
fn test_kill_by_name_serialization() {
    let kill = WsIncoming {
        msg_type: "session.kill".to_string(),
        request_id: Some("kill-9".to_string()),
        session: Some("victim".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_string(&kill).unwrap();
    assert!(json.contains(r#""type":"session.kill""#));
    assert!(json.contains(r#""session":"victim""#));
    assert!(json.contains(r#""requestId":"kill-9""#));
    assert!(!kill.session.as_deref().unwrap_or_default().is_empty());
}

#[test]
fn test_normalize_ws_url() {
    assert_eq!(
        normalize_ws_url("http://127.0.0.1:54321", "dev").unwrap(),
        "ws://127.0.0.1:54321/api/ws?session=dev"
    );
    // Trailing slash on base is trimmed.
    assert_eq!(
        normalize_ws_url("http://127.0.0.1:54321/", "dev").unwrap(),
        "ws://127.0.0.1:54321/api/ws?session=dev"
    );
    // https becomes wss.
    assert_eq!(
        normalize_ws_url("https://example.com:8443", "dev").unwrap(),
        "wss://example.com:8443/api/ws?session=dev"
    );
    // Names with slashes are query-encoded, not path-encoded.
    let encoded = normalize_ws_url("http://127.0.0.1:1", "a/b").unwrap();
    assert_eq!(encoded, "ws://127.0.0.1:1/api/ws?session=a%2Fb");
    // Spaces encode as %20 (query-value encoding, not form +).
    let spaced = normalize_ws_url("http://127.0.0.1:1", "my sess").unwrap();
    assert_eq!(spaced, "ws://127.0.0.1:1/api/ws?session=my%20sess");
}

#[test]
fn test_request_id_unique() {
    let a = request_id();
    let b = request_id();
    assert!(!a.is_empty());
    assert!(!b.is_empty());
    assert_ne!(a, b, "request ids must be unique per call");
}
