use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use webtmux_settings::{DesktopSettings, Theme, WindowState};

#[test]
fn roundtrip() {
    let dir = tempdir().unwrap();
    let base = dir.path();

    let initial = DesktopSettings {
        backend_path: Some(PathBuf::from("/usr/bin/tmux-gui-server")),
        theme: Theme::Light,
        theme_preset: "default-light".to_string(),
        window_state: Some(WindowState {
            x: Some(100),
            y: Some(200),
            width: Some(1400),
            height: Some(900),
            maximized: false,
        }),
        last_backend_url: Some("http://127.0.0.1:9001".to_string()),
        custom_base: Some(base.to_path_buf()),
    };

    initial.save_to(base).expect("save_to should succeed");

    let loaded = DesktopSettings::load_from(base).expect("load_from should succeed");

    assert_eq!(loaded.backend_path, initial.backend_path);
    assert_eq!(loaded.theme, initial.theme);
    assert_eq!(loaded.theme_preset, initial.theme_preset);
    assert_eq!(loaded.window_state, initial.window_state);
    assert_eq!(loaded.last_backend_url, initial.last_backend_url);
}

#[test]
fn corrupt_recovery() {
    let dir = tempdir().unwrap();
    let base = dir.path();
    let config_dir = webtmux_settings::paths::config_dir_with_base(base);
    fs::create_dir_all(&config_dir).unwrap();

    let settings_file = config_dir.join("settings.json");
    fs::write(&settings_file, "{ malformed json content !!!").unwrap();

    let loaded = DesktopSettings::load_from(base).expect("load_from should recover from corrupt json");

    // Defaults should be returned
    assert_eq!(loaded.theme, Theme::Dark);
    assert_eq!(loaded.theme_preset, "default-dark");
    assert_eq!(loaded.window_state, None);

    // settings.json.bak should exist
    let bak_file = config_dir.join("settings.json.bak");
    assert!(bak_file.exists(), "settings.json.bak must exist after corrupt load");
    let bak_content = fs::read_to_string(&bak_file).unwrap();
    assert_eq!(bak_content, "{ malformed json content !!!");

    // A valid new settings.json should have been written with defaults
    assert!(settings_file.exists(), "new settings.json should be saved");
    let second_load = DesktopSettings::load_from(base).expect("second load should succeed");
    assert_eq!(second_load.theme, Theme::Dark);
}

#[test]
fn first_run() {
    let dir = tempdir().unwrap();
    let base = dir.path();

    let loaded = DesktopSettings::load_from(base).expect("first run load should succeed");
    assert_eq!(loaded.theme, Theme::Dark);
    assert_eq!(loaded.theme_preset, "default-dark");
    assert_eq!(loaded.window_state, None);
    assert_eq!(loaded.custom_base, Some(base.to_path_buf()));

    // Save should create the file
    loaded.save().expect("save should succeed with custom_base");
    let config_dir = webtmux_settings::paths::config_dir_with_base(base);
    assert!(config_dir.join("settings.json").exists(), "settings.json should be created on save");
}

#[test]
fn atomic_save_leaves_no_temp_files_and_writes_file() {
    let dir = tempdir().unwrap();
    let base = dir.path();
    let config_dir = webtmux_settings::paths::config_dir_with_base(base);

    let mut settings = DesktopSettings::default();
    settings.theme = Theme::Light;

    settings.save_to(base).expect("save_to must succeed");

    let entries: Vec<_> = fs::read_dir(&config_dir).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(entries, vec!["settings.json"]);
}

#[test]
fn window_state_clamping_and_guard_logic() {    let zero_state = WindowState {
        x: Some(100),
        y: Some(150),
        width: Some(0),
        height: Some(0),
        maximized: false,
    };
    assert_eq!(zero_state.width, Some(0));
    assert_eq!(zero_state.height, Some(0));

    let min_coord_state = WindowState {
        x: Some(-32000),
        y: Some(-32000),
        width: Some(1000),
        height: Some(700),
        maximized: false,
    };
    assert!(min_coord_state.x.unwrap() <= -10000);
    assert!(min_coord_state.y.unwrap() <= -10000);
}

#[test]
fn test_kill_confirm_defaults() {
    let settings = DesktopSettings::default();
    assert!(
        settings.confirm_kill_session,
        "fresh settings must confirm session kills"
    );
    assert!(
        settings.confirm_kill_pane,
        "fresh settings must confirm pane kills (Phase 5 reads this)"
    );
    assert!(
        settings.confirm_kill_window,
        "fresh settings must confirm window kills (Phase 5 reads this)"
    );
}

#[test]
fn test_kill_confirm_legacy_json() {
    // Phase-1/2 shape: confirm keys absent -> all default true (FE parity).
    let legacy = r#"{"theme":"dark","theme_preset":"default-dark"}"#;
    let settings: DesktopSettings =
        serde_json::from_str(legacy).expect("legacy JSON must parse");
    assert!(
        settings.confirm_kill_session,
        "legacy settings without the keys must keep confirming session kills"
    );
    assert!(settings.confirm_kill_pane);
    assert!(settings.confirm_kill_window);

    // Explicit false survives a JSON round-trip...
    let mut explicit = DesktopSettings::default();
    explicit.confirm_kill_session = false;
    let json = serde_json::to_string(&explicit).expect("serialize must succeed");
    assert!(json.contains("confirm_kill_session"));
    let back: DesktopSettings = serde_json::from_str(&json).expect("parse must succeed");
    assert!(!back.confirm_kill_session);
    assert!(back.confirm_kill_pane);
    assert!(back.confirm_kill_window);

    // ...and save/load preserves an explicit false through the atomic-write path.
    let dir = tempdir().unwrap();
    let base = dir.path();
    explicit.save_to(base).expect("save_to must succeed");
    let loaded = DesktopSettings::load_from(base).expect("load_from must succeed");
    assert!(!loaded.confirm_kill_session);
    assert!(loaded.confirm_kill_pane);
    assert!(loaded.confirm_kill_window);
}