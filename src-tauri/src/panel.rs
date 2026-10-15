//! The floating **Notetaker panel** — a compact, always-on-top companion window shown while a
//! meeting is being recorded (My thoughts / Transcript / Summary). It mirrors `overlay.rs`'s
//! secondary-window lifecycle (create hidden at setup, show/hide on demand, a `READY` flag proving
//! the page loaded so a 404 never gets surfaced), but unlike the overlay it is a normal interactive
//! window — the user types notes into it — so it is NOT made click-through/non-activating.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

const PANEL_LABEL: &str = "panel";
const WIDTH: f64 = 440.0;
const HEIGHT: f64 = 600.0;

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

/// Dock the panel to the right edge of the monitor it is currently on, vertically centred — the
/// familiar companion-panel position (and well clear of a meeting window on the left).
fn position(window: &tauri::WebviewWindow) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let screen = m.size();
        let origin = m.position();
        if let Ok(win) = window.outer_size() {
            let x = origin.x as f64 + screen.width as f64 - win.width as f64 - 24.0;
            let y = origin.y as f64 + (screen.height as f64 - win.height as f64) / 2.0;
            let _ = window.set_position(PhysicalPosition::new(
                x.max(origin.x as f64),
                y.max(origin.y as f64),
            ));
            return;
        }
    }
    let _ = window.set_position(PhysicalPosition::new(160.0, 120.0));
}
