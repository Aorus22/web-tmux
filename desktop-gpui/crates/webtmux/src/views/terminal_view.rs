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

use crate::app_state::{decide_key_route, AppState, KeyRoute, WheelDelta};
use gpui::*;
use parking_lot::Mutex;
use std::sync::Arc;
use webtmux_terminal::{
    keystroke_to_bytes, modifiers_to_mouse_code, mouse_button_report, pixel_to_cell_with_side,
    selection_type_from_clicks, AlacPoint, ColorPalette, Column, Line, Side, Terminal,
    TerminalRenderer,
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
    /// Whether font metrics in `renderer` have been measured from the text
    /// system. Font measurement requires a `&mut Window`, which is only
    /// available in `Render::render` — so measurement is deferred to the next
    /// frame after construction or any `set_*` mutation.
    renderer_needs_measure: bool,
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
        // Base renderer settings; the creation site (pane grid) immediately
        // overrides palette + font from the live prefs, so spawned panes
        // match the active theme instead of staying dark.
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
            renderer_needs_measure: true,
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
        self.renderer_needs_measure = true;
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
    ///
    /// Cell dimensions are set to a rough estimate and re-measured from the
    /// text system on the next render pass, so mouse hit-testing and painting
    /// always share the same real metrics.
    pub fn set_font_size(&mut self, size: Pixels, cx: &mut Context<Self>) {
        self.renderer.font_size = size;
        self.renderer.cell_width = size * 0.6;
        self.renderer.cell_height = size * self.renderer.line_height_multiplier;
        self.renderer_needs_measure = true;
        cx.notify();
    }

    /// Dynamically update font family and font size used by this terminal view.
    ///
    /// Cell dimensions are set to a rough estimate and re-measured from the
    /// text system on the next render pass, so mouse hit-testing and painting
    /// always share the same real metrics.
    pub fn set_font(&mut self, family: String, size: Pixels, cx: &mut Context<Self>) {
        self.renderer.font_family = family;
        self.renderer.font_size = size;
        self.renderer.cell_width = size * 0.6;
        self.renderer.cell_height = size * self.renderer.line_height_multiplier;
        self.renderer_needs_measure = true;
        cx.notify();
    }

    /// Convert a window pixel position into a terminal grid point plus the
    /// cell side under the pointer.
    ///
    /// Lazily refreshes `self.renderer`'s font metrics, then maps pixels to
    /// grid cells using the exact geometry the paint pass uses — including the
    /// scrollback offset, because the renderer shifts screen rows back into
    /// history while the viewport is scrolled.
    fn pixel_to_term_point(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
    ) -> (AlacPoint, Side) {
        if self.renderer_needs_measure {
            self.renderer.measure_cell(window);
            self.renderer_needs_measure = false;
        }
        self.pixel_to_term_point_measured(position)
    }

    /// Like [`Self::pixel_to_term_point`], but uses the renderer's current
    /// metrics without re-measuring. Safe once at least one paint pass has
    /// synced the measured metrics (the paint pass syncs them every frame).
    fn pixel_to_term_point_measured(&self, position: Point<Pixels>) -> (AlacPoint, Side) {
        let bounds = *self.last_bounds.lock();
        let Some(bounds) = bounds else {
            return (AlacPoint::new(Line(0), Column(0)), Side::Left);
        };

        let origin = Point {
            x: bounds.origin.x + self.padding.left,
            y: bounds.origin.y + self.padding.top,
        };

        let term = self.terminal.lock();
        let (viewport_point, side) = pixel_to_cell_with_side(
            position,
            origin,
            self.renderer.cell_width,
            self.renderer.cell_height,
            term.cols(),
            term.rows(),
        );
        // The paint pass draws screen row `i` from grid line `i - offset`, so
        // hit-testing has to name the same line: selection anchors, word/line
        // selection and copying all run on grid coordinates. This is exactly
        // alacritty's `viewport_to_point` subtraction.
        let display_offset = term.with_term(|t| t.grid().display_offset()) as i32;
        let grid_point = AlacPoint::new(
            Line(viewport_point.line.0 - display_offset),
            viewport_point.column,
        );
        (grid_point, side)
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

    /// Whether the current selection holds any copyable text.
    pub fn has_selection(&self) -> bool {
        self.terminal
            .lock()
            .selection_text()
            .is_some_and(|text| !text.is_empty())
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

    /// Copy the selection and clear it ("cut").
    ///
    /// A terminal has no writable buffer to remove text from, so the visible
    /// result of a cut is the copied text leaving the screen selection — which
    /// is also what `Put`-less terminals do for this entry.
    pub fn cut_selection(&self, cx: &mut Context<Self>) -> bool {
        if !self.copy_selection(cx) {
            return false;
        }
        self.terminal.lock().clear_selection();
        cx.notify();
        true
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
        // Pitfall 1: Ctrl+Shift+P must bubble to `TogglePalette` instead of
        // entering the pty — early-return before any byte conversion so the
        // palette opens from terminal focus (DLG-01 per D5).
        if crate::views::palette::is_palette_keystroke(
            event.keystroke.modifiers.control,
            event.keystroke.modifiers.shift,
            event.keystroke.modifiers.platform,
            &event.keystroke.key,
        ) {
            return;
        }
        // Clipboard routing via the headless-tested helper (TERM-05):
        // Ctrl+Shift+C / Cmd+C with a selection copies, Ctrl+Shift+V /
        // Cmd+V pastes, everything else (incl. Ctrl+C with an empty
        // selection) falls through to the interrupt byte path below.
        let has_selection = self.has_selection();
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

        // Plain right-click belongs to the context menu
        // (`views::terminal_context_menu`), so it is not forwarded to the
        // program; Shift+right-click still reaches a mouse-reporting app
        // through the `mouse_button_report` path below.
        if event.button == MouseButton::Right && !event.modifiers.shift {
            cx.notify();
            return;
        }

        if self.last_bounds.lock().is_some() {
            let (pt, side) = self.pixel_to_term_point(event.position, window);

            let mut term = self.terminal.lock();
            let mode = term.mode();
            let mouse_mods = modifiers_to_mouse_code(&event.modifiers);

            if let Some(bytes) = mouse_button_report(event.button, true, pt, mouse_mods, mode) {
                drop(term);
                self.write_to_pty(&bytes, cx);
            } else if event.button == MouseButton::Left {
                let sel_type = selection_type_from_clicks(event.click_count);
                term.start_selection(pt, side, sel_type);
                self.is_selecting = true;
            }
        }

        cx.notify();
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.last_bounds.lock().is_some() {
            let (pt, side) = self.pixel_to_term_point(event.position, window);

            let mut term = self.terminal.lock();
            if self.is_selecting {
                term.update_selection(pt, side);
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

    /// Window-level drag continuation for text selection.
    ///
    /// Registered from the canvas paint pass via `window.on_mouse_event` while
    /// a selection drag is active, so pointer movement outside the terminal
    /// bounds (e.g. over the sidebar) keeps updating the selection. X11 and
    /// Wayland both deliver pointer events to the window during an implicit
    /// button-press grab — including the release — so the drag always
    /// terminates on mouse up.
    fn on_drag_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.is_selecting {
            let (pt, side) = self.pixel_to_term_point_measured(event.position);
            self.terminal.lock().update_selection(pt, side);
            cx.notify();
        }
    }

    /// Window-level counterpart of [`Self::on_drag_mouse_move`]: ends any
    /// active selection drag no matter where the pointer was released.
    fn on_drag_mouse_up(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
        if event.button == MouseButton::Left && self.is_selecting {
            self.is_selecting = false;
            cx.notify();
        }
    }

    fn on_mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Right-click release: the menu owns the plain right-click (see
        // `on_mouse_down`), so only the Shift passthrough is forwarded.
        if event.button == MouseButton::Right && !event.modifiers.shift {
            cx.notify();
            return;
        }

        if self.last_bounds.lock().is_some() {
            let (pt, _) = self.pixel_to_term_point(event.position, window);

            let term = self.terminal.lock();
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
        window: &mut Window,
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
        let (pt, _) = self.pixel_to_term_point(event.position, window);
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
        // Layout-key resync hook (TERM-06): first mount records and skips,
        // later topology changes schedule the 150/325ms pair. Dedupe inside
        // `observe_layout_key_and_schedule` keeps per-pane renders to one
        // schedule per key change.
        let _ = self.app.update(cx, |app, cx| {
            app.observe_layout_key_and_schedule(cx);
        });
        let focus_handle = self.focus_handle.clone();
        let is_focused = focus_handle.is_focused(window);
        let term_arc = Arc::clone(&self.terminal);
        let renderer = self.renderer.clone();
        let last_bounds = Arc::clone(&self.last_bounds);
        let padding = self.padding;
        let app_weak = self.app.clone();
        let pane_id = self.pane_id.clone();
        // The paint closure below is `move` and takes the pane/app handles, so
        // keep a pair for the context-menu wrapper at the end.
        let menu_pane_id = pane_id.clone();
        let menu_app_weak = app_weak.clone();
        let resize_cb = self.resize_callback.clone();
        let view_handle = cx.entity().downgrade();
        let is_selecting = self.is_selecting;

        let root = div()
            // Stable id: the right-click menu derives its open state from it
            // (`ContextMenuExt`), so poll-tick re-renders must keep it identical.
            .id(format!("terminal-root/{}", pane_id))
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

                        // Keep the view's renderer in sync with the measured
                        // cell metrics so mouse hit-testing uses exactly the
                        // same geometry as this paint pass.
                        let _ = view_handle.update(cx, |this, cx| {
                            if this.renderer_needs_measure
                                || this.renderer.cell_width != measured.cell_width
                                || this.renderer.cell_height != measured.cell_height
                            {
                                this.renderer.cell_width = measured.cell_width;
                                this.renderer.cell_height = measured.cell_height;
                                this.renderer_needs_measure = false;
                                cx.notify();
                            }
                        });

                        // While a selection drag is active, listen for mouse
                        // move/up at the window level so the drag continues
                        // and terminates even when the pointer leaves the
                        // terminal bounds (e.g. over the sidebar).
                        if is_selecting {
                            let drag_view = view_handle.clone();
                            window.on_mouse_event(
                                move |event: &MouseMoveEvent, _phase, _window, cx| {
                                    let _ = drag_view.update(cx, |this, cx| {
                                        this.on_drag_mouse_move(event, cx);
                                    });
                                },
                            );
                            let drag_view = view_handle.clone();
                            window.on_mouse_event(
                                move |event: &MouseUpEvent, _phase, _window, cx| {
                                    let _ = drag_view.update(cx, |this, cx| {
                                        this.on_drag_mouse_up(event, cx);
                                    });
                                },
                            );
                        }

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
                        // resize path fires and only ARMS (never sends —
                        // Pitfall 4: the 100ms AppState debounce owns sends).
                        let mut term = term_arc.lock();
                        if cols != term.cols() || rows != term.rows() {
                            term.resize(cols, rows);
                            drop(term);
                            if let Some(ref cb) = resize_cb {
                                cb(cols, rows);
                            } else {
                                let pane = pane_id.clone();
                                let _ = app_weak.update(cx, |app, cx| {
                                    if app.arm_viewport_for_pane(&pane, cols, rows).is_some() {
                                        app.schedule_debounced_resize(cx);
                                    }
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
            );

        // Right-click Copy / Cut / Paste on the pane's terminal.
        crate::views::terminal_context_menu::with_terminal_context_menu(
            root,
            menu_pane_id,
            menu_app_weak,
        )
        .into_any_element()
    }
}
