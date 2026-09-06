//! Capture-replay → alacritty mapping (TERM-02 core).
//!
//! Byte-oriented port of FE `applyCapture`
//! (`fe/src/features/terminal/terminalRegistry.ts:31-65`): a capture blob is
//! `scrollback-history lines + visible screen rows` with `screenRows` telling
//! the client where the split is. History lines not yet ingested feed first
//! (alacritty pushes them into scrollback naturally), then a clear + home +
//! explicitly-positioned repaint of the visible grid.
//!
//! There is deliberately NO `pendingScreen`/`writingScreen` frame queue:
//! xterm's `write` is async (hence the FE queue) while alacritty's
//! `Processor::advance` is synchronous — in-order commits on the GPUI thread
//! give frame atomicity for free. The ordering invariant (one commit at a
//! time, pump order) is preserved by the `AppState` commit arms instead.

/// Turn one `terminal.snapshot` capture blob into the single atomic feed for
/// one frame, advancing `ingested_history` by the replayed delta.
///
/// - `screen_rows` non-positive (`None`/`<= 0`) → legacy branch: clear + home
///   + positioned repaint of ALL lines, counter untouched.
/// - Otherwise the blob splits at `len - rows`: lines above are history (only
///   the delta above `ingested_history` feeds), the tail is the screen.
/// - Shrank history (`history.len() < ingested_history`) resets the counter
///   to 0 (no scrollback-trim API exists) and still paints the screen.
///
/// Splitting uses char-boundary APIs only (`str::replace`, `str::split`) —
/// byte-slicing the blob would panic on UTF-8 boundaries (Pitfall 3).
pub fn apply_capture(
    data: &str,
    screen_rows: Option<i32>,
    ingested_history: &mut usize,
) -> Vec<u8> {
    // Normalize CRLF/CR → LF FIRST (FE `/\r\n?/g` parity), then split.
    let normalized = data.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.split('\n').collect();

    // Explicit origin per row: full-width rows would auto-wrap before CRLF
    // is consumed (registry.ts:41-45).
    fn positioned(rows: &[&str]) -> String {
        rows.iter()
            .enumerate()
            .map(|(i, row)| format!("\x1b[{};1H{}", i + 1, row))
            .collect::<Vec<_>>()
            .join("")
    }

    let rows = screen_rows.unwrap_or(0);
    if rows <= 0 {
        return format!("\x1b[2J\x1b[H{}", positioned(&lines)).into_bytes();
    }

    let split = lines.len().saturating_sub(rows as usize);
    let (history, screen) = lines.split_at(split);

    let mut payload = String::new();
    if history.len() < *ingested_history {
        *ingested_history = 0;
    } else {
        let delta = &history[*ingested_history..];
        if !delta.is_empty() {
            payload.push_str(&delta.join("\r\n"));
            payload.push_str("\r\n");
        }
        *ingested_history = history.len();
    }
    payload.push_str("\x1b[2J\x1b[H");
    payload.push_str(&positioned(screen));
    payload.into_bytes()
}
