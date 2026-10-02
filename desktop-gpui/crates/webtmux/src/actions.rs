//! Keyboard shortcut action definitions for the command palette and terminal.
//!
//! Reference pattern (`web-term` `actions.rs:5-41`): one `actions!` group +
//! a `bind_*` helper registering `KeyBinding::new` strings. The `ctrl-shift-p`
//! string follows the same multi-modifier grammar as the reference
//! `ctrl-shift-tab`; `KeyBinding::new` panics on parse error, so the
//! headless `test_palette_keybinding_parses` contract locks A2.

use gpui::*;

actions!(webtmux_palette, [TogglePalette]);
actions!(webtmux_terminal, [TerminalTab, TerminalShiftTab]);

/// Register the palette keybinding (Ctrl+Shift+P, FE `App.tsx:176-179`
/// parity via `useKeyboardShortcut`).
pub fn bind_palette_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-p", TogglePalette, None)]);
}

/// Register terminal Tab keybindings in the `Terminal` key context.
///
/// `gpui_component::Root` (which wraps the whole window) binds `tab` /
/// `shift-tab` to window focus navigation in its `Root` context. GPUI matches
/// keymap bindings BEFORE dispatching `on_key_down` listeners, and a matched
/// action stops propagation — so without a deeper binding the terminal never
/// sees Tab and shell completion silently does nothing. A binding on the
/// deeper `Terminal` context (set by `TerminalView`) takes precedence per the
/// dispatch-tree shadowing rule, while input/dialog focus navigation keeps
/// the `Root` binding everywhere else.
pub fn bind_terminal_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", TerminalTab, Some("Terminal")),
        KeyBinding::new("shift-tab", TerminalShiftTab, Some("Terminal")),
    ]);
}
