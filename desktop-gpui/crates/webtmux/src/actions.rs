//! Keyboard shortcut action definitions for the command palette.
//!
//! Reference pattern (`web-term` `actions.rs:5-41`): one `actions!` group +
//! a `bind_*` helper registering `KeyBinding::new` strings. The `ctrl-shift-p`
//! string follows the same multi-modifier grammar as the reference
//! `ctrl-shift-tab`; `KeyBinding::new` panics on parse error, so the
//! headless `test_palette_keybinding_parses` contract locks A2.

use gpui::*;

actions!(webtmux_palette, [TogglePalette]);

/// Register the palette keybinding (Ctrl+Shift+P, FE `App.tsx:176-179`
/// parity via `useKeyboardShortcut`).
pub fn bind_palette_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-p", TogglePalette, None)]);
}
