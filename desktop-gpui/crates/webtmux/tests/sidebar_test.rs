use webtmux::app_state::AppState;
use webtmux_settings::DesktopSettings;

#[test]
fn test_sidebar_toggle_snap() {
    let settings = DesktopSettings::default();
    let mut app = AppState::new(settings, None);

    // Initial state per spec is open (true)
    assert!(app.sidebar_open);

    // Toggle once -> closed (false)
    app.sidebar_open = !app.sidebar_open;
    assert!(!app.sidebar_open);

    // Toggle twice -> open (true)
    app.sidebar_open = !app.sidebar_open;
    assert!(app.sidebar_open);

    // Expanded sessions set operates cleanly
    assert!(app.expanded_sessions.is_empty());
    app.expanded_sessions.insert("dev".to_string());
    assert!(app.expanded_sessions.contains("dev"));
    app.expanded_sessions.remove("dev");
    assert!(!app.expanded_sessions.contains("dev"));
}
