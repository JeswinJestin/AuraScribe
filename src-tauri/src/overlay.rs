//! Small always-on-top recording indicator, shown only while listening/processing.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

const OVERLAY_LABEL: &str = "overlay";
const WIDTH: f64 = 220.0;
const HEIGHT: f64 = 56.0;

/// Set only by the overlay page itself calling `overlay_ready`. Nothing else can set it,
/// so it is proof that our page loaded rather than a 404 or a connection error.
static READY: AtomicBool = AtomicBool::new(false);

pub fn mark_ready() {
    READY.store(true, Ordering::Release);
    tracing::debug!("Overlay page reported ready");
}

/// Must be called from the setup hook (window creation on other threads can
/// deadlock on Windows).
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }

    // The two hosts disagree about how to name this page. `next dev` serves the route as
    // `/overlay/` and 404s on `/overlay/index.html`; the exported bundle only contains the
    // file `overlay/index.html`. Getting it wrong in dev put Next's 404 page inside a
    // transparent, undecorated, always-on-top window — see docs/HANDOFF.md.
    let path = if tauri::is_dev() { "overlay/" } else { "overlay/index.html" };

    let window = WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App(path.into()))
        .title("AuraScribe")
        .inner_size(WIDTH, HEIGHT)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .visible(false)
        .focused(false)
        .build()?;

    // The overlay is clickable — clicking the pill stops dictation (see the overlay page). But
    // injection pastes into whatever window has focus, so if clicking the overlay *activated*
    // it, the transcript would land in the overlay instead of the user's app. `WS_EX_NOACTIVATE`
    // makes the window receive the click without ever becoming the foreground window, so the
    // target app stays focused throughout. Without this, click-to-stop would corrupt every
    // dictation it ended.
    make_non_activating(&window);

    Ok(())
}

/// Mark the overlay window as non-activating so clicking it never steals focus from the app the
/// user is dictating into. Windows-only; a no-op elsewhere.
#[cfg(target_os = "windows")]
fn make_non_activating(window: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    let Ok(handle) = window.hwnd() else {
        tracing::warn!("Could not get overlay HWND; click-to-stop may steal focus");
        return;
    };
    // Reconstruct our own `windows`-crate HWND from the raw handle so this is independent of
    // whatever `windows` version Tauri exposes `hwnd()` as.
    let hwnd = HWND(handle.0);
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new = ex | (WS_EX_NOACTIVATE.0 as isize) | (WS_EX_TOOLWINDOW.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new);
    }
}

#[cfg(not(target_os = "windows"))]
fn make_non_activating(_window: &tauri::WebviewWindow) {}

pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        tracing::warn!("Overlay window does not exist; cannot show the recording indicator");
        return;
    };
    // Never surface the overlay unless its page confirmed it loaded. A 404, a missing
    // dev server, or a wrong asset path all render the webview's own opaque error page,
    // which would then sit on top of everything the user is doing — undecorated,
    // always-on-top, with no way to dismiss it. Both times this shipped, that error box
    // was the symptom. Silence is the correct failure mode here.
    //
    // A dictation that starts before the page has reported ready is not lost: `overlay_ready`
    // re-runs this the moment the page loads (see commands.rs), so the indicator still appears.
    if !READY.load(Ordering::Acquire) {
        tracing::warn!("Overlay page not ready yet; it will appear once the page loads (overlay_ready)");
        return;
    }
    // A sleep/resume cycle can leave the window minimized or with a stale state; undo that before
    // positioning so it can actually appear.
    let _ = window.unminimize();
    position(&window);
    let _ = window.show();
    // Re-assert on-top on every show: another always-on-top window (or the OS) can drop the flag,
    // which would leave the indicator technically shown but hidden behind the user's app.
    let _ = window.set_always_on_top(true);
    // After the machine sleeps and wakes, Windows' WebView2 can suspend this hidden webview, so
    // showing it reveals a BLANK transparent window — the pill never paints even though recording
    // started (the owner's "sound plays but no overlay after sleep" report). Running script forces
    // the webview to resume, and the page's refresh hook re-reads authoritative status and repaints
    // the pill. Harmless when the webview was already awake. See docs/HANDOFF.md.
    let _ = window.eval("window.__auraOverlayRefresh && window.__auraOverlayRefresh()");
    tracing::info!("Overlay indicator shown");
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = window.hide();
    }
}

/// Place the pill near the bottom centre of the monitor the overlay is CURRENTLY on, falling back
/// to the primary monitor, then to a fixed on-screen spot. Using the current monitor (with its
/// coordinate offset) keeps the pill on the screen the user is working on in a multi-monitor setup
/// instead of always parking it on the primary display — a real "sometimes I can't see it" cause.
fn position(window: &tauri::WebviewWindow) {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let screen = m.size();
        let origin = m.position();
        if let Ok(win_size) = window.outer_size() {
            let x = origin.x as f64 + (screen.width as f64 - win_size.width as f64) / 2.0;
            let y = origin.y as f64 + screen.height as f64 - win_size.height as f64 - 96.0;
            let _ = window.set_position(PhysicalPosition::new(
                x.max(origin.x as f64),
                y.max(origin.y as f64),
            ));
            return;
        }
    }
    // Last resort: somewhere clearly on-screen rather than an unpredictable default position.
    let _ = window.set_position(PhysicalPosition::new(120.0, 120.0));
}
