//! Root application state, polling pump, and supervisor lifecycle.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::WindowExt as _;
use tokio::sync::oneshot;
use webtmux_backend_client::{
    connect_session, connect_session_with_pending, validate_session_name, RestClient,
    CommandResult, SessionSnapshot, SessionWsHandle, SharedPending, TransportState, TmuxTree,
    WsIncoming, WsOutgoing, EV_CONNECTION_READY, EV_STATE_DELTA, EV_STATE_SNAPSHOT,
    EV_SERVER_ERROR, EV_TERMINAL_OUTPUT, EV_TERMINAL_SNAPSHOT, EV_TMUX_DISCONNECTED,
    EV_TMUX_RECONNECTING, MSG_SESSION_KILL, MSG_SESSION_RENAME, MSG_TERMINAL_CAPTURE,
    MSG_TERMINAL_INPUT, MSG_WINDOW_SELECT,
};
use webtmux_settings::DesktopSettings;
use webtmux_supervisor::{BackendInfo, BackendStatus, SpawnOptions, Supervisor};
use webtmux_terminal::{apply_capture, Terminal, TerminalEvent};
use crate::views::{status::render_status_page, tab_strip::render_title_bar};

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
    /// The alacritty-backed grid; mutated synchronously on the GPUI thread.
    pub terminal: Terminal,
    /// Exactly-once gate: first `terminal.snapshot` replaces, later ones drop
    /// until `invalidate_pane_snapshot` re-arms (reconnect / layout resync).
    pub snapshot_written: bool,
    /// Scrollback lines already pushed to this instance (FE `ingestedHistory`).
    pub ingested_history: usize,
}

impl PaneTerminal {
    fn fresh() -> Self {
        Self {
            terminal: Terminal::new(80, 24),
            snapshot_written: false,
            ingested_history: 0,
        }
    }
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

/// Title-bar window-tab model derived from the active snapshot (SHELL-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowTab {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub active: bool,
}

/// Kill transport route per D6 (see `AppState::kill_route`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillRoute {
    /// Victim's own socket is live — kill rides it (no explicit field).
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
            terminals: HashMap::new(),
            pane_session: HashMap::new(),
            tui_scroll: HashMap::new(),
            pane_titles: HashMap::new(),
            pending_viewport: None,
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
        self.terminals.get(pane_id).map(|e| e.terminal.grid_text())
    }

    /// Replay counter for a pane (contract tests).
    pub fn pane_ingested_history(&self, pane_id: &str) -> Option<usize> {
        self.terminals.get(pane_id).map(|e| e.ingested_history)
    }

    /// Last OSC title for a pane (D8).
    pub fn pane_title(&self, pane_id: &str) -> Option<String> {
        self.pane_titles.get(pane_id).cloned()
    }

    /// D3: per-pane TUI-scroll switch — absent means ON (FE `?? true` parity).
    pub fn tui_scroll(&self, pane_id: &str) -> bool {
        self.tui_scroll.get(pane_id).copied().unwrap_or(true)
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
        entry.terminal.process_bytes(&feed);
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
            entry.terminal.process_bytes(&feed);
        } else {
            self.pane_entry(pane_id)
                .terminal
                .process_bytes(data.as_bytes());
        }
        self.drain_pane_events(pane_id);
        true
    }

    /// Drain alacritty events for a pane: `Title` commits the last-title map
    /// (D8, feeds Phase-5 headers), `Bell` is a no-op.
    fn drain_pane_events(&mut self, pane_id: &str) {
        let events = match self.terminals.get(pane_id) {
            Some(e) => e.terminal.drain_events(),
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
