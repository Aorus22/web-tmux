//! Kill-confirm flow gate (SESS-05 setting contract, Phase 3 Plan 02 Task 1).
//!
//! The kill flow consults `DesktopSettings.confirm_kill_session`: confirm dialog
//! when true, direct kill when false. Pure `AppState` level — no GPUI runtime,
//! no sockets needed.

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
