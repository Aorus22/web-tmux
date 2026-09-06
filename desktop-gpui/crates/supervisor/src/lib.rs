//! webtmux-supervisor — backend process supervisor: spawn, port handshake, readiness, lifecycle.
//!
//! Framework-free (tokio only, no GPUI) so the spawn/handshake/readiness/kill
//! logic is testable headless in CI.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use parking_lot::Mutex;
use tokio::io::AsyncBufReadExt;
use tokio::process::Child;

/// Maximum characters stored in the stderr ring buffer (newest-wins).
pub const MAX_STDERR_TAIL_CHARS: usize = 2000;

/// Handle information for a running backend child process.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BackendInfo {
    /// Loopback base URL derived from the handshake, e.g. "http://127.0.0.1:51234".
    pub base_url: String,
    pub pid: u32,
    pub port: u16,
}

/// Lifecycle status of the managed backend process.
/// Exactly three variants per UI-SPEC S3 contract (early exit folds into Failed reason).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BackendStatus {
    /// Process launched; waiting for the `BACKEND_PORT` handshake and readiness.
    Starting,
    /// Handshake received and readiness probe passing; backend serving.
    Ready(BackendInfo),
    /// Startup failed before or during readiness (or backend exited early).
    /// `stderr_tail` is the newest-wins tail of child stderr, capped at 2000 chars.
    Failed {
        reason: String,
        stderr_tail: String,
    },
}

/// Errors originating from the supervisor.
#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    #[error("startup failed: {reason} (stderr: {stderr_tail})")]
    Failed {
        reason: String,
        stderr_tail: String,
    },
    #[error("process IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("stop error: {0}")]
    Stop(String),
}

/// Options controlling backend spawn.
#[derive(Debug, Clone)]
pub struct SpawnOptions {
    /// Path to the backend executable (bundled binary, or a dev override).
    pub backend_path: PathBuf,
    /// Explicit tmux binary path if resolved on desktop side.
    pub tmux_bin: Option<PathBuf>,
    /// Max time to wait for readiness after the handshake (default 10s).
    pub readiness_timeout: Duration,
    /// Max time to wait for the BACKEND_PORT handshake (default 10s).
    pub handshake_timeout: Duration,
}

impl SpawnOptions {
    /// Create new SpawnOptions with the given backend executable path.
    pub fn new(backend_path: impl Into<PathBuf>) -> Self {
        Self {
            backend_path: backend_path.into(),
            tmux_bin: Self::resolve_tmux_binary(),
            readiness_timeout: Duration::from_secs(10),
            handshake_timeout: Duration::from_secs(10),
        }
    }

    pub fn with_readiness_timeout(mut self, timeout: Duration) -> Self {
        self.readiness_timeout = timeout;
        self
    }

    pub fn with_handshake_timeout(mut self, timeout: Duration) -> Self {
        self.handshake_timeout = timeout;
        self
    }

    pub fn with_tmux_bin(mut self, bin: impl Into<PathBuf>) -> Self {
        self.tmux_bin = Some(bin.into());
        self
    }

    /// Resolve tmux binary path on the host platform.
    pub fn resolve_tmux_binary() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("TMUXGUI_TMUX_BIN") {
            if !p.is_empty() {
                return Some(PathBuf::from(p));
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                let winget_tmux = PathBuf::from(local_app_data)
                    .join("Microsoft")
                    .join("WinGet")
                    .join("Links")
                    .join("tmux.exe");
                if winget_tmux.exists() {
                    return Some(winget_tmux);
                }
            }
        }
        Some(PathBuf::from("tmux"))
    }

    /// Return the environment variable pairs injected into the child process.
    pub fn env_vars(&self) -> Vec<(&'static str, String)> {
        let mut vars = vec![
            ("TMUXGUI_HOST", "127.0.0.1".to_string()),
            ("TMUXGUI_PORT", "0".to_string()),
        ];
        if let Some(ref tmux_bin) = self.tmux_bin {
            vars.push(("TMUXGUI_TMUX_BIN", tmux_bin.to_string_lossy().to_string()));
        }
        vars
    }

    /// Construct the child Command with env-only configuration and EMPTY argv.
    pub fn build_command(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.backend_path);
        // Ensure child does NOT attach to launcher tmux session
        cmd.env_remove("TMUX");
        cmd.env_remove("TMUX_PANE");
        for (k, v) in self.env_vars() {
            cmd.env(k, v);
        }
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.kill_on_drop(true);
        cmd
    }
}

/// Parse the first occurrence of `BACKEND_PORT:<port>` from a line.
pub fn parse_handshake_line(line: &str) -> Option<u16> {
    if let Some(idx) = line.find("BACKEND_PORT:") {
        let rest = &line[idx + "BACKEND_PORT:".len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u16>().ok()
    } else {
        None
    }
}

/// Process supervisor managing the Go backend child process lifecycle.
pub struct Supervisor {
    status_tx: tokio::sync::watch::Sender<BackendStatus>,
    status_rx: tokio::sync::watch::Receiver<BackendStatus>,
    info: Option<BackendInfo>,
    child: Arc<Mutex<Option<Child>>>,
    stderr_tail: Arc<Mutex<String>>,
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Supervisor {
    /// Create a new Supervisor instance in Starting status.
    pub fn new() -> Self {
        let (status_tx, status_rx) = tokio::sync::watch::channel(BackendStatus::Starting);
        Self {
            status_tx,
            status_rx,
            info: None,
            child: Arc::new(Mutex::new(None)),
            stderr_tail: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Read current status.
    pub fn status(&self) -> BackendStatus {
        self.status_rx.borrow().clone()
    }

    /// Subscribe to status change notifications.
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<BackendStatus> {
        self.status_rx.clone()
    }

    /// Return backend info if currently ready/running.
    pub fn info(&self) -> Option<&BackendInfo> {
        self.info.as_ref()
    }

    /// Return snapshot of newest stderr tail.
    pub fn stderr_tail(&self) -> String {
        self.stderr_tail.lock().clone()
    }

    /// Spawn the backend process, perform port handshake and readiness polling.
    pub async fn spawn(&mut self, opts: SpawnOptions) -> Result<BackendInfo, SupervisorError> {
        let _ = self.status_tx.send(BackendStatus::Starting);
        self.stderr_tail.lock().clear();

        let mut cmd = opts.build_command();
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let reason = format!("Failed to spawn backend binary '{}': {}", opts.backend_path.display(), e);
                let tail = format!("Execution error: {}", e);
                let status = BackendStatus::Failed {
                    reason: reason.clone(),
                    stderr_tail: tail.clone(),
                };
                let _ = self.status_tx.send(status);
                return Err(SupervisorError::Failed {
                    reason,
                    stderr_tail: tail,
                });
            }
        };

        let pid = child.id().unwrap_or(0);
        let mut stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Spawn stderr collector into ring buffer
        let stderr_tail = self.stderr_tail.clone();
        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let mut lock = stderr_tail.lock();
                    lock.push_str(&line);
                    lock.push('\n');
                    if lock.len() > MAX_STDERR_TAIL_CHARS {
                        let excess = lock.len() - MAX_STDERR_TAIL_CHARS;
                        if let Some((idx, _)) = lock.char_indices().find(|(i, _)| *i >= excess) {
                            *lock = lock[idx..].to_string();
                        }
                    }
                }
            });
        }

        // Wait for BACKEND_PORT handshake
        let mut port_found: Option<u16> = None;
        if let Some(stdout_stream) = stdout.take() {
            let mut reader = tokio::io::BufReader::new(stdout_stream);
            let mut handshake_line = String::new();
            let handshake_fut = async {
                loop {
                    handshake_line.clear();
                    match reader.read_line(&mut handshake_line).await {
                        Ok(0) => return Err("Stdout closed without BACKEND_PORT handshake".to_string()),
                        Ok(_) => {
                            if let Some(port) = parse_handshake_line(&handshake_line) {
                                return Ok((port, reader));
                            }
                        }
                        Err(e) => return Err(format!("Stdout read error: {}", e)),
                    }
                }
            };

            match tokio::time::timeout(opts.handshake_timeout, handshake_fut).await {
                Ok(Ok((port, remaining_reader))) => {
                    port_found = Some(port);
                    // Drain remaining stdout in background to prevent child EPIPE (delta D6)
                    tokio::spawn(async move {
                        let mut lines = remaining_reader.lines();
                        while let Ok(Some(_)) = lines.next_line().await {}
                    });
                }
                Ok(Err(err_msg)) => {
                    let tail = self.stderr_tail();
                    let reason = format!("Backend handshake missing: {}", err_msg);
                    let status = BackendStatus::Failed {
                        reason: reason.clone(),
                        stderr_tail: tail.clone(),
                    };
                    let _ = self.status_tx.send(status);
                    let _ = child.kill().await;
                    return Err(SupervisorError::Failed { reason, stderr_tail: tail });
                }
                Err(_) => {
                    let tail = self.stderr_tail();
                    let reason = format!("Backend handshake timed out after {:?}", opts.handshake_timeout);
                    let status = BackendStatus::Failed {
                        reason: reason.clone(),
                        stderr_tail: tail.clone(),
                    };
                    let _ = self.status_tx.send(status);
                    let _ = child.kill().await;
                    return Err(SupervisorError::Failed { reason, stderr_tail: tail });
                }
            }
        }

        let port = match port_found {
            Some(p) => p,
            None => {
                let tail = self.stderr_tail();
                let reason = "No stdout available to read handshake".to_string();
                let status = BackendStatus::Failed {
                    reason: reason.clone(),
                    stderr_tail: tail.clone(),
                };
                let _ = self.status_tx.send(status);
                let _ = child.kill().await;
                return Err(SupervisorError::Failed { reason, stderr_tail: tail });
            }
        };

        let base_url = format!("http://127.0.0.1:{}", port);
        let info = BackendInfo {
            base_url: base_url.clone(),
            pid,
            port,
        };

        // Poll readiness probe (GET /api/health)
        let probe_url = format!("{}/api/health", base_url);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(1000))
            .build()
            .unwrap_or_default();

        let readiness_fut = async {
            let start = tokio::time::Instant::now();
            loop {
                // Check if child exited prematurely
                if let Ok(Some(exit_status)) = child.try_wait() {
                    return Err(format!("backend exited early ({})", exit_status));
                }

                if let Ok(resp) = client.get(&probe_url).send().await {
                    if resp.status().is_success() || resp.status().as_u16() < 500 {
                        return Ok(());
                    }
                }

                if start.elapsed() > opts.readiness_timeout {
                    return Err(format!("Readiness probe timed out after {:?}", opts.readiness_timeout));
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        };

        if let Err(err_reason) = readiness_fut.await {
            let tail = self.stderr_tail();
            let status = BackendStatus::Failed {
                reason: err_reason.clone(),
                stderr_tail: tail.clone(),
            };
            let _ = self.status_tx.send(status);
            let _ = child.kill().await;
            return Err(SupervisorError::Failed { reason: err_reason, stderr_tail: tail });
        }

        // Ready!
        let _ = self.status_tx.send(BackendStatus::Ready(info.clone()));
        self.info = Some(info.clone());
        *self.child.lock() = Some(child);

        // Spawn child exit monitor
        let child_arc = Arc::clone(&self.child);
        let status_tx = self.status_tx.clone();
        let stderr_tail_arc = self.stderr_tail.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let mut lock = child_arc.lock();
                if let Some(child) = lock.as_mut() {
                    match child.try_wait() {
                        Ok(Some(exit_status)) => {
                            let tail = stderr_tail_arc.lock().clone();
                            let reason = format!("backend exited early ({})", exit_status);
                            let _ = status_tx.send(BackendStatus::Failed {
                                reason,
                                stderr_tail: tail,
                            });
                            break;
                        }
                        Ok(None) => continue,
                        Err(_) => {
                            let tail = stderr_tail_arc.lock().clone();
                            let _ = status_tx.send(BackendStatus::Failed {
                                reason: "backend exited unexpectedly".to_string(),
                                stderr_tail: tail,
                            });
                            break;
                        }
                    }
                } else {
                    // Stopped cleanly
                    break;
                }
            }
        });

        Ok(info)
    }

    /// Return child PID if currently spawned.
    pub fn child_pid(&self) -> Option<u32> {
        self.child.lock().as_ref().and_then(|c| c.id())
    }

    /// Stop the backend process: graceful terminate with grace duration, then hard kill.
    pub async fn stop(&mut self, grace: Duration) -> Result<(), SupervisorError> {
        let child_opt = self.child.lock().take();
        if let Some(mut child) = child_opt {
            #[cfg(unix)]
            {
                if let Some(pid) = child.id() {
                    unsafe {
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                }
            }

            let wait_fut = child.wait();
            match tokio::time::timeout(grace, wait_fut).await {
                Ok(Ok(_)) => {}
                _ => {
                    let _ = child.kill().await;
                }
            }
        }
        self.info = None;
        Ok(())
    }

    /// Check if a previously recorded backend is still serving, or clear it.
    pub async fn adopt_or_clear(base_url: Option<String>) -> Option<BackendInfo> {
        let base_url = base_url?;
        let probe_url = format!("{}/api/health", base_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .ok()?;

        if let Ok(resp) = client.get(&probe_url).send().await {
            if resp.status().is_success() {
                let port = probe_url
                    .split(':')
                    .nth(2)?
                    .split('/')
                    .next()?
                    .parse::<u16>()
                    .unwrap_or(0);
                return Some(BackendInfo {
                    base_url,
                    pid: 0,
                    port,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_handshake() {
        assert_eq!(parse_handshake_line("BACKEND_PORT:54321"), Some(54321));
        assert_eq!(parse_handshake_line("prefix BACKEND_PORT:8080 suffix"), Some(8080));
        assert_eq!(parse_handshake_line("BACKEND_PORT:0"), Some(0));
        assert_eq!(parse_handshake_line("BACKEND_PORT:notaport"), None);
        assert_eq!(parse_handshake_line("just a regular log line"), None);
    }

    #[test]
    fn test_spawn_options_env() {
        let opts = SpawnOptions::new("tmux-gui-server");
        let env_vars = opts.env_vars();

        let host = env_vars.iter().find(|(k, _)| *k == "TMUXGUI_HOST");
        assert_eq!(host.unwrap().1, "127.0.0.1");

        let port = env_vars.iter().find(|(k, _)| *k == "TMUXGUI_PORT");
        assert_eq!(port.unwrap().1, "0");

        let cmd = opts.build_command();
        let std_cmd = cmd.as_std();
        assert_eq!(std_cmd.get_args().count(), 0);
    }
}
