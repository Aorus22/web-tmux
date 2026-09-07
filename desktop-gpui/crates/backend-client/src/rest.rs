//! HTTP REST client implementation for interacting with the Go web-tmux backend.

use std::time::Duration;
use thiserror::Error;
use crate::models::{
    ApiErrorResponse, CreateSessionRequest, CreateSessionResponse, HealthResponse, TmuxInfo,
    TmuxTree,
};

#[derive(Debug, Error)]
pub enum RestError {
    #[error("HTTP request error ({url}): {source}")]
    Request {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("API error {status} for {url}: {message}")]
    Api {
        url: String,
        status: u16,
        message: String,
    },
    #[error("Failed to decode JSON response from {url}: {source}")]
    Decode {
        url: String,
        #[source]
        source: reqwest::Error,
    },
}

/// Typed REST client for web-tmux backend.
#[derive(Debug, Clone)]
pub struct RestClient {
    base_url: String,
    http: reqwest::Client,
}

impl RestClient {
    /// Create a new RestClient targeting the specified base URL (e.g. "http://127.0.0.1:54321").
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self { base_url, http }
    }

    /// Return the configured base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Check if backend is reachable and reports healthy via GET /api/health.
    pub async fn health(&self) -> Result<HealthResponse, RestError> {
        let url = format!("{}/api/health", self.base_url);
        let resp = self.http.get(&url).send().await.map_err(|e| RestError::Request {
            url: url.clone(),
            source: e,
        })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = Self::extract_error_message(resp, &url).await;
            return Err(RestError::Api { url, status, message });
        }

        resp.json::<HealthResponse>().await.map_err(|e| RestError::Decode {
            url,
            source: e,
        })
    }

    /// Fetch tmux runtime info via GET /api/tmux/info.
    pub async fn info(&self) -> Result<TmuxInfo, RestError> {
        let url = format!("{}/api/tmux/info", self.base_url);
        let resp = self.http.get(&url).send().await.map_err(|e| RestError::Request {
            url: url.clone(),
            source: e,
        })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = Self::extract_error_message(resp, &url).await;
            return Err(RestError::Api { url, status, message });
        }

        resp.json::<TmuxInfo>().await.map_err(|e| RestError::Decode {
            url,
            source: e,
        })
    }

    /// Fetch the full session->window->pane tree via GET /api/sessions.
    pub async fn tree(&self) -> Result<TmuxTree, RestError> {
        let url = format!("{}/api/sessions", self.base_url);
        let resp = self.http.get(&url).send().await.map_err(|e| RestError::Request {
            url: url.clone(),
            source: e,
        })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = Self::extract_error_message(resp, &url).await;
            return Err(RestError::Api { url, status, message });
        }

        resp.json::<TmuxTree>().await.map_err(|e| RestError::Decode {
            url,
            source: e,
        })
    }

    /// Create a new session via POST /api/sessions.
    pub async fn create_session(
        &self,
        req: &CreateSessionRequest,
    ) -> Result<CreateSessionResponse, RestError> {
        let url = format!("{}/api/sessions", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(req)
            .send()
            .await
            .map_err(|e| RestError::Request {
                url: url.clone(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = Self::extract_error_message(resp, &url).await;
            return Err(RestError::Api { url, status, message });
        }

        resp.json::<CreateSessionResponse>().await.map_err(|e| RestError::Decode {
            url,
            source: e,
        })
    }

    /// Validate a tmux executable via POST /api/tmux/binary (Phase 6, SET-03
    /// per D6). Mirrors the `create_session` POST/error shape — no new error
    /// type, no new crate.
    ///
    /// The path is an opaque string client-side (never spawned or probed —
    /// T-06-04): the backend resolves bare names via `LookPath` and probes
    /// `resolved -V` under 3s; failures arrive as
    /// `400 {"error": "tmux binary %q is not usable: …"}` surfaced verbatim.
    pub async fn set_tmux_binary(&self, path: &str) -> Result<TmuxInfo, RestError> {
        let url = format!("{}/api/tmux/binary", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({ "path": path }))
            .send()
            .await
            .map_err(|e| RestError::Request {
                url: url.clone(),
                source: e,
            })?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = Self::extract_error_message(resp, &url).await;
            return Err(RestError::Api { url, status, message });
        }

        resp.json::<TmuxInfo>().await.map_err(|e| RestError::Decode {
            url,
            source: e,
        })
    }

    async fn extract_error_message(resp: reqwest::Response, url: &str) -> String {
        let status = resp.status();
        let fallback = format!("{}: {} {}", url, status.as_u16(), status.canonical_reason().unwrap_or(""));
        match resp.text().await {
            Ok(body) => {
                if let Ok(err_json) = serde_json::from_str::<ApiErrorResponse>(&body) {
                    if !err_json.error.is_empty() {
                        return err_json.error;
                    }
                }
                if !body.trim().is_empty() {
                    body
                } else {
                    fallback
                }
            }
            Err(_) => fallback,
        }
    }
}

/// Settings status-line copy for the tmux-binary row (FE
/// `TerminalSettings.tsx:20` parity): `Using {binary} ({version})` on ok,
/// the raw backend error string on failure (never a generic code).
pub fn binary_status_copy(result: &Result<TmuxInfo, String>) -> String {
    match result {
        Ok(info) => format!("Using {} ({})", info.binary, info.version),
        Err(message) => message.clone(),
    }
}
