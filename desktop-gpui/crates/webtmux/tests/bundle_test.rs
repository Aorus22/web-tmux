//! Phase 7 (PKG-02): adjacent-backend resolution contract tests.
//!
//! The dist bundle layout (`dist/tmux-gui-{windows,linux}-x64/` with the Go
//! sidecar adjacent to the app exe) must agree byte-for-byte with what
//! `resolve_backend_path_with_current_exe` expects. These tests prove the
//! contract headless with a tempdir fake exe dir — no display, no build.

use std::path::PathBuf;
use webtmux::bundle::resolve_backend_path_with_current_exe;
use webtmux_settings::DesktopSettings;

fn sidecar_name() -> String {
    if cfg!(windows) {
        "tmux-gui-server.exe".to_string()
    } else {
        "tmux-gui-server".to_string()
    }
}

fn app_exe_name() -> String {
    if cfg!(windows) {
        "webtmux.exe".to_string()
    } else {
        "webtmux".to_string()
    }
}

/// Fake packaged bundle: tempdir containing a fake app exe + adjacent sidecar.
fn fake_bundle_dir() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir must create");
    let fake_exe = dir.path().join(app_exe_name());
    let fake_sidecar = dir.path().join(sidecar_name());
    std::fs::write(&fake_exe, b"fake-app").expect("fake app exe must write");
    std::fs::write(&fake_sidecar, b"fake-sidecar").expect("fake sidecar must write");
    (dir, fake_exe, fake_sidecar)
}

#[test]
fn test_adjacent_backend_resolution() {
    let (_dir, fake_exe, fake_sidecar) = fake_bundle_dir();
    let settings = DesktopSettings::default();

    let resolved =
        resolve_backend_path_with_current_exe(&settings, Some(fake_exe.clone()));

    assert_eq!(
        resolved,
        fake_sidecar.canonicalize().expect("fake sidecar must canonicalize"),
        "adjacent tmux-gui-server beside the exe must resolve (dist layout contract)"
    );
}

#[test]
fn test_settings_override_wins() {
    let (_dir, fake_exe, _fake_sidecar) = fake_bundle_dir();
    let pinned = if cfg!(windows) {
        PathBuf::from(r"C:\pinned\tmux-gui-server.exe")
    } else {
        PathBuf::from("/pinned/tmux-gui-server")
    };
    let mut settings = DesktopSettings::default();
    settings.backend_path = Some(pinned.clone());

    let resolved = resolve_backend_path_with_current_exe(&settings, Some(fake_exe));

    assert_eq!(
        resolved, pinned,
        "absolute backend_path override must beat adjacency (paranoid-user pin)"
    );
}
