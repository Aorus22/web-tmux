//! Kill-confirm flow gates (SESS-05 + PANE-05/PANE-08 per D7, Phase 5 Task 2).
//!
//! The kill flows consult their own flag only: session reads
//! `confirm_kill_session`, pane reads `confirm_kill_pane`, window reads
//! `confirm_kill_window`. Split-direction mapping (PANE-02 pitfall lock) and
//! rename prefill/gate helpers (DLG-02 per D6) stay pure for headless tests.

use webtmux::app_state::AppState;
use webtmux_settings::DesktopSettings;

#[test]
fn test_kill_flow_gate() {
    // Fresh settings confirm.
    let app = AppState::new(DesktopSettings::default(), None);
    assert!(
        app.kill_requires_confirm(),
        "fresh settings must require kill confirmation"
    );

    // Explicit false allows a direct kill.
    let mut settings = DesktopSettings::default();
    settings.confirm_kill_session = false;
    let app = AppState::new(settings, None);
    assert!(
        !app.kill_requires_confirm(),
        "explicit false must allow a direct kill"
    );

    // Pane/window flags do not steer the session gate (Phase 5/6 read those).
    let mut settings = DesktopSettings::default();
    settings.confirm_kill_pane = false;
    settings.confirm_kill_window = false;
    let app = AppState::new(settings, None);
    assert!(
        app.kill_requires_confirm(),
        "session gate reads only confirm_kill_session"
    );
}

#[test]
fn test_pane_kill_gate() {
    // Pane flag only: confirm dialog when true, direct kill when false.
    let app = AppState::new(DesktopSettings::default(), None);
    assert!(
        app.kill_requires_confirm_pane(),
        "fresh settings must require pane kill confirmation"
    );
    let mut settings = DesktopSettings::default();
    settings.confirm_kill_pane = false;
    let app = AppState::new(settings, None);
    assert!(
        !app.kill_requires_confirm_pane(),
        "explicit false must allow a direct pane kill"
    );
    // Session/window flags do not steer the pane gate.
    let mut settings = DesktopSettings::default();
    settings.confirm_kill_session = false;
    settings.confirm_kill_window = false;
    let app = AppState::new(settings, None);
    assert!(
        app.kill_requires_confirm_pane(),
        "pane gate reads only confirm_kill_pane"
    );
}

#[test]
fn test_window_kill_gate() {
    let app = AppState::new(DesktopSettings::default(), None);
    assert!(
        app.kill_requires_confirm_window(),
        "fresh settings must require window kill confirmation"
    );
    let mut settings = DesktopSettings::default();
    settings.confirm_kill_window = false;
    let app = AppState::new(settings, None);
    assert!(
        !app.kill_requires_confirm_window(),
        "explicit false must allow a direct window kill"
    );
    let mut settings = DesktopSettings::default();
    settings.confirm_kill_session = false;
    settings.confirm_kill_pane = false;
    let app = AppState::new(settings, None);
    assert!(
        app.kill_requires_confirm_window(),
        "window gate reads only confirm_kill_window"
    );
}

#[test]
fn test_split_direction_map() {
    // Split right -> horizontal (tmux -h), Split down -> vertical (tmux -v).
    // PANE-02 pitfall lock: the flag names the new layout axis, not the divider.
    assert_eq!(AppState::pane_split_direction(true), "horizontal");
    assert_eq!(AppState::pane_split_direction(false), "vertical");
    let right = AppState::build_pane_split("%3", "horizontal");
    assert_eq!(right.direction.as_deref(), Some("horizontal"));
    let down = AppState::build_pane_split("%3", "vertical");
    assert_eq!(down.direction.as_deref(), Some("vertical"));
}

#[test]
fn test_rename_prefill() {
    // Pane prefill: title || current_command || '' (FE PaneContextMenu parity).
    assert_eq!(AppState::pane_rename_prefill("editor", "nvim"), "editor");
    assert_eq!(AppState::pane_rename_prefill("", "nvim"), "nvim");
    assert_eq!(AppState::pane_rename_prefill("", ""), "");
    // Window prefill: name (FE WindowTabs parity).
    assert_eq!(AppState::window_rename_prefill("main"), "main");
    // Empty gate rejects blank/whitespace (DLG-02 per D6).
    assert!(AppState::rename_name_allowed("dev"));
    assert!(!AppState::rename_name_allowed(""));
    assert!(!AppState::rename_name_allowed("   "));
}
