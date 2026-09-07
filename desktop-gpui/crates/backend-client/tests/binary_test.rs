//! SET-03 tmux-binary validation contracts (Phase 6 plan 06-02 Task 1, D6).
//!
//! - `test_set_tmux_binary_shape`: the client POSTs exactly `{path}` to
//!   `/api/tmux/binary`, parses a 200 `{binary, version, ok}` into `TmuxInfo`,
//!   and surfaces a 400 `{"error": …}` via `extract_error_message` (never a
//!   generic status code). Mock `TcpListener` stub — no live backend needed.
//! - `test_binary_status_copy`: the pure settings status-line helper renders
//!   `Using {binary} ({version})` on ok and the raw backend error verbatim on
//!   failure (FE `TerminalSettings.tsx:20` parity).

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use webtmux_backend_client::{binary_status_copy, RestClient, RestError, TmuxInfo};

/// Mock that serves one canned response and reports the raw request bytes so
/// the test can assert the exact POST shape (method + path + JSON body).
async fn spawn_capturing_mock(
    status_line: &'static str,
    body: String,
) -> (String, oneshot::Receiver<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = vec![0u8; 8192];
            let n = socket.read(&mut buf).await.unwrap_or(0);
            buf.truncate(n);
            let _ = tx.send(buf);
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status_line,
                body.len(),
                body
            );
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.flush().await;
        }
    });
    (format!("http://{}", addr), rx)
}

#[tokio::test]
async fn test_set_tmux_binary_shape() {
    // 1. 200 ok parses into TmuxInfo.
    let ok_body = r#"{"binary":"C:\\tmux\\tmux.exe","version":"tmux 3.4","ok":true}"#;
    let (base_url, req_rx) =
        spawn_capturing_mock("200 OK", ok_body.to_string()).await;
    let client = RestClient::new(&base_url);
    let info = client
        .set_tmux_binary("C:\\tmux\\tmux.exe")
        .await
        .expect("set_tmux_binary failed");
    assert_eq!(info.binary, "C:\\tmux\\tmux.exe");
    assert_eq!(info.version, "tmux 3.4");
    assert!(info.ok);

    // 2. The request is exactly POST /api/tmux/binary with a `{path}` body.
    let raw = req_rx.await.expect("mock never received the request");
    let text = String::from_utf8_lossy(&raw);
    let request_line = text.lines().next().unwrap_or("");
    assert!(
        request_line.starts_with("POST /api/tmux/binary "),
        "must POST the binary endpoint, got request line: {request_line}"
    );
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("");
    let value: serde_json::Value =
        serde_json::from_str(body).expect("request body must be JSON");
    assert_eq!(
        value,
        serde_json::json!({ "path": "C:\\tmux\\tmux.exe" }),
        "POST body must be exactly {{path}}"
    );

    // 3. A 400 `{"error": …}` surfaces the backend string verbatim (never a
    // generic `400 Bad Request` code).
    let err_body = r#"{"error":"tmux binary \"bogus\" is not usable: exit status 1"}"#;
    let (err_base, _) = spawn_capturing_mock("400 Bad Request", err_body.to_string()).await;
    let err = RestClient::new(&err_base)
        .set_tmux_binary("bogus")
        .await
        .unwrap_err();
    match err {
        RestError::Api { status, message, .. } => {
            assert_eq!(status, 400);
            assert_eq!(
                message, "tmux binary \"bogus\" is not usable: exit status 1"
            );
        }
        other => panic!("expected RestError::Api, got {:?}", other),
    }
}

#[test]
fn test_binary_status_copy() {
    // Ok renders the FE-verbatim `Using {binary} ({version})` line.
    let info = TmuxInfo {
        version: "tmux 3.4".to_string(),
        ok: true,
        binary: "C:\\tmux\\tmux.exe".to_string(),
    };
    assert_eq!(
        binary_status_copy(&Ok(info)),
        "Using C:\\tmux\\tmux.exe (tmux 3.4)"
    );

    // Failure renders the raw backend error string (never a generic code).
    let err = "tmux binary \"bogus\" is not usable: exit status 1".to_string();
    assert_eq!(binary_status_copy(&Err(err.clone())), err);
}
