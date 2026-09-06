//! Root application state and supervisor lifecycle pump.

use std::sync::LazyLock;
use gpui::*;
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

        let view_weak = cx.entity().downgrade();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SupervisorEvent>();

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
                    let _ = tx.send(SupervisorEvent::Ready(info));
                }
                Err(e) => {
                    let (reason, stderr_tail) = match e {
                        webtmux_supervisor::SupervisorError::Failed { reason, stderr_tail } => {
                            (reason, stderr_tail)
                        }
                        other => (other.to_string(), String::new()),
                    };
                    let _ = tx.send(SupervisorEvent::Failed { reason, stderr_tail });
                }
            }
        });

        // Observe events inside GPUI foreground executor with weak.upgrade() leak prevention
        cx.spawn(move |_view, cx: &mut AsyncApp| {
            let view_weak = view_weak.clone();
            let cx_handle = cx.clone();
            async move {
                while let Some(event) = rx.recv().await {
                    let view_weak = view_weak.clone();
                    let _ = cx_handle.update(|cx: &mut App| {
                        if let Some(entity) = view_weak.upgrade() {
                            entity.update(cx, |this, cx| {
                                match event {
                                    SupervisorEvent::Status(st) => {
                                        this.backend_status = st;
                                    }
                                    SupervisorEvent::Ready(info) => {
                                        this.base_url = Some(info.base_url.clone());
                                        this.backend_status = BackendStatus::Ready(info);
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

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .child(title_bar)
            .child(
                div()
                    .flex_1()
                    .size_full()
                    .child(render_status_page(&status, |this, _, _window, cx| {
                        this.start_supervisor(cx);
                    }, cx)),
            )
    }
}
