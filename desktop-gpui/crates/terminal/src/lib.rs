//! webtmux-terminal: terminal emulation engine (alacritty_terminal + GPUI).
//!
//! Port of the web-term `terminal` crate module split with Phase-4 deltas:
//! D1 scrollback 2000 (`terminal.rs`), D2 fixed dark palette default
//! (`colors.rs`), D5 no ALT_SCREEN arrow fallback in the wheel path
//! (`mouse.rs`), no async frame queue (`capture.rs`). No `view` module —
//! `TerminalView` lives in the `webtmux` app crate over the `AppState` store
//! (hidden sessions unmount views but must keep ingesting, Pitfall 6).

pub mod capture;
pub mod colors;
pub mod event;
pub mod input;
pub mod mouse;
pub mod render;
pub mod terminal;

pub use capture::apply_capture;
pub use colors::ColorPalette;
pub use event::{GpuiEventProxy, TerminalEvent};
pub use input::keystroke_to_bytes;
pub use mouse::{
    modifiers_to_mouse_code, mouse_button_report, pixel_to_cell, scroll_report,
    selection_type_from_clicks,
};
pub use render::{BackgroundRect, BatchedTextRun, CellDimensions, TerminalRenderer};
pub use terminal::{TermDimensions, Terminal, TerminalConfig};

// Re-exported alacritty geometry/mode types so app crates can name them
// without adding a direct dependency (pins unchanged, no new packages).
pub use alacritty_terminal::index::{Column, Line, Point as AlacPoint};
pub use alacritty_terminal::term::TermMode;
