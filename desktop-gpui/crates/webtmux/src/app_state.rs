//! Root application state, polling pump, and supervisor lifecycle.

use std::collections::HashSet;
use std::sync::LazyLock;
use gpui::*;
use gpui::prelude::FluentBuilder;
use webtmux_backend_client::{RestClient, TmuxTree};
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
