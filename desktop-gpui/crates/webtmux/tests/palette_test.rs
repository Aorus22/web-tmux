//! DLG-01 command-palette model contracts (Phase 6 plan 06-02 Task 2, D5).
//!
//! The live view renders through the vendored `command` module (filter/focus/
//! keyboard owned upstream); these tests lock the pure model underneath:
//! exactly the 7 FE actions in FE order plus one entry per tree session,
//! case-insensitive label+keyword filtering, FE guard mirroring, the
//! terminal key-route, and the `ctrl-shift-p` keystroke parse (A2 —
//! `KeyBinding::new` panics on parse error).

use gpui::KeyBinding;
use webtmux::actions::TogglePalette;
use webtmux::views::palette::{
    is_palette_keystroke, open_session_items, palette_action_enabled, palette_action_items,
    palette_filter, PaletteActionId,
};

fn labels_for(query: &str) -> Vec<&'static str> {
    let items = palette_action_items();
    palette_filter(query, &items)
        .into_iter()
        .map(|i| i.label)
        .collect()
}

#[test]
fn test_palette_groups() {
    // Exactly the 7 FE action labels in FE `CommandPalette.tsx:48-97` order —
    // extras fail this test (v2 scope lock).
    let items = palette_action_items();
    let labels: Vec<_> = items.iter().map(|i| i.label).collect();
    assert_eq!(
        labels,
        vec![
            "New Session",
            "New Window",
            "Split Right",
            "Split Down",
            "Zoom Pane",
            "Next Layout",
            "Kill Pane",
        ]
    );

    // Open Session group: one entry per tree session, in tree order
    // (`CommandPalette.tsx:98-115`); empty tree yields no entries.
    let names = vec!["dev".to_string(), "ops".to_string()];
    assert_eq!(open_session_items(&names), vec!["dev", "ops"]);
    let empty: Vec<String> = Vec::new();
    assert!(open_session_items(&empty).is_empty());
}

#[test]
fn test_palette_filter() {
    // Substring over label + keywords, case-insensitive.
    assert_eq!(labels_for("split"), vec!["Split Right", "Split Down"]);
    assert_eq!(labels_for("SPLIT"), vec!["Split Right", "Split Down"]);
    assert_eq!(labels_for("kill"), vec!["Kill Pane"]);
    // Keyword-only hit (no label contains "horizontal").
    assert_eq!(labels_for("horizontal"), vec!["Split Right"]);
    assert_eq!(labels_for("layout"), vec!["Next Layout"]);
    // Empty query matches everything; nonsense matches nothing.
    assert_eq!(labels_for(""), labels_for(" "));
    assert_eq!(labels_for("").len(), 7);
    assert!(labels_for("zzz-no-match").is_empty());
}

#[test]
fn test_palette_guards() {
    // FE guard mirror (T-06-05): window actions need an active session, pane
    // actions need an active pane, New Session always applies.
    assert!(palette_action_enabled(PaletteActionId::NewSession, false, false));
    assert!(!palette_action_enabled(
        PaletteActionId::NewWindow,
        false,
        false
    ));
    assert!(palette_action_enabled(
        PaletteActionId::NewWindow,
        true,
        false
    ));
    assert!(!palette_action_enabled(
        PaletteActionId::NextLayout,
        false,
        true
    ));
    assert!(palette_action_enabled(
        PaletteActionId::NextLayout,
        true,
        false
    ));
    for id in [
        PaletteActionId::SplitRight,
        PaletteActionId::SplitDown,
        PaletteActionId::ZoomPane,
        PaletteActionId::KillPane,
    ] {
        assert!(
            !palette_action_enabled(id, true, false),
            "{id:?} must need an active pane"
        );
        assert!(
            palette_action_enabled(id, true, true),
            "{id:?} must apply with session + pane"
        );
    }
}

#[test]
fn test_palette_keystroke_route() {
    // Ctrl+Shift+P bubbles to TogglePalette (Pitfall 1); anything else —
    // including the copy/paste chords — keeps its existing route.
    assert!(is_palette_keystroke(true, true, false, "p"));
    assert!(is_palette_keystroke(true, true, false, "P"));
    assert!(!is_palette_keystroke(true, false, false, "p"));
    assert!(!is_palette_keystroke(false, true, false, "p"));
    assert!(!is_palette_keystroke(false, false, false, "p"));
    assert!(!is_palette_keystroke(true, true, false, "c"));
    assert!(!is_palette_keystroke(true, true, false, "v"));
}

#[test]
fn test_palette_keybinding_parses() {
    // A2 Wave-0 check: `KeyBinding::new` panics on parse error, so
    // constructing the exact registered binding proves `ctrl-shift-p` parses.
    let _ = KeyBinding::new("ctrl-shift-p", TogglePalette, None);
}
