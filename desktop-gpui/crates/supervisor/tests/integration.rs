use std::path::PathBuf;
use std::time::Duration;
use webtmux_supervisor::{BackendStatus, SpawnOptions, Supervisor, parse_handshake_line};

fn resolve_backend_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TEST_BACKEND_PATH") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }
    let candidates = [
        PathBuf::from("../../tmux-gui-server.exe"),
        PathBuf::from("../../tmux-gui-server"),
        PathBuf::from("tmux-gui-server.exe"),
        PathBuf::from("tmux-gui-server"),
        PathBuf::from("../tmux-gui-server.exe"),
        PathBuf::from("../tmux-gui-server"),
        PathBuf::from("../../../tmux-gui-server.exe"),
        PathBuf::from("../../../tmux-gui-server"),
    ];
    for c in candidates {
        if c.exists() {
            return Some(c.canonicalize().unwrap_or(c));
        }
    }
    None
}

#[test]
fn test_parse_handshake_unit() {
    assert_eq!(parse_handshake_line("BACKEND_PORT:54321"), Some(54321));
    assert_eq!(parse_handshake_line("prefix BACKEND_PORT:8080 suffix"), Some(8080));
    assert_eq!(parse_handshake_line("BACKEND_PORT:0"), Some(0));
    assert_eq!(parse_handshake_line("BACKEND_PORT:notaport"), None);
    assert_eq!(parse_handshake_line("just a regular log line"), None);
}

#[test]
fn test_spawn_options_env_contract() {
    let opts = SpawnOptions::new("my-tmux-gui-server");
    let env_vars = opts.env_vars();

    assert!(env_vars.iter().any(|(k, v)| *k == "TMUXGUI_PORT" && v == "0"));
    assert!(env_vars.iter().any(|(k, v)| *k == "TMUXGUI_HOST" && v == "127.0.0.1"));
    assert!(env_vars.iter().any(|(k, _)| *k == "TMUXGUI_TMUX_BIN"));

    let cmd = opts.build_command();
    let std_cmd = cmd.as_std();
    assert_eq!(std_cmd.get_args().count(), 0, "argv must be empty");
}

#[tokio::test]
async fn reaches_ready() {
    let backend_path = match resolve_backend_path() {
        Some(p) => p,
        None => {
            eprintln!("Fixture missing: build via scripts/build-test-backend");
            return;
        }
    };

    let opts = SpawnOptions::new(backend_path);
    let mut supervisor = Supervisor::new();
    let info = supervisor.spawn(opts).await.expect("supervisor should reach ready");
    assert!(info.base_url.starts_with("http://127.0.0.1:"));
    match supervisor.status() {
        BackendStatus::Ready(ref ready_info) => {
            assert_eq!(ready_info.port, info.port);
        }
        other => panic!("expected BackendStatus::Ready, got {:?}", other),
    }

    supervisor.stop(Duration::from_secs(5)).await.unwrap();
}

#[tokio::test]
async fn invalid_path() {
    let opts = SpawnOptions::new("nonexistent_backend_path_xyz_123");
    let mut supervisor = Supervisor::new();
    let result = supervisor.spawn(opts).await;
    assert!(result.is_err());
    match supervisor.status() {
        BackendStatus::Failed { reason, stderr_tail } => {
            assert!(!reason.is_empty());
            assert!(!stderr_tail.is_empty());
        }
        other => panic!("expected BackendStatus::Failed, got {:?}", other),
    }
}

#[tokio::test]
async fn handshake_timeout() {
    // Spawn a non-responsive command (like 'more' or 'timeout' or powershell sleep)
    // with a tiny handshake timeout
    #[cfg(target_os = "windows")]
    let dummy = "cmd.exe";
    #[cfg(not(target_os = "windows"))]
    let dummy = "sleep";

    let mut opts = SpawnOptions::new(dummy);
    opts.handshake_timeout = Duration::from_millis(200);

    let mut supervisor = Supervisor::new();
    let result = supervisor.spawn(opts).await;
    assert!(result.is_err());
    match supervisor.status() {
        BackendStatus::Failed { reason, .. } => {
            assert!(reason.to_lowercase().contains("timed out") || reason.to_lowercase().contains("missing"));
        }
        other => panic!("expected BackendStatus::Failed, got {:?}", other),
    }
}

#[tokio::test]
async fn early_exit() {
    // Command that exits immediately with code 1
    #[cfg(target_os = "windows")]
    let cmd_bin = "cmd.exe";
    #[cfg(not(target_os = "windows"))]
    let cmd_bin = "false";

    let mut opts = SpawnOptions::new(cmd_bin);
    opts.handshake_timeout = Duration::from_secs(2);

    let mut supervisor = Supervisor::new();
    let result = supervisor.spawn(opts).await;
    assert!(result.is_err());
    match supervisor.status() {
        BackendStatus::Failed { reason, .. } => {
            assert!(!reason.is_empty());
        }
        other => panic!("expected BackendStatus::Failed, got {:?}", other),
    }
}

#[tokio::test]
async fn adopt_or_clear() {
    let backend_path = match resolve_backend_path() {
        Some(p) => p,
        None => {
            eprintln!("Fixture missing: build via scripts/build-test-backend");
            return;
        }
    };

    let opts = SpawnOptions::new(backend_path);
    let mut supervisor = Supervisor::new();
    let info = supervisor.spawn(opts).await.unwrap();

    let adopted = Supervisor::adopt_or_clear(Some(info.base_url.clone())).await;
    assert!(adopted.is_some(), "adopt_or_clear should return info for live backend");
    assert_eq!(adopted.unwrap().port, info.port);

    supervisor.stop(Duration::from_secs(2)).await.unwrap();

    let cleared = Supervisor::adopt_or_clear(Some(info.base_url)).await;
    assert!(cleared.is_none(), "adopt_or_clear should return None after stop");
}
