//! Phase 4 tracer (Task 1 RED): apply_capture + live-output contracts.
//!
//! Split/legacy/shrank-history/unicode-CRLF replay semantics per
//! 04-RESEARCH §Technical Mechanics 2, plus TERM-01 live-output parity:
//! a scripted byte stream through `Processor` must yield the recorded grid.

use webtmux_terminal::{apply_capture, Terminal};

fn vim_fixture() -> (String, i32) {
    let raw = include_str!("fixtures/terminal_capture_vim.json");
    let v: serde_json::Value = serde_json::from_str(raw).expect("vim fixture parses");
    (
        v["data"].as_str().expect("data").to_string(),
        v["screen_rows"].as_i64().expect("screen_rows") as i32,
    )
}

#[test]
fn test_apply_capture_split() {
    let (data, screen_rows) = vim_fixture();
    let total_lines = data.replace("\r\n", "\n").replace('\r', "\n").split('\n').count();
    let mut ingested = 0usize;
    let feed = apply_capture(&data, Some(screen_rows), &mut ingested);

    // History delta above ingestedHistory feeds once (CRLF-joined + trailing CRLF).
    let history_len = total_lines - screen_rows as usize;
    assert_eq!(ingested, history_len, "ingestedHistory advances by the delta");

    let text = String::from_utf8(feed).expect("feed is valid UTF-8");
    let lines: Vec<&str> = data.split('\n').collect();
    let delta = lines[..history_len].join("\r\n") + "\r\n";
    let screen = &lines[history_len..];
    let mut expected = delta;
    expected.push_str("\x1b[2J\x1b[H");
    for (i, row) in screen.iter().enumerate() {
        expected.push_str(&format!("\x1b[{};1H{}", i + 1, row));
    }
    assert_eq!(text, expected);

    // Repeating the identical frame feeds no history twice (function-level
    // exactly-once: delta above the counter is empty).
    let again = apply_capture(&data, Some(screen_rows), &mut ingested);
    let again_text = String::from_utf8(again).expect("feed is valid UTF-8");
    assert!(
        !again_text.starts_with(&lines[..history_len].join("\r\n")),
        "repeat frame must not re-feed history"
    );
    assert_eq!(ingested, history_len);
}

#[test]
fn test_apply_capture_legacy() {
    let (data, _) = vim_fixture();
    let lines: Vec<&str> = data.split('\n').collect();
    let mut expected = "\x1b[2J\x1b[H".to_string();
    for (i, row) in lines.iter().enumerate() {
        expected.push_str(&format!("\x1b[{};1H{}", i + 1, row));
    }
    for rows in [None, Some(0), Some(-3)] {
        let mut ingested = 0usize;
        let feed = apply_capture(&data, rows, &mut ingested);
        assert_eq!(
            String::from_utf8(feed).expect("feed is valid UTF-8"),
            expected,
            "legacy branch for screenRows={rows:?}"
        );
        assert_eq!(ingested, 0, "legacy branch leaves the counter alone");
    }
}

#[test]
fn test_apply_capture_shrank_history() {
    let (data, screen_rows) = vim_fixture();
    let mut ingested = 999usize; // history shorter than the counter: shrank
    let feed = apply_capture(&data, Some(screen_rows), &mut ingested);
    assert_eq!(ingested, 0, "shrank history resets the counter to 0");

    // Screen still paints precisely.
    let lines: Vec<&str> = data.split('\n').collect();
    let screen = &lines[lines.len() - screen_rows as usize..];
    let mut expected = "\x1b[2J\x1b[H".to_string();
    for (i, row) in screen.iter().enumerate() {
        expected.push_str(&format!("\x1b[{};1H{}", i + 1, row));
    }
    assert_eq!(String::from_utf8(feed).expect("feed is valid UTF-8"), expected);
}

#[test]
fn test_apply_capture_unicode_crlf() {
    let raw = include_str!("fixtures/terminal_capture_unicode.json");
    let v: serde_json::Value = serde_json::from_str(raw).expect("unicode fixture parses");
    let data = v["data"].as_str().expect("data");
    let screen_rows = v["screen_rows"].as_i64().expect("screen_rows") as i32;

    let mut ingested = 0usize;
    let feed = apply_capture(data, Some(screen_rows), &mut ingested);
    let text = String::from_utf8(feed.clone()).expect("feed is valid UTF-8");
    // Runes survive the split (char-boundary only, never byte-sliced).
    for rune in ["こんにちは世界", "こんばんは", "🎉🚀", "🖥️", "行"] {
        assert!(text.contains(rune), "feed must preserve {rune}");
    }
    assert!(text.contains("\x1b[2J\x1b[H"), "screen repaint present");
    assert!(ingested > 0, "unicode history advances the counter");

    // The produced feed itself parses as UTF-8 and feeds the grid without panic.
    let mut term = Terminal::new(80, 24);
    term.process_bytes(&feed);
}

#[test]
fn test_live_output_parity() {
    let raw = include_str!("fixtures/terminal_output_frames.json");
    let v: serde_json::Value = serde_json::from_str(raw).expect("frames fixture parses");
    let cols = v["cols"].as_u64().expect("cols") as usize;
    let rows = v["rows"].as_u64().expect("rows") as usize;
    let chunks: Vec<String> = serde_json::from_value(v["chunks"].clone()).expect("chunks");
    let expected: Vec<String> =
        serde_json::from_value(v["expected_grid"].clone()).expect("expected_grid");

    let mut term = Terminal::new(cols, rows);
    for chunk in &chunks {
        term.process_bytes(chunk.as_bytes());
    }
    let grid = term.grid_text();
    for (i, want) in expected.iter().enumerate() {
        assert_eq!(&grid[i], want, "grid row {i} matches the recorded frame");
    }

    // vim-like alt-screen segment: enter, paint a marker, leave, content back.
    term.process_bytes(b"\x1b[?1049h\x1b[HALT-SCREEN-MARKER");
    assert!(term.is_alternate_screen(), "alt-screen entered");
    assert!(
        term.grid_text()[0].starts_with("ALT-SCREEN-MARKER"),
        "alt-screen marker paints"
    );
    term.process_bytes(b"\x1b[?1049l");
    assert!(!term.is_alternate_screen(), "alt-screen left");
    let grid = term.grid_text();
    for (i, want) in expected.iter().enumerate() {
        assert_eq!(&grid[i], want, "main grid restored after alt-screen");
    }
}
