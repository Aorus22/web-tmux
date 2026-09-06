//! Root application state, polling pump, and supervisor lifecycle.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use gpui::*;
use gpui::prelude::FluentBuilder;
use webtmux_backend_client::{
    connect_session_with_pending, RestClient, SessionSnapshot, SessionWsHandle, SharedPending,
    TransportState, TmuxTree, WsOutgoing, EV_STATE_DELTA, EV_STATE_SNAPSHOT, EV_SERVER_ERROR,
    EV_TERMINAL_OUTPUT, EV_TERMINAL_SNAPSHOT, EV_TMUX_DISCONNECTED, EV_TMUX_RECONNECTING,
};
use webtmux_settings::DesktopSettings;
use webtmux_supervisor::{BackendInfo, BackendStatus, SpawnOptions, Supervisor};
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

/// Title-bar window-tab model derived from the active snapshot (SHELL-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowTab {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub active: bool,
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

    /// DLG1 Create Session dialog form entity, held alive while the modal is
    /// open. Reset to `None` on dismiss; replaced on every (re)open.
    pub create_session_form:
        Option<gpui::Entity<crate::views::create_session_dialog::CreateSessionForm>>,
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
    /// `terminal.snapshot`/`terminal.output` parse-and-ignore: accepted,
    /// committed nowhere (Phase 4 owns them).
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
                if let Some(snap) = msg.snapshot.clone() {
                    entry.snapshot = Some(snap);
                    entry.transport = TransportState::Connected;
                    true
                } else {
                    false
                }
            }
            EV_TERMINAL_SNAPSHOT | EV_TERMINAL_OUTPUT => true, // parse-and-ignore
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
                                            entry.last_error = Some(err);
                                        }
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
