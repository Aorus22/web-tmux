//! Root application state, polling pump, and supervisor lifecycle.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock};
use std::time::Instant;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::WindowExt as _;
use parking_lot::Mutex;
use tokio::sync::oneshot;
use webtmux_backend_client::{
    connect_session, connect_session_with_pending, validate_session_name, RestClient,
    CommandResult, SessionSnapshot, SessionWsHandle, SharedPending, TransportState, TmuxTree,
    WsIncoming, WsOutgoing, EV_CONNECTION_READY, EV_STATE_DELTA, EV_STATE_SNAPSHOT,
    EV_SERVER_ERROR, EV_TERMINAL_OUTPUT, EV_TERMINAL_SNAPSHOT, EV_TMUX_DISCONNECTED,
    EV_TMUX_RECONNECTING, MSG_PANE_BREAK, MSG_PANE_KILL, MSG_PANE_RENAME, MSG_PANE_RESIZE,
    MSG_PANE_SELECT, MSG_PANE_SPLIT, MSG_PANE_SWAP, MSG_PANE_ZOOM, MSG_SESSION_KILL,
    MSG_SESSION_RENAME, MSG_TERMINAL_CAPTURE, MSG_TERMINAL_INPUT, MSG_TERMINAL_RESIZE,
    MSG_WINDOW_BREAK_ACTIVE, MSG_WINDOW_CREATE, MSG_WINDOW_KILL, MSG_WINDOW_LAYOUT,
    MSG_WINDOW_MOVE, MSG_WINDOW_RENAME, MSG_WINDOW_SELECT,
};
use webtmux_settings::{
    clamp_font_size, clamp_line_height, clamp_scrollback, DesktopSettings, Theme as SettingsTheme,
};
use webtmux_supervisor::{BackendInfo, BackendStatus, SpawnOptions, Supervisor};
use webtmux_terminal::{
    apply_capture, scroll_report, AlacPoint, ColorPalette, TermMode, Terminal, TerminalConfig,
    TerminalEvent,
};
use crate::views::terminal_view::TerminalView;
use crate::views::{status::render_status_page, tab_strip::render_title_bar};
use crate::pane_geometry::drag_step_throttled;

/// Global multi-thread Tokio runtime entered once at application boot.
pub static TOKIO_RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create Tokio runtime")
});

/// Events pumped from the async supervisor task to GPUI main thread.
pub enum SupervisorEvent {
    Status(BackendStatus),
    Ready(BackendInfo),
    Failed { reason: String, stderr_tail: String },
}

/// One open workspace tab: live socket binding + committed snapshot (Phase 3).
pub struct OpenSession {
    /// Per-connection monotonic generation, bumped per (re)connect and rename
    /// re-resolution. The read pump captures it at spawn; `apply_event` drops
    /// any event whose generation no longer matches (STATE-04 layer (a)).
    pub generation: u64,
    /// Transport state for Phase 7 (STATE-03) rendering; stored since Phase 3.
    pub transport: TransportState,
    /// Last committed snapshot (via the triple generation guard).
    pub snapshot: Option<SessionSnapshot>,
    /// Live socket handle; `None` until `ensure_session_socket` connects.
    /// Dropping it aborts the pumps (teardown-on-close, D4).
    pub handle: Option<SessionWsHandle>,
    /// Correlated-request table shared with the handle's read pump.
    pub pending: SharedPending,
    /// Last `server.error` / connect failure for the tab.
    pub last_error: Option<String>,
}

impl Default for OpenSession {
    fn default() -> Self {
        Self {
            generation: 0,
            transport: TransportState::Disconnected,
            snapshot: None,
            handle: None,
            pending: SharedPending::default(),
            last_error: None,
        }
    }
}

/// One pane's live terminal plus its capture-replay guards (Phase 4, TERM-02).
///
/// Lives in the `AppState` store — never inside views — so hidden sessions
/// keep ingesting while unmounted (Pitfall 6).
pub struct PaneTerminal {
    /// Shared handle to the alacritty-backed grid, mutated synchronously on
    /// the GPUI thread. `TerminalView`s hold a clone for paint/input; the
    /// store's handle keeps hidden sessions ingesting while unmounted.
    pub terminal: Arc<Mutex<Terminal>>,
    /// Exactly-once gate: first `terminal.snapshot` replaces, later ones drop
    /// until `invalidate_pane_snapshot` re-arms (reconnect / layout resync).
    pub snapshot_written: bool,
    /// Scrollback lines already pushed to this instance (FE `ingestedHistory`).
    pub ingested_history: usize,
}

impl PaneTerminal {
    fn fresh() -> Self {
        Self {
            terminal: Arc::new(Mutex::new(Terminal::new(80, 24))),
            snapshot_written: false,
            ingested_history: 0,
        }
    }
}

/// One in-progress divider drag (Phase 5, D3 per `PaneResizeHandle.tsx`).
///
/// `mouse_down` on a divider handle records `(pane_id, direction, start_px)`
/// with `last_cells = 0` and `last_sent = now`; moves compute the incremental
/// step and fire throttled `pane.resize` sends. `cell_px` is the dragged
/// pane's axis cell size (`cell_w` for vertical dividers, `cell_h` for
/// horizontal ones) so the divider tracks the pointer exactly.
#[derive(Debug, Clone)]
pub struct PaneDragState {
    pub pane_id: String,
    pub direction: char,
    pub start_px: f32,
    pub cell_px: f32,
    pub last_cells: i64,
    pub last_sent: Instant,
}

/// Generation-tagged pending viewport armed by `TerminalView` resize
/// callbacks (Phase 4, TERM-06). Armed here, SENT in plan 04-02 — never from
/// paint (Pitfall 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingViewport {
    pub generation: u64,
    pub cols: usize,
    pub rows: usize,
}

// --- Phase 4 plan 04-02 Task 1: FE-parity wheel policy (TERM-04) --------
///
/// FE ground truth (`fe/src/features/terminal/useTerminal.ts:124-170`):
/// capture-phase consume; TUI-switch ON → notch accumulator (100px/notch,
/// line×16, page×100, burst clamp 3) emitting PageUp/PageDown repeats over
/// `terminal.input`; OFF → native scrollback. GPUI port adds the SGR branch
/// first (mouse-reporting apps get SGR 64/65 via `scroll_report`).
///
/// Sign note: DOM `WheelEvent.deltaY` is negative on wheel-up, while GPUI
/// `ScrollDelta` positive-y means wheel-up (reference `view.rs` treats
/// positive as up for both `scroll_report` and `scroll_display`). The TUI key
/// mapping below is therefore mirrored vs the FE text (`pages > 0 → PageUp`)
/// but semantically identical: wheel-up pages up in both systems.
pub const WHEEL_NOTCH_PX: f32 = 100.0;
pub const WHEEL_MAX_BURST: i32 = 3;
pub const WHEEL_LINE_PX: f32 = 16.0;
pub const PAGE_UP_SEQ: &str = "\x1b[5~";
pub const PAGE_DOWN_SEQ: &str = "\x1b[6~";

/// Headless wheel delta mirroring GPUI `ScrollDelta` without the GPUI event
/// type (pure-apply pattern: decision stays testable, the view converts).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelDelta {
    Lines(f32),
    Pixels(f32),
}

/// Pure wheel outcome: bytes to send over `terminal.input`, a scrollback
/// delta to apply, or sub-notch silence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WheelAction {
    Sgr(Vec<u8>),
    Pages(Vec<u8>),
    Scrollback(i32),
    Ignored,
}

/// FE normalization into pixels for the TUI notch accumulator:
/// line deltas scale ×16, pixel deltas pass through (page deltas do not exist
/// in GPUI; FE page×100 has no caller here).
pub fn wheel_delta_to_px(delta: WheelDelta) -> f32 {
    match delta {
        WheelDelta::Lines(y) => y * WHEEL_LINE_PX,
        WheelDelta::Pixels(y) => y,
    }
}

/// Reference pixel→lines conversion (`view.rs:410-427` Pixels branch) plus the
/// Lines passthrough: the TUI-off scrollback path.
pub fn wheel_delta_to_lines(delta: WheelDelta, cell_h: f32) -> i32 {
    match delta {
        WheelDelta::Lines(y) => y.round() as i32,
        WheelDelta::Pixels(y) => {
            if cell_h > 0.0 {
                (y / cell_h).round() as i32
            } else {
                0
            }
        }
    }
}

fn mouse_reporting_on(mode: TermMode) -> bool {
    mode.intersects(
        TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_MOTION | TermMode::MOUSE_DRAG,
    )
}

/// Pure wheel decision (mode bits + tui flag + delta → action + new accum).
/// Headless-testable; the GPUI handler only converts + emits.
pub fn decide_wheel_action(
    mode: TermMode,
    tui_on: bool,
    delta: WheelDelta,
    accum_px: f32,
    cell_h: f32,
    point: AlacPoint,
    mods: u8,
) -> (WheelAction, f32) {
    // Mouse-reporting apps get verbatim SGR passthrough (no accumulation).
    if mouse_reporting_on(mode) {
        let lines = wheel_delta_to_lines(delta, cell_h);
        if lines == 0 {
            return (WheelAction::Ignored, accum_px);
        }
        match scroll_report(lines, point, mods, mode) {
            Some(bytes) => return (WheelAction::Sgr(bytes), accum_px),
            None => return (WheelAction::Ignored, accum_px),
        }
    }
    if tui_on {
        let accum2 = accum_px + wheel_delta_to_px(delta);
        let notches = (accum2 / WHEEL_NOTCH_PX).trunc() as i32;
        if notches == 0 {
            return (WheelAction::Ignored, accum2);
        }
        let pages = notches.clamp(-WHEEL_MAX_BURST, WHEEL_MAX_BURST);
        let new_accum = accum2 - notches as f32 * WHEEL_NOTCH_PX;
        let seq = if pages > 0 {
            PAGE_UP_SEQ
        } else {
            PAGE_DOWN_SEQ
        };
        let mut bytes = Vec::with_capacity(seq.len() * pages.unsigned_abs() as usize);
        for _ in 0..pages.unsigned_abs() {
            bytes.extend_from_slice(seq.as_bytes());
        }
        return (WheelAction::Pages(bytes), new_accum);
    }
    // TUI-off: scrollback delta (safe no-op-ish over alt-screen per A4 —
    // alacritty clamps internally).
    let lines = wheel_delta_to_lines(delta, cell_h);
    if lines == 0 {
        (WheelAction::Ignored, accum_px)
    } else {
        (WheelAction::Scrollback(lines), accum_px)
    }
}

// --- Phase 4 plan 04-02 Task 1: copy/paste key routing (TERM-05) ---------
///
/// Mirrors the `TerminalView::on_key_down` checks (which stay as the runtime
/// path): `Ctrl+Shift+C` or `Cmd+C` with a selection copies, `Ctrl+Shift+V`
/// or `Cmd+V` pastes, everything else falls to the `keystroke_to_bytes` path
/// (so `Ctrl+C` with an empty selection becomes the `\x03` interrupt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRoute {
    Copy,
    Paste,
    Terminal,
}

pub fn decide_key_route(
    has_selection: bool,
    ctrl: bool,
    shift: bool,
    platform: bool,
    key: &str,
) -> KeyRoute {
    let is_copy = (ctrl && shift && key.eq_ignore_ascii_case("c"))
        || (platform && key.eq_ignore_ascii_case("c"));
    if is_copy && has_selection {
        return KeyRoute::Copy;
    }
    let is_paste = (ctrl && shift && key.eq_ignore_ascii_case("v"))
        || (platform && key.eq_ignore_ascii_case("v"));
    if is_paste {
        return KeyRoute::Paste;
    }
    KeyRoute::Terminal
}

// --- Phase 4 plan 04-02 Task 2: debounced resize + layout resync (TERM-06)
/// Nominal cell size for the px fallback (`fe/src/lib/geometry.ts:23-24`).
pub const RESIZE_CELL_W_PX: f32 = 8.0;
pub const RESIZE_CELL_H_PX: f32 = 18.0;

/// Clamp to the server contract (`cols>=2, rows>=1`; handler rejects
/// `cols<=0||rows<=0`, service enforces `cols<2||rows<1`).
pub fn clamp_viewport(cols: usize, rows: usize) -> (i32, i32) {
    (cols.max(2) as i32, rows.max(1) as i32)
}

/// FE `pxToColsRows` parity: container px → tmux viewport with the same floor.
pub fn px_to_cols_rows(width_px: f32, height_px: f32) -> (usize, usize) {
    (
        (width_px / RESIZE_CELL_W_PX).round().max(2.0) as usize,
        (height_px / RESIZE_CELL_H_PX).round().max(1.0) as usize,
    )
}

/// FE `actualViewport` parity (`PaneWorkspace.tsx:41-57`): scale one visible
/// pane's measured xterm size to the whole window via tmux cell geometry. A
/// full/zoomed pane yields exactly its measured size. Falls back to px
/// conversion before any terminal registers.
pub fn actual_viewport(
    measured_cols: usize,
    measured_rows: usize,
    pane_w: usize,
    pane_h: usize,
    win_w: usize,
    win_h: usize,
    fallback_w_px: f32,
    fallback_h_px: f32,
) -> (usize, usize) {
    if pane_w > 0 && pane_h > 0 && win_w > 0 && win_h > 0 {
        let cols = ((measured_cols as f32 * win_w as f32) / pane_w as f32)
            .round()
            .max(2.0) as usize;
        let rows = ((measured_rows as f32 * win_h as f32) / pane_h as f32)
            .round()
            .max(1.0) as usize;
        (cols, rows)
    } else {
        px_to_cols_rows(fallback_w_px, fallback_h_px)
    }
}

/// Pure constructor for window-level `terminal.resize` (clamped, no hello).
pub fn build_terminal_resize(cols: usize, rows: usize) -> WsIncoming {
    let (cols, rows) = clamp_viewport(cols, rows);
    WsIncoming {
        msg_type: MSG_TERMINAL_RESIZE.to_string(),
        cols: Some(cols),
        rows: Some(rows),
        ..Default::default()
    }
}

/// Stable layout key (`activeWindow|WxH|layout|pane-id:cells,zoom…`, FE
/// `layoutKey` parity). Terminal output and active-pane changes leave it
/// unchanged, so resync fires only on real topology/geometry changes.
pub fn compute_layout_key(
    active_window: &str,
    win_w: usize,
    win_h: usize,
    layout: &str,
    panes: &[(String, usize, usize, usize, usize, bool)],
) -> String {
    let parts: Vec<String> = panes
        .iter()
        .map(|(id, left, top, w, h, zoomed)| {
            format!("{id}:{left},{top},{w},{h},{}", if *zoomed { 1 } else { 0 })
        })
        .collect();
    format!("{active_window}|{win_w}x{win_h}|{layout}|{}", parts.join(";"))
}

/// Layout-key resync decision: first mount skips (initial captures cover it),
/// identical keys schedule nothing, changes schedule the 150/325ms pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutResync {
    FirstMountSkip,
    NoChange,
    Resync,
}

/// Title-bar window-tab model derived from the active snapshot (SHELL-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowTab {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub active: bool,
}

/// One same-window swap target for the pane menu picker (Phase 5, PANE-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapCandidate {
    pub id: String,
    pub label: String,
}

/// Kill transport route per D6 (see `AppState::kill_route`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillRoute {    /// Victim's own socket is live — kill rides it (no explicit field).
    Victim,
    /// No victim socket — ride another live socket with explicit `session`.
    ViaOther(String),
    /// Zero live sockets — ephemeral one-shot (commits nothing).
    Ephemeral,
}

/// Root Application State Entity.
pub struct AppState {
    pub settings: DesktopSettings,
    pub spawn_opts: Option<SpawnOptions>,
    pub backend_status: BackendStatus,
    pub base_url: Option<String>,
    pub is_maximized: bool,
    pub supervisor: std::sync::Arc<parking_lot::Mutex<Option<Supervisor>>>,

    // Phase 2: REST Client, Polling, Sidebar & Session State
    pub rest_client: Option<RestClient>,
    pub tree: TmuxTree,
    pub active_session: Option<String>,
    pub expanded_sessions: HashSet<String>,
    pub sidebar_open: bool,
    pub poll_generation: u64,
    pub tree_error: Option<String>,

    // Phase 3: per-session WS tabs (SESS-02) + generation guard (STATE-04).
    // `active_session` stays a view pointer only; sockets live in `sessions`.
    pub open_sessions: Vec<String>,
    pub sessions: HashMap<String, OpenSession>,

    /// Settings placeholder page flag (SHELL-01, Phase-6-owned page): the gear
    /// sets `active_session = None` + this to true; tabs stay open underneath.
    pub showing_settings: bool,

    /// DLG1 Rename Session dialog form entity, held alive while the modal is
    /// open. Reset to `None` on dismiss; replaced on every (re)open.
    pub rename_session_form:
        Option<gpui::Entity<crate::views::rename_session_dialog::RenameSessionForm>>,
    /// DLG1 Kill-confirm dialog form entity, held alive while the modal is
    /// open. Reset to `None` on dismiss; replaced on every (re)open.
    pub kill_session_form:
        Option<gpui::Entity<crate::views::session_context_menu::KillSessionForm>>,
    /// Rename pended while `ensure_session_socket(target)` connects (D5):
    /// `(target, new_name, dialog window)`. Flushed on connect, dropped with
    /// an inline error when the connect fails.
    pub pending_rename: Option<(String, String, AnyWindowHandle)>,

    /// DLG1 Create Session dialog form entity, held alive while the modal is
    /// open. Reset to `None` on dismiss; replaced on every (re)open.
    pub create_session_form:
        Option<gpui::Entity<crate::views::create_session_dialog::CreateSessionForm>>,

    /// DLG1 Rename Pane dialog form entity (Phase 5, PANE-05 + DLG-02).
    /// Same lifetime as the session rename form: replaced on every open,
    /// `None` on dismiss.
    pub rename_pane_form:
        Option<gpui::Entity<crate::views::rename_pane_dialog::RenamePaneForm>>,
    /// Kill-confirm dialog form entity for panes (Phase 5, PANE-05 per D7).
    pub kill_pane_form:
        Option<gpui::Entity<crate::views::pane_context_menu::KillPaneForm>>,
    /// DLG1 Rename Window dialog form entity (Phase 5, PANE-08 + DLG-02).
    /// Same lifetime as the session rename form: replaced on every open,
    /// `None` on dismiss.
    pub rename_window_form:
        Option<gpui::Entity<crate::views::rename_window_dialog::RenameWindowForm>>,
    /// Kill-confirm dialog form entity for windows (Phase 5, PANE-08 per D7).
    pub kill_window_form: Option<gpui::Entity<crate::views::tab_strip::KillWindowForm>>,
    // Phase 4: pane-id-keyed terminal store (TERM-01/02/03/07). Owns every
    // pane's `Terminal`; views borrow/render the active session's entries.
    pub terminals: HashMap<String, PaneTerminal>,
    /// Pane → owning session attribution for input routing + D7 retirement.
    pub pane_session: HashMap<String, String>,
    /// Per-pane TUI-scroll override (D3); absent means ON (`unwrap_or(true)`).
    pub tui_scroll: HashMap<String, bool>,
    /// Last OSC title per pane (D8); bell is a no-op.
    pub pane_titles: HashMap<String, String>,
    /// Resize armed by views, sent by plan 04-02 (TERM-06).
    pub pending_viewport: Option<PendingViewport>,
    /// Per-pane wheel notch accumulator (px leftover, FE `wheelAccum` parity).
    pub wheel_accum: HashMap<String, f32>,
    /// Debounce sequence: bumped per arm, timers drop when stale (poll-pump
    /// generation pattern — second arm supersedes the first).
    pub resize_seq: u64,
    /// Last seen layout key for the 150/325ms resync pair; `None` = first
    /// mount (initial captures already cover it — skip the pair).
    pub last_layout_key: Option<String>,
    /// Retained per-pane views for the active window (tracer layout; Phase 5
    /// owns geometry). Views hold only a shared terminal clone — the store
    /// above stays the owner, so hidden sessions keep ingesting.
    pub terminal_views: HashMap<String, Entity<TerminalView>>,
    /// Last measured workspace container size in px (Phase 5 grid probe).
    /// `None` before the first measure — the grid falls back to 800x600.
    pub workspace_size: Option<(f32, f32)>,
    /// In-progress divider drag, if any (Phase 5, D3). `Some` between the
    /// divider `mouse_down` and the matching `mouse_up`/`mouse_up_out`;
    /// grid-level `mouse_move` streams positions through it.
    pub pane_drag: Option<PaneDragState>,
}

impl AppState {
    pub fn new(settings: DesktopSettings, spawn_opts: Option<SpawnOptions>) -> Self {
        let is_maximized = settings.window_state.as_ref().map(|s| s.maximized).unwrap_or(false);
        Self {
            settings,
            spawn_opts,
            backend_status: BackendStatus::Starting,
            base_url: None,
            is_maximized,
            supervisor: std::sync::Arc::new(parking_lot::Mutex::new(None)),

            rest_client: None,
            tree: TmuxTree::default(),
            active_session: None,
            expanded_sessions: HashSet::new(),
            sidebar_open: true,
            poll_generation: 0,
            tree_error: None,
            create_session_form: None,
            open_sessions: Vec::new(),
            sessions: HashMap::new(),
            showing_settings: false,
            rename_session_form: None,
            kill_session_form: None,
            pending_rename: None,
            rename_pane_form: None,
            kill_pane_form: None,
            rename_window_form: None,
            kill_window_form: None,
            terminals: HashMap::new(),
            pane_session: HashMap::new(),
            tui_scroll: HashMap::new(),
            pane_titles: HashMap::new(),
            pending_viewport: None,
            wheel_accum: HashMap::new(),
            resize_seq: 0,
            last_layout_key: None,
            terminal_views: HashMap::new(),
            workspace_size: None,
            pane_drag: None,
        }
    }

    /// Determine current workspace state (Error, Empty, SelectSession, ActiveSession).
    pub fn workspace_state(&self) -> crate::views::session_states::WorkspaceState {
        crate::views::session_states::determine_workspace_state(
            self.tree_error.as_deref(),
            !self.tree.sessions.is_empty(),
            self.active_session.is_some(),
        )
    }

    /// Trigger an immediate REST polling fetch with generation guard discarding stale ticks.
    pub fn trigger_poll(&mut self, cx: &mut Context<Self>) {
        let client = match &self.rest_client {
            Some(c) => c.clone(),
            None => return,
        };

        self.poll_generation += 1;
        let expected_gen = self.poll_generation;

        cx.spawn(move |view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                let result = client.tree().await;
                let _ = cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, cx| {
                            if this.poll_generation != expected_gen {
                                return;
                            }
                            match result {
                                Ok(tree) => {
                                    if this.active_session.is_none() && !tree.sessions.is_empty() {
                                        this.active_session = Some(tree.sessions[0].session.name.clone());
                                    } else if let Some(active) = &this.active_session {
                                        if !tree.sessions.iter().any(|s| &s.session.name == active) {
                                            this.active_session = tree.sessions.first().map(|s| s.session.name.clone());
                                        }
                                    }
                                    this.tree = tree;
                                    this.tree_error = None;
                                }
                                Err(e) => {
                                    this.tree_error = Some(e.to_string());
                                }
                            }
                            cx.notify();
                        });
                    }
                });
            }
        }).detach();
    }

    /// Start 1500ms background polling loop.
    pub fn start_polling_loop(&mut self, cx: &mut Context<Self>) {
        cx.spawn(|view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(1500));
                // First tick completes immediately
                interval.tick().await;

                loop {
                    interval.tick().await;
                    let should_continue = cx_handle.update(|cx: &mut App| {
                        if let Some(entity) = view_weak.upgrade() {
                            entity.update(cx, |this, cx| {
                                if let BackendStatus::Ready(_) = &this.backend_status {
                                    this.trigger_poll(cx);
                                    true
                                } else {
                                    false
                                }
                            })
                        } else {
                            false
                        }
                    });

                    if !should_continue {
                        break;
                    }
                }
            }
        }).detach();
    }

    /// Stop any running supervisor and terminate child process.
    pub fn stop_supervisor(&self) {
        let sup_opt = self.supervisor.lock().take();
        if let Some(mut sup) = sup_opt {
            TOKIO_RT.spawn(async move {
                let _ = sup.stop(std::time::Duration::from_millis(500)).await;
            });
        }
    }

    /// Spawn the supervisor lifecycle in a background thread and observe transitions.
    pub fn start_supervisor(&mut self, cx: &mut Context<Self>) {
        // Phase 6 startup: fix any legacy theme/theme_preset skew (Pitfall 2)
        // on the one boot path with a live Context (main.rs stays untouched).
        self.resync_theme_on_startup(cx);
        let spawn_opts = match self.spawn_opts.clone() {
            Some(opts) => opts,
            None => {
                self.backend_status = BackendStatus::Failed {
                    reason: "No backend spawn options configured".to_string(),
                    stderr_tail: "Please build the backend binary: scripts/build-test-backend".to_string(),
                };
                cx.notify();
                return;
            }
        };

        self.backend_status = BackendStatus::Starting;
        cx.notify();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SupervisorEvent>();
        let supervisor_arc = self.supervisor.clone();

        // Run supervisor inside Tokio runtime
        TOKIO_RT.spawn(async move {
            let mut supervisor = Supervisor::new();
            let mut status_rx = supervisor.subscribe();

            let tx_status = tx.clone();
            tokio::spawn(async move {
                while status_rx.changed().await.is_ok() {
                    let st = status_rx.borrow().clone();
                    if tx_status.send(SupervisorEvent::Status(st)).is_err() {
                        break;
                    }
                }
            });

            match supervisor.spawn(spawn_opts).await {
                Ok(info) => {
                    *supervisor_arc.lock() = Some(supervisor);
                    let _ = tx.send(SupervisorEvent::Ready(info));
                }
                Err(e) => {
                    let (reason, stderr_tail) = match e {
                        webtmux_supervisor::SupervisorError::Failed { reason, stderr_tail } => {
                            (reason, stderr_tail)
                        }
                        other => (other.to_string(), String::new()),
                    };
                    *supervisor_arc.lock() = Some(supervisor);
                    let _ = tx.send(SupervisorEvent::Failed { reason, stderr_tail });
                }
            }
        });

        // Observe events inside GPUI foreground executor with weak.upgrade() leak prevention
        cx.spawn(move |view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                while let Some(event) = rx.recv().await {
                    let _ = cx_handle.update(|cx: &mut App| {
                        if let Some(entity) = view_weak.upgrade() {
                            entity.update(cx, |this, cx| {
                                match event {
                                    SupervisorEvent::Status(st) => {
                                        this.backend_status = st;
                                    }
                                    SupervisorEvent::Ready(info) => {
                                        this.base_url = Some(info.base_url.clone());
                                        let client = RestClient::new(&info.base_url);
                                        this.rest_client = Some(client);
                                        this.backend_status = BackendStatus::Ready(info);
                                        this.trigger_poll(cx);
                                        this.start_polling_loop(cx);
                                    }
                                    SupervisorEvent::Failed { reason, stderr_tail } => {
                                        this.backend_status = BackendStatus::Failed {
                                            reason,
                                            stderr_tail,
                                        };
                                    }
                                }
                                cx.notify();
                            });
                        }
                    });
                }
            }
        }).detach();
    }
}

impl AppState {
    // -- Phase 3: per-session tab lifecycle + triple generation guard --------

    /// Open a session as a workspace tab: append to `open_sessions` (if new)
    /// and activate. Pure state only — socket connect is `ensure_session_socket`.
    pub fn open_session(&mut self, name: &str) {
        if !self.sessions.contains_key(name) {
            self.sessions.insert(name.to_string(), OpenSession::default());
        }
        if !self.open_sessions.iter().any(|s| s == name) {
            self.open_sessions.push(name.to_string());
        }
        self.active_session = Some(name.to_string());
    }

    /// Switch the view pointer. Never touches sockets — no connect/close here,
    /// ever (success criterion 1). Callers `cx.notify()` after.
    pub fn set_active_session(&mut self, name: Option<String>) {
        self.active_session = name;
    }

    /// Kill-confirm gate (SESS-05): true opens the confirm dialog, false kills
    /// directly. Reads only `confirm_kill_session` — the pane/window flags
    /// are Phase 5/6 owned (FE `settingsStore.ts` parity).
    pub fn kill_requires_confirm(&self) -> bool {
        self.settings.confirm_kill_session
    }

    /// Close a tab: remove from `open_sessions`, drop the entry (which aborts
    /// its pumps — teardown-on-close, D4), then neighbor activation
    /// `open[min(idx, len-1)]`, `None` when empty. The tmux session itself
    /// keeps running. Killing the last tab falls back through the existing
    /// `workspace_state()` routing (A3).
    pub fn close_session(&mut self, name: &str) {
        if let Some(idx) = self.open_sessions.iter().position(|s| s == name) {
            self.open_sessions.remove(idx);
            self.sessions.remove(name);
            if self.active_session.as_deref() == Some(name) {
                self.active_session = if self.open_sessions.is_empty() {
                    None
                } else {
                    Some(self.open_sessions[idx.min(self.open_sessions.len() - 1)].clone())
                };
            }
        }
    }

    /// Rename re-resolution (pure half): migrate the entry old→new with
    /// `generation + 1`, drop the stale socket handle, move the tab position,
    /// `active_session`, and `expanded_sessions`. In-flight old-generation
    /// events die on guard layer (a). The caller reconnects via
    /// `ensure_session_socket`. Returns false when `old` is unknown, `new`
    /// already exists, or `new` is empty.
    pub fn rename_session_entry(&mut self, old: &str, new: &str) -> bool {
        if new.is_empty() || old == new {
            return false;
        }
        if !self.sessions.contains_key(old) || self.sessions.contains_key(new) {
            return false;
        }
        let mut entry = self.sessions.remove(old).expect("checked above");
        entry.generation += 1;
        entry.handle = None;
        entry.transport = TransportState::Connecting;
        self.sessions.insert(new.to_string(), entry);
        for item in self.open_sessions.iter_mut() {
            if item == old {
                *item = new.to_string();
            }
        }
        if self.active_session.as_deref() == Some(old) {
            self.active_session = Some(new.to_string());
        }
        if self.expanded_sessions.remove(old) {
            self.expanded_sessions.insert(new.to_string());
        }
        true
    }

    /// Triple generation guard apply path (STATE-04). Returns true when the
    /// event was accepted (state committed or classified ignorable), false
    /// when dropped by a guard layer:
    /// (a) generation equality with the current entry,
    /// (b) envelope session match (absent `session` treated as belonging,
    ///     FE `websocket.ts:140-148` parity),
    /// (c) tab-liveness (entry still in the map).
    /// `terminal.snapshot`/`terminal.output` commit into the pane-id-keyed
    /// store (Phase 4, TERM-02/07); malformed terminal frames drop-and-continue
    /// (accepted, committed nowhere — never unwrap in the commit path).
    pub fn apply_event(&mut self, session: &str, generation: u64, msg: &WsOutgoing) -> bool {
        let entry = match self.sessions.get_mut(session) {
            Some(e) => e,
            None => return false, // (c) tab-liveness
        };
        if generation != entry.generation {
            return false; // (a) stale or future generation
        }
        if let Some(tag) = msg.session.as_deref() {
            if tag != session {
                return false; // (b) cross-session leak
            }
        }
        match msg.msg_type.as_str() {
            EV_STATE_SNAPSHOT | EV_STATE_DELTA => {
                let snap = match msg.snapshot.clone() {
                    Some(s) => s,
                    None => return false,
                };
                let was_connected = self
                    .sessions
                    .get(session)
                    .is_some_and(|e| e.transport == TransportState::Connected);
                if let Some(entry) = self.sessions.get_mut(session) {
                    entry.snapshot = Some(snap.clone());
                    entry.transport = TransportState::Connected;
                }
                // D7 attribution + retirement scoped to this session: other
                // sessions' panes are untouched (TERM-07).
                let present: HashSet<String> =
                    snap.panes.iter().map(|p| p.id.clone()).collect();
                for pane in &present {
                    self.attribute_pane(session, pane);
                }
                self.retire_stale_panes(session, &present);
                // D6: transition into Connected re-arms + re-captures this
                // session's registered panes (idempotent via the gate; a
                // fresh first snapshot has no registered panes → no-op).
                if !was_connected {
                    self.recapture_session(session);
                }
                true
            }
            EV_TERMINAL_SNAPSHOT | EV_TERMINAL_OUTPUT => {
                let pane_id = match msg.pane_id.clone() {
                    Some(p) if !p.is_empty() => p,
                    _ => return true, // malformed: drop-and-continue
                };
                let data = msg.data.clone().unwrap_or_default();
                if msg.msg_type == EV_TERMINAL_SNAPSHOT {
                    self.commit_terminal_snapshot(session, &pane_id, &data, msg.screen_rows);
                } else {
                    self.commit_terminal_output(
                        session,
                        &pane_id,
                        &data,
                        msg.replace,
                        msg.screen_rows,
                    );
                }
                true
            }
            EV_CONNECTION_READY => {
                // D6: belt-and-braces with the snapshot-transition path above
                // (gate-safe if it already re-captured).
                self.recapture_session(session);
                true
            }
            EV_TMUX_DISCONNECTED => {
                entry.transport = TransportState::Disconnected;
                true
            }
            EV_TMUX_RECONNECTING => {
                entry.transport = TransportState::Reconnecting;
                true
            }
            EV_SERVER_ERROR => {
                entry.last_error = msg.message.clone();
                true
            }
            _ => true, // connection.ready, command acks, unknown: nothing to commit
        }
    }

    // -- Phase 4: pane-id-keyed TerminalStore + terminal sends --------------

    /// Fresh-or-existing store entry (create-on-demand: terminal frames may
    /// precede the first snapshot on reconnect; D7 retirement keeps it bound).
    pub fn pane_entry(&mut self, pane_id: &str) -> &mut PaneTerminal {
        self.terminals
            .entry(pane_id.to_string())
            .or_insert_with(PaneTerminal::fresh)
    }

    /// Headless grid dump for a pane (contract tests, views).
    pub fn pane_grid_text(&self, pane_id: &str) -> Option<Vec<String>> {
        self.terminals
            .get(pane_id)
            .map(|e| e.terminal.lock().grid_text())
    }

    /// Replay counter for a pane (contract tests).
    pub fn pane_ingested_history(&self, pane_id: &str) -> Option<usize> {
        self.terminals.get(pane_id).map(|e| e.ingested_history)
    }

    /// Last OSC title for a pane (D8).
    pub fn pane_title(&self, pane_id: &str) -> Option<String> {
        self.pane_titles.get(pane_id).cloned()
    }

    /// D3: per-pane TUI-scroll switch — absent falls back to the persisted
    /// `tui_scroll_default` (FE `?? true` parity via the `"JetBrains Mono"`
    /// settings default of `true`; Phase 6 D2/D9).
    pub fn tui_scroll(&self, pane_id: &str) -> bool {
        self.tui_scroll
            .get(pane_id)
            .copied()
            .unwrap_or(self.settings.tui_scroll_default)
    }

    /// Current selection text for a pane (TERM-05 headless lock; the GPUI
    /// clipboard itself is unavailable in tests, so copy/paste helpers assert
    /// on this plus the `terminal.input` byte path instead of `cx.clipboard`).
    pub fn pane_selection_text(&self, pane_id: &str) -> Option<String> {
        self.terminals
            .get(pane_id)
            .and_then(|e| e.terminal.lock().selection_text())
    }

    /// FE-parity wheel emit (TERM-04): decide via `decide_wheel_action` with
    /// the per-pane accumulator, persist the leftover, then emit — SGR/Page
    /// bytes ride `terminal.input` on the owning socket (one message, server
    /// batcher owns chunking), scrollback applies `scroll_display` to the
    /// store terminal. Returns the decided action for headless assertions;
    /// sends no-op without a live socket but the action/accum still update.
    pub fn apply_wheel(
        &mut self,
        pane_id: &str,
        delta: WheelDelta,
        cell_h: f32,
        point: AlacPoint,
        mods: u8,
    ) -> WheelAction {
        let mode = self
            .terminals
            .get(pane_id)
            .map(|e| e.terminal.lock().mode())
            .unwrap_or_else(TermMode::empty);
        let tui_on = self.tui_scroll(pane_id);
        let accum = self.wheel_accum.get(pane_id).copied().unwrap_or(0.0);
        let (action, new_accum) =
            decide_wheel_action(mode, tui_on, delta, accum, cell_h, point, mods);
        self.wheel_accum.insert(pane_id.to_string(), new_accum);
        match &action {
            WheelAction::Sgr(bytes) | WheelAction::Pages(bytes) => {
                if let Ok(data) = String::from_utf8(bytes.clone()) {
                    let _ = self.send_terminal_input(pane_id, data);
                }
            }
            WheelAction::Scrollback(lines) => {
                if *lines != 0 {
                    if let Some(entry) = self.terminals.get(pane_id) {
                        entry.terminal.lock().scroll_display(*lines);
                    }
                }
            }
            WheelAction::Ignored => {}
        }
        action
    }

    // -- Phase 4 plan 04-02 Task 2: debounced resize state machine ---------

    /// Arm a pending viewport from a measured grid size (TERM-06).
    /// Zero-size measures never arm (T-04-05); identical re-arms dedupe.
    /// Returns the debounce sequence when scheduled, `None` when skipped.
    pub fn arm_viewport_for_pane(
        &mut self,
        pane_id: &str,
        cols: usize,
        rows: usize,
    ) -> Option<u64> {
        if cols == 0 || rows == 0 {
            return None;
        }
        if let Some(pending) = &self.pending_viewport {
            if pending.cols == cols && pending.rows == rows {
                return None;
            }
        }
        let generation = self
            .owning_session(pane_id)
            .and_then(|s| self.sessions.get(&s).map(|e| e.generation))
            .unwrap_or(0);
        self.pending_viewport = Some(PendingViewport {
            generation,
            cols,
            rows,
        });
        self.resize_seq += 1;
        Some(self.resize_seq)
    }

    /// Fire the debounced resize when `expected_seq` is still current.
    /// Stale generations drop (second arm supersedes the first); firing
    /// consumes the pending viewport so rapid arms collapse to ONE send.
    /// Recomputes `actualViewport` at fire time from settled sizes with
    /// `cols.max(2)/rows.max(1)` clamping (T-04-05).
    pub fn take_debounced_resize(
        &mut self,
        expected_seq: u64,
    ) -> Option<(String, WsIncoming)> {
        if self.resize_seq != expected_seq {
            return None;
        }
        let pending = self.pending_viewport.take()?;
        self.debounced_envelope_for_pending(pending.cols, pending.rows)
    }

    /// Fire-time viewport computation shared by the timer and headless tests:
    /// scale the settled measured size to the whole window via tmux cell
    /// geometry, else fall back to the settled size clamped.
    fn debounced_envelope_for_pending(
        &self,
        cols: usize,
        rows: usize,
    ) -> Option<(String, WsIncoming)> {
        let session = self.active_session.clone().or_else(|| {
            self.active_window_pane_ids()
                .first()
                .and_then(|p| self.owning_session(p))
        })?;
        let (actual_cols, actual_rows) = self.scaled_viewport(cols, rows, &session);
        let msg = build_terminal_resize(actual_cols, actual_rows);
        Some((session, msg))
    }

    /// Scale a settled measured size through the session snapshot geometry.
    fn scaled_viewport(&self, cols: usize, rows: usize, session: &str) -> (usize, usize) {
        let entry = match self.sessions.get(session) {
            Some(e) => e,
            None => return (cols.max(2), rows.max(1)),
        };
        let snap = match entry.snapshot.as_ref() {
            Some(s) => s,
            None => return (cols.max(2), rows.max(1)),
        };
        let panes: Vec<_> = snap
            .panes
            .iter()
            .filter(|p| p.window_id == snap.active_window)
            .collect();
        if panes.is_empty() {
            return (cols.max(2), rows.max(1));
        }
        let visible: Vec<_> = match panes.iter().find(|p| p.zoomed) {
            Some(z) => vec![*z],
            None => panes,
        };
        let first = match visible.first() {
            Some(p) => *p,
            None => return (cols.max(2), rows.max(1)),
        };
        let window_obj = snap.windows.iter().find(|w| w.id == snap.active_window);
        let ww = window_obj.map(|w| w.width).unwrap_or(first.width);
        let wh = window_obj.map(|w| w.height).unwrap_or(first.height);
        actual_viewport(cols, rows, first.width, first.height, ww, wh, 0.0, 0.0)
    }

    /// Fire-and-forget window-level `terminal.resize` on a session's own
    /// socket (clamped; never from paint — Pitfall 4; never `hello` — D4).
    pub fn send_terminal_resize(&self, session: &str, cols: usize, rows: usize) -> bool {
        let msg = build_terminal_resize(cols, rows);
        match self.sessions.get(session).and_then(|e| e.handle.as_ref()) {
            Some(handle) => handle.send_command(msg).is_ok(),
            None => false,
        }
    }

    /// 100ms generation-tagged debounce sender: recomputes at fire time from
    /// settled sizes and sends exactly one clamped `terminal.resize`.
    /// Timer bodies are manual-UAT class (like 02-02/03-02 visual checks);
    /// headless tests cover arm/collapse/clamp via `take_debounced_resize`.
    pub fn schedule_debounced_resize(&mut self, cx: &mut Context<Self>) {
        let seq = self.resize_seq;
        cx.spawn(move |view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let _ = cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, _cx| {
                            if let Some((session, msg)) = this.take_debounced_resize(seq) {
                                if let Some(handle) = this
                                    .sessions
                                    .get(&session)
                                    .and_then(|e| e.handle.as_ref())
                                {
                                    let _ = handle.send_command(msg);
                                }
                            }
                        });
                    }
                });
            }
        })
        .detach();
    }

    /// Current layout key for the active snapshot (FE `layoutKey` parity).
    /// Empty when there is no active window/panes (caller skips resync).
    pub fn current_layout_key(&self) -> String {
        let name = match self.active_session.as_deref() {
            Some(n) => n,
            None => return String::new(),
        };
        let snap = match self.sessions.get(name).and_then(|e| e.snapshot.as_ref()) {
            Some(s) => s,
            None => return String::new(),
        };
        let panes: Vec<_> = snap
            .panes
            .iter()
            .filter(|p| p.window_id == snap.active_window)
            .collect();
        if panes.is_empty() {
            return String::new();
        }
        let visible: Vec<_> = match panes.iter().find(|p| p.zoomed) {
            Some(z) => vec![*z],
            None => panes,
        };
        let window_obj = snap.windows.iter().find(|w| w.id == snap.active_window);
        let ww = window_obj
            .map(|w| w.width)
            .unwrap_or_else(|| visible[0].width);
        let wh = window_obj
            .map(|w| w.height)
            .unwrap_or_else(|| visible[0].height);
        let layout = window_obj.map(|w| w.layout.as_str()).unwrap_or("");
        let rows: Vec<(String, usize, usize, usize, usize, bool)> = visible
            .iter()
            .map(|p| {
                (
                    p.id.clone(),
                    p.left,
                    p.top,
                    p.width,
                    p.height,
                    p.zoomed,
                )
            })
            .collect();
        compute_layout_key(&snap.active_window, ww, wh, layout, &rows)
    }

    /// Layout-key tracker: first mount records and skips, identical keys
    /// schedule nothing, changes record and request the 150/325ms pair.
    pub fn decide_layout_resync(&mut self, new_key: String) -> LayoutResync {
        if new_key.is_empty() {
            return LayoutResync::NoChange;
        }
        match &self.last_layout_key {
            None => {
                self.last_layout_key = Some(new_key);
                LayoutResync::FirstMountSkip
            }
            Some(prev) if *prev == new_key => LayoutResync::NoChange,
            _ => {
                self.last_layout_key = Some(new_key);
                LayoutResync::Resync
            }
        }
    }

    /// Visible panes with a registered store entry — the 325ms capture
    /// targets (FE `terminalRegistry.has` parity; unregistered excluded).
    pub fn layout_resync_panes(&self, visible: &[String]) -> Vec<String> {
        visible
            .iter()
            .filter(|p| self.terminals.contains_key(p.as_str()))
            .cloned()
            .collect()
    }

    /// Observe the current layout key and, on change (and not first mount),
    /// schedule the 150ms resize + 325ms invalidate-and-recapture pair per
    /// visible registered pane. Called from the workspace render path with a
    /// live `Context` (manual-UAT timing; decision logic stays headless).
    pub fn observe_layout_key_and_schedule(&mut self, cx: &mut Context<Self>) {
        let key = self.current_layout_key();
        if key.is_empty() {
            return;
        }
        if self.decide_layout_resync(key) != LayoutResync::Resync {
            return;
        }
        cx.spawn(move |view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                let _ = cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, _cx| {
                            if let Some(pending) = this.pending_viewport {
                                let (cols, rows) = this.scaled_viewport(
                                    pending.cols,
                                    pending.rows,
                                    &this.active_session.clone().unwrap_or_default(),
                                );
                                if let Some(session) = this.active_session.clone() {
                                    let _ = this.send_terminal_resize(&session, cols, rows);
                                }
                            } else if let Some(session) = this.active_session.clone() {
                                // No settled measure yet: still report the scaled
                                // snapshot geometry so tmux learns the new layout.
                                let panes = this.active_window_pane_ids();
                                let _ = panes;
                                let _ = this.send_terminal_resize(&session, 80, 24);
                            }
                        });
                    }
                });
                tokio::time::sleep(std::time::Duration::from_millis(175)).await;
                let _ = cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, _cx| {
                            let visible = this.active_window_pane_ids();
                            let targets = this.layout_resync_panes(&visible);
                            for pane in &targets {
                                this.invalidate_pane_snapshot(pane);
                            }
                            for pane in &targets {
                                let _ = this.request_pane_capture(pane);
                            }
                        });
                    }
                });
            }
        })
        .detach();
    }

    /// Attribute pane → session (D7). When attribution CHANGES sessions the
    /// entry resets fresh: tmux may reuse a `%N`, and a stale grid must never
    /// replay under a new owner.
    fn attribute_pane(&mut self, session: &str, pane_id: &str) {
        let changed = self
            .pane_session
            .get(pane_id)
            .is_some_and(|s| s != session);
        self.pane_session
            .insert(pane_id.to_string(), session.to_string());
        if changed {
            self.terminals
                .insert(pane_id.to_string(), PaneTerminal::fresh());
        }
    }

    /// Resolve the owning session for a pane: explicit attribution first, then
    /// the latest committed snapshots (Phase-3 D5 lesson — never the active
    /// proxy). `None` is a typed miss: the caller sends nothing.
    pub fn owning_session(&self, pane_id: &str) -> Option<String> {
        if let Some(s) = self.pane_session.get(pane_id) {
            return Some(s.clone());
        }
        for (name, entry) in &self.sessions {
            if let Some(snap) = entry.snapshot.as_ref() {
                if snap.panes.iter().any(|p| p.id == pane_id) {
                    return Some(name.clone());
                }
            }
        }
        None
    }

    /// Commit a `terminal.snapshot` frame: exactly-once gate + apply_capture.
    /// Synchronous on the GPUI thread in pump order (frame atomicity, no
    /// queue). Returns true when applied, false when dropped by the gate.
    pub fn commit_terminal_snapshot(
        &mut self,
        session: &str,
        pane_id: &str,
        data: &str,
        screen_rows: Option<i32>,
    ) -> bool {
        self.attribute_pane(session, pane_id);
        let entry = self.pane_entry(pane_id);
        if entry.snapshot_written {
            return false; // exactly-once: remount re-requests die here
        }
        entry.snapshot_written = true;
        let feed = apply_capture(data, screen_rows, &mut entry.ingested_history);
        entry.terminal.lock().process_bytes(&feed);
        self.drain_pane_events(pane_id);
        true
    }

    /// Commit a `terminal.output` frame: `replace=true` runs the capture path
    /// WITHOUT the gate (FE `replaceScreen` is not idempotent-gated);
    /// `replace=false` feeds raw bytes straight to the Processor.
    pub fn commit_terminal_output(
        &mut self,
        session: &str,
        pane_id: &str,
        data: &str,
        replace: bool,
        screen_rows: Option<i32>,
    ) -> bool {
        self.attribute_pane(session, pane_id);
        if replace {
            let entry = self.pane_entry(pane_id);
            let feed = apply_capture(data, screen_rows, &mut entry.ingested_history);
            entry.terminal.lock().process_bytes(&feed);
        } else {
            self.pane_entry(pane_id)
                .terminal
                .lock()
                .process_bytes(data.as_bytes());
        }
        self.drain_pane_events(pane_id);
        true
    }

    /// Drain alacritty events for a pane: `Title` commits the last-title map
    /// (D8, feeds Phase-5 headers), `Bell` is a no-op.
    fn drain_pane_events(&mut self, pane_id: &str) {
        let events = match self.terminals.get(pane_id) {
            Some(e) => e.terminal.lock().drain_events(),
            None => return,
        };
        for ev in events {
            if let TerminalEvent::Title(title) = ev {
                self.pane_titles.insert(pane_id.to_string(), title);
            }
        }
    }

    /// Re-arm the exactly-once gate so the next snapshot replaces (layout-key
    /// resync / reconnect / manual resync).
    pub fn invalidate_pane_snapshot(&mut self, pane_id: &str) {
        if let Some(e) = self.terminals.get_mut(pane_id) {
            e.snapshot_written = false;
        }
    }

    /// D7: retire entries attributed to `session` but absent from its latest
    /// panes (counters and titles drop too).
    pub fn retire_stale_panes(&mut self, session: &str, present: &HashSet<String>) {
        let stale: Vec<String> = self
            .pane_session
            .iter()
            .filter(|(pane, sess)| sess.as_str() == session && !present.contains(pane.as_str()))
            .map(|(pane, _)| pane.clone())
            .collect();
        for pane in stale {
            self.terminals.remove(&pane);
            self.pane_session.remove(&pane);
            self.pane_titles.remove(&pane);
        }
    }

    // -- Phase 4 task 3: active-window pane helpers + view retention --------

    /// Pane IDs of the ACTIVE window of the ACTIVE session (tracer layout;
    /// Phase 5 derives geometry from the same snapshot).
    pub fn active_window_pane_ids(&self) -> Vec<String> {
        let name = match self.active_session.as_deref() {
            Some(n) => n,
            None => return Vec::new(),
        };
        let snap = match self.sessions.get(name).and_then(|e| e.snapshot.as_ref()) {
            Some(s) => s,
            None => return Vec::new(),
        };
        snap.panes
            .iter()
            .filter(|p| p.window_id == snap.active_window)
            .map(|p| p.id.clone())
            .collect()
    }

    /// Drop retained view entities with no live pane behind them (retired or
    /// vanished from every snapshot). Store entries ahead of any snapshot
    /// keep their views (capture in flight).
    pub fn prune_terminal_views(&mut self) {
        let mut live = HashSet::new();
        for entry in self.sessions.values() {
            if let Some(snap) = entry.snapshot.as_ref() {
                live.extend(snap.panes.iter().map(|p| p.id.clone()));
            }
        }
        live.extend(self.terminals.keys().cloned());
        self.terminal_views.retain(|pane, _| live.contains(pane));
    }

    /// Pure constructor for `terminal.input`: owning-socket resolve + envelope.
    /// The payload rides `String` (exact UTF-8 round-trip, locked by the
    /// task-1 contract test — never lossy).
    pub fn build_terminal_input(
        &self,
        pane_id: &str,
        data: String,
    ) -> Option<(String, WsIncoming)> {
        let session = self.owning_session(pane_id)?;
        let msg = WsIncoming {
            msg_type: MSG_TERMINAL_INPUT.to_string(),
            pane_id: Some(pane_id.to_string()),
            data: Some(data),
            ..Default::default()
        };
        Some((session, msg))
    }

    /// Fire-and-forget `terminal.input` on the OWNING session's socket (the
    /// receiver is dropped — uncorrelated by server design, same shape as
    /// `send_window_select`). False on miss or dead socket; never panics.
    /// No `hello` is sent anywhere (D4).
    pub fn send_terminal_input(&self, pane_id: &str, data: String) -> bool {
        let (session, msg) = match self.build_terminal_input(pane_id, data) {
            Some(v) => v,
            None => return false,
        };
        match self.sessions.get(&session).and_then(|e| e.handle.as_ref()) {
            Some(handle) => handle.send_command(msg).is_ok(),
            None => false,
        }
    }

    /// Pure constructor for `terminal.capture` (initial + resync captures).
    pub fn build_terminal_capture(&self, pane_id: &str) -> Option<(String, WsIncoming)> {
        let session = self.owning_session(pane_id)?;
        let msg = WsIncoming {
            msg_type: MSG_TERMINAL_CAPTURE.to_string(),
            pane_id: Some(pane_id.to_string()),
            ..Default::default()
        };
        Some((session, msg))
    }

    /// Fire-and-forget `terminal.capture` on the owning socket (FE
    /// initial-capture parity). False on miss or dead socket.
    pub fn request_pane_capture(&self, pane_id: &str) -> bool {
        let (session, msg) = match self.build_terminal_capture(pane_id) {
            Some(v) => v,
            None => return false,
        };
        match self.sessions.get(&session).and_then(|e| e.handle.as_ref()) {
            Some(handle) => handle.send_command(msg).is_ok(),
            None => false,
        }
    }

    /// D6: re-arm + re-capture every registered pane of `session` (reconnect).
    /// Invalidation always runs (headless-testable); sends no-op without
    /// live sockets. Idempotent and cheap.
    pub fn recapture_session(&mut self, session: &str) {
        let panes: Vec<String> = self
            .pane_session
            .iter()
            .filter(|(_, s)| s.as_str() == session)
            .map(|(p, _)| p.clone())
            .collect();
        for pane in &panes {
            self.invalidate_pane_snapshot(pane);
        }
        for pane in &panes {
            let _ = self.request_pane_capture(pane);
        }
    }

    /// Title-bar window-tab model for the active session (SHELL-01): windows
    /// ordered by index, activeness from `active_window`. Empty vec when there
    /// is no active session, no entry, or no snapshot — never panics.
    pub fn window_tabs(&self) -> Vec<WindowTab> {
        let name = match self.active_session.as_deref() {
            Some(n) => n,
            None => return Vec::new(),
        };
        let snapshot = match self.sessions.get(name).and_then(|e| e.snapshot.as_ref()) {
            Some(s) => s,
            None => return Vec::new(),
        };
        let mut windows = snapshot.windows.clone();
        windows.sort_by_key(|w| w.index);
        windows
            .into_iter()
            .map(|w| {
                let active = w.id == snapshot.active_window;
                WindowTab {
                    id: w.id,
                    index: w.index,
                    name: w.name,
                    active,
                }
            })
            .collect()
    }

    /// Fire-and-forget `window.select` on the active session's socket
    /// (SHELL-01, FE `WindowTabs.tsx:120` parity): correlated send, no await,
    /// no optimistic flip — state follows via delta. No-op when there is no
    /// active session or its socket is not connected. The dropped receiver's
    /// pending entry is cleaned when the server's `command.success` arrives.
    pub fn send_window_select(&self, window_id: &str) {
        let Some(active) = self.active_session.as_deref() else {
            return;
        };
        let Some(handle) = self.sessions.get(active).and_then(|e| e.handle.as_ref()) else {
            return;
        };
        let msg = WsIncoming {
            msg_type: MSG_WINDOW_SELECT.to_string(),
            pane_id: Some(window_id.to_string()),
            ..Default::default()
        };
        let _ = handle.send_command(msg);
    }

    // -- Phase 5 plan 05-02 Task 2: swap scope, rename lookups, toolbar -----

    /// Swap-picker row label (FE `PaneContextMenu.tsx:166` parity):
    /// `currentCommand || title || currentPath`. Note the order differs
    /// deliberately from the rename prefill (`title || current_command`).
    pub fn swap_candidate_label(
        current_command: &str,
        title: &str,
        current_path: &str,
    ) -> String {
        if !current_command.is_empty() {
            current_command.to_string()
        } else if !title.is_empty() {
            title.to_string()
        } else {
            current_path.to_string()
        }
    }

    /// Same-window swap candidates for `pane_id` (PANE-05 per D7, pitfall 6):
    /// panes of the active session's snapshot sharing the pane's window,
    /// excluding the pane itself. Empty when the pane is unknown, alone in
    /// its window, or no snapshot is committed — the Swap entry disables.
    pub fn swap_candidates(&self, pane_id: &str) -> Vec<SwapCandidate> {
        let name = match self.active_session.as_deref() {
            Some(n) => n,
            None => return Vec::new(),
        };
        let snap = match self.sessions.get(name).and_then(|e| e.snapshot.as_ref()) {
            Some(s) => s,
            None => return Vec::new(),
        };
        let window_id = match snap.panes.iter().find(|p| p.id == pane_id) {
            Some(p) => p.window_id.clone(),
            None => return Vec::new(),
        };
        snap.panes
            .iter()
            .filter(|p| p.window_id == window_id && p.id != pane_id)
            .map(|p| SwapCandidate {
                id: p.id.clone(),
                label: Self::swap_candidate_label(
                    &p.current_command,
                    &p.title,
                    &p.current_path,
                ),
            })
            .collect()
    }

    /// `(title, current_command)` prefill sources for the Rename Pane dialog
    /// (DLG-02 per D6: `title || current_command || ''`). Searches every
    /// committed snapshot; `None` when the pane is unknown.
    pub fn pane_for_rename(&self, pane_id: &str) -> Option<(String, String)> {
        for entry in self.sessions.values() {
            if let Some(snap) = entry.snapshot.as_ref() {
                if let Some(p) = snap.panes.iter().find(|p| p.id == pane_id) {
                    return Some((p.title.clone(), p.current_command.clone()));
                }
            }
        }
        None
    }

    /// Window name prefill for the Rename Window dialog (DLG-02 per D6:
    /// `w.name`). `None` when the window is unknown.
    pub fn window_name(&self, window_id: &str) -> Option<String> {
        for entry in self.sessions.values() {
            if let Some(snap) = entry.snapshot.as_ref() {
                if let Some(w) = snap.windows.iter().find(|w| w.id == window_id) {
                    return Some(w.name.clone());
                }
            }
        }
        None
    }

    /// Active window id of the active session (`@N`), the `paneId` target for
    /// every `window.*` command (no `windowId` field exists server-side).
    pub fn active_window_id(&self) -> Option<String> {
        let name = self.active_session.as_deref()?;
        let snap = self.sessions.get(name)?.snapshot.as_ref()?;
        Some(snap.active_window.clone())
    }

    // -- Phase 5 Task 2 (05-01): pane/window correlated sends + kill gates (D5/D7) --

    /// Kill-confirm gate for panes (PANE-05 per D7): true opens the confirm
    /// dialog, false kills directly. Reads only `confirm_kill_pane`.
    pub fn kill_requires_confirm_pane(&self) -> bool {
        self.settings.confirm_kill_pane
    }

    /// Kill-confirm gate for windows (PANE-08 per D7). Reads only
    /// `confirm_kill_window`.
    pub fn kill_requires_confirm_window(&self) -> bool {
        self.settings.confirm_kill_window
    }

    /// Split-direction vocabulary (PANE-02 pitfall lock): Split right →
    /// `"horizontal"` (tmux `-h`), Split down → `"vertical"` (tmux `-v`).
    /// The flag names the new layout axis, not the divider.
    pub fn pane_split_direction(split_right: bool) -> &'static str {
        if split_right {
            "horizontal"
        } else {
            "vertical"
        }
    }

    /// Pane rename prefill (DLG-02 per D6): `title || current_command || ''`.
    pub fn pane_rename_prefill(title: &str, current_command: &str) -> String {
        if !title.is_empty() {
            title.to_string()
        } else if !current_command.is_empty() {
            current_command.to_string()
        } else {
            String::new()
        }
    }

    /// Window rename prefill (DLG-02 per D6): `name`.
    pub fn window_rename_prefill(name: &str) -> String {
        name.to_string()
    }

    /// Rename non-empty gate (DLG-02 per D6): trim + non-empty only — tmux
    /// titles accept anything, so no session-name validation applies.
    pub fn rename_name_allowed(name: &str) -> bool {
        !name.trim().is_empty()
    }

    /// Per-pane TUI-scroll override writer (PH1 header switch). Absent reads
    /// the persisted `tui_scroll_default` via `tui_scroll` (Phase 6 D2/D9).
    pub fn set_tui_scroll(&mut self, pane_id: &str, enabled: bool) {
        self.tui_scroll.insert(pane_id.to_string(), enabled);
    }

    // -- Phase 6 tracer: theme live-apply + terminal pref apply (D3/D9) ----

    /// Terminal palette for a UI preset name through its linked terminal
    /// preset (FE `resolvedTerminalTheme` parity — the legacy explicit
    /// override stays ignored; one theme for the whole app). Pure: lookup +
    /// `ColorPalette::from_rgb_u32`, no socket, no cx.
    pub fn terminal_palette_for_ui_preset(preset_name: &str) -> ColorPalette {
        let term = crate::themes_generated::terminal_preset_for_ui(preset_name);
        ColorPalette::from_rgb_u32(
            term.foreground,
            term.background,
            term.cursor,
            term.foreground,
            [
                term.black,
                term.red,
                term.green,
                term.yellow,
                term.blue,
                term.magenta,
                term.cyan,
                term.white,
                term.bright_black,
                term.bright_red,
                term.bright_green,
                term.bright_yellow,
                term.bright_blue,
                term.bright_magenta,
                term.bright_cyan,
                term.bright_white,
            ],
        )
    }

    /// Palette for the currently persisted preset (live-apply + new-view
    /// default source — future views resolve the same preset via settings).
    pub fn current_terminal_palette(&self) -> ColorPalette {
        Self::terminal_palette_for_ui_preset(&self.settings.theme_preset)
    }

    /// Single mutation point for theme picks (D3): table lookup (fallback
    /// `[0]`, `getUiTheme` parity), persist the preset, derive Dark/Light
    /// from `is_dark` (FE `isLightUiTheme` parity), apply the widget theme,
    /// push the linked palette to every live `terminal_views` entry (future
    /// views resolve the persisted preset as their default), synchronous
    /// `save()`, then `notify()`.
    pub fn set_theme_preset(&mut self, preset_id: &str, cx: &mut Context<Self>) {
        let preset = crate::themes_generated::ui_preset_by_name(preset_id);
        let name = preset.name.to_string();
        let is_dark = preset.is_dark;
        self.settings.theme_preset = name.clone();
        self.settings.theme = if is_dark {
            SettingsTheme::Dark
        } else {
            SettingsTheme::Light
        };
        crate::theme::apply_theme(self.settings.theme, cx);
        let palette = Self::terminal_palette_for_ui_preset(&name);
        for view in self.terminal_views.values() {
            view.update(cx, |v, cx| v.set_palette(palette.clone(), cx));
        }
        let _ = self.settings.save();
        cx.notify();
    }

    /// Load-time re-sync (Pitfall 2): re-derive `settings.theme` from the
    /// preset `is_dark`. Returns true when a legacy skew was corrected.
    pub fn resync_theme_from_preset(&mut self) -> bool {
        let preset =
            crate::themes_generated::ui_preset_by_name(&self.settings.theme_preset);
        let want = if preset.is_dark {
            SettingsTheme::Dark
        } else {
            SettingsTheme::Light
        };
        if self.settings.theme != want {
            self.settings.theme = want;
            true
        } else {
            false
        }
    }

    /// Startup re-sync with a live context (Pitfall 2): fix a legacy
    /// theme/theme_preset skew, persist the fix, re-apply the widget theme.
    /// Called once from `start_supervisor` — the one boot path with a live
    /// `Context` (main.rs itself is plan-external and stays untouched).
    pub fn resync_theme_on_startup(&mut self, cx: &mut Context<Self>) {
        if self.resync_theme_from_preset() {
            let _ = self.settings.save();
        }
        crate::theme::apply_theme(self.settings.theme, cx);
    }

    /// Appearance filter writer (D4): `all`/`dark`/`light` only; persists +
    /// notifies so the card grid re-filters on the next render.
    pub fn set_theme_mode_filter(&mut self, filter: &str, cx: &mut Context<Self>) {
        if !matches!(filter, "all" | "dark" | "light") {
            return;
        }
        if self.settings.theme_mode_filter == filter {
            return;
        }
        self.settings.theme_mode_filter = filter.to_string();
        let _ = self.settings.save();
        cx.notify();
    }

    /// Re-arm the debounced viewport for every pane with a live view using
    /// the store terminal's current grid size, then schedule the single
    /// debounced send (Pitfall 4 — never font-apply without viewport re-arm
    /// or tmux keeps formatting for the old viewport).
    fn rearm_viewports_for_font_change(&mut self, cx: &mut Context<Self>) {
        let panes: Vec<String> = self.terminal_views.keys().cloned().collect();
        for pane in &panes {
            let (cols, rows) = self
                .terminals
                .get(pane)
                .map(|e| {
                    let t = e.terminal.lock();
                    (t.cols(), t.rows())
                })
                .unwrap_or((80, 24));
            self.arm_viewport_for_pane(pane, cols, rows);
        }
        self.schedule_debounced_resize(cx);
    }

    /// Font-size apply (D9): clamp 8–32, persist, push via `set_font_size`
    /// to all live views, re-arm the viewport, notify.
    pub fn set_terminal_font_size(&mut self, size: f32, cx: &mut Context<Self>) {
        let size = clamp_font_size(size);
        self.settings.font_size = size;
        let _ = self.settings.save();
        let px_size = px(size);
        for view in self.terminal_views.values() {
            view.update(cx, |v, cx| v.set_font_size(px_size, cx));
        }
        self.rearm_viewports_for_font_change(cx);
        cx.notify();
    }

    /// Font-family apply (D9): honest single family (default JetBrains Mono;
    /// free text falls back — D8). Persists, pushes via `set_font` with the
    /// current size to all live views, re-arms the viewport.
    pub fn set_terminal_font_family(&mut self, family: &str, cx: &mut Context<Self>) {
        let family = family.trim();
        if family.is_empty() {
            return;
        }
        self.settings.font_family = family.to_string();
        let _ = self.settings.save();
        let fam = family.to_string();
        let px_size = px(self.settings.font_size);
        for view in self.terminal_views.values() {
            let f = fam.clone();
            view.update(cx, |v, cx| v.set_font(f, px_size, cx));
        }
        self.rearm_viewports_for_font_change(cx);
        cx.notify();
    }

    /// Line-height apply (D9): clamp 1–2, persist, update live renderers'
    /// multiplier + cell height in place, re-arm the viewport, notify.
    pub fn set_terminal_line_height(&mut self, line_height: f32, cx: &mut Context<Self>) {
        let lh = clamp_line_height(line_height);
        self.settings.line_height = lh;
        let _ = self.settings.save();
        for view in self.terminal_views.values() {
            view.update(cx, |v, cx| {
                let r = v.renderer_mut();
                r.line_height_multiplier = lh;
                r.cell_height = r.font_size * lh;
                cx.notify();
            });
        }
        self.rearm_viewports_for_font_change(cx);
        cx.notify();
    }

    /// Scrollback apply (D9): clamp 100–50000, persist, recreate each store
    /// `Terminal` IN PLACE (same `Arc`, so live views follow) preserving
    /// grid size, reset the exactly-once guards, then `invalidate` +
    /// `request_pane_capture` per pane (Pitfall 3 — never recreate without
    /// re-capture; the `ingestedHistory` guard dedupes the replay).
    pub fn set_scrollback_lines(&mut self, lines: usize, cx: &mut Context<Self>) {
        let lines = clamp_scrollback(lines);
        self.settings.scrollback_lines = lines;
        let _ = self.settings.save();
        let cfg = TerminalConfig {
            scrollback_limit: lines,
        };
        let panes: Vec<String> = self.terminals.keys().cloned().collect();
        for pane in &panes {
            if let Some(entry) = self.terminals.get_mut(pane) {
                let (cols, rows) = {
                    let t = entry.terminal.lock();
                    (t.cols(), t.rows())
                };
                *entry.terminal.lock() =
                    Terminal::with_config(cols, rows, cfg.clone());
                entry.snapshot_written = false;
                entry.ingested_history = 0;
            }
        }
        for pane in &panes {
            self.invalidate_pane_snapshot(pane);
        }
        for pane in &panes {
            let _ = self.request_pane_capture(pane);
        }
        cx.notify();
    }

    /// TUI-scroll-default writer (D9): persists + notifies; per-pane
    /// overrides keep precedence through `tui_scroll()`.
    pub fn set_tui_scroll_default(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.tui_scroll_default = enabled;
        let _ = self.settings.save();
        cx.notify();
    }

    // -- Pure envelope constructors (headless-testable, no socket) ---------

    pub fn build_pane_select(pane_id: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_PANE_SELECT.to_string(),
            pane_id: Some(pane_id.to_string()),
            ..Default::default()
        }
    }

    pub fn build_pane_split(pane_id: &str, direction: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_PANE_SPLIT.to_string(),
            pane_id: Some(pane_id.to_string()),
            direction: Some(direction.to_string()),
            ..Default::default()
        }
    }

    pub fn build_pane_resize(pane_id: &str, direction: char, amount: i32) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_PANE_RESIZE.to_string(),
            pane_id: Some(pane_id.to_string()),
            direction: Some(direction.to_string()),
            amount: Some(amount),
            ..Default::default()
        }
    }

    pub fn build_pane_kill(pane_id: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_PANE_KILL.to_string(),
            pane_id: Some(pane_id.to_string()),
            ..Default::default()
        }
    }

    pub fn build_pane_rename(pane_id: &str, title: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_PANE_RENAME.to_string(),
            pane_id: Some(pane_id.to_string()),
            title: Some(title.to_string()),
            ..Default::default()
        }
    }

    pub fn build_pane_zoom(pane_id: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_PANE_ZOOM.to_string(),
            pane_id: Some(pane_id.to_string()),
            ..Default::default()
        }
    }

    pub fn build_pane_break(pane_id: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_PANE_BREAK.to_string(),
            pane_id: Some(pane_id.to_string()),
            ..Default::default()
        }
    }

    pub fn build_pane_swap(pane_id: &str, other_pane_id: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_PANE_SWAP.to_string(),
            pane_id: Some(pane_id.to_string()),
            other_pane_id: Some(other_pane_id.to_string()),
            ..Default::default()
        }
    }

    pub fn build_window_select(window_id: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_WINDOW_SELECT.to_string(),
            pane_id: Some(window_id.to_string()),
            ..Default::default()
        }
    }

    pub fn build_window_create(name: Option<&str>) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_WINDOW_CREATE.to_string(),
            name: name.map(|s| s.to_string()),
            ..Default::default()
        }
    }

    pub fn build_window_rename(window_id: &str, name: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_WINDOW_RENAME.to_string(),
            pane_id: Some(window_id.to_string()),
            name: Some(name.to_string()),
            ..Default::default()
        }
    }

    pub fn build_window_kill(window_id: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_WINDOW_KILL.to_string(),
            pane_id: Some(window_id.to_string()),
            ..Default::default()
        }
    }

    pub fn build_window_layout(window_id: &str, layout: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_WINDOW_LAYOUT.to_string(),
            pane_id: Some(window_id.to_string()),
            layout: Some(layout.to_string()),
            ..Default::default()
        }
    }

    pub fn build_window_move(window_id: &str, offset: i32) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_WINDOW_MOVE.to_string(),
            pane_id: Some(window_id.to_string()),
            amount: Some(offset),
            ..Default::default()
        }
    }

    pub fn build_window_break_active(window_id: &str) -> WsIncoming {
        WsIncoming {
            msg_type: MSG_WINDOW_BREAK_ACTIVE.to_string(),
            pane_id: Some(window_id.to_string()),
            ..Default::default()
        }
    }

    // -- Fire-and-forget selects + drag steps (D5: receiver dropped) --------

    /// Fire-and-forget `pane.select` on the OWNING session's socket (never
    /// the active proxy). False on miss or dead socket; never panics.
    pub fn send_pane_select(&self, pane_id: &str) -> bool {
        let Some(session) = self.owning_session(pane_id) else {
            return false;
        };
        let Some(handle) = self.sessions.get(&session).and_then(|e| e.handle.as_ref()) else {
            return false;
        };
        let _ = handle.send_command(Self::build_pane_select(pane_id));
        true
    }

    /// Fire-and-forget `pane.resize` step for divider drags (D3): the
    /// receiver is dropped — no per-step await/task spam; the pending entry
    /// cleans itself on reply and truth follows via snapshot deltas. Never
    /// sends `terminal.resize` (Phase-4 layout-key timers own viewport
    /// resync) and never sends cumulative displacement.
    pub fn submit_pane_resize(&self, pane_id: &str, direction: char, amount: i32) -> bool {
        if amount == 0 {
            return false;
        }
        let Some(session) = self.owning_session(pane_id) else {
            return false;
        };
        let Some(handle) = self.sessions.get(&session).and_then(|e| e.handle.as_ref()) else {
            return false;
        };
        let _ = handle.send_command(Self::build_pane_resize(pane_id, direction, amount));
        true
    }

    // -- Phase 5 plan 05-02 Task 1: divider-drag state machine (D3) ---------

    /// Record a divider `mouse_down`: `(pane_id, direction, start_px)` with
    /// `last_cells = 0` and `last_sent = now`, so the first move inside the
    /// 40ms window throttles (FE `lastSent: 0` parity is intentionally NOT
    /// copied — the plan's throttle contract requires the opening window to
    /// hold). Zero/negative `cell_px` never arms (T-05-02 zero-size guard).
    pub fn begin_pane_drag(
        &mut self,
        pane_id: &str,
        direction: char,
        start_px: f32,
        cell_px: f32,
    ) {
        self.begin_pane_drag_at(pane_id, direction, start_px, cell_px, Instant::now());
    }

    /// Testable half of `begin_pane_drag` with an explicit clock.
    pub fn begin_pane_drag_at(
        &mut self,
        pane_id: &str,
        direction: char,
        start_px: f32,
        cell_px: f32,
        now: Instant,
    ) {
        if cell_px <= 0.0 {
            return;
        }
        self.pane_drag = Some(PaneDragState {
            pane_id: pane_id.to_string(),
            direction,
            start_px,
            cell_px,
            last_cells: 0,
            last_sent: now,
        });
    }

    /// Poll the in-progress drag at `pos_px` on the drag axis: applies the
    /// `step == 0` / 40ms-throttle / FLIP rules and advances `last_cells` +
    /// `last_sent` only when a step fires. Returns `(pane_id, direction,
    /// amount)` for the caller to send. `None` when no drag is active or the
    /// move throttles.
    pub fn poll_pane_drag(&mut self, now: Instant, pos_px: f32) -> Option<(String, char, i32)> {
        let elapsed_ms = {
            let drag = self.pane_drag.as_ref()?;
            now.saturating_duration_since(drag.last_sent).as_millis() as u64
        };
        let (direction, start_px, last_cells, cell_px) = {
            let drag = self.pane_drag.as_ref()?;
            (
                drag.direction,
                drag.start_px,
                drag.last_cells,
                drag.cell_px,
            )
        };
        let (send_dir, amount, new_last) =
            drag_step_throttled(direction, pos_px, start_px, last_cells, cell_px, elapsed_ms)?;
        let drag = self.pane_drag.as_mut()?;
        drag.last_cells = new_last;
        drag.last_sent = now;
        Some((drag.pane_id.clone(), send_dir, amount))
    }

    /// Stream one grid-level `mouse_move` through the drag: polls on the
    /// drag's own axis and fires the step via `submit_pane_resize`
    /// (fire-and-forget, receiver dropped — no per-step await). Returns true
    /// when a step was sent. Never sends `terminal.resize` — the Phase-4
    /// layout-key timers (150/325ms, self-debouncing mid-drag) own viewport
    /// resync, and drag steps never arm them directly.
    pub fn move_pane_drag(&mut self, pos_px: f32) -> bool {
        let Some((pane_id, dir, amt)) = self.poll_pane_drag(Instant::now(), pos_px) else {
            return false;
        };
        self.submit_pane_resize(&pane_id, dir, amt)
    }

    /// Stream a grid-level move given in window coords: picks the drag axis
    /// from the recorded direction (`R`/`L` → x, `U`/`D` → y).
    pub fn stream_pane_drag(&mut self, pos_x: f32, pos_y: f32) -> bool {
        let is_vertical = matches!(self.pane_drag.as_ref().map(|d| d.direction), Some('U' | 'D'));
        self.move_pane_drag(if is_vertical { pos_y } else { pos_x })
    }

    /// End the drag on `mouse_up` (inside or outside the grid).
    pub fn end_pane_drag(&mut self) {
        self.pane_drag = None;
    }

    // -- Correlated mutating submits (D5: 10s await + inline error) ---------

    /// Sync half shared by pane mutating submits: enqueue on the owning
    /// socket. Err when the pane has no live socket (caller records inline).
    fn send_pane_command(
        &self,
        pane_id: &str,
        msg: WsIncoming,
    ) -> Result<(String, oneshot::Receiver<CommandResult>), String> {
        let session = self
            .owning_session(pane_id)
            .ok_or_else(|| format!("pane \"{pane_id}\" has no owning session"))?;
        let rx = self
            .sessions
            .get(&session)
            .and_then(|e| e.handle.as_ref())
            .ok_or_else(|| format!("session \"{session}\" is not connected"))?
            .send_command(msg)
            .map_err(|e| e.to_string())?;
        Ok((session, rx))
    }

    /// Sync half shared by window mutating submits: enqueue on the active
    /// session's socket with the `@N` target riding `paneId` (no `windowId`
    /// field exists server-side). Err when there is no active socket.
    fn send_window_command(
        &self,
        msg: WsIncoming,
    ) -> Result<(String, oneshot::Receiver<CommandResult>), String> {
        let active = self
            .active_session
            .clone()
            .ok_or_else(|| "no active session".to_string())?;
        let rx = self
            .sessions
            .get(&active)
            .and_then(|e| e.handle.as_ref())
            .ok_or_else(|| format!("session \"{active}\" is not connected"))?
            .send_command(msg)
            .map_err(|e| e.to_string())?;
        Ok((active, rx))
    }

    /// Public sync half for dialog submits (Phase 5, 05-02): enqueue a pane
    /// command on the owning socket. Same semantics as the submit-internal
    /// half; the caller owns the correlated await (dialog feedback).
    pub fn try_send_pane_command(
        &self,
        pane_id: &str,
        msg: WsIncoming,
    ) -> Result<(String, oneshot::Receiver<CommandResult>), String> {
        self.send_pane_command(pane_id, msg)
    }

    /// Public sync half for dialog submits (Phase 5, 05-02): enqueue a window
    /// command on the active socket.
    pub fn try_send_window_command(
        &self,
        msg: WsIncoming,
    ) -> Result<(String, oneshot::Receiver<CommandResult>), String> {
        self.send_window_command(msg)
    }

    /// Await a correlated reply with dialog feedback (Phase 5, 05-02): the
    /// same 10s forget-timeout as `await_pane_command_result`, but the
    /// outcome routes into `on_complete` so rename/kill dialogs can close on
    /// success or render the inline error with the dialog open and the tab
    /// untouched. Success commits nothing locally (snapshot is the sole
    /// truth). The closure runs on the GPUI thread via the entity update.
    pub fn await_command_feedback(
        &mut self,
        rx: oneshot::Receiver<CommandResult>,
        on_complete: impl FnOnce(&mut Self, Result<(), String>, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(move |view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                let outcome: Result<(), String> = match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    rx,
                )
                .await
                {
                    Ok(Ok(cmd)) if cmd.ok => Ok(()),
                    Ok(Ok(cmd)) => Err(cmd.message.unwrap_or_else(|| "Command failed".to_string())),
                    Ok(Err(_)) => Err("Request was cancelled".to_string()),
                    Err(_) => Err("Request timed out".to_string()),
                };
                let _ = cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, cx| {
                            on_complete(this, outcome, cx);
                        });
                    }
                });
            }
        })
        .detach();
    }

    /// Await a correlated pane/window reply (10s forget-timeout, T-05-03):
    /// success commits nothing locally (snapshot is the sole truth —
    /// self-heals); `command.error`/timeout records inline on the session
    /// entry (`last_error`) with the view left untouched (no optimistic
    /// flips; drag/mutation errors surface inline like Phase 3).
    fn await_pane_command_result(
        &mut self,
        session: String,
        rx: oneshot::Receiver<CommandResult>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(move |view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                let outcome: Result<(), String> = match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    rx,
                )
                .await
                {
                    Ok(Ok(cmd)) if cmd.ok => Ok(()),
                    Ok(Ok(cmd)) => Err(cmd.message.unwrap_or_else(|| "Command failed".to_string())),
                    Ok(Err(_)) => Err("Request was cancelled".to_string()),
                    Err(_) => Err("Request timed out".to_string()),
                };
                let _ = cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, cx| {
                            if let Err(e) = outcome {
                                this.note_session_error(&session, e);
                            }
                            cx.notify();
                        });
                    }
                });
            }
        })
        .detach();
    }

    /// Correlated `pane.split` on the owning socket (Split right →
    /// `"horizontal"`, Split down → `"vertical"`).
    pub fn submit_pane_split(
        &mut self,
        pane_id: &str,
        direction: &str,
        cx: &mut Context<Self>,
    ) {
        let msg = Self::build_pane_split(pane_id, direction);
        match self.send_pane_command(pane_id, msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(session) = self.owning_session(pane_id) {
                    self.note_session_error(&session, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `pane.zoom` toggle on the owning socket (no client zoom
    /// state — server snapshot is the sole truth).
    pub fn submit_pane_zoom(&mut self, pane_id: &str, cx: &mut Context<Self>) {
        let msg = Self::build_pane_zoom(pane_id);
        match self.send_pane_command(pane_id, msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(session) = self.owning_session(pane_id) {
                    self.note_session_error(&session, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `pane.kill` on the owning socket.
    pub fn submit_pane_kill(&mut self, pane_id: &str, cx: &mut Context<Self>) {
        let msg = Self::build_pane_kill(pane_id);
        match self.send_pane_command(pane_id, msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(session) = self.owning_session(pane_id) {
                    self.note_session_error(&session, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `pane.break` on the owning socket.
    pub fn submit_pane_break(&mut self, pane_id: &str, cx: &mut Context<Self>) {
        let msg = Self::build_pane_break(pane_id);
        match self.send_pane_command(pane_id, msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(session) = self.owning_session(pane_id) {
                    self.note_session_error(&session, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `pane.swap` on the owning socket.
    pub fn submit_pane_swap(
        &mut self,
        pane_id: &str,
        other_pane_id: &str,
        cx: &mut Context<Self>,
    ) {
        let msg = Self::build_pane_swap(pane_id, other_pane_id);
        match self.send_pane_command(pane_id, msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(session) = self.owning_session(pane_id) {
                    self.note_session_error(&session, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `pane.rename` on the owning socket (title travels as JSON
    /// `title` into `select-pane -t %N -T <title>` argv — no shell, T-05-01).
    pub fn submit_pane_rename(
        &mut self,
        pane_id: &str,
        title: &str,
        cx: &mut Context<Self>,
    ) {
        if !Self::rename_name_allowed(title) {
            return;
        }
        let msg = Self::build_pane_rename(pane_id, title.trim());
        match self.send_pane_command(pane_id, msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(session) = self.owning_session(pane_id) {
                    self.note_session_error(&session, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `window.layout` on the active socket (layout strings pass
    /// through verbatim incl `next-layout`).
    pub fn submit_window_layout(
        &mut self,
        window_id: &str,
        layout: &str,
        cx: &mut Context<Self>,
    ) {
        let msg = Self::build_window_layout(window_id, layout);
        match self.send_window_command(msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(active) = self.active_session.clone() {
                    self.note_session_error(&active, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `window.rename` on the active socket.
    pub fn submit_window_rename(
        &mut self,
        window_id: &str,
        name: &str,
        cx: &mut Context<Self>,
    ) {
        if !Self::rename_name_allowed(name) {
            return;
        }
        let msg = Self::build_window_rename(window_id, name.trim());
        match self.send_window_command(msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(active) = self.active_session.clone() {
                    self.note_session_error(&active, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `window.kill` on the active socket.
    pub fn submit_window_kill(&mut self, window_id: &str, cx: &mut Context<Self>) {
        let msg = Self::build_window_kill(window_id);
        match self.send_window_command(msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(active) = self.active_session.clone() {
                    self.note_session_error(&active, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `window.move` on the active socket (offset ±1 for menu
    /// Move Left/Right).
    pub fn submit_window_move(
        &mut self,
        window_id: &str,
        offset: i32,
        cx: &mut Context<Self>,
    ) {
        let msg = Self::build_window_move(window_id, offset);
        match self.send_window_command(msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(active) = self.active_session.clone() {
                    self.note_session_error(&active, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `window.break-active` on the active socket.
    pub fn submit_window_break_active(
        &mut self,
        window_id: &str,
        cx: &mut Context<Self>,
    ) {
        let msg = Self::build_window_break_active(window_id);
        match self.send_window_command(msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(active) = self.active_session.clone() {
                    self.note_session_error(&active, e);
                }
                cx.notify();
            }
        }
    }

    /// Correlated `window.create` on the active socket (Plus button, no args
    /// beyond an optional name).
    pub fn submit_window_create(
        &mut self,
        name: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let msg = Self::build_window_create(name.as_deref());
        match self.send_window_command(msg) {
            Ok((session, rx)) => self.await_pane_command_result(session, rx, cx),
            Err(e) => {
                if let Some(active) = self.active_session.clone() {
                    self.note_session_error(&active, e);
                }
                cx.notify();
            }
        }
    }

    /// Rename submit orchestration (SESS-04): `validate_session_name`
    /// pre-flight (T-03-05), then `session.rename` on the TARGET session's own
    /// socket (D5 FE-quirk correction — never the active socket for a
    /// non-active target). When the target tab is not open,
    /// `ensure_session_socket(target)` runs first and the rename pends until
    /// the connect lands (`pending_rename`, flushed in `ensure_session_socket`).
    pub fn submit_rename(
        &mut self,
        target: &str,
        new_name: String,
        window_handle: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        let new_name = new_name.trim().to_string();
        if let Err(e) = validate_session_name(&new_name) {
            self.set_rename_error(Some(e.to_string()), cx);
            self.set_rename_submitting(false, cx);
            cx.notify();
            return;
        }
        if new_name == target {
            // Nothing to change — dismiss without touching the socket.
            self.rename_session_form = None;
            let _ = window_handle.update(cx, |_, window, cx| {
                window.close_dialog(cx);
            });
            cx.notify();
            return;
        }
        match self.send_rename_on_target(target, &new_name) {
            Ok(rx) => self.await_rename_result(target.to_string(), new_name, window_handle, rx, cx),
            Err(_) => {
                let Some(base) = self.base_url.clone() else {
                    self.set_rename_error(
                        Some("Backend is not ready yet. Try again in a moment.".to_string()),
                        cx,
                    );
                    self.set_rename_submitting(false, cx);
                    cx.notify();
                    return;
                };
                self.pending_rename = Some((target.to_string(), new_name, window_handle));
                self.ensure_session_socket(&base, target, cx);
                cx.notify();
            }
        }
    }

    /// Sync half of rename: enqueue correlated `session.rename` (`newName` +
    /// auto `requestId`) on the target's own socket. Err when the target has
    /// no live socket (caller ensures first).
    fn send_rename_on_target(
        &self,
        target: &str,
        new_name: &str,
    ) -> Result<oneshot::Receiver<CommandResult>, String> {
        let handle = self
            .sessions
            .get(target)
            .and_then(|e| e.handle.as_ref())
            .ok_or_else(|| format!("session \"{target}\" is not connected"))?;
        let msg = WsIncoming {
            msg_type: MSG_SESSION_RENAME.to_string(),
            new_name: Some(new_name.to_string()),
            ..Default::default()
        };
        handle.send_command(msg).map_err(|e| e.to_string())
    }

    /// Await the correlated rename reply (10s forget-timeout, T-03-07):
    /// success runs re-resolution (old→new migration with generation+1,
    /// T-03-06) + reconnect + poll + dialog close; `command.error`/timeout
    /// renders inline with the dialog open and the tab untouched.
    fn await_rename_result(
        &mut self,
        target: String,
        new_name: String,
        window_handle: AnyWindowHandle,
        rx: oneshot::Receiver<CommandResult>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(move |view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                let outcome: Result<(), String> = match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    rx,
                )
                .await
                {
                    Ok(Ok(cmd)) if cmd.ok => Ok(()),
                    Ok(Ok(cmd)) => Err(cmd.message.unwrap_or_else(|| "Rename failed".to_string())),
                    Ok(Err(_)) => Err("Rename request was cancelled".to_string()),
                    Err(_) => Err("Rename request timed out".to_string()),
                };
                let _ = cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, cx| {
                            match outcome {
                                Ok(()) => {
                                    if this.rename_session_entry(&target, &new_name) {
                                        if let Some(base) = this.base_url.clone() {
                                            this.ensure_session_socket(&base, &new_name, cx);
                                        }
                                        this.rename_session_form = None;
                                        this.trigger_poll(cx);
                                        let _ = window_handle.update(cx, |_, window, cx| {
                                            window.close_dialog(cx);
                                        });
                                    } else {
                                        this.set_rename_error(
                                            Some(
                                                "Could not apply rename — the session list changed."
                                                    .to_string(),
                                            ),
                                            cx,
                                        );
                                        this.set_rename_submitting(false, cx);
                                    }
                                }
                                Err(e) => {
                                    this.set_rename_error(Some(e), cx);
                                    this.set_rename_submitting(false, cx);
                                }
                            }
                            cx.notify();
                        });
                    }
                });
            }
        })
        .detach();
    }

    fn set_rename_error(&mut self, message: Option<String>, cx: &mut Context<Self>) {
        if let Some(form) = self.rename_session_form.clone() {
            form.update(cx, |f, cx| {
                f.error_message = message;
                cx.notify();
            });
        }
    }

    fn set_rename_submitting(&mut self, submitting: bool, cx: &mut Context<Self>) {
        if let Some(form) = self.rename_session_form.clone() {
            form.update(cx, |f, cx| {
                f.is_submitting = submitting;
                cx.notify();
            });
        }
    }

    /// Kill transport route per D6.
    pub fn kill_route(&self, target: &str) -> KillRoute {
        if self
            .sessions
            .get(target)
            .and_then(|e| e.handle.as_ref())
            .is_some()
        {
            KillRoute::Victim
        } else if let Some((name, _)) = self.sessions.iter().find(|(_, e)| e.handle.is_some()) {
            KillRoute::ViaOther(name.clone())
        } else {
            KillRoute::Ephemeral
        }
    }

    /// Sync half of kill: enqueue correlated `session.kill` on the victim
    /// socket, or on another live socket with the explicit `session` field
    /// (Go honors cross-session kill, `handler.go:186-193`). Err when zero
    /// sockets are live (caller runs the ephemeral one-shot).
    fn send_kill(
        &self,
        target: &str,
        route: &KillRoute,
    ) -> Result<oneshot::Receiver<CommandResult>, String> {
        let (socket_name, explicit) = match route {
            KillRoute::Victim => (target, None),
            KillRoute::ViaOther(other) => (other.as_str(), Some(target.to_string())),
            KillRoute::Ephemeral => return Err("no live socket".to_string()),
        };
        let handle = self
            .sessions
            .get(socket_name)
            .and_then(|e| e.handle.as_ref())
            .ok_or_else(|| "session socket is not connected".to_string())?;
        let msg = WsIncoming {
            msg_type: MSG_SESSION_KILL.to_string(),
            session: explicit,
            ..Default::default()
        };
        handle.send_command(msg).map_err(|e| e.to_string())
    }

    /// Kill entry point for both paths (SESS-05): the confirm dialog's Kill
    /// button passes its window (closed on success); direct kills pass None.
    pub fn execute_kill(
        &mut self,
        target: &str,
        window_handle: Option<AnyWindowHandle>,
        cx: &mut Context<Self>,
    ) {
        match self.kill_route(target) {
            KillRoute::Ephemeral => self.execute_ephemeral_kill(target, window_handle, cx),
            route => match self.send_kill(target, &route) {
                Ok(rx) => self.await_kill_result(target.to_string(), window_handle, rx, cx),
                Err(e) => {
                    self.note_session_error(target, e.clone());
                    self.set_kill_error(Some(e), cx);
                    self.set_kill_submitting(false, cx);
                    cx.notify();
                }
            },
        }
    }

    /// Await the correlated kill reply (10s forget-timeout): success drops the
    /// entry + neighbor activation + poll (+ dialog close when open); error
    /// renders inline destructive with the dialog open and the tab untouched.
    fn await_kill_result(
        &mut self,
        target: String,
        window_handle: Option<AnyWindowHandle>,
        rx: oneshot::Receiver<CommandResult>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(move |view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                let outcome: Result<(), String> = match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    rx,
                )
                .await
                {
                    Ok(Ok(cmd)) if cmd.ok => Ok(()),
                    Ok(Ok(cmd)) => Err(cmd.message.unwrap_or_else(|| "Kill failed".to_string())),
                    Ok(Err(_)) => Err("Kill request was cancelled".to_string()),
                    Err(_) => Err("Kill request timed out".to_string()),
                };
                let _ = cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, cx| {
                            match outcome {
                                Ok(()) => {
                                    this.finish_kill_success(&target);
                                    this.kill_session_form = None;
                                    this.trigger_poll(cx);
                                    if let Some(wh) = window_handle {
                                        let _ = wh.update(cx, |_, window, cx| {
                                            window.close_dialog(cx);
                                        });
                                    }
                                }
                                Err(e) => {
                                    this.note_session_error(&target, e.clone());
                                    this.set_kill_error(Some(e), cx);
                                    this.set_kill_submitting(false, cx);
                                }
                            }
                            cx.notify();
                        });
                    }
                });
            }
        })
        .detach();
    }

    /// Ephemeral one-shot kill (D6, zero live sockets): connect → kill →
    /// close on `TOKIO_RT`, committing nothing locally — the bootstrap
    /// snapshot is never forwarded so no phantom tab can appear. Tab-close
    /// and poll still run on correlated success.
    fn execute_ephemeral_kill(
        &mut self,
        target: &str,
        window_handle: Option<AnyWindowHandle>,
        cx: &mut Context<Self>,
    ) {
        let Some(base) = self.base_url.clone() else {
            let e = "Backend is not ready yet. Try again in a moment.".to_string();
            self.note_session_error(target, e.clone());
            self.set_kill_error(Some(e), cx);
            self.set_kill_submitting(false, cx);
            cx.notify();
            return;
        };
        let target_name = target.to_string();
        cx.spawn(move |view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                let (tx, rx) = tokio::sync::oneshot::channel();
                let (ephemeral_base, ephemeral_target) = (base.clone(), target_name.clone());
                TOKIO_RT.spawn(async move {
                    let outcome: Result<(), String> = async {
                        let handle = connect_session(&ephemeral_base, &ephemeral_target, 0)
                            .await
                            .map_err(|e| e.to_string())?;
                        let msg = WsIncoming {
                            msg_type: MSG_SESSION_KILL.to_string(),
                            session: Some(ephemeral_target.clone()),
                            ..Default::default()
                        };
                        let waiter = handle.send_command(msg).map_err(|e| e.to_string())?;
                        match tokio::time::timeout(std::time::Duration::from_secs(10), waiter)
                            .await
                        {
                            Ok(Ok(cmd)) if cmd.ok => Ok(()),
                            Ok(Ok(cmd)) => {
                                Err(cmd.message.unwrap_or_else(|| "Kill failed".to_string()))
                            }
                            Ok(Err(_)) => Err("Kill request was cancelled".to_string()),
                            Err(_) => Err("Kill request timed out".to_string()),
                        }
                        // `handle` drops here: one-shot torn down, bootstrap
                        // committed nowhere (D6).
                    }
                    .await;
                    let _ = tx.send(outcome);
                });
                let outcome = match rx.await {
                    Ok(r) => r,
                    Err(_) => return,
                };
                let _ = cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, cx| {
                            match outcome {
                                Ok(()) => {
                                    this.finish_kill_success(&target_name);
                                    this.kill_session_form = None;
                                    this.trigger_poll(cx);
                                    if let Some(wh) = window_handle {
                                        let _ = wh.update(cx, |_, window, cx| {
                                            window.close_dialog(cx);
                                        });
                                    }
                                }
                                Err(e) => {
                                    this.note_session_error(&target_name, e.clone());
                                    this.set_kill_error(Some(e), cx);
                                    this.set_kill_submitting(false, cx);
                                }
                            }
                            cx.notify();
                        });
                    }
                });
            }
        })
        .detach();
    }

    /// Kill-success commit: drop the entry (even when its tab was never
    /// open), neighbor activation via `close_session`, caller polls.
    pub fn finish_kill_success(&mut self, target: &str) {
        self.close_session(target);
        self.sessions.remove(target);
    }

    /// Record a kill/rename transport error on the tab for later surfaces.
    /// No-op when the entry is gone (the poll already shows the truth).
    pub fn note_session_error(&mut self, target: &str, err: String) {
        if let Some(entry) = self.sessions.get_mut(target) {
            entry.last_error = Some(err);
        }
    }

    fn set_kill_error(&mut self, message: Option<String>, cx: &mut Context<Self>) {
        if let Some(form) = self.kill_session_form.clone() {
            form.update(cx, |f, cx| {
                f.error_message = message;
                cx.notify();
            });
        }
    }

    fn set_kill_submitting(&mut self, submitting: bool, cx: &mut Context<Self>) {
        if let Some(form) = self.kill_session_form.clone() {
            form.update(cx, |f, cx| {
                f.is_submitting = submitting;
                cx.notify();
            });
        }
    }

    /// Connect-on-open (D4): create the entry with `generation + 1` and spawn
    /// the `connect_session` handshake on `TOKIO_RT` (never block the GPUI
    /// thread — Pitfall 6). Bootstrap commits flow through `apply_event`;
    /// `state.resync` on `connection.ready` is answered inside the read pump.
    /// No-op when a handle already exists. Drops the outcome when the tab was
    /// closed or re-resolved while connecting.
    pub fn ensure_session_socket(&mut self, base_url: &str, session: &str, cx: &mut Context<Self>) {
        if !self.open_sessions.iter().any(|s| s == session) {
            self.open_sessions.push(session.to_string());
        }
        let entry = self
            .sessions
            .entry(session.to_string())
            .or_insert_with(OpenSession::default);
        if entry.handle.is_some() {
            return;
        }
        entry.generation += 1;
        entry.transport = TransportState::Connecting;
        entry.last_error = None;

        let gen = entry.generation;
        let pending = entry.pending.clone();
        let session_name = session.to_string();
        let base = base_url.to_string();

        cx.spawn(move |view_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
            let cx_handle = cx.clone();
            async move {
                let (tx, rx) = tokio::sync::oneshot::channel();
                let session_for_connect = session_name.clone();
                TOKIO_RT.spawn(async move {
                    let res =
                        connect_session_with_pending(&base, &session_for_connect, gen, pending)
                            .await;
                    let _ = tx.send(res.map_err(|e| e.to_string()));
                });
                let outcome = match rx.await {
                    Ok(r) => r,
                    Err(_) => return,
                };
                let _ = cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, cx| {
                            match outcome {
                                Ok(handle) => {
                                    let live = match this.sessions.get(&handle.session().to_string()) {
                                        Some(e) => e.generation == handle.generation(),
                                        None => false,
                                    };
                                    if !live {
                                        return; // closed / re-resolved mid-connect: drop
                                    }
                                    let ev_rx = handle.subscribe_events();
                                    let key = handle.session().to_string();
                                    if let Some(entry) = this.sessions.get_mut(&key) {
                                        entry.handle = Some(handle);
                                        entry.transport = TransportState::Connected;
                                    }
                                    // Flush a rename pended while this socket
                                    // was connecting (D5).
                                    if this
                                        .pending_rename
                                        .as_ref()
                                        .is_some_and(|(t, _, _)| t == &key)
                                    {
                                        let (_, new_name, wh) = this
                                            .pending_rename
                                            .take()
                                            .expect("checked above");
                                        let target = key.clone();
                                        match this.send_rename_on_target(&target, &new_name) {
                                            Ok(rx) => this.await_rename_result(
                                                target, new_name, wh, rx, cx,
                                            ),
                                            Err(e) => {
                                                this.set_rename_error(
                                                    Some(format!("Rename failed: {e}")),
                                                    cx,
                                                );
                                                this.set_rename_submitting(false, cx);
                                            }
                                        }
                                    }
                                    cx.notify();
                                    // Forward pump: tagged events → guarded apply.
                                    cx.spawn(
                                        move |fwd_weak: WeakEntity<Self>, cx: &mut AsyncApp| {
                                            let fwd_handle = cx.clone();
                                            async move {
                                                while let Ok(ev) = ev_rx.recv_async().await {
                                                    let _ = fwd_handle.update(|cx: &mut App| {
                                                        if let Some(ent) = fwd_weak.upgrade() {
                                                            ent.update(cx, |t, cx| {
                                                                t.apply_event(
                                                                    &ev.session,
                                                                    ev.generation,
                                                                    &ev.msg,
                                                                );
                                                                cx.notify();
                                                            });
                                                        }
                                                    });
                                                }
                                            }
                                        },
                                    )
                                    .detach();
                                }
                                Err(err) => {
                                    if let Some(entry) = this.sessions.get_mut(&session_name) {
                                        if entry.generation == gen {
                                            entry.transport = TransportState::Disconnected;
                                            entry.last_error = Some(err.clone());
                                        }
                                    }
                                    // A pended rename dies with the connect —
                                    // inline error, flag never sticks (T-03-07).
                                    if this
                                        .pending_rename
                                        .as_ref()
                                        .is_some_and(|(t, _, _)| t == &session_name)
                                    {
                                        this.pending_rename.take();
                                        this.set_rename_error(
                                            Some(format!("Could not connect: {err}")),
                                            cx,
                                        );
                                        this.set_rename_submitting(false, cx);
                                    }
                                    cx.notify();
                                }
                            }
                        });
                    }
                });
            }
        })
        .detach();
    }
}

impl Render for AppState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title_bar = render_title_bar(self, cx);
        let status = self.backend_status.clone();

        let is_ready = matches!(status, BackendStatus::Ready(_));

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .child(title_bar)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .size_full()
                    .when(is_ready, |s| {
                        s.child(crate::views::sidebar::render_sidebar(self, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .size_full()
                                    .bg(rgb(0x1e1e1e))
                                    .child(crate::views::session_states::render_workspace_body(self, cx))
                            )
                    })
                    .when(!is_ready, |s| {
                        s.child(render_status_page(
                            &status,
                            |this, _, _window, cx| {
                                this.start_supervisor(cx);
                            },
                            |this, _, _window, cx| {
                                this.stop_supervisor();
                                cx.quit();
                            },
                            cx,
                        ))
                    }),
            )
    }
}
