//! Client-side-decoration frame geometry: the transparent shadow margin, the
//! `_GTK_FRAME_EXTENTS` advertisement, and the client↔frame rectangle
//! conversion that keeps persisted window geometry stable.
//!
//! # Why the margin exists
//!
//! X11 gives a client no shadow of its own, and Mutter paints its own shadow
//! around the *rectangle* of the window — which is exactly the dark "wedge" that
//! shows through transparent rounded corners. The window therefore reserves
//! [`WINDOW_SHADOW_MARGIN`] transparent logical pixels all around, paints its own
//! shadow into them ([`frame_shadow`]), and advertises the same inset as
//! `_GTK_FRAME_EXTENTS` so Mutter places and shadows the window like a GTK CSD
//! window instead of twinning our rounded corners with a square shadow.
//!
//! # Units and the growth contract
//!
//! * `_GTK_FRAME_EXTENTS` is in **physical** pixels, so the margin is multiplied
//!   by the window scale factor (`16 logical → 32 physical` at scale 2).
//! * Mutter **grows the client by `2e`** when the property is written after map
//!   (`1024×1104 → 1056×1136` for `e = 16` logical). The window therefore
//!   requests the *frame/card* size only; the app paints the margin itself and
//!   the visible card comes out at exactly the requested size. Adding the margin
//!   to the requested size as well would double it.
//! * Because the client rectangle grows, [`client_to_frame_rect`] subtracts the
//!   margin back before geometry is persisted. Without it every launch would save
//!   a window 32 logical pixels larger than the last.
//!
//! Only the X11 backend advertises extents; on Wayland (or when the property
//! write fails) [`extents_active`] stays false and geometry is passed through
//! untouched.
//!
//! # Surfaces (B3)
//!
//! The app opens exactly one platform window (`main.rs::open_window`) with
//! `WindowDecorations::Client`; every overlay in the product — `gpui-component`
//! dialogs, the command palette, popovers and the toast stack — is an in-window
//! deferred element of that same managed surface, not a separate override-redirect
//! popup. There is therefore no second X window to advertise extents on, no
//! popup-sized client to keep margin-free, and no override-redirect surface that
//! needs its own outline. The frame outline and the margin live on the one
//! managed window and are inherited by all of its overlays.

use std::sync::atomic::{AtomicBool, Ordering};

use gpui::{px, BoxShadow, Pixels, Window};

/// Transparent logical margin reserved around the card for its own shadow.
pub const WINDOW_SHADOW_MARGIN: f32 = 16.0;

/// Width of the resize hit band at window edges. The full shadow padding, so
/// there is no dead band between the grab zone and the panel.
pub const RESIZE_HIT: f32 = WINDOW_SHADOW_MARGIN;

/// The reserved margin as a GPUI pixel value.
pub fn shadow_padding() -> Pixels {
    px(WINDOW_SHADOW_MARGIN)
}

/// The frame shadow, painted by GPUI into [`WINDOW_SHADOW_MARGIN`].
///
/// GPUI's blur radius is CSS-semantics (visible reach ≈ offset + ~1.5×blur),
/// so the old `4px + 12px` pair filled the 16px margin edge-to-edge: the blur
/// tail clipped at the surface boundary and the shadow read as a straight
/// dark box around the rounded frame instead of a soft glow. `3px + 10px`
/// keeps the visible falloff inside the margin (reach ≈ 15px) and the lower
/// alpha keeps the band from reading as a solid border box.
pub fn frame_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(3.), gpui::rgba(0x0000004D).into()).blur_radius(px(10.)),
    ]
}

/// Whether `_GTK_FRAME_EXTENTS` was successfully advertised, which is also the
/// signal that Mutter grew the client by `2e` (see the module docs).
static EXTENTS_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Only ever advertise once per process.
static ADVERTISED: AtomicBool = AtomicBool::new(false);

pub fn extents_active() -> bool {
    EXTENTS_ACTIVE.load(Ordering::SeqCst)
}

/// `_GTK_FRAME_EXTENTS` in physical pixels for this window's scale factor.
pub fn extents_px(window: &Window) -> u32 {
    (WINDOW_SHADOW_MARGIN * window.scale_factor()).round() as u32
}

/// Recover the frame (card) rectangle from a client rectangle measured while
/// extents are active. Identity when they are not.
///
/// Only the size is corrected: GPUI's X11 backend deliberately ignores the
/// origin carried by a resize `ConfigureNotify` ("it contains wrong values"), so
/// the bounds origin it reports stays the requested frame position even after
/// Mutter grows the client around it. Subtracting the margin from the origin
/// would shift every restore by the margin instead.
pub fn client_to_frame_rect(x: i32, y: i32, w: u32, h: u32) -> (i32, i32, u32, u32) {
    if !extents_active() {
        return (x, y, w, h);
    }
    let e = WINDOW_SHADOW_MARGIN.round() as i32;
    let shrunk_w = (w as i32 - 2 * e).max(1) as u32;
    let shrunk_h = (h as i32 - 2 * e).max(1) as u32;
    (x, y, shrunk_w, shrunk_h)
}

/// Advertise the CSD frame to the compositor, retrying in the background.
///
/// The X window exists once the view is built but the property is allowed to
/// arrive after map, so the write is retried until it sticks. Writing the same
/// value repeatedly is a no-op; a different value shifts the frame
/// incrementally, which is why this only ever writes one value per process.
pub fn advertise_frame_extents(window: &Window) {
    #[cfg(target_os = "linux")]
    {
        if ADVERTISED.swap(true, Ordering::SeqCst) {
            return;
        }
        let Some(window_id) = x11_window_id(window) else {
            eprintln!("[webtmux] CSD: no X11 window handle; skipping _GTK_FRAME_EXTENTS");
            return;
        };
        let extents = extents_px(window);
        if extents == 0 {
            return;
        }
        let spawn = std::thread::Builder::new()
            .name("webtmux-csd-extents".to_string())
            .spawn(move || {
                for attempt in 0..40u32 {
                    match set_extents_once(window_id, extents) {
                        Ok(()) => {
                            EXTENTS_ACTIVE.store(true, Ordering::SeqCst);
                            eprintln!(
                                "[webtmux] CSD: _GTK_FRAME_EXTENTS={extents} on window {window_id:#x} (attempt {attempt})"
                            );
                            return;
                        }
                        Err(error) => {
                            if attempt == 39 {
                                eprintln!(
                                    "[webtmux] CSD: _GTK_FRAME_EXTENTS failed on {window_id:#x}: {error}"
                                );
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            });
        if let Err(error) = spawn {
            eprintln!("[webtmux] CSD: could not spawn extents thread: {error}");
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = window;
    }
}

/// The X11/XCB window id behind a GPUI window, when the backend is X11.
#[cfg(target_os = "linux")]
fn x11_window_id(window: &Window) -> Option<u32> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    // `Window` also has an inherent `window_handle()` (an `AnyWindowHandle`);
    // call the trait method explicitly.
    match HasWindowHandle::window_handle(window).ok()?.as_raw() {
        RawWindowHandle::Xcb(handle) => Some(handle.window.get()),
        RawWindowHandle::Xlib(handle) => Some(handle.window as u32),
        _ => None,
    }
}

/// One `ChangeProperty` attempt: intern the atom, replace the value, flush.
#[cfg(target_os = "linux")]
fn set_extents_once(window_id: u32, extents: u32) -> Result<(), String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as XprotoConnectionExt, PropMode};
    use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;

    let (conn, _screen) = x11rb::connect(None).map_err(|error| error.to_string())?;
    let atom = conn
        .intern_atom(false, b"_GTK_FRAME_EXTENTS")
        .map_err(|error| error.to_string())?
        .reply()
        .map_err(|error| error.to_string())?
        .atom;
    conn.change_property32(
        PropMode::REPLACE,
        window_id,
        atom,
        AtomEnum::CARDINAL,
        &[extents; 4],
    )
    .map_err(|error| error.to_string())?
    .check()
    .map_err(|error| error.to_string())?;
    conn.flush().map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_shadow_stays_inside_the_margin() {
        // The blur must not reach past the reserved margin, or the margin
        // becomes visible as a clipped edge.
        let shadows = frame_shadow();
        assert_eq!(shadows.len(), 1);
        let reach = f32::from(shadows[0].blur_radius) / 2.0
            + f32::from(shadows[0].offset.y).abs()
            + f32::from(shadows[0].spread_radius);
        assert!(
            reach <= WINDOW_SHADOW_MARGIN,
            "shadow reach {reach} exceeds margin {WINDOW_SHADOW_MARGIN}"
        );
    }

    #[test]
    fn geometry_passthrough_without_extents() {
        // In a test process the X property is never written, so the conversion
        // must be the identity.
        assert!(!extents_active());
        assert_eq!(client_to_frame_rect(10, 20, 1200, 660), (10, 20, 1200, 660));
    }

    #[test]
    fn margin_and_resize_band_agree() {
        assert_eq!(RESIZE_HIT, WINDOW_SHADOW_MARGIN);
        assert_eq!(shadow_padding(), px(WINDOW_SHADOW_MARGIN));
    }
}
