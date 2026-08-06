#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod asr;
mod audio;
mod chunking;
mod cleanup;
mod commands;
mod db;
mod engine;
mod hotkey;
mod injection;
#[cfg(feature = "moonshine")]
mod moonshine;
mod overlay;
mod sound;
mod system;
mod tray;

use crate::app_state::AppState;
use crate::commands::Status;
use crate::db::Database;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

/// Shrink the window if its configured size doesn't fit the monitor it opened on, then
/// re-centre. The default is sized for a 1080p desktop; on a 1366x768 laptop that default
/// would be born larger than the screen, with its own controls off the edge. Only ever
/// shrinks — a large display keeps the designed size. Tauri still enforces `minWidth` /
/// `minHeight`, so this cannot collapse the layout below what it can render.
fn fit_to_screen(window: &tauri::WebviewWindow) {
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };
    // The work area is the screen minus the taskbar — the real space a window can occupy.
    // A percentage-of-screen guess was wrong in practice: 90% of 1080 clamped the height to
    // 972 on an ordinary 1080p desktop, shrinking the window below its design size for no
    // reason. The work area on that same machine is 1032 tall, which fits it exactly.
    let area = monitor.work_area().size;
    let Ok(outer) = window.outer_size() else {
        return;
    };

    if outer.width <= area.width && outer.height <= area.height {
        return;
    }

    let fitted =
        tauri::PhysicalSize::new(outer.width.min(area.width), outer.height.min(area.height));
    tracing::info!(
        "Window {}x{} doesn't fit {}x{} work area; fitting to {}x{}",
        outer.width, outer.height, area.width, area.height, fitted.width, fitted.height
    );
    let _ = window.set_size(fitted);
    let _ = window.center();
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aurascribe=debug,tauri=info".into()),
        )
        .init();

    tauri::Builder::default()
        // Must be registered first. Without it, launching from the Start Menu while the app
        // is already in the tray started a *second* process, which auto-loaded the model,
        // decided it was already set up, and stayed hidden — so clicking the icon looked
        // like it did nothing at all. Now a second launch surfaces the running window.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tracing::info!("Second launch detected; showing the existing window");
            tray::show_main_window(app);
        }))
        // global-shortcut is the only plugin left: it is what registers the dictation
        // hotkey. Eight others (store, notification, opener, shell, dialog, fs, process,
        // clipboard-manager) were registered but never called from Rust or the frontend —
        // pure binary size, build time, and IPC surface. Removed in Round 6.
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // No window-state plugin, deliberately. It restored a saved size on every launch,
        // silently overriding both the configured default and `minWidth`/`minHeight` — the
        // settings window stayed pinned at 505x758, under its own 860 minimum, long after
        // the default became 1080x720, so config changes appeared to do nothing. It also
        // restored a stale position, fighting `center: true` and risking an off-screen
        // window after a monitor change. The layout has a designed size: it opens at it,
        // centred, every time.
        .setup(|app| {
            let app_handle = app.handle().clone();

            let db = tauri::async_runtime::block_on(async { Database::new().await })?;
            let asr = Arc::new(engine::Asr::new()?);

            let settings = tauri::async_runtime::block_on(async { db.load_settings().await })?;

            sound::set_enabled(settings.sound_cues != 0);

            let mut initial_status = Status {
                hotkey_mode: settings.hotkey_mode.clone(),
                ai_cleanup_enabled: settings.ai_cleanup_enabled != 0,
                ..Status::default()
            };

            // Best-effort: if the configured model is already on disk from a
            // previous run, load it now so the app is ready without the user
            // having to revisit Settings every launch.
            {
                let asr = asr.clone();
                let model_id = settings.whisper_model.clone();
                if asr.is_downloaded(&model_id) {
                    match asr.load_model(&model_id) {
                        Ok(()) => {
                            tracing::info!(model = %model_id, "Auto-loaded model at startup");
                            initial_status.is_model_loaded = true;
                            initial_status.loaded_model = Some(model_id.clone());
                        }
                        Err(e) => tracing::warn!("Failed to auto-load model: {}", e),
                    }
                } else {
                    tracing::info!(model = %model_id, "No model on disk yet — showing setup window");
                }
            }
            let state = AppState {
                db: Arc::new(Mutex::new(db)),
                status: Arc::new(Mutex::new(initial_status)),
                audio_buffer: Arc::new(Mutex::new(Vec::new())),
                audio_sample_rate: Arc::new(Mutex::new(16000)),
                recording_handle: Arc::new(Mutex::new(None)),
                stop_flag: Arc::new(Mutex::new(false)),
                asr,
                chunk_state: Arc::new(Mutex::new(Default::default())),
                chunk_task: Arc::new(Mutex::new(None)),
            };
            app.manage(state);

            tray::build(&app_handle)?;
            overlay::create(&app_handle)?;

            if let Err(e) = hotkey::apply(&app_handle, &settings.hotkey, &settings.hotkey_mode) {
                tracing::warn!("Failed to register default hotkey \"{}\": {}", settings.hotkey, e);
            }

            // Keep the app running in the tray when the settings window is closed.
            if let Some(main_window) = app.get_webview_window("main") {
                let window_handle = main_window.clone();
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_handle.hide();
                    }
                });

                fit_to_screen(&main_window);

                // Always show on launch. Hiding once a model was loaded meant that
                // launching the app deliberately — from the Start Menu, by double-clicking
                // the icon — produced no window and no feedback, which reads as a failed
                // launch. The tray is what keeps it alive after the window is closed; it is
                // not a reason to withhold the window when someone asks for the app.
                tray::show_main_window(&app_handle);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_status,
            commands::start_recording,
            commands::stop_recording,
            commands::load_model,
            commands::download_model,
            commands::get_downloaded_models,
            commands::get_available_models,
            commands::delete_model,
            commands::get_dictionary,
            commands::add_dictionary_entry,
            commands::delete_dictionary_entry,
            commands::get_snippets,
            commands::add_snippet,
            commands::delete_snippet,
            commands::get_app_profiles,
            commands::add_app_profile,
            commands::delete_app_profile,
            commands::get_transcripts,
            commands::clear_transcripts,
            commands::get_stats,
            commands::list_audio_devices,
            commands::set_start_at_login,
            commands::open_settings_folder,
            commands::check_microphone_permission,
            commands::request_microphone_permission,
            commands::check_accessibility_permission,
            commands::request_accessibility_permission,
            commands::get_log_file_path,
            commands::overlay_ready,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                tracing::info!("AuraScribe shutting down");
            }
        });
}
