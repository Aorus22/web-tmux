use std::path::PathBuf;
use webtmux_settings::DesktopSettings;

/// Resolve the path to the Go backend executable (`tmux-gui-server`).
///
/// Precedence:
/// 1. Explicit user override in `DesktopSettings::backend_path`.
/// 2. Packaged bundle layout: `tmux-gui-server` binary adjacent to `current_exe`.
/// 3. Development workspace paths (`tmux-gui-server.exe` at repo root or adjacent dirs).
pub fn resolve_backend_path_with_current_exe(
    settings: &DesktopSettings,
    current_exe: Option<PathBuf>,
) -> PathBuf {
    if let Some(ref path) = settings.backend_path {
        return path.clone();
    }

    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let bin_name = format!("tmux-gui-server{}", exe_suffix);

    // 1. Packaged bundle layout: adjacent to running executable
    if let Some(exe) = current_exe {
        if let Some(parent) = exe.parent() {
            let adjacent = parent.join(&bin_name);
            if adjacent.exists() {
                return adjacent.canonicalize().unwrap_or(adjacent);
            }
        }
    }

    // 2. Development resolution candidates
    let candidates = [
        PathBuf::from(&bin_name),
        PathBuf::from("..").join(&bin_name),
        PathBuf::from("../..").join(&bin_name),
        PathBuf::from("../../..").join(&bin_name),
        PathBuf::from("desktop/resources").join(&bin_name),
        PathBuf::from("../desktop/resources").join(&bin_name),
        PathBuf::from("../../desktop/resources").join(&bin_name),
        PathBuf::from("desktop-gpui/test-support").join(&bin_name),
    ];

    for c in candidates {
        if c.exists() {
            return c.canonicalize().unwrap_or(c);
        }
    }

    // Fallback path
    PathBuf::from(bin_name)
}

/// Convenience wrapper resolving with standard `std::env::current_exe()`.
pub fn resolve_backend_path(settings: &DesktopSettings) -> PathBuf {
    resolve_backend_path_with_current_exe(settings, std::env::current_exe().ok())
}
