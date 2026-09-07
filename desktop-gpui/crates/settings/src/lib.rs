//! Desktop settings store.
//!
//! Holds UI preferences and window geometry persistence for web-tmux.

pub mod paths;

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// UI theme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

/// Window dimensions and layout state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WindowState {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub maximized: bool,
}

fn default_theme_preset() -> String {
    "default-dark".to_string()
}

fn default_true() -> bool {
    true
}

fn default_font_family() -> String {
    "JetBrains Mono".to_string()
}

fn default_font_size() -> f32 {
    14.0
}

fn default_line_height() -> f32 {
    1.35
}

fn default_scrollback_lines() -> usize {
    2000
}

fn default_theme_mode_filter() -> String {
    "all".to_string()
}

/// FE `TerminalSettings.tsx:54-90` parity (T-06-03): font size 8–32.
/// Non-finite inputs fall back to the FE default (14.0).
pub fn clamp_font_size(v: f32) -> f32 {
    if !v.is_finite() {
        return 14.0;
    }
    v.clamp(8.0, 32.0)
}

/// FE parity: line height 1–2. Non-finite inputs fall back to 1.35.
pub fn clamp_line_height(v: f32) -> f32 {
    if !v.is_finite() {
        return 1.35;
    }
    v.clamp(1.0, 2.0)
}

/// FE parity: scrollback 100–50000 lines.
pub fn clamp_scrollback(v: usize) -> usize {
    v.clamp(100, 50_000)
}

/// Desktop settings store holding UI preferences and window geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopSettings {
    #[serde(default)]
    pub backend_path: Option<PathBuf>,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_theme_preset")]
    pub theme_preset: String,
    #[serde(default)]
    pub window_state: Option<WindowState>,
    #[serde(default)]
    pub last_backend_url: Option<String>,
    /// Confirm before killing a tmux session (SESS-05, honored since Phase 3).
    /// `default_true` so legacy Phase-1/2 settings files (keys absent) keep
    /// confirming — FE `settingsStore.ts` parity (`confirmKillSession: true`).
    #[serde(default = "default_true")]
    pub confirm_kill_session: bool,
    /// Confirm before killing a pane (Phase 5 reads this; struct parity now).
    #[serde(default = "default_true")]
    pub confirm_kill_pane: bool,
    /// Confirm before killing a window (Phase 5 reads this; struct parity now).
    #[serde(default = "default_true")]
    pub confirm_kill_window: bool,
    /// Explicit tmux executable. Empty means use the backend's PATH resolver
    /// (FE `settingsStore.ts` `tmuxBinary: ''` parity, D2).
    #[serde(default)]
    pub tmux_binary: String,
    /// Single embedded family (FE default stack collapses to its first face;
    /// free text falls back — D8). Labelled honestly in the settings page.
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// FE `fontSize: 14` with 8–32 clamps (D9).
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    /// FE `lineHeight: 1.35` with 1–2 clamps (D9).
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    /// FE `scrollbackLines: 2000` with 100–50000 clamps (D9).
    #[serde(default = "default_scrollback_lines")]
    pub scrollback_lines: usize,
    /// Default per-pane TUI-scroll when no per-pane override exists (FE
    /// `tuiScrollPanes` default-ON parity; `AppState::tui_scroll` falls back
    /// here instead of hardcoded `true`).
    #[serde(default = "default_true")]
    pub tui_scroll_default: bool,
    /// Appearance filter (`all`/`dark`/`light`; reference parity, D4).
    #[serde(default = "default_theme_mode_filter")]
    pub theme_mode_filter: String,

    #[serde(skip)]
    pub custom_base: Option<PathBuf>,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            backend_path: None,
            theme: Theme::Dark,
            theme_preset: default_theme_preset(),
            window_state: None,
            last_backend_url: None,
            confirm_kill_session: true,
            confirm_kill_pane: true,
            confirm_kill_window: true,
            tmux_binary: String::new(),
            font_family: default_font_family(),
            font_size: default_font_size(),
            line_height: default_line_height(),
            scrollback_lines: default_scrollback_lines(),
            tui_scroll_default: true,
            theme_mode_filter: default_theme_mode_filter(),
            custom_base: None,
        }
    }
}

impl DesktopSettings {
    /// Load settings from the default config directory.
    pub fn load() -> Result<Self, SettingsError> {
        let base = paths::default_base_dir();
        Self::load_from(&base)
    }

    /// Load settings using an injectable base directory (for testing).
    pub fn load_from(base: &Path) -> Result<Self, SettingsError> {
        let file_path = paths::settings_path_with_base(base);
        if !file_path.exists() {
            let settings = Self {
                custom_base: Some(base.to_path_buf()),
                ..Default::default()
            };
            return Ok(settings);
        }

        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => return Err(SettingsError::Io(e)),
        };

        match serde_json::from_str::<DesktopSettings>(&content) {
            Ok(mut settings) => {
                settings.custom_base = Some(base.to_path_buf());
                Ok(settings)
            }
            Err(_) => {
                // Corrupt file recovery: rename to settings.json.bak and regenerate defaults
                let bak_path = file_path.with_extension("json.bak");
                let _ = fs::rename(&file_path, &bak_path);

                let settings = Self {
                    custom_base: Some(base.to_path_buf()),
                    ..Default::default()
                };
                let _ = settings.save_to(base);
                Ok(settings)
            }
        }
    }

    /// Save settings to current base directory (or default).
    pub fn save(&self) -> Result<(), SettingsError> {
        if let Some(ref base) = self.custom_base {
            self.save_to(base)
        } else {
            self.save_to(&paths::default_base_dir())
        }
    }

    /// Save settings using an injectable base directory.
    pub fn save_to(&self, base: &Path) -> Result<(), SettingsError> {
        let dir = paths::ensure_dirs_with_base(base)?;
        let file_path = paths::settings_path_with_base(base);
        let content = serde_json::to_string_pretty(self)?;

        // Atomic write: write to temp file in same directory, then rename
        let tmp_file = tempfile::NamedTempFile::new_in(&dir)?;
        let tmp_path = tmp_file.path().to_path_buf();
        fs::write(&tmp_path, content)?;

        // Set user-only permissions (0600) on Unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&tmp_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                let _ = fs::set_permissions(&tmp_path, perms);
            }
        }

        // Atomically replace destination file
        tmp_file.persist(&file_path).map_err(|e| e.error)?;

        Ok(())
    }
}
