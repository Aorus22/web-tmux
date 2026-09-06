//! Data transfer models for web-tmux REST endpoints matching Go backend JSON.

use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// GET /api/health response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
    pub tmux: HealthTmux,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthTmux {
    pub installed: bool,
    pub version: String,
}

/// GET /api/tmux/info response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxInfo {
    pub version: String,
    pub ok: bool,
    pub binary: String,
}

/// Sidebar representation: sessions with windows, panes grouped by window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TmuxTree {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub sessions: Vec<SessionTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTreeNode {
    pub session: TmuxSession,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub windows: Vec<WindowTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowTreeNode {
    pub window: TmuxWindow,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub panes: Vec<TmuxPane>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxSession {
    pub name: String,
    pub windows: usize,
    pub attached: usize,
    pub created_at: i64,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxWindow {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub active: bool,
    pub panes: usize,
    pub width: usize,
    pub height: usize,
    pub layout: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxPane {
    pub id: String,
    pub index: usize,
    pub window_id: String,
    pub active: bool,
    pub zoomed: bool,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
    pub pid: usize,
    pub current_command: String,
    pub current_path: String,
    pub title: String,
}

/// POST /api/sessions request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_command: Option<String>,
}

/// POST /api/sessions successful (201) response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionResponse {
    pub name: String,
}

/// Generic error JSON response from backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error: String,
}
