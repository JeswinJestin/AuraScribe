//! The floating **Notetaker panel** — a compact, always-on-top companion window shown while a
//! meeting is being recorded (My thoughts / Transcript / Summary). It mirrors `overlay.rs`'s
//! secondary-window lifecycle (create hidden at setup, show/hide on demand, a `READY` flag proving
//! the page loaded so a 404 never gets surfaced), but unlike the overlay it is a normal interactive
//! window — the user types notes into it — so it is NOT made click-through/non-activating.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

const PANEL_LABEL: &str = "panel";
const WIDTH: f64 = 400.0;
const HEIGHT: f64 = 720.0;
/// Margins when docking to the right edge (logical-ish px; applied to the physical monitor rect).
const EDGE_GAP: i32 = 14;
const BOTTOM_GAP: i32 = 56; // clear the Windows taskbar

/// Set only by the panel page calling `notetaker_ready` — proof the page loaded (not a 404).
static READY: AtomicBool = AtomicBool::new(false);

pub fn mark_ready() {
    READY.store(true, Ordering::Release);
    tracing::debug!("Notetaker panel reported ready");
}

/// Create the (hidden) panel window. Call from the setup hook — building windows on other threads
/// can deadlock on Windows.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(PANEL_LABEL).is_some() {
        return Ok(());
    }
    // `next dev` serves the route as `/panel/`; the static export is `panel/index.html`. Getting
    // this wrong renders Next's 404 inside the window — see the overlay's identical note.
    let path = if tauri::is_dev() { "panel/" } else { "panel/index.html" };

    WebviewWindowBuilder::new(app, PANEL_LABEL, WebviewUrl::App(path.into()))
        .title("AuraScribe Notetaker")
        .inner_size(WIDTH, HEIGHT)
        .min_inner_size(320.0, 380.0)
        .resizable(true)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(false)
        .shadow(true)
        .visible(false)
        .focused(false)
        .build()?;
    Ok(())
}

pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        tracing::warn!("Notetaker panel window does not exist; cannot show it");
        return;
    };
    // Don't surface a window whose page hasn't confirmed it loaded (a 404/error page would sit
    // on top of everything). It will appear once the page reports ready (`notetaker_ready`).
    if !READY.load(Ordering::Acquire) {
        tracing::warn!("Notetaker panel not ready yet; it will appear once its page loads");
        return;
    }
    position(&window);
    let _ = window.show();
    let _ = window.set_always_on_top(true);
    let _ = window.set_focus();
    tracing::debug!("Notetaker panel shown");
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        let _ = window.hide();
    }
}

/// Dock the panel to the **right edge, near full height** of the monitor it is currently on — the
/// Wispr-style companion panel. Sizes the window to the monitor height (clear of the taskbar) and a
/// fixed width, then positions it flush to the right with a small gap.
fn position(window: &tauri::WebviewWindow) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let screen = m.size();
        let origin = m.position();
        let scale = m.scale_factor();
        // Convert the logical margins/width to physical pixels for this monitor.
        let gap = (EDGE_GAP as f64 * scale).round() as i32;
        let bottom = (BOTTOM_GAP as f64 * scale).round() as i32;
        let w = (WIDTH * scale).round() as u32;
        let h = (screen.height as i32 - gap - bottom).max(300) as u32;

        let _ = window.set_size(PhysicalSize::new(w, h));
        let x = origin.x + screen.width as i32 - w as i32 - gap;
        let y = origin.y + gap;
        let _ = window.set_position(PhysicalPosition::new(x.max(origin.x), y.max(origin.y)));
        return;
    }
    let _ = window.set_position(PhysicalPosition::new(160, 120));
}
