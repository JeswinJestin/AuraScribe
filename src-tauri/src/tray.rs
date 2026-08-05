//! Tray icon with three visual states: idle, listening, processing.

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::commands::Status;

const TRAY_ID: &str = "main-tray";

const IDLE_ICON: &[u8] = include_bytes!("../icons/tray-idle.png");
const LISTENING_ICON: &[u8] = include_bytes!("../icons/tray-listening.png");
const PROCESSING_ICON: &[u8] = include_bytes!("../icons/tray-processing.png");

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "Open Settings", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit AuraScribe", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

    let icon = Image::from_bytes(IDLE_ICON)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("AuraScribe — Idle")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_icon(app: &AppHandle, status: &Status) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let (bytes, tooltip) = if status.is_recording {
        (LISTENING_ICON, "AuraScribe — Listening")
    } else if status.is_processing {
        (PROCESSING_ICON, "AuraScribe — Processing")
    } else {
        (IDLE_ICON, "AuraScribe — Idle")
    };

    if let Ok(icon) = Image::from_bytes(bytes) {
        let _ = tray.set_icon(Some(icon));
    }
    let _ = tray.set_tooltip(Some(tooltip));
}

/// Bring the settings window to the user. Also called when a second launch is detected, so
/// clicking the Start Menu icon surfaces the running instance instead of doing nothing.
pub fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        tracing::error!("No 'main' window to show");
        return;
    };

    // Order matters: unminimize before show, or a minimised window can come back still
    // minimised. Each step is logged on failure — "the window didn't open" was reported as a
    // total mystery precisely because every one of these calls discarded its error.
    if let Err(e) = window.unminimize() {
        tracing::debug!("unminimize failed (usually harmless): {}", e);
    }
    if let Err(e) = window.show() {
        tracing::error!("Failed to show main window: {}", e);
        return;
    }
    if let Err(e) = window.set_focus() {
        tracing::warn!("Failed to focus main window: {}", e);
    }
}
