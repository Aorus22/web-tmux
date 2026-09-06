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
