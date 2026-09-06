//! Phase 4 tracer (Task 1 RED): keystroke table + byte-mapping contracts.
//!
//! `test_keystroke_table` ports the reference `input.rs` table coverage
//! verbatim (enter/backspace/tab/arrows/APP_CURSOR/modified-arrows/nav/F-keys/
//! Ctrl-letter/Alt-ESC/key_char). `test_input_byte_roundtrip` locks the
//! STATE.md byte-mapping seam: every `keystroke_to_bytes` output plus a
//! CJK/emoji paste string must survive `String::from_utf8` exactly —
//! `from_utf8_lossy` is banned (fail loudly, never silently corrupt).

use alacritty_terminal::term::TermMode;
use gpui::{Keystroke, Modifiers};
use webtmux_terminal::keystroke_to_bytes;

fn ks(key: &str) -> Keystroke {
    Keystroke {
        modifiers: Modifiers::default(),
        key: key.into(),
        key_char: None,
    }
}

fn ks_mods(key: &str, shift: bool, alt: bool, control: bool) -> Keystroke {
    Keystroke {
        modifiers: Modifiers {
            shift,
            alt,
            control,
            ..Default::default()
        },
        key: key.into(),
        key_char: None,
    }
}

fn ks_char(key: &str, ch: &str) -> Keystroke {
    Keystroke {
        modifiers: Modifiers::default(),
        key: key.into(),
        key_char: Some(ch.into()),
    }
}

// --- verbatim port of the reference input.rs tests -------------------------

#[test]
fn test_enter_and_backspace() {
    assert_eq!(
        keystroke_to_bytes(&ks("enter"), TermMode::empty()),
        Some(b"\r".to_vec())
    );
    assert_eq!(
        keystroke_to_bytes(&ks("backspace"), TermMode::empty()),
        Some(b"\x7f".to_vec())
    );
}

#[test]
fn test_tab_and_shift_tab() {
    assert_eq!(
        keystroke_to_bytes(&ks("tab"), TermMode::empty()),
        Some(b"\t".to_vec())
    );
    assert_eq!(
        keystroke_to_bytes(&ks_mods("tab", true, false, false), TermMode::empty()),
        Some(b"\x1b[Z".to_vec())
    );
}

#[test]
fn test_arrow_keys_normal_and_app_cursor() {
    assert_eq!(
        keystroke_to_bytes(&ks("up"), TermMode::empty()),
        Some(b"\x1b[A".to_vec())
    );
    assert_eq!(
        keystroke_to_bytes(&ks("up"), TermMode::APP_CURSOR),
        Some(b"\x1bOA".to_vec())
    );
    assert_eq!(
        keystroke_to_bytes(&ks("down"), TermMode::empty()),
        Some(b"\x1b[B".to_vec())
    );
    assert_eq!(
        keystroke_to_bytes(&ks("down"), TermMode::APP_CURSOR),
        Some(b"\x1bOB".to_vec())
    );
}

#[test]
fn test_modified_arrows() {
    // mod_code = 1 + 4 = 5
    assert_eq!(
        keystroke_to_bytes(&ks_mods("up", false, false, true), TermMode::empty()),
        Some(b"\x1b[1;5A".to_vec())
    );
}

#[test]
fn test_ctrl_c_and_ctrl_d() {
    assert_eq!(
        keystroke_to_bytes(&ks_mods("c", false, false, true), TermMode::empty()),
        Some(vec![0x03])
    );
    assert_eq!(
        keystroke_to_bytes(&ks_mods("d", false, false, true), TermMode::empty()),
        Some(vec![0x04])
    );
}

#[test]
fn test_function_and_navigation_keys() {
    assert_eq!(
        keystroke_to_bytes(&ks("f1"), TermMode::empty()),
        Some(b"\x1bOP".to_vec())
    );
    assert_eq!(
        keystroke_to_bytes(&ks("pageup"), TermMode::empty()),
        Some(b"\x1b[5~".to_vec())
    );
}

// --- full-table contract (D-table) ------------------------------------------

#[test]
fn test_keystroke_table() {
    let empty = TermMode::empty();
    // enter / backspace / tab / S-tab / space / escape
    assert_eq!(keystroke_to_bytes(&ks("enter"), empty), Some(b"\r".to_vec()));
    assert_eq!(
        keystroke_to_bytes(&ks("backspace"), empty),
        Some(b"\x7f".to_vec())
    );
    assert_eq!(keystroke_to_bytes(&ks("tab"), empty), Some(b"\t".to_vec()));
    assert_eq!(
        keystroke_to_bytes(&ks_mods("tab", true, false, false), empty),
        Some(b"\x1b[Z".to_vec())
    );
    assert_eq!(keystroke_to_bytes(&ks("space"), empty), Some(b" ".to_vec()));
    assert_eq!(
        keystroke_to_bytes(&ks("escape"), empty),
        Some(b"\x1b".to_vec())
    );

    // arrows normal + APP_CURSOR variants
    for (key, normal, app) in [
        ("up", b"\x1b[A".as_slice(), b"\x1bOA".as_slice()),
        ("down", b"\x1b[B".as_slice(), b"\x1bOB".as_slice()),
        ("right", b"\x1b[C".as_slice(), b"\x1bOC".as_slice()),
        ("left", b"\x1b[D".as_slice(), b"\x1bOD".as_slice()),
    ] {
        assert_eq!(keystroke_to_bytes(&ks(key), empty), Some(normal.to_vec()));
        assert_eq!(
            keystroke_to_bytes(&ks(key), TermMode::APP_CURSOR),
            Some(app.to_vec())
        );
    }

    // modified arrows: ESC[1;<mod> (shift=2, alt=3, ctrl=5, shift+ctrl=6)
    assert_eq!(
        keystroke_to_bytes(&ks_mods("up", true, false, false), empty),
        Some(b"\x1b[1;2A".to_vec())
    );
    assert_eq!(
        keystroke_to_bytes(&ks_mods("down", false, true, false), empty),
        Some(b"\x1b[1;3B".to_vec())
    );
    assert_eq!(
        keystroke_to_bytes(&ks_mods("left", false, false, true), empty),
        Some(b"\x1b[1;5D".to_vec())
    );
    assert_eq!(
        keystroke_to_bytes(&ks_mods("right", true, false, true), empty),
        Some(b"\x1b[1;6C".to_vec())
    );

    // navigation cluster
    for (key, seq) in [
        ("home", b"\x1b[H".as_slice()),
        ("end", b"\x1b[F".as_slice()),
        ("pageup", b"\x1b[5~".as_slice()),
        ("pagedown", b"\x1b[6~".as_slice()),
        ("insert", b"\x1b[2~".as_slice()),
        ("delete", b"\x1b[3~".as_slice()),
    ] {
        assert_eq!(keystroke_to_bytes(&ks(key), empty), Some(seq.to_vec()), "{key}");
    }

    // F1–F12
    for (key, seq) in [
        ("f1", b"\x1bOP".as_slice()),
        ("f2", b"\x1bOQ".as_slice()),
        ("f3", b"\x1bOR".as_slice()),
        ("f4", b"\x1bOS".as_slice()),
        ("f5", b"\x1b[15~".as_slice()),
        ("f6", b"\x1b[17~".as_slice()),
        ("f7", b"\x1b[18~".as_slice()),
        ("f8", b"\x1b[19~".as_slice()),
        ("f9", b"\x1b[20~".as_slice()),
        ("f10", b"\x1b[21~".as_slice()),
        ("f11", b"\x1b[23~".as_slice()),
        ("f12", b"\x1b[24~".as_slice()),
    ] {
        assert_eq!(keystroke_to_bytes(&ks(key), empty), Some(seq.to_vec()), "{key}");
    }

    // Ctrl-letter math: upper - '@'
    assert_eq!(
        keystroke_to_bytes(&ks_mods("a", false, false, true), empty),
        Some(vec![0x01])
    );
    assert_eq!(
        keystroke_to_bytes(&ks_mods("z", false, false, true), empty),
        Some(vec![0x1a])
    );

    // Alt-ESC-prefix
    assert_eq!(
        keystroke_to_bytes(&ks_mods("x", false, true, false), empty),
        Some(vec![0x1b, b'x'])
    );

    // key_char passthrough (incl. CJK)
    assert_eq!(
        keystroke_to_bytes(&ks_char("a", "a"), empty),
        Some(b"a".to_vec())
    );
    assert_eq!(
        keystroke_to_bytes(&ks_char("\u{4e2d}", "\u{4e2d}"), empty),
        Some("\u{4e2d}".as_bytes().to_vec())
    );
}

// --- byte-mapping seam (STATE.md blocker) ------------------------------------

#[test]
fn test_input_byte_roundtrip() {
    let empty = TermMode::empty();
    let cases: Vec<Keystroke> = vec![
        ks("enter"),
        ks("backspace"),
        ks("tab"),
        ks_mods("tab", true, false, false),
        ks("space"),
        ks("escape"),
        ks("up"),
        ks("down"),
        ks("left"),
        ks("right"),
        ks("home"),
        ks("end"),
        ks("pageup"),
        ks("pagedown"),
        ks("insert"),
        ks("delete"),
        ks("f1"),
        ks("f5"),
        ks("f12"),
        ks_mods("up", false, false, true),
        ks_mods("c", false, false, true),
        ks_mods("x", false, true, false),
        ks_char("a", "a"),
        ks_char("\u{4e2d}", "\u{4e2d}"),
    ];
    for k in &cases {
        if let Some(bytes) = keystroke_to_bytes(k, empty) {
            String::from_utf8(bytes).expect("keystroke bytes must be valid UTF-8");
        }
        // APP_CURSOR variants must round-trip too.
        if let Some(bytes) = keystroke_to_bytes(k, TermMode::APP_CURSOR) {
            String::from_utf8(bytes).expect("app-cursor bytes must be valid UTF-8");
        }
    }
    // CJK/emoji paste round-trips exactly (Go []byte(string) parity).
    let paste = "hello \u{4e2d}\u{6587} 🎉🚀 \u{0009}tab";
    assert_eq!(
        String::from_utf8(paste.as_bytes().to_vec()).expect("paste must round-trip"),
        paste
    );
}
