//! Mouse event handling for the terminal emulator.
//!
//! Provides coordinate conversion, selection type determination, and SGR mouse reporting.

use alacritty_terminal::index::{Column, Line, Point as AlacPoint};
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::TermMode;
use gpui::{Modifiers, MouseButton, Pixels, Point};

/// Convert pixel coordinate (e.g. mouse position) to terminal grid (col, line).
pub fn pixel_to_cell(
    position: Point<Pixels>,
    origin: Point<Pixels>,
    cell_width: Pixels,
    cell_height: Pixels,
    max_cols: usize,
    max_rows: usize,
) -> AlacPoint {
    let col = ((position.x - origin.x) / cell_width).floor();
    let col = (col.max(0.0) as usize).min(max_cols.saturating_sub(1));

    let row = ((position.y - origin.y) / cell_height).floor();
    let row = (row.max(0.0) as i32).min(max_rows.saturating_sub(1) as i32);

    AlacPoint::new(Line(row), Column(col))
}

/// Map mouse click count into an Alacritty SelectionType (1 = simple, 2 = semantic word, 3+ = line).
pub fn selection_type_from_clicks(click_count: usize) -> SelectionType {
    match click_count {
        1 => SelectionType::Simple,
        2 => SelectionType::Semantic,
        _ => SelectionType::Lines,
    }
}

/// Convert GPUI modifier keys to X11/SGR mouse modifier bits.
/// Bit 2 (4) = Shift, Bit 3 (8) = Alt, Bit 4 (16) = Ctrl.
pub fn modifiers_to_mouse_code(modifiers: &Modifiers) -> u8 {
    let mut code = 0;
    if modifiers.shift {
        code |= 4;
    }
    if modifiers.alt {
        code |= 8;
    }
    if modifiers.control {
        code |= 16;
    }
    code
}

/// Generate SGR (1006) mouse button report escape sequences.
pub fn mouse_button_report(
    button: MouseButton,
    pressed: bool,
    point: AlacPoint,
    modifiers: u8,
    mode: TermMode,
) -> Option<Vec<u8>> {
    if !mode.intersects(TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_MOTION | TermMode::MOUSE_DRAG) {
        return None;
    }

    let button_code = match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        _ => return None,
    };

    let button_value = button_code | modifiers;
    let col = point.column.0 + 1;
    let row = point.line.0 + 1;
    let action = if pressed { b'M' } else { b'm' };

    let sequence = format!("\x1b[<{};{};{}{}", button_value, col, row, action as char);
    Some(sequence.into_bytes())
}

/// Generate scroll wheel report escape sequences.
pub fn scroll_report(
    delta: i32,
    point: AlacPoint,
    modifiers: u8,
    mode: TermMode,
) -> Option<Vec<u8>> {
    if mode.intersects(TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_MOTION | TermMode::MOUSE_DRAG) {
        // 64 = wheel up, 65 = wheel down
        let button_code = if delta > 0 { 64 } else { 65 };
        let button_value = button_code | modifiers;
        let col = point.column.0 + 1;
        let row = point.line.0 + 1;
        let sequence = format!("\x1b[<{};{};{}M", button_value, col, row);
        return Some(sequence.into_bytes());
    }

    // D5 (Phase 4): the FE wheel policy owns TUI paging (PageUp/PageDown
    // repeats via the AppState wheel path, plan 04-02) — the reference
    // ALT_SCREEN → arrow-key fallback is intentionally NOT wired into any
    // wheel path, so no second caller reaches for it here. Without mouse
    // reporting the caller falls back to scrollback scrolling.
    None
}

/// Dead-ported reference helper, intentionally uncalled (D5 — see
/// `scroll_report` above). Kept so the omission is explicit and reviewable
/// rather than a silent drift from the reference module.
#[allow(dead_code)]
fn scroll_to_arrow_keys(delta: i32, mode: TermMode) -> Vec<u8> {
    let count = delta.unsigned_abs().min(5) as usize;
    let arrow_seq: &[u8] = if delta > 0 {
        if mode.contains(TermMode::APP_CURSOR) {
            b"\x1bOA"
        } else {
            b"\x1b[A"
        }
    } else if mode.contains(TermMode::APP_CURSOR) {
        b"\x1bOB"
    } else {
        b"\x1b[B"
    };

    let mut result = Vec::with_capacity(arrow_seq.len() * count);
    for _ in 0..count {
        result.extend_from_slice(arrow_seq);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px};

    #[test]
    fn test_pixel_to_cell_calculation() {
        let pos = point(px(105.0), px(45.0));
        let origin = point(px(5.0), px(5.0));
        let cell_w = px(10.0);
        let cell_h = px(20.0);

        let pt = pixel_to_cell(pos, origin, cell_w, cell_h, 80, 24);
        assert_eq!(pt.column.0, 10);
        assert_eq!(pt.line.0, 2);
    }

    #[test]
    fn test_pixel_to_cell_clamping() {
        let pos = point(px(1000.0), px(1000.0));
        let origin = point(px(0.0), px(0.0));
        let cell_w = px(10.0);
        let cell_h = px(20.0);

        let pt = pixel_to_cell(pos, origin, cell_w, cell_h, 80, 24);
        assert_eq!(pt.column.0, 79);
        assert_eq!(pt.line.0, 23);
    }

    #[test]
    fn test_selection_type_mapping() {
        assert_eq!(selection_type_from_clicks(1), SelectionType::Simple);
        assert_eq!(selection_type_from_clicks(2), SelectionType::Semantic);
        assert_eq!(selection_type_from_clicks(3), SelectionType::Lines);
    }

    #[test]
    fn test_mouse_button_report_format() {
        let pt = AlacPoint::new(Line(4), Column(9));
        let mode = TermMode::MOUSE_REPORT_CLICK;

        let press = mouse_button_report(MouseButton::Left, true, pt, 0, mode).unwrap();
        assert_eq!(String::from_utf8(press).unwrap(), "\x1b[<0;10;5M");

        let release = mouse_button_report(MouseButton::Left, false, pt, 0, mode).unwrap();
        assert_eq!(String::from_utf8(release).unwrap(), "\x1b[<0;10;5m");
    }

    #[test]
    fn test_scroll_wheel_reporting() {
        let pt = AlacPoint::new(Line(0), Column(0));
        let mode = TermMode::MOUSE_REPORT_CLICK;

        let up = scroll_report(1, pt, 0, mode).unwrap();
        assert_eq!(String::from_utf8(up).unwrap(), "\x1b[<64;1;1M");

        let down = scroll_report(-1, pt, 0, mode).unwrap();
        assert_eq!(String::from_utf8(down).unwrap(), "\x1b[<65;1;1M");
    }

    #[test]
    fn test_scroll_no_fallback_without_mouse_mode() {
        // D5: without mouse reporting there is no ALT_SCREEN arrow fallback;
        // the wheel path (plan 04-02) owns TUI paging, scrollback is default.
        let pt = AlacPoint::new(Line(0), Column(0));
        assert_eq!(scroll_report(2, pt, 0, TermMode::ALT_SCREEN), None);
        assert_eq!(scroll_report(-1, pt, 0, TermMode::empty()), None);
    }
}
