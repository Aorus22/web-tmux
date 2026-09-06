//! Keyboard input translation for the terminal emulator.
//!
//! Converts GPUI `Keystroke` events into ANSI/VT escape sequences to send to PTY / SSH.

use alacritty_terminal::term::TermMode;
use gpui::Keystroke;

/// Convert a GPUI keystroke into terminal escape sequence bytes.
pub fn keystroke_to_bytes(keystroke: &Keystroke, mode: TermMode) -> Option<Vec<u8>> {
    let key = keystroke.key.as_str();

    // Handle modified arrow keys (Shift, Alt, Ctrl)
    let has_modifiers = keystroke.modifiers.shift
        || keystroke.modifiers.alt
        || keystroke.modifiers.control;

    if has_modifiers && matches!(key, "up" | "down" | "right" | "left") {
        let mut mod_code = 1;
        if keystroke.modifiers.shift {
            mod_code += 1;
        }
        if keystroke.modifiers.alt {
            mod_code += 2;
        }
        if keystroke.modifiers.control {
            mod_code += 4;
        }
        let dir = match key {
            "up" => 'A',
            "down" => 'B',
            "right" => 'C',
            "left" => 'D',
            _ => unreachable!(),
        };
        return Some(format!("\x1b[1;{}{}", mod_code, dir).into_bytes());
    }

    // Special keys
    match key {
        "space" => {
            if keystroke.modifiers.control {
                return Some(vec![0x00]); // Ctrl+Space = NUL
            }
            return Some(b" ".to_vec());
        }
        "enter" => return Some(b"\r".to_vec()),
        "escape" => return Some(b"\x1b".to_vec()),
        "backspace" => return Some(b"\x7f".to_vec()),
        "tab" => {
            if keystroke.modifiers.shift {
                return Some(b"\x1b[Z".to_vec());
            }
            return Some(b"\t".to_vec());
        }

        // Arrow keys
        "up" => {
            if mode.contains(TermMode::APP_CURSOR) {
                return Some(b"\x1bOA".to_vec());
            }
            return Some(b"\x1b[A".to_vec());
        }
        "down" => {
            if mode.contains(TermMode::APP_CURSOR) {
                return Some(b"\x1bOB".to_vec());
            }
            return Some(b"\x1b[B".to_vec());
        }
        "right" => {
            if mode.contains(TermMode::APP_CURSOR) {
                return Some(b"\x1bOC".to_vec());
            }
            return Some(b"\x1b[C".to_vec());
        }
        "left" => {
            if mode.contains(TermMode::APP_CURSOR) {
                return Some(b"\x1bOD".to_vec());
            }
            return Some(b"\x1b[D".to_vec());
        }

        // Navigation keys
        "home" => return Some(b"\x1b[H".to_vec()),
        "end" => return Some(b"\x1b[F".to_vec()),
        "pageup" => return Some(b"\x1b[5~".to_vec()),
        "pagedown" => return Some(b"\x1b[6~".to_vec()),
        "insert" => return Some(b"\x1b[2~".to_vec()),
        "delete" => return Some(b"\x1b[3~".to_vec()),

        // Function keys
        "f1" => return Some(b"\x1bOP".to_vec()),
        "f2" => return Some(b"\x1bOQ".to_vec()),
        "f3" => return Some(b"\x1bOR".to_vec()),
        "f4" => return Some(b"\x1bOS".to_vec()),
        "f5" => return Some(b"\x1b[15~".to_vec()),
        "f6" => return Some(b"\x1b[17~".to_vec()),
        "f7" => return Some(b"\x1b[18~".to_vec()),
        "f8" => return Some(b"\x1b[19~".to_vec()),
        "f9" => return Some(b"\x1b[20~".to_vec()),
        "f10" => return Some(b"\x1b[21~".to_vec()),
        "f11" => return Some(b"\x1b[23~".to_vec()),
        "f12" => return Some(b"\x1b[24~".to_vec()),

        _ => {}
    }

    // Ctrl+key combinations
    if keystroke.modifiers.control && key.len() == 1 {
        let ch = key.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            let upper = ch.to_ascii_uppercase();
            let ctrl_byte = (upper as u8) - b'@';
            return Some(vec![ctrl_byte]);
        }

        match ch {
            '@' => return Some(vec![0x00]),
            '[' => return Some(vec![0x1b]),
            '\\' => return Some(vec![0x1c]),
            ']' => return Some(vec![0x1d]),
            '^' => return Some(vec![0x1e]),
            '_' => return Some(vec![0x1f]),
            '?' => return Some(vec![0x7f]),
            _ => {}
        }
    }

    // Alt+key combinations
    if keystroke.modifiers.alt && key.len() == 1 {
        let ch = key.chars().next().unwrap();
        if ch.is_ascii() {
            return Some(vec![0x1b, ch as u8]);
        }
    }

    // Typed character via key_char
    if !keystroke.modifiers.control && !keystroke.modifiers.alt {
        if let Some(key_char) = &keystroke.key_char {
            return Some(key_char.as_bytes().to_vec());
        }
    }

    // Single ASCII / UTF-8 fallback
    if key.len() == 1 && !keystroke.modifiers.control {
        let ch = key.chars().next().unwrap();
        let ch = if keystroke.modifiers.shift {
            ch.to_ascii_uppercase()
        } else {
            ch
        };
        return Some(ch.to_string().into_bytes());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Modifiers;

    #[test]
    fn test_enter_and_backspace() {
        let enter = Keystroke {
            modifiers: Modifiers::default(),
            key: "enter".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&enter, TermMode::empty()), Some(b"\r".to_vec()));

        let backspace = Keystroke {
            modifiers: Modifiers::default(),
            key: "backspace".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&backspace, TermMode::empty()), Some(b"\x7f".to_vec()));
    }

    #[test]
    fn test_tab_and_shift_tab() {
        let tab = Keystroke {
            modifiers: Modifiers::default(),
            key: "tab".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&tab, TermMode::empty()), Some(b"\t".to_vec()));

        let shift_tab = Keystroke {
            modifiers: Modifiers {
                shift: true,
                ..Default::default()
            },
            key: "tab".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&shift_tab, TermMode::empty()), Some(b"\x1b[Z".to_vec()));
    }

    #[test]
    fn test_arrow_keys_normal_and_app_cursor() {
        let up = Keystroke {
            modifiers: Modifiers::default(),
            key: "up".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&up, TermMode::empty()), Some(b"\x1b[A".to_vec()));
        assert_eq!(keystroke_to_bytes(&up, TermMode::APP_CURSOR), Some(b"\x1bOA".to_vec()));

        let down = Keystroke {
            modifiers: Modifiers::default(),
            key: "down".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&down, TermMode::empty()), Some(b"\x1b[B".to_vec()));
        assert_eq!(keystroke_to_bytes(&down, TermMode::APP_CURSOR), Some(b"\x1bOB".to_vec()));
    }

    #[test]
    fn test_modified_arrows() {
        let ctrl_up = Keystroke {
            modifiers: Modifiers {
                control: true,
                ..Default::default()
            },
            key: "up".into(),
            key_char: None,
        };
        // mod_code = 1 + 4 = 5
        assert_eq!(keystroke_to_bytes(&ctrl_up, TermMode::empty()), Some(b"\x1b[1;5A".to_vec()));
    }

    #[test]
    fn test_ctrl_c_and_ctrl_d() {
        let ctrl_c = Keystroke {
            modifiers: Modifiers {
                control: true,
                ..Default::default()
            },
            key: "c".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&ctrl_c, TermMode::empty()), Some(vec![0x03]));

        let ctrl_d = Keystroke {
            modifiers: Modifiers {
                control: true,
                ..Default::default()
            },
            key: "d".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&ctrl_d, TermMode::empty()), Some(vec![0x04]));
    }

    #[test]
    fn test_function_and_navigation_keys() {
        let f1 = Keystroke {
            modifiers: Modifiers::default(),
            key: "f1".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&f1, TermMode::empty()), Some(b"\x1bOP".to_vec()));

        let pageup = Keystroke {
            modifiers: Modifiers::default(),
            key: "pageup".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&pageup, TermMode::empty()), Some(b"\x1b[5~".to_vec()));
    }
}
