//! WebSocket transport DTOs for the local web-tmux backend (`GET /api/ws?session=<name>`).
//!
//! Envelope shapes mirror `be/internal/realtime/protocol.go` verbatim. `SessionSnapshot`
//! reuses the Phase-2 `TmuxSession`/`TmuxWindow`/`TmuxPane` DTOs with the same
//! `deserialize_null_default` null-slice discipline. No `hello` is sent in Phase 3
//! (D7 — `hello` is a viewport resize, not a handshake); the client answers
//! `connection.ready` with `state.resync`.

use serde::{Deserialize, Deserializer, Serialize};

use crate::models::{TmuxPane, TmuxSession, TmuxWindow};

/// Message-type constants, verbatim from `be/internal/realtime/protocol.go:58-97`.
pub const MSG_HELLO: &str = "hello";
pub const MSG_TERMINAL_INPUT: &str = "terminal.input";
pub const MSG_TERMINAL_RESIZE: &str = "terminal.resize";
pub const MSG_TERMINAL_CAPTURE: &str = "terminal.capture";
pub const MSG_PANE_SELECT: &str = "pane.select";
pub const MSG_PANE_SPLIT: &str = "pane.split";
pub const MSG_PANE_RESIZE: &str = "pane.resize";
pub const MSG_PANE_KILL: &str = "pane.kill";
pub const MSG_PANE_RENAME: &str = "pane.rename";
pub const MSG_PANE_ZOOM: &str = "pane.zoom";
pub const MSG_PANE_BREAK: &str = "pane.break";
pub const MSG_PANE_SWAP: &str = "pane.swap";
pub const MSG_WINDOW_SELECT: &str = "window.select";
pub const MSG_WINDOW_CREATE: &str = "window.create";
pub const MSG_WINDOW_RENAME: &str = "window.rename";
pub const MSG_WINDOW_KILL: &str = "window.kill";
pub const MSG_WINDOW_LAYOUT: &str = "window.layout";
pub const MSG_WINDOW_MOVE: &str = "window.move";
pub const MSG_WINDOW_BREAK_ACTIVE: &str = "window.break-active";
pub const MSG_SESSION_CREATE: &str = "session.create";
pub const MSG_SESSION_RENAME: &str = "session.rename";
pub const MSG_SESSION_KILL: &str = "session.kill";
pub const MSG_STATE_RESYNC: &str = "state.resync";

/// Server → client event types, verbatim from `protocol.go:85-97`.
pub const EV_CONNECTION_READY: &str = "connection.ready";
pub const EV_STATE_SNAPSHOT: &str = "state.snapshot";
pub const EV_STATE_DELTA: &str = "state.delta";
pub const EV_TERMINAL_SNAPSHOT: &str = "terminal.snapshot";
pub const EV_TERMINAL_OUTPUT: &str = "terminal.output";
pub const EV_COMMAND_SUCCESS: &str = "command.success";
pub const EV_COMMAND_ERROR: &str = "command.error";
pub const EV_TMUX_DISCONNECTED: &str = "tmux.disconnected";
pub const EV_TMUX_RECONNECTING: &str = "tmux.reconnecting";
pub const EV_SERVER_ERROR: &str = "server.error";

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Client → server envelope (`Incoming` in `protocol.go:7-42`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WsIncoming {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "requestId", skip_serializing_if = "Option::is_none", default)]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cols: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rows: Option<i32>,
    #[serde(rename = "paneId", skip_serializing_if = "Option::is_none", default)]
    pub pane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub amount: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cwd: Option<String>,
    #[serde(rename = "command", skip_serializing_if = "Option::is_none", default)]
    pub initial_command: Option<String>,
    #[serde(rename = "newName", skip_serializing_if = "Option::is_none", default)]
    pub new_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    #[serde(rename = "otherPaneId", skip_serializing_if = "Option::is_none", default)]
    pub other_pane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub layout: Option<String>,
}

/// Server → client envelope (`Outgoing` in `protocol.go:45-56`).
///
/// `replace` is a plain `bool` (not `Option`) because Go omits `false`
/// (`json:"replace,omitempty"`); `seq` is always 0 today but kept for
/// forward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WsOutgoing {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "requestId", default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub session: Option<String>,
    #[serde(rename = "paneId", default)]
    pub pane_id: Option<String>,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub replace: bool,
    #[serde(rename = "screenRows", default)]
    pub screen_rows: Option<i32>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub seq: u64,
    #[serde(default)]
    pub snapshot: Option<SessionSnapshot>,
}

/// Full per-session state dump (`Snapshot` in `be/internal/tmux/model.go:46-52`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub session: TmuxSession,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub windows: Vec<TmuxWindow>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub panes: Vec<TmuxPane>,
    #[serde(default)]
    pub active_window: String,
    #[serde(default)]
    pub active_pane: String,
    /// Present on `state.snapshot` envelopes (Go omits `false`); never on deltas.
    #[serde(default)]
    pub replace: bool,
    /// Forward-compat sequence counter (always 0 today).
    #[serde(default)]
    pub seq: u64,
}

/// Per-tab transport state (rendered by Phase 7 `STATE-03`; stored since Phase 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransportState {
    Connecting,
    Connected,
    Reconnecting,
    #[default]
    Disconnected,
}

/// Correlated command outcome delivered to the awaiting `send_command` caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub request_id: String,
    pub ok: bool,
    pub message: Option<String>,
}

/// Generate a unique correlation id (unique within the 10s pending window is enough).
pub fn request_id() -> String {
    format!("{:x}", rand::random::<u64>())
}

/// Percent-encode a session name as a URL query value (`encodeURIComponent` set:
/// unreserved `A-Za-z0-9 -_.!~*'()` stay literal, everything else — including `/`
/// which `ValidateSessionName` allows — is `%XX` encoded).
fn encode_query_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WsUrlError {
    #[error("unsupported base URL scheme (expected http:// or https://): {0}")]
    UnsupportedScheme(String),
}

/// Map a supervisor `base_url` (`http(s)://host:port`) to the per-session WS URL
/// (`ws(s)://host:port/api/ws?session=<query-encoded>`).
pub fn normalize_ws_url(base_url: &str, session: &str) -> Result<String, WsUrlError> {
    let base = base_url.trim_end_matches('/');
    let rest = if let Some(rest) = base.strip_prefix("http://") {
        ("ws://", rest)
    } else if let Some(rest) = base.strip_prefix("https://") {
        ("wss://", rest)
    } else {
        return Err(WsUrlError::UnsupportedScheme(base_url.to_string()));
    };
    Ok(format!(
        "{}{}/api/ws?session={}",
        rest.0,
        rest.1,
        encode_query_value(session)
    ))
}

// ---------------------------------------------------------------------------
// Live transport: connect_session + split-pump SessionWsHandle (Task 2)
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::oneshot;
use tokio_tungstenite::connect_async;

/// Per-session pending correlation map shared between the handle and the read pump.
pub type SharedPending = Arc<Mutex<HashMap<String, oneshot::Sender<CommandResult>>>>;

/// One inbound frame tagged with the connection it arrived on.
///
/// The pump captures `(session, generation)` at spawn; the GPUI apply path drops
/// events whose generation no longer matches (STATE-04 guard layer (a)).
#[derive(Debug, Clone)]
pub struct WsPumpEvent {
    pub session: String,
    pub generation: u64,
    pub msg: WsOutgoing,
}

#[derive(Debug, thiserror::Error)]
pub enum WsConnectError {
    #[error("invalid base URL: {0}")]
    Url(#[from] WsUrlError),
    #[error("websocket connect failed for {url}: {source}")]
    Connect {
        url: String,
        #[source]
        source: tokio_tungstenite::tungstenite::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WsSendError {
    #[error("session socket is disconnected")]
    Disconnected,
    #[error("failed to encode command: {0}")]
    Encode(String),
}

/// Flume-based handle for one session socket.
///
/// Shape mirrors the reference `terminal_ws.rs` split-pump: an unbounded outbound
/// queue drained by a write pump, and a read pump forwarding tagged events. All I/O
/// lives in tasks spawned on the caller's runtime — never await `connect_async` or
/// pump futures on the GPUI thread; spawn them on `TOKIO_RT`.
pub struct SessionWsHandle {
    session: String,
    generation: u64,
    outbound: flume::Sender<Option<String>>,
    events: flume::Receiver<WsPumpEvent>,
    pending: SharedPending,
    closed: Arc<AtomicBool>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl std::fmt::Debug for SessionWsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionWsHandle")
            .field("session", &self.session)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl Drop for SessionWsHandle {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
        let _ = self.outbound.send(None);
        for task in &self.tasks {
            task.abort();
        }
    }
}

impl SessionWsHandle {
    /// Connection session this handle is bound to.
    pub fn session(&self) -> &str {
        &self.session
    }

    /// Generation captured at spawn (compare against the entry before sending).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Shared pending-correlation map (same `Arc` the read pump resolves into).
    pub fn pending(&self) -> SharedPending {
        self.pending.clone()
    }

    /// Receive the next tagged inbound event (`None` once the socket is gone).
    pub async fn recv_event(&self) -> Option<WsPumpEvent> {
        self.events.recv_async().await.ok()
    }

    /// Send a correlated command: registers `pending[request_id]` BEFORE enqueueing
    /// (no lost-wakeup race), then queues the serialized envelope. Assigns a fresh
    /// `request_id` when the message carries none.
    pub fn send_command(
        &self,
        mut msg: WsIncoming,
    ) -> Result<oneshot::Receiver<CommandResult>, WsSendError> {
        if msg.request_id.as_deref().unwrap_or_default().is_empty() {
            msg.request_id = Some(request_id());
        }
        let id = msg.request_id.clone().unwrap_or_default();
        let (tx, rx) = oneshot::channel();
        match self.pending.lock() {
            Ok(mut pending) => pending.insert(id.clone(), tx),
            Err(_) => return Err(WsSendError::Disconnected),
        };
        let json = serde_json::to_string(&msg).map_err(|e| {
            if let Ok(mut pending) = self.pending.lock() {
                pending.remove(&id);
            }
            WsSendError::Encode(e.to_string())
        })?;
        if self.closed.load(Ordering::SeqCst) || self.outbound.send(Some(json)).is_err() {
            if let Ok(mut pending) = self.pending.lock() {
                pending.remove(&id);
            }
            return Err(WsSendError::Disconnected);
        }
        Ok(rx)
    }

    /// Tear down the socket: the write pump exits, the server sees EOF, and the
    /// read pump drains to `None`. The handle stays usable for `recv_event` drain.
    pub fn disconnect(&self) {
        self.closed.store(true, Ordering::SeqCst);
        let _ = self.outbound.send(None);
    }
}

/// Connect one session socket and spawn its split pumps on the caller's runtime.
pub async fn connect_session(
    base_url: &str,
    session: &str,
    generation: u64,
) -> Result<SessionWsHandle, WsConnectError> {
    connect_session_with_pending(base_url, session, generation, SharedPending::default()).await
}

/// `connect_session` with a caller-provided pending map so `AppState` entries can
/// share the same correlation table as the handle (T-03-03 audit point).
pub async fn connect_session_with_pending(
    base_url: &str,
    session: &str,
    generation: u64,
    pending: SharedPending,
) -> Result<SessionWsHandle, WsConnectError> {
    let url = normalize_ws_url(base_url, session)?;
    let (ws, _) = connect_async(&url).await.map_err(|e| WsConnectError::Connect {
        url: url.clone(),
        source: e,
    })?;
    let (mut sink, mut stream) = ws.split();

    let (outbound_tx, outbound_rx) = flume::unbounded::<Option<String>>();
    let (events_tx, events_rx) = flume::unbounded::<WsPumpEvent>();
    let closed = Arc::new(AtomicBool::new(false));

    let session_name = session.to_string();

    // Write pump: drain the outbound queue into Text frames; None = shutdown.
    let write_closed = closed.clone();
    let write_task = tokio::spawn(async move {
        while let Ok(item) = outbound_rx.recv_async().await {
            let text = match item {
                Some(text) => text,
                None => break,
            };
            if write_closed.load(Ordering::SeqCst) {
                break;
            }
            if sink
                .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Read pump: parse Text frames drop-and-continue (never unwrap — T-03-01),
    // resolve command correlations, answer ready with resync, forward the rest tagged.
    let read_session = session_name.clone();
    let read_outbound = outbound_tx.clone();
    let read_pending = pending.clone();
    let read_closed = closed.clone();
    let read_task = tokio::spawn(async move {
        let forward = |msg: WsOutgoing| {
            let _ = events_tx.send(WsPumpEvent {
                session: read_session.clone(),
                generation,
                msg,
            });
        };
        loop {
            if read_closed.load(Ordering::SeqCst) {
                break;
            }
            let msg = match stream.next().await {
                Some(Ok(m)) => m,
                Some(Err(_)) | None => {
                    forward(WsOutgoing {
                        msg_type: EV_TMUX_DISCONNECTED.to_string(),
                        session: Some(read_session.clone()),
                        ..Default::default()
                    });
                    break;
                }
            };
            match msg {
                tokio_tungstenite::tungstenite::Message::Text(text) => {
                    let parsed: Result<WsOutgoing, _> = serde_json::from_str(&text);
                    let incoming = match parsed {
                        Ok(m) => m,
                        Err(_) => continue, // malformed frame: drop-and-continue
                    };
                    match incoming.msg_type.as_str() {
                        EV_CONNECTION_READY => {
                            // FE sockets.ts parity: belt-and-braces full refresh.
                            let resync = WsIncoming {
                                msg_type: MSG_STATE_RESYNC.to_string(),
                                ..Default::default()
                            };
                            if let Ok(json) = serde_json::to_string(&resync) {
                                let _ = read_outbound.send(Some(json));
                            }
                            forward(incoming);
                        }
                        EV_COMMAND_SUCCESS | EV_COMMAND_ERROR => {
                            let ok = incoming.msg_type == EV_COMMAND_SUCCESS;
                            if let Some(id) = incoming.request_id.clone() {
                                let waiter = match read_pending.lock() {
                                    Ok(mut p) => p.remove(&id),
                                    Err(_) => None,
                                };
                                if let Some(tx) = waiter {
                                    let _ = tx.send(CommandResult {
                                        request_id: id,
                                        ok,
                                        message: incoming.message.clone(),
                                    });
                                }
                                // Late reply with no waiter: ignored (forget-timeout).
                            }
                        }
                        _ => forward(incoming),
                    }
                }
                tokio_tungstenite::tungstenite::Message::Close(_) => {
                    forward(WsOutgoing {
                        msg_type: EV_TMUX_DISCONNECTED.to_string(),
                        session: Some(read_session.clone()),
                        ..Default::default()
                    });
                    break;
                }
                _ => continue, // Binary/Ping/Pong/Frame: ignorable
            }
        }
    });

    Ok(SessionWsHandle {
        session: session_name,
        generation,
        outbound: outbound_tx,
        events: events_rx,
        pending,
        closed,
        tasks: vec![write_task, read_task],
    })
}
