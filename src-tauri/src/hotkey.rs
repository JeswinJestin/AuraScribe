//! Global hotkey registration driving push-to-talk and toggle dictation modes.

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::app_state::AppState;

pub fn apply(app: &AppHandle, combo: &str, mode: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    gs.unregister_all().map_err(|e| e.to_string())?;

    let mode = mode.to_string();

    gs.on_shortcut(combo, move |app, _shortcut, event| {
        let app = app.clone();
        match event.state() {
            ShortcutState::Pressed => {
                let mode = mode.clone();
                tauri::async_runtime::spawn(async move {
                    if mode == "toggle" {
                        toggle_recording(&app).await;
                    } else {
                        begin_recording(&app).await;
                    }
                });
            }
            ShortcutState::Released => {
                let mode = mode.clone();
                tauri::async_runtime::spawn(async move {
                    if mode != "toggle" {
                        end_recording(&app).await;
                    }
                });
            }
        }
    })
    .map_err(|e| e.to_string())?;

    Ok(())
}

async fn toggle_recording(app: &AppHandle) {
    let state = app.state::<AppState>();
    let is_recording = { state.status.lock().await.is_recording };
    let result = if is_recording {
        crate::commands::stop_recording(state, app.clone()).await
    } else {
        crate::commands::start_recording(state, app.clone()).await
    };
    if let Err(e) = result {
        tracing::warn!("Hotkey-triggered recording toggle failed: {}", e);
    }
}

async fn begin_recording(app: &AppHandle) {
    let state = app.state::<AppState>();
    let is_recording = { state.status.lock().await.is_recording };
    if !is_recording {
        if let Err(e) = crate::commands::start_recording(state, app.clone()).await {
            tracing::warn!("Hotkey-triggered recording start failed: {}", e);
        }
    }
}

async fn end_recording(app: &AppHandle) {
    let state = app.state::<AppState>();
    let is_recording = { state.status.lock().await.is_recording };
    if is_recording {
        if let Err(e) = crate::commands::stop_recording(state, app.clone()).await {
            tracing::warn!("Hotkey-triggered recording stop failed: {}", e);
        }
    }
}
