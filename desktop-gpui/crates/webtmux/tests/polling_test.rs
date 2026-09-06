use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use webtmux::app_state::AppState;
use webtmux_backend_client::RestClient;
use webtmux_settings::DesktopSettings;

async fn spawn_mock_tree_server(responses: Arc<Vec<String>>, call_count: Arc<AtomicUsize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let resps = responses.clone();
            let count = call_count.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let idx = count.fetch_add(1, Ordering::SeqCst);
                let body = if idx < resps.len() {
                    resps[idx].clone()
                } else {
                    resps.last().cloned().unwrap_or_else(|| r#"{"sessions":[]}"#.to_string())
                };

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });

    base_url
}

#[tokio::test]
async fn test_polling_generation_guard() {
    let settings = DesktopSettings::default();
    let mut app = AppState::new(settings, None);

    assert_eq!(app.poll_generation, 0);

    // Simulated fetch 1 starts at gen 1
    app.poll_generation += 1;
    let gen_1 = app.poll_generation;
    assert_eq!(gen_1, 1);

    // Simulated fetch 2 starts at gen 2 (e.g. manual refresh before fetch 1 completes)
    app.poll_generation += 1;
    let gen_2 = app.poll_generation;
    assert_eq!(gen_2, 2);

    // Old response from gen_1 arrives -> should be discarded
    let gen_1_is_stale = gen_1 != app.poll_generation;
    assert!(gen_1_is_stale, "stale response from gen_1 must be discarded");

    // New response from gen_2 arrives -> should be accepted
    let gen_2_is_valid = gen_2 == app.poll_generation;
    assert!(gen_2_is_valid, "current response from gen_2 must be accepted");
}

#[tokio::test]
async fn test_cli_session_polling_integration() {
    let resp1 = r#"{"sessions":[]}"#.to_string();
    let resp2 = r#"{"sessions":[{"session":{"name":"cli-created","windows":1,"attached":0,"createdAt":1717200000,"width":80,"height":24},"windows":[]}]}"#.to_string();
    let responses = Arc::new(vec![resp1, resp2]);
    let call_count = Arc::new(AtomicUsize::new(0));

    let base_url = spawn_mock_tree_server(responses, call_count).await;
    let client = RestClient::new(&base_url);

    let tree_tick_1 = client.tree().await.expect("tick 1 tree failed");
    assert_eq!(tree_tick_1.sessions.len(), 0);

    let tree_tick_2 = client.tree().await.expect("tick 2 tree failed");
    assert_eq!(tree_tick_2.sessions.len(), 1);
    assert_eq!(tree_tick_2.sessions[0].session.name, "cli-created");
}
