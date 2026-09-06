//! Phase 3 tracer: WS envelope DTOs, snapshot normalization, kill
//! serialization, and URL/request-id helpers (Task 1) plus mock-server
//! interop: bootstrap flow, correlated commands, malformed frames (Task 2).

use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use webtmux_backend_client::{
    connect_session, normalize_ws_url, request_id, SessionSnapshot, WsIncoming, WsOutgoing,
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

/// Read one text frame from the mock side with a timeout.
async fn mock_recv_text(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    >,
) -> Option<String> {
    let msg = tokio::time::timeout(std::time::Duration::from_secs(5), read.next())
        .await
        .ok()??;
    match msg {
        Ok(Message::Text(t)) => Some(t.to_string()),
        _ => None,
    }
}

#[tokio::test]
async fn test_ws_bootstrap_flow() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);
    let seen_path: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let seen_path_srv = seen_path.clone();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let seen = seen_path_srv.clone();
        let ws = tokio_tungstenite::accept_hdr_async(
            stream,
            move |req: &tokio_tungstenite::tungstenite::handshake::server::Request, resp| {
                *seen.lock().unwrap() = req.uri().path_and_query().map(|p| p.to_string());
                Ok(resp)
            },
        )
        .await
        .unwrap();
        let (mut write, mut read) = ws.split();

        // Unsolicited bootstrap: ready + snapshot with no client message first.
        write
            .send(Message::Text(
                r#"{"type":"connection.ready","session":"dev"}"#.into(),
            ))
            .await
            .unwrap();
        let snap = include_str!("fixtures/ws_snapshot_full.json");
        write.send(Message::Text(snap.into())).await.unwrap();

        // Expect state.resync within 2s.
        let mut saw_resync = false;
        let mut saw_hello = false;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            let remain = deadline - std::time::Instant::now();
            match tokio::time::timeout(remain, read.next()).await {
                Ok(Some(Ok(Message::Text(t)))) => {
                    let v: serde_json::Value = serde_json::from_str(&t).unwrap();
                    match v.get("type").and_then(|x| x.as_str()) {
                        Some("state.resync") => {
                            saw_resync = true;
                            break;
                        }
                        Some("hello") => saw_hello = true,
                        _ => {}
                    }
                }
                _ => break,
            }
        }
        // Bootstrap window: keep watching briefly for a stray hello.
        if saw_resync {
            while let Ok(Some(Ok(Message::Text(t)))) = tokio::time::timeout(
                std::time::Duration::from_millis(400),
                read.next(),
            )
            .await
            {
                let v: serde_json::Value = serde_json::from_str(&t).unwrap();
                if v.get("type").and_then(|x| x.as_str()) == Some("hello") {
                    saw_hello = true;
                }
            }
        }
        (saw_resync, saw_hello)
    });

    let handle = connect_session(&base_url, "dev", 1)
        .await
        .expect("connect failed");

    // Ready + snapshot surface as tagged events for generation 1.
    let ready = tokio::time::timeout(std::time::Duration::from_secs(5), handle.recv_event())
        .await
        .expect("no ready event")
        .expect("event channel closed");
    assert_eq!(ready.session, "dev");
    assert_eq!(ready.generation, 1);
    assert_eq!(ready.msg.msg_type, "connection.ready");

    let snap_ev = tokio::time::timeout(std::time::Duration::from_secs(5), handle.recv_event())
        .await
        .expect("no snapshot event")
        .expect("event channel closed");
    assert_eq!(snap_ev.msg.msg_type, "state.snapshot");
    assert_eq!(
        snap_ev.msg.snapshot.as_ref().unwrap().session.name,
        "dev"
    );

    handle.disconnect();
    let (saw_resync, saw_hello) =
        tokio::time::timeout(std::time::Duration::from_secs(10), server)
            .await
            .expect("server hung")
            .unwrap();
    assert!(saw_resync, "client must answer ready with state.resync");
    assert!(!saw_hello, "client must never send hello (D7)");
    assert_eq!(
        seen_path.lock().unwrap().as_deref(),
        Some("/api/ws?session=dev")
    );
}

#[tokio::test]
async fn test_ws_correlated_command() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws.split();
        write
            .send(Message::Text(
                r#"{"type":"connection.ready","session":"dev"}"#.into(),
            ))
            .await
            .unwrap();

        // First command: reply command.success echoing the requestId.
        let mut first_id = String::new();
        while let Some(t) = mock_recv_text(&mut read).await {
            let v: serde_json::Value = serde_json::from_str(&t).unwrap();
            if v.get("type").and_then(|x| x.as_str()) == Some("session.rename") {
                first_id = v
                    .get("requestId")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                assert_eq!(v.get("newName").and_then(|x| x.as_str()), Some("newname"));
                break;
            }
        }
        assert!(!first_id.is_empty(), "server never saw session.rename");
        write
            .send(Message::Text(
                format!(r#"{{"type":"command.success","requestId":"{}"}}"#, first_id).into(),
            ))
            .await
            .unwrap();

        // Second command's receiver is dropped (forget-timeout); late reply ignored.
        while let Some(t) = mock_recv_text(&mut read).await {
            let v: serde_json::Value = serde_json::from_str(&t).unwrap();
            if v.get("type").and_then(|x| x.as_str()) == Some("session.rename") {
                let late_id = v
                    .get("requestId")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                write
                    .send(Message::Text(
                        format!(r#"{{"type":"command.success","requestId":"{}"}}"#, late_id)
                            .into(),
                    ))
                    .await
                    .unwrap();
                break;
            }
        }
        // Prove the pump survived: a snapshot right after is still delivered.
        let snap = include_str!("fixtures/ws_snapshot_full.json");
        write.send(Message::Text(snap.into())).await.unwrap();
    });

    let handle = connect_session(&base_url, "dev", 1)
        .await
        .expect("connect failed");
    // Drain the unsolicited ready.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle.recv_event())
        .await
        .unwrap();

    let rename = WsIncoming {
        msg_type: "session.rename".to_string(),
        request_id: None, // handle assigns one
        new_name: Some("newname".to_string()),
        ..Default::default()
    };
    let rx = handle.send_command(rename).expect("send_command failed");
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), rx)
        .await
        .expect("correlated command timed out")
        .expect("sender dropped");
    assert!(result.ok);
    assert!(!result.request_id.is_empty());

    // Forget-timeout path: drop the waiter, reply still handled without panic.
    let rename2 = WsIncoming {
        msg_type: "session.rename".to_string(),
        request_id: None,
        new_name: Some("later".to_string()),
        ..Default::default()
    };
    let rx2 = handle.send_command(rename2).expect("send_command failed");
    drop(rx2);
    // Next valid event still arrives: pump did not die on the orphan reply.
    loop {
        let ev = tokio::time::timeout(std::time::Duration::from_secs(10), handle.recv_event())
            .await
            .expect("pump died after orphan reply")
            .expect("event channel closed");
        if ev.msg.msg_type == "state.snapshot" {
            break;
        }
    }
    handle.disconnect();
    tokio::time::timeout(std::time::Duration::from_secs(10), server)
        .await
        .expect("server hung")
        .unwrap();
}

#[tokio::test]
async fn test_ws_malformed_frame_dropped() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, _read) = ws.split();
        write
            .send(Message::Text(
                r#"{"type":"connection.ready","session":"dev"}"#.into(),
            ))
            .await
            .unwrap();
        write
            .send(Message::Text("THIS IS NOT JSON {{{".into()))
            .await
            .unwrap();
        let snap = include_str!("fixtures/ws_snapshot_full.json");
        write.send(Message::Text(snap.into())).await.unwrap();
    });

    let handle = connect_session(&base_url, "dev", 1)
        .await
        .expect("connect failed");
    // Ready arrives, malformed frame is dropped, snapshot still delivered.
    let mut saw_snapshot = false;
    for _ in 0..3 {
        let ev = tokio::time::timeout(std::time::Duration::from_secs(5), handle.recv_event())
            .await
            .expect("pump panicked or stalled on malformed frame")
            .expect("event channel closed");
        if ev.msg.msg_type == "state.snapshot" {
            saw_snapshot = true;
            break;
        }
    }
    assert!(saw_snapshot, "valid event after malformed frame was lost");
    handle.disconnect();
}
