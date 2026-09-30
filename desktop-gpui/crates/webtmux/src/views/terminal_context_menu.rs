//! Terminal right-click menu: Copy / Cut / Paste.
//!
//! Attached to the terminal root, which carries the STABLE
//! `terminal-root/<pane>` id — `ContextMenuExt` derives its open state from the
//! element id, so poll-tick re-renders must not rebuild it with a different one.
//!
//! Actions route through `AppState::terminal_views` (the same entity the
//! keyboard `Ctrl+Shift+C/V` path drives), never through a captured view
//! handle, so a pane that closed since the menu opened is a no-op.
//!
//! Right-click passthrough: a plain right-click opens this menu and is *not*
//! forwarded to the running program; `Shift`+right-click is forwarded to
//! mouse-reporting apps (`vim`, `htop`) — the usual "hold Shift to bypass"
//! convention (see `TerminalView::on_mouse_down`).

use gpui::*;
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};

use crate::app_state::AppState;
use crate::views::terminal_view::TerminalView;

/// One entry of the terminal menu.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TerminalMenuAction {
    Copy,
    Cut,
    Paste,
}

/// Wrap a terminal root element in its right-click menu.
pub fn with_terminal_context_menu(
    root: impl InteractiveElement + ParentElement + Styled + IntoElement + 'static,
    pane_id: String,
    app_weak: WeakEntity<AppState>,
) -> impl IntoElement {
    root.context_menu(move |menu, _window, cx| {
        let view = terminal_view(&app_weak, &pane_id, cx);
        let has_selection = view.as_ref().is_some_and(|view| view.read(cx).has_selection());
        // Copy/Cut are meaningless without a selection, Paste without text.
        let clipboard_has_text = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .is_some_and(|text| !text.is_empty());

        let copy_weak = app_weak.clone();
        let copy_pane = pane_id.clone();
        let cut_weak = app_weak.clone();
        let cut_pane = pane_id.clone();
        let paste_weak = app_weak.clone();
        let paste_pane = pane_id.clone();

        menu.item(
            PopupMenuItem::new("Copy")
                .disabled(!has_selection)
                .on_click(move |_, _, cx| {
                    run(&copy_weak, &copy_pane, TerminalMenuAction::Copy, cx);
                }),
        )
        .item(
            PopupMenuItem::new("Cut")
                .disabled(!has_selection)
                .on_click(move |_, _, cx| {
                    run(&cut_weak, &cut_pane, TerminalMenuAction::Cut, cx);
                }),
        )
        .separator()
        .item(
            PopupMenuItem::new("Paste")
                .disabled(!clipboard_has_text)
                .on_click(move |_, _, cx| {
                    run(&paste_weak, &paste_pane, TerminalMenuAction::Paste, cx);
                }),
        )
    })
}

/// Resolve the live terminal view entity for a pane, if it still exists.
fn terminal_view(
    app_weak: &WeakEntity<AppState>,
    pane_id: &str,
    cx: &mut App,
) -> Option<Entity<TerminalView>> {
    let app = app_weak.upgrade()?;
    app.read(cx).terminal_views.get(pane_id).cloned()
}

/// Apply a menu action to the pane's terminal view.
fn run(app_weak: &WeakEntity<AppState>, pane_id: &str, action: TerminalMenuAction, cx: &mut App) {
    let Some(view) = terminal_view(app_weak, pane_id, cx) else {
        return;
    };
    view.update(cx, |view, cx| match action {
        TerminalMenuAction::Copy => {
            view.copy_selection(cx);
        }
        TerminalMenuAction::Cut => {
            view.cut_selection(cx);
        }
        TerminalMenuAction::Paste => {
            view.paste_clipboard(cx);
        }
    });
}
