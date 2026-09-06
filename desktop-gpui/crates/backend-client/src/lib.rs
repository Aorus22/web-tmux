//! Typed HTTP/WS client for the local web-tmux backend.

pub mod models;
pub mod rest;
pub mod validation;
pub mod ws;

pub use models::{
    ApiErrorResponse, CreateSessionRequest, CreateSessionResponse, HealthResponse, HealthTmux,
    SessionTreeNode, TmuxInfo, TmuxPane, TmuxSession, TmuxTree, TmuxWindow, WindowTreeNode,
};
pub use rest::{RestClient, RestError};
pub use validation::{validate_session_name, ValidationError};
pub use ws::{
    connect_session, connect_session_with_pending, normalize_ws_url, request_id, CommandResult,
    SessionSnapshot, SessionWsHandle, SharedPending, TransportState, WsConnectError, WsIncoming,
    WsOutgoing, WsPumpEvent, WsSendError, WsUrlError, EV_COMMAND_ERROR, EV_COMMAND_SUCCESS, EV_CONNECTION_READY,
    EV_SERVER_ERROR, EV_STATE_DELTA, EV_STATE_SNAPSHOT, EV_TERMINAL_OUTPUT, EV_TERMINAL_SNAPSHOT,
    EV_TMUX_DISCONNECTED, EV_TMUX_RECONNECTING, MSG_HELLO, MSG_PANE_BREAK, MSG_PANE_KILL,
    MSG_PANE_RENAME, MSG_PANE_RESIZE, MSG_PANE_SELECT, MSG_PANE_SPLIT, MSG_PANE_SWAP, MSG_PANE_ZOOM,
    MSG_SESSION_CREATE, MSG_SESSION_KILL, MSG_SESSION_RENAME, MSG_STATE_RESYNC,
    MSG_TERMINAL_CAPTURE, MSG_TERMINAL_INPUT, MSG_TERMINAL_RESIZE, MSG_WINDOW_BREAK_ACTIVE,
    MSG_WINDOW_CREATE, MSG_WINDOW_KILL, MSG_WINDOW_LAYOUT, MSG_WINDOW_MOVE, MSG_WINDOW_RENAME,
    MSG_WINDOW_SELECT,
};
