use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use webtmux_backend_client::{
    CreateSessionRequest, RestClient, RestError, TmuxTree,
};

async fn spawn_mock_server(status_line: &'static str, body: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let body_clone = body.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status_line,
                    body_clone.len(),
                    body_clone
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });

    base_url
}

#[test]
fn test_parse_sessions_tree() {
    let full_json = include_str!("fixtures/tree_full.json");
    let tree: TmuxTree = serde_json::from_str(full_json).expect("failed to parse tree_full.json");
    assert_eq!(tree.sessions.len(), 1);
    let s0 = &tree.sessions[0];
    assert_eq!(s0.session.name, "dev");
    assert_eq!(s0.session.windows, 2);
    assert_eq!(s0.session.attached, 1);
    assert_eq!(s0.windows.len(), 2);

    let w0 = &s0.windows[0];
    assert_eq!(w0.window.id, "@0");
    assert_eq!(w0.window.name, "editor");
    assert_eq!(w0.panes.len(), 2);
    assert_eq!(w0.panes[0].id, "%0");
    assert_eq!(w0.panes[0].current_command, "nvim");

    let empty_json = include_str!("fixtures/tree_empty.json");
    let tree_empty: TmuxTree = serde_json::from_str(empty_json).expect("failed to parse tree_empty.json");
    assert!(tree_empty.sessions.is_empty());

    let null_slices_json = include_str!("fixtures/tree_null_slices.json");
    let tree_null: TmuxTree = serde_json::from_str(null_slices_json).expect("failed to parse tree_null_slices.json");
    assert!(tree_null.sessions.is_empty());
}

#[tokio::test]
async fn test_create_session_request() {
    // 1. Test serialization of request
    let req_minimal = CreateSessionRequest {
        name: "dev".to_string(),
        cwd: None,
        initial_command: None,
    };
    let json_min = serde_json::to_string(&req_minimal).unwrap();
    assert_eq!(json_min, r#"{"name":"dev"}"#);

    let req_full = CreateSessionRequest {
        name: "dev".to_string(),
        cwd: Some("C:\\projects".to_string()),
        initial_command: Some("nvim".to_string()),
    };
    let json_full = serde_json::to_string(&req_full).unwrap();
    assert!(json_full.contains(r#""name":"dev""#));
    assert!(json_full.contains(r#""cwd":"C:\\projects""#));
    assert!(json_full.contains(r#""initialCommand":"nvim""#));

    // 2. Test successful 201 response via mock server
    let base_url = spawn_mock_server("201 Created", r#"{"name":"dev"}"#.to_string()).await;
    let client = RestClient::new(&base_url);
    let resp = client.create_session(&req_minimal).await.expect("create_session failed");
    assert_eq!(resp.name, "dev");

    // 3. Test 409 conflict duplicate response via mock server
    let dup_body = include_str!("fixtures/create_error_duplicate.json");
    let dup_base_url = spawn_mock_server("409 Conflict", dup_body.to_string()).await;
    let dup_client = RestClient::new(&dup_base_url);
    let err = dup_client.create_session(&req_minimal).await.unwrap_err();
    match err {
        RestError::Api { status, message } => {
            assert_eq!(status, 409);
            assert_eq!(message, "duplicate session: dev");
        }
        other => panic!("expected RestError::Api, got {:?}", other),
    }
}

#[tokio::test]
async fn test_rest_client_health_and_tree() {
    let health_body = include_str!("fixtures/health.json");
    let base_url = spawn_mock_server("200 OK", health_body.to_string()).await;
    let client = RestClient::new(&base_url);

    let health = client.health().await.expect("health failed");
    assert_eq!(health.status, "ok");
    assert!(health.tmux.installed);
    assert_eq!(health.tmux.version, "tmux 3.4");

    let tree_body = include_str!("fixtures/tree_full.json");
    let tree_base_url = spawn_mock_server("200 OK", tree_body.to_string()).await;
    let tree_client = RestClient::new(&tree_base_url);

    let tree = tree_client.tree().await.expect("tree failed");
    assert_eq!(tree.sessions.len(), 1);
    assert_eq!(tree.sessions[0].session.name, "dev");
}
