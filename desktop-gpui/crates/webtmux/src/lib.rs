#![recursion_limit = "512"]

//! webtmux desktop client (GPUI) library.

pub mod actions;
pub mod app_state;
pub mod bundle;
pub mod csd;
pub mod glass;
pub mod gtk_theme;
pub mod icons;
pub mod pane_geometry;
pub mod theme;
pub mod themes_generated;
pub mod views;
pub mod window_state;

/// Serializes unit tests that mutate the process-global desktop palette (the
/// GTK override lives in a `static`), so parallel test threads cannot observe
/// each other's install/clear.
#[cfg(test)]
pub(crate) mod test_util {
    use parking_lot::{Mutex, MutexGuard};

    static LOCK: Mutex<()> = Mutex::new(());

    pub(crate) fn serial_lock() -> MutexGuard<'static, ()> {
        LOCK.lock()
    }
}

