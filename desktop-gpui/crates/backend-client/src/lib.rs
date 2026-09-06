//! Typed HTTP/WS client for the local web-tmux backend.

pub mod models;
pub mod rest;
pub mod validation;

pub use models::{
    ApiErrorResponse, CreateSessionRequest, CreateSessionResponse, HealthResponse, HealthTmux,
    SessionTreeNode, TmuxInfo, TmuxPane, TmuxSession, TmuxTree, TmuxWindow, WindowTreeNode,
};
pub use rest::{RestClient, RestError};
pub use validation::{validate_session_name, ValidationError};
