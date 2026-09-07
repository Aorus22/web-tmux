//! Command palette on Ctrl+Shift+P (Phase 6 plan 06-02 Task 2, DLG-01 per D5).
//!
//! Built on the vendored `gpui-component 0.6.0` `command` module — never a
//! hand-rolled list (anti-pattern). A3 Wave-0 read of the module render API,
//! noted per the plan acceptance criteria:
//!
//! - `Command::new(&state: &Entity<CommandState>) -> Command` owns entries +
//!   policy; `.group(CommandGroup)` / `.placeholder(..)` / `.empty(..)` /
//!   `.on_confirm(Fn(IndexPath, &mut Window, &mut App))` / `.on_cancel(..)`
//!   configure it (`command/command.rs:58-...`).
//! - `CommandState::new(window: &mut Window, cx: &mut Context<Self>)` holds
//!   query/focus/selection/scroll (`command/state.rs:141`). `Confirm`
//!   dispatches the selected item's boxed `Action` when set, then the deferred
//!   `on_confirm(IndexPath)` (`state.rs:confirm`); `Cancel` clears a non-empty
//!   query first and only then runs `on_cancel` (`state.rs:on_action_cancel`).
//! - `CommandItem::new().label(..).keywords(..).action(..).checked(..)` and
//!   `CommandGroup::new().label(..).item(..)/.items(..)`
//!   (`command/item.rs:37-.../153-...`). Matching is label+keywords
//!   substring, case-insensitive, inside the module.
//! - The item `disabled` flag is crate-private with no public builder, so
//!   inapplicable actions are HIDDEN (filtered from the group), never sent
//!   raw (T-06-05).
//!
//! Dispatch rides `on_confirm` back into `AppState` (existing correlated
//! sends / `open_session`, fire-and-forget like chip clicks); the palette
//! holds no sockets and spawns no processes.

use gpui::*;
use gpui_component::command::{Command, CommandGroup, CommandItem};

use crate::app_state::AppState;

// --- Pure palette model (headless-tested, no sockets/cx) -------------------

/// Palette action ids in FE `CommandPalette.tsx:48-97` order. Locked to the
/// FE list — extras are v2 (the groups test fails on any extra).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteActionId {
    NewSession,
    NewWindow,
    SplitRight,
    SplitDown,
    ZoomPane,
    NextLayout,
    KillPane,
}

/// One Actions-group row: FE label verbatim + search keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteActionItem {
    pub id: PaletteActionId,
    pub label: &'static str,
    pub keywords: &'static [&'static str],
}

/// Exactly the 7 FE actions in FE order (DLG-01 per D5).
pub fn palette_action_items() -> Vec<PaletteActionItem> {
    vec![
        PaletteActionItem {
            id: PaletteActionId::NewSession,
            label: "New Session",
            keywords: &["new", "session", "create"],
        },
        PaletteActionItem {
            id: PaletteActionId::NewWindow,
            label: "New Window",
            keywords: &["new", "window", "create"],
        },
        PaletteActionItem {
            id: PaletteActionId::SplitRight,
            label: "Split Right",
            keywords: &["split", "horizontal", "right", "pane"],
        },
        PaletteActionItem {
            id: PaletteActionId::SplitDown,
            label: "Split Down",
            keywords: &["split", "vertical", "down", "pane"],
        },
        PaletteActionItem {
            id: PaletteActionId::ZoomPane,
            label: "Zoom Pane",
            keywords: &["zoom", "pane", "fullscreen"],
        },
        PaletteActionItem {
            id: PaletteActionId::NextLayout,
            label: "Next Layout",
            keywords: &["layout", "next", "window"],
        },
        PaletteActionItem {
            id: PaletteActionId::KillPane,
            label: "Kill Pane",
            keywords: &["kill", "pane", "close"],
        },
    ]
}

/// FE guard mirror (T-06-05): window actions need an active session, pane
/// actions need an active pane; New Session always applies (its dialog works
/// with zero sessions). Inapplicable items are hidden, never dispatched.
pub fn palette_action_enabled(
    id: PaletteActionId,
    has_active_session: bool,
    has_active_pane: bool,
) -> bool {
    match id {
        PaletteActionId::NewSession => true,
        PaletteActionId::NewWindow | PaletteActionId::NextLayout => has_active_session,
        PaletteActionId::SplitRight
        | PaletteActionId::SplitDown
        | PaletteActionId::ZoomPane
        | PaletteActionId::KillPane => has_active_pane,
    }
}

/// Pure substring filter over label + keywords, case-insensitive (mirrors the
/// module's matching for headless contracts; the live view filters via the
/// vendored `Command`). Empty query matches everything.
pub fn palette_filter<'a>(
    query: &str,
    items: &'a [PaletteActionItem],
) -> Vec<&'a PaletteActionItem> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return items.iter().collect();
    }
    items
        .iter()
        .filter(|item| {
            item.label.to_lowercase().contains(&q)
                || item.keywords.iter().any(|k| k.to_lowercase().contains(&q))
        })
        .collect()
}

/// Open-Session group rows: one entry per tree session name, in tree order
/// (FE `CommandPalette.tsx:98-115`).
pub fn open_session_items<'a>(names: &'a [String]) -> Vec<&'a str> {
    names.iter().map(|s| s.as_str()).collect()
}

/// Ctrl+Shift+P detector shared by the `TogglePalette` binding and the
/// `TerminalView::on_key_down` early-return (Pitfall 1). Ctrl-only: that is
/// exactly what `bind_palette_keys` registers, so a handled keystroke is
/// never swallowed without a dispatch (see SUMMARY for the platform note).
pub fn is_palette_keystroke(ctrl: bool, shift: bool, _platform: bool, key: &str) -> bool {
    ctrl && shift && key.eq_ignore_ascii_case("p")
}

// --- GPUI palette view (vendored Command over the pure model) ---------------

/// Render the open palette overlay: guard-filtered Actions group + Open
/// Session group over the vendored `Command`. The caller guarantees
/// `palette_open` (the `CommandState` entity exists).
pub fn render_palette(app: &mut AppState, cx: &mut Context<AppState>) -> impl IntoElement {
    let state = app
        .palette_state
        .clone()
        .expect("palette rendered without CommandState");
    let has_session = app.active_session.is_some();
    let has_pane = app.palette_active_pane().is_some();

    let actions: Vec<CommandItem> = palette_action_items()
        .into_iter()
        .filter(|item| palette_action_enabled(item.id, has_session, has_pane))
        .map(|item| {
            CommandItem::new()
                .label(item.label)
                .keywords(item.keywords.iter().copied())
        })
        .collect();

    let session_names: Vec<String> = app
        .tree
        .sessions
        .iter()
        .map(|n| n.session.name.clone())
        .collect();
    let sessions: Vec<CommandItem> = open_session_items(&session_names)
        .into_iter()
        .map(|name| CommandItem::new().label(name.to_string()))
        .collect();

    let app_weak = cx.weak_entity();
    // FE parity: the Open Session group renders only when sessions exist
    // (`sessions.length > 0`); section indices (0 = Actions, 1 = sessions)
    // stay stable because the group is absent, never empty.
    let mut command = Command::new(&state)
        .group(CommandGroup::new().label("Actions").items(actions))
        .placeholder("Type a command or search…");
    if !sessions.is_empty() {
        command = command.group(CommandGroup::new().label("Open Session").items(sessions));
    }
    let command = command
        .empty(|_, _, _| {
            div()
                .px(px(12.0))
                .py(px(8.0))
                .text_sm()
                .child("No results found.")
                .into_any_element()
        })
        .on_confirm(move |index, window, cx| {
            if let Some(app) = app_weak.upgrade() {
                let _ = app.update(cx, |this, cx| {
                    this.confirm_palette_selection(index.section, index.row, window, cx);
                });
            }
        })
        .on_cancel({
            let app_weak = cx.weak_entity();
            move |_window, cx| {
                if let Some(app) = app_weak.upgrade() {
                    let _ = app.update(cx, |this, cx| {
                        this.hide_palette(cx);
                    });
                }
            }
        });

    let preset_name = app.settings.theme_preset.clone();
    let card = crate::theme::preset_card(&preset_name);
    let border = crate::theme::preset_border(&preset_name);

    div()
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .items_center()
        .pt(px(72.0))
        .px(px(16.0))
        .child(
            div()
                .w(px(560.0))
                .max_w_full()
                .rounded(px(8.0))
                .border_1()
                .border_color(border)
                .bg(card)
                .child(command),
        )
        .into_any_element()
}
