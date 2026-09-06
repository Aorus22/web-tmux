//! Live per-pane terminal view (Phase 4 tracer, TERM-01/02/03/07).
//!
//! Structural port of the reference `TerminalView` (canvas + focus +
//! key/mouse/scroll handlers) rewired for web-tmux ownership:
//!
//! - The `AppState` store owns every pane's `Terminal` (shared
//!   `Arc<Mutex<...>>`); this view holds a clone for paint/input only and
//!   never creates terminals. Unmount drops the clone — the store keeps
//!   ingesting hidden sessions (Pitfall 6).
//! - Bytes out route through the `AppState` hop (`send_terminal_input` on the
//!   OWNING session's socket); sizes out ARM `pending_viewport` only and never
//!   send from paint (Pitfall 4 — plan 04-02 sends).
//! - No per-view terminal-event subscription: the flume event queue is mpsc,
//!   and the `AppState` commit path is its single consumer (D8 titles). A
//!   second drain here would steal `Title` events from the commit path.

use crate::app_state::{decide_key_route, AppState, KeyRoute, PendingViewport, WheelDelta};
use gpui::*;
use parking_lot::Mutex;
use std::sync::Arc;
use webtmux_terminal::{
    keystroke_to_bytes, modifiers_to_mouse_code, mouse_button_report, pixel_to_cell,
    selection_type_from_clicks, AlacPoint, ColorPalette, Column, Line, Terminal, TerminalRenderer,
};

pub type InputCallback = Arc<dyn Fn(&[u8]) + Send + Sync>;
pub type ResizeCallback = Arc<dyn Fn(usize, usize) + Send + Sync>;
pub type TitleCallback = Arc<dyn Fn(&str) + Send + Sync>;
pub type BellCallback = Arc<dyn Fn() + Send + Sync>;

/// Dumb view over one store-owned pane terminal: bytes out via the AppState
/// hop (or `InputCallback` override), sizes out via pending-viewpor
/// arming (or `ResizeCallback` override).
pub struct TerminalView {
    pane_id: String,
    terminal: Arc<Mutex<Terminal>>,
    app: WeakEntity<AppState>,
    renderer: TerminalRenderer,
    focus_handle: FocusHandle,
    padding: Edges<Pixels>,
    is_selecting: bool,
    last_bounds: Arc<Mutex<Option<Bounds<Pixels>>>>,
    input_callback: Option<InputCallback>,
    resize_callback: Option<ResizeCallback>,
    // Phase-5-owned: fed from the AppState title map, never by a per-view
    // event drain (which would race the commit-path consumer).
    #[allow(dead_code)]
    title_callback: Option<TitleCallback>,
    #[allow(dead_code)]
    bell_callback: Option<BellCallback>,
}

impl TerminalView {
    /// Construct a view over the store entry's shared terminal handle.
    pub fn new(
        pane_id: String,
        terminal: Arc<Mutex<Terminal>>,
        app: WeakEntity<AppState>,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        // D2: FE settings defaults — 14px / 1.35 line-height / fixed dark
        // palette until Phase 6 ports real prefs + the 78 presets.
        let renderer = TerminalRenderer::new(
            "JetBrains Mono".to_string(),
            px(14.0),
            1.35,
            ColorPalette::dark_default(),
        );
        Self {
            pane_id,
            terminal,
            app,
            renderer,
            focus_handle,
            padding: Edges::all(px(4.0)),
            is_selecting: false,
            last_bounds: Arc::new(Mutex::new(None)),
            input_callback: None,
            resize_callback: None,
            title_callback: None,
            bell_callback: None,
        }
    }

    /// Pane this view renders.
    pub fn pane_id(&self) -> &str {
        &self.pane_id
    }

    /// Attach custom terminal renderer.
    pub fn with_renderer(mut self, renderer: TerminalRenderer) -> Self {
        self.renderer = renderer;
        self
    }

    /// Set inner canvas padding.
    pub fn with_padding(mut self, padding: Edges<Pixels>) -> Self {
        self.padding = padding;
        self
    }

    /// Override invoked with terminal input bytes instead of the AppState hop
    /// (headless/unit contexts; production views leave this unset).
    pub fn with_input_callback<F: Fn(&[u8]) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.input_callback = Some(Arc::new(f));
        self
    }

    /// Override invoked on grid-size change instead of arming
    /// `pending_viewport` (headless/unit contexts).
    pub fn with_resize_callback<F: Fn(usize, usize) + Send + Sync + 'static>(
        mut self,
        f: F,
    ) -> Self {
        self.resize_callback = Some(Arc::new(f));
        self
    }

    /// Set callback invoked when terminal title changes.
    pub fn with_title_callback<F: Fn(&str) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.title_callback = Some(Arc::new(f));
        self
    }

    /// Set callback invoked when terminal bell triggers.
    pub fn with_bell_callback<F: Fn() + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.bell_callback = Some(Arc::new(f));
        self
    }

    /// Update callback invoked when terminal generates input bytes.
    pub fn set_input_callback<F: Fn(&[u8]) + Send + Sync + 'static>(&mut self, f: F) {
        self.input_callback = Some(Arc::new(f));
    }

    /// Update callback invoked when terminal grid dimensions change.
    pub fn set_resize_callback<F: Fn(usize, usize) + Send + Sync + 'static>(&mut self, f: F) {
        self.resize_callback = Some(Arc::new(f));
    }

    /// Update callback invoked when terminal title changes.
    pub fn set_title_callback<F: Fn(&str) + Send + Sync + 'static>(&mut self, f: F) {
        self.title_callback = Some(Arc::new(f));
    }

    /// Update callback invoked when terminal bell triggers.
    pub fn set_bell_callback<F: Fn() + Send + Sync + 'static>(&mut self, f: F) {
        self.bell_callback = Some(Arc::new(f));
    }

    /// Access the shared store terminal handle.
    pub fn terminal(&self) -> Arc<Mutex<Terminal>> {
        Arc::clone(&self.terminal)
    }

    /// Access the renderer.
    pub fn renderer(&self) -> &TerminalRenderer {
        &self.renderer
    }

    /// Mutable access to the renderer.
    pub fn renderer_mut(&mut self) -> &mut TerminalRenderer {
        &mut self.renderer
    }

    /// Dynamically update the color palette used by this terminal view.
    pub fn set_palette(&mut self, palette: ColorPalette, cx: &mut Context<Self>) {
        self.renderer.palette = palette;
        cx.notify();
    }

    /// Return reference to the current color palette.
    pub fn palette(&self) -> &ColorPalette {
        &self.renderer.palette
    }

    /// Dynamically update the font size used by this terminal view.
    pub fn set_font_size(&mut self, size: Pixels, cx: &mut Context<Self>) {
        self.renderer.font_size = size;
        self.renderer.cell_width = size * 0.6;
        self.renderer.cell_height = size * self.renderer.line_height_multiplier;
        cx.notify();
    }

    /// Dynamically update font family and font size used by this terminal view.
    pub fn set_font(&mut self, family: String, size: Pixels, cx: &mut Context<Self>) {
        self.renderer.font_family = family;
        self.renderer.font_size = size;
        self.renderer.cell_width = size * 0.6;
        self.renderer.cell_height = size * self.renderer.line_height_multiplier;
        cx.notify();
    }

    /// Focus handle for keyboard input routing.
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    /// Send input bytes out: `InputCallback` override when set, otherwise the
    /// AppState-owned hop (`terminal.input` on the owning socket, byte-safe
    /// `String` carrier — never lossy).
    pub fn write_to_pty(&self, bytes: &[u8], cx: &mut Context<Self>) {
        if let Some(ref cb) = self.input_callback {
            cb(bytes);
            return;
        }
        let Ok(data) = String::from_utf8(bytes.to_vec()) else {
            return;
        };
        let pane = self.pane_id.clone();
        let _ = self.app.update(cx, |app, cx| {
            app.send_terminal_input(&pane, data);
            cx.notify();
        });
    }

    /// Copy current selection text to system clipboard.
    pub fn copy_selection(&self, cx: &mut Context<Self>) -> bool {
        if let Some(text) = self.terminal.lock().selection_text() {
            if !text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                return true;
            }
        }
        false
    }

    /// Paste text from system clipboard into terminal input (one
    /// `terminal.input` message — the server batcher chunks large writes).
    pub fn paste_clipboard(&self, cx: &mut Context<Self>) -> bool {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            if !text.is_empty() {
                self.write_to_pty(text.as_bytes(), cx);
                return true;
            }
        }
        false
    }

    /// Scroll display viewport by delta lines.
    pub fn scroll_display(&self, delta: i32, cx: &mut Context<Self>) {
        self.terminal.lock().scroll_display(delta);
        cx.notify();
    }

    /// Scroll viewport to bottom (active cursor position).
    pub fn scroll_to_bottom(&self, cx: &mut Context<Self>) {
        self.terminal.lock().scroll_to_bottom();
        cx.notify();
    }

    /// Scroll viewport to top of scrollback history.
    pub fn scroll_to_top(&self, cx: &mut Context<Self>) {
        self.terminal.lock().scroll_to_top();
        cx.notify();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        // Clipboard routing via the headless-tested helper (TERM-05):
        // Ctrl+Shift+C / Cmd+C with a selection copies, Ctrl+Shift+V /
        // Cmd+V pastes, everything else (incl. Ctrl+C with an empty
        // selection) falls through to the interrupt byte path below.
        let has_selection = self
            .terminal
            .lock()
            .selection_text()
            .is_some_and(|t| !t.is_empty());
        match decide_key_route(
            has_selection,
            event.keystroke.modifiers.control,
            event.keystroke.modifiers.shift,
            event.keystroke.modifiers.platform,
            &event.keystroke.key,
        ) {
            KeyRoute::Copy => {
                if self.copy_selection(cx) {
                    return;
                }
            }
            KeyRoute::Paste => {
                if self.paste_clipboard(cx) {
                    return;
                }
            }
            KeyRoute::Terminal => {}
        }

        // Any key press resets scrollback offset to zero (bottom of terminal)
        {
            let mut term = self.terminal.lock();
            term.scroll_to_bottom();
        }

        // Ctrl+C with an EMPTY selection falls through here as interrupt
        // passthrough (FE useTerminal parity).
        let mode = self.terminal.lock().mode();
        if let Some(bytes) = keystroke_to_bytes(&event.keystroke, mode) {
            self.write_to_pty(&bytes, cx);
        }

        cx.notify();
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let bounds = *self.last_bounds.lock();

        if let Some(bounds) = bounds {
            let origin = Point {
                x: bounds.origin.x + self.padding.left,
                y: bounds.origin.y + self.padding.top,
            };
            let mut term = self.terminal.lock();
            let pt = pixel_to_cell(
                event.position,
                origin,
                self.renderer.cell_width,
                self.renderer.cell_height,
                term.cols(),
                term.rows(),
            );

            let mode = term.mode();
            let mouse_mods = modifiers_to_mouse_code(&event.modifiers);

            if let Some(bytes) = mouse_button_report(event.button, true, pt, mouse_mods, mode) {
                drop(term);
                self.write_to_pty(&bytes, cx);
            } else if event.button == MouseButton::Left {
                let sel_type = selection_type_from_clicks(event.click_count);
                term.start_selection(pt, sel_type);
                self.is_selecting = true;
            }
        }

        cx.notify();
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let bounds = *self.last_bounds.lock();
        if let Some(bounds) = bounds {
            let origin = Point {
                x: bounds.origin.x + self.padding.left,
                y: bounds.origin.y + self.padding.top,
            };
            let mut term = self.terminal.lock();
            let pt = pixel_to_cell(
                event.position,
                origin,
                self.renderer.cell_width,
                self.renderer.cell_height,
                term.cols(),
                term.rows(),
            );

            if self.is_selecting {
                term.update_selection(pt);
                cx.notify();
            } else if let Some(button) = event.pressed_button {
                let mode = term.mode();
                let mouse_mods = modifiers_to_mouse_code(&event.modifiers);
                if let Some(bytes) = mouse_button_report(button, true, pt, mouse_mods, mode) {
                    drop(term);
                    self.write_to_pty(&bytes, cx);
                }
            }
        }
    }

    fn on_mouse_up(&mut self, event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let bounds = *self.last_bounds.lock();
        if let Some(bounds) = bounds {
            let origin = Point {
                x: bounds.origin.x + self.padding.left,
                y: bounds.origin.y + self.padding.top,
            };
            let term = self.terminal.lock();
            let pt = pixel_to_cell(
                event.position,
                origin,
                self.renderer.cell_width,
                self.renderer.cell_height,
                term.cols(),
                term.rows(),
            );

            let mode = term.mode();
            let mouse_mods = modifiers_to_mouse_code(&event.modifiers);
            if let Some(bytes) = mouse_button_report(event.button, false, pt, mouse_mods, mode) {
                drop(term);
                self.write_to_pty(&bytes, cx);
            }
        }

        if event.button == MouseButton::Left {
            self.is_selecting = false;
        }

        cx.notify();
    }

    fn on_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // FE-parity wheel policy (TERM-04): convert to the headless
        // `WheelDelta`, compute the SGR point, then emit via the AppState
        // accumulator path (SGR/Page bytes over terminal.input, scrollback
        // via scroll_display). Capture-phase consume is implicit: GPUI
        // delivers the wheel to the focused view and we never propagate.
        let delta = match event.delta {
            ScrollDelta::Lines(d) => WheelDelta::Lines(d.y),
            ScrollDelta::Pixels(d) => {
                let dy: f32 = d.y.into();
                WheelDelta::Pixels(dy)
            }
        };
        let cell_h: f32 = self.renderer.cell_height.into();
        let bounds = *self.last_bounds.lock();
        let pt = if let Some(bounds) = bounds {
            let origin = Point {
                x: bounds.origin.x + self.padding.left,
                y: bounds.origin.y + self.padding.top,
            };
            let term = self.terminal.lock();
            pixel_to_cell(
                event.position,
                origin,
                self.renderer.cell_width,
                self.renderer.cell_height,
                term.cols(),
                term.rows(),
            )
        } else {
            AlacPoint::new(Line(0), Column(0))
        };
        let mouse_mods = modifiers_to_mouse_code(&event.modifiers);
        let pane = self.pane_id.clone();
        let _ = self.app.update(cx, |app, cx| {
            app.apply_wheel(&pane, delta, cell_h, pt, mouse_mods);
            cx.notify();
        });
        cx.notify();
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        let is_focused = focus_handle.is_focused(window);
        let term_arc = Arc::clone(&self.terminal);
        let renderer = self.renderer.clone();
        let last_bounds = Arc::clone(&self.last_bounds);
        let padding = self.padding;
        let app_weak = self.app.clone();
        let pane_id = self.pane_id.clone();
        let resize_cb = self.resize_callback.clone();

        div()
            .size_full()
            .track_focus(&focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .child(
                canvas(
                    move |bounds, _window, _cx| {
                        *last_bounds.lock() = Some(bounds);
                        bounds
                    },
                    move |bounds, _, window, cx| {
                        // "M"-advance measure per paint (D2: 14px JetBrains Mono).
                        let mut measured = renderer.clone();
                        measured.measure_cell(window);

                        let avail_w: f32 =
                            (bounds.size.width - padding.left - padding.right).into();
                        let avail_h: f32 =
                            (bounds.size.height - padding.top - padding.bottom).into();
                        let cw: f32 = measured.cell_width.into();
                        let ch: f32 = measured.cell_height.into();

                        // Zero-size measures never arm (T-04-05): a container
                        // with no area must not shrink the shared viewport.
                        if avail_w <= 0.0 || avail_h <= 0.0 || cw <= 0.0 || ch <= 0.0 {
                            let term = term_arc.lock();
                            let raw_term_arc = term.term_arc();
                            let raw_term = raw_term_arc.lock();
                            measured.paint(bounds, padding, &raw_term, is_focused, window, cx);
                            return;
                        }

                        let cols = ((avail_w / cw).floor() as usize).max(1);
                        let rows = ((avail_h / ch).floor() as usize).max(1);

                        // Local resize applies immediately; on change the
                        // resize path fires and only ARMS (never sends).
                        let mut term = term_arc.lock();
                        if cols != term.cols() || rows != term.rows() {
                            term.resize(cols, rows);
                            drop(term);
                            if let Some(ref cb) = resize_cb {
                                cb(cols, rows);
                            } else {
                                let pane = pane_id.clone();
                                let _ = app_weak.update(cx, |app, cx| {
                                    let generation = app
                                        .owning_session(&pane)
                                        .and_then(|s| app.sessions.get(&s).map(|e| e.generation))
                                        .unwrap_or(0);
                                    app.pending_viewport = Some(PendingViewport {
                                        generation,
                                        cols,
                                        rows,
                                    });
                                    cx.notify();
                                });
                            }
                        } else {
                            drop(term);
                        }

                        let term = term_arc.lock();
                        let raw_term_arc = term.term_arc();
                        let raw_term = raw_term_arc.lock();
                        measured.paint(bounds, padding, &raw_term, is_focused, window, cx);
                    },
                )
                .size_full(),
            )
    }
}
