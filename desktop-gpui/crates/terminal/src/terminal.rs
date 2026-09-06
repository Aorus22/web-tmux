//! Terminal state management wrapping alacritty_terminal::Term.

use crate::event::{GpuiEventProxy, TerminalEvent};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point as AlacPoint};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;
use parking_lot::Mutex;
use std::sync::Arc;

/// Dimensions adapter for alacritty Term initialization and resize.
#[derive(Debug, Clone, Copy)]
pub struct TermDimensions {
    columns: usize,
    screen_lines: usize,
}

impl TermDimensions {
    pub fn new(columns: usize, screen_lines: usize) -> Self {
        Self {
            columns: columns.max(1),
            screen_lines: screen_lines.max(1),
        }
    }
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.screen_lines
    }

    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    fn columns(&self) -> usize {
        self.columns
    }

    fn last_column(&self) -> Column {
        Column(self.columns.saturating_sub(1))
    }
}

/// Configuration options for the terminal emulator.
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    pub scrollback_limit: usize,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        // D1 (Phase 4): FE settings default (settingsStore.ts:40), not the
        // reference's 10_000 — 1:1 scrollback depth with Electron until
        // Phase 6 (SET-02) wires real terminal prefs.
        Self {
            scrollback_limit: 2000,
        }
    }
}

/// Core terminal engine managing grid, cursor, selection, and VTE byte processing.
pub struct Terminal {
    term: Arc<Mutex<Term<GpuiEventProxy>>>,
    parser: Processor,
    cols: usize,
    rows: usize,
    event_rx: flume::Receiver<TerminalEvent>,
}

impl Terminal {
    /// Create a new terminal instance with specified columns and rows.
    pub fn new(cols: usize, rows: usize) -> Self {
        Self::with_config(cols, rows, TerminalConfig::default())
    }

    /// Create a new terminal instance with custom configuration.
    pub fn with_config(cols: usize, rows: usize, config: TerminalConfig) -> Self {
        let (event_tx, event_rx) = flume::unbounded();
        let event_proxy = GpuiEventProxy::new(event_tx);

        let term_config = Config {
            scrolling_history: config.scrollback_limit,
            ..Default::default()
        };

        let dimensions = TermDimensions::new(cols, rows);
        let term = Term::new(term_config, &dimensions, event_proxy);

        Self {
            term: Arc::new(Mutex::new(term)),
            parser: Processor::new(),
            cols,
            rows,
            event_rx,
        }
    }

    /// Process raw bytes from PTY / SSH stream into terminal grid.
    pub fn process_bytes(&mut self, bytes: &[u8]) {
        let mut term = self.term.lock();
        self.parser.advance(&mut *term, bytes);
    }

    /// Resize the terminal grid to new columns and rows.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        if cols == 0 || rows == 0 {
            return;
        }
        self.cols = cols;
        self.rows = rows;

        let dimensions = TermDimensions::new(cols, rows);
        let mut term = self.term.lock();
        term.resize(dimensions);
    }

    /// Get current column count.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get current row count.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Check if terminal is currently using the alternate screen (e.g. vim, htop).
    pub fn is_alternate_screen(&self) -> bool {
        self.term.lock().mode().contains(TermMode::ALT_SCREEN)
    }

    /// Get active terminal mode flags.
    pub fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }

    /// Scroll display by delta lines (+delta scrolls up into history, -delta scrolls down).
    pub fn scroll_display(&mut self, delta: i32) {
        let mut term = self.term.lock();
        term.scroll_display(Scroll::Delta(delta));
    }

    /// Jump viewport back to active terminal line (bottom).
    pub fn scroll_to_bottom(&mut self) {
        let mut term = self.term.lock();
        term.scroll_display(Scroll::Bottom);
    }

    /// Jump viewport to oldest line in scrollback.
    pub fn scroll_to_top(&mut self) {
        let mut term = self.term.lock();
        term.scroll_display(Scroll::Top);
    }

    /// Start a mouse text selection.
    pub fn start_selection(&mut self, point: AlacPoint, selection_type: SelectionType) {
        let mut term = self.term.lock();
        term.selection = Some(Selection::new(selection_type, point, alacritty_terminal::index::Side::Left));
    }

    /// Update an ongoing mouse text selection.
    pub fn update_selection(&mut self, point: AlacPoint) {
        let mut term = self.term.lock();
        if let Some(selection) = term.selection.as_mut() {
            selection.update(point, alacritty_terminal::index::Side::Right);
        }
    }

    /// Clear the active text selection.
    pub fn clear_selection(&mut self) {
        let mut term = self.term.lock();
        term.selection = None;
    }

    /// Extract the currently selected text as a String.
    pub fn selection_text(&self) -> Option<String> {
        let term = self.term.lock();
        term.selection_to_string()
    }

    /// Execute a closure with read access to the underlying Term.
    pub fn with_term<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Term<GpuiEventProxy>) -> R,
    {
        let term = self.term.lock();
        f(&term)
    }

    /// Execute a closure with mutable access to the underlying Term.
    pub fn with_term_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Term<GpuiEventProxy>) -> R,
    {
        let mut term = self.term.lock();
        f(&mut term)
    }

    /// Clone the Arc pointer to the terminal mutex.
    pub fn term_arc(&self) -> Arc<Mutex<Term<GpuiEventProxy>>> {
        Arc::clone(&self.term)
    }

    /// Get receiver for terminal events (redraws, titles, bells, exits).
    pub fn event_channel(&self) -> flume::Receiver<TerminalEvent> {
        self.event_rx.clone()
    }

    /// Headless grid dump: trimmed text per visible row (contract tests).
    pub fn grid_text(&self) -> Vec<String> {
        let term = self.term.lock();
        let grid = term.grid();
        let cols = grid.columns();
        (0..grid.screen_lines())
            .map(|r| {
                let line = Line(r as i32);
                let s: String = (0..cols)
                    .map(|c| {
                        let cell = &grid[AlacPoint::new(line, Column(c))];
                        if cell.c == '\0' { ' ' } else { cell.c }
                    })
                    .collect();
                s.trim_end().to_string()
            })
            .collect()
    }

    /// Drain pending terminal events without blocking (D8 title commits).
    pub fn drain_events(&self) -> Vec<TerminalEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.event_rx.try_recv() {
            out.push(ev);
        }
        out
    }
}
