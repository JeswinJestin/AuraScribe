#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod asr;
mod audio;
mod chunking;
mod cleanup;
mod commands;
mod db;
mod engine;
mod expand;
mod hotkey;
mod injection;
#[cfg(feature = "moonshine")]
mod moonshine;
#[cfg(feature = "moonshine")]
mod parakeet;
#[cfg(feature = "moonshine")]
mod dolphin;
#[cfg(feature = "moonshine")]
mod nemo_ctc;
mod overlay;
mod sound;
mod streaks;
mod system;
mod tray;

use crate::app_state::AppState;
use crate::commands::Status;
use crate::db::Database;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

/// Size the main window to comfortably fit whatever display it opens on, then centre it.
///
/// Works in **logical** pixels — the same unit the window config's `width`/`height`/`minWidth`/
/// `minHeight` use — so it is correct on any DPI/scale. The previous version only *shrank* an
/// oversized window and did the maths in physical pixels; on a high-DPI or smaller laptop the
/// 1480x936 design size scales up past the screen and the clamp left the window a wrong shape with
/// its own controls off the edge (the "width isn't right" report). Now the window is always ~92%
/// of the monitor's work area, clamped so it is never larger than the design size and never smaller
/// than the min size Tauri can render.
fn fit_to_screen(window: &tauri::WebviewWindow) {
    let Ok(Some(monitor)) = window.current_monitor() else {
        return;
    };
    let scale = monitor.scale_factor();
    if !(scale > 0.0) {
        return;
    }
    // work_area() is the screen minus the taskbar, in PHYSICAL px.
    let area = monitor.work_area().size;
    let (target_w, target_h) = fitted_window_size(area.width, area.height, scale);

    tracing::info!(
        "Sizing window to {:.0}x{:.0} logical (work area {}x{} physical @ {}x scale)",
        target_w, target_h, area.width, area.height, scale
    );
    let _ = window.set_size(tauri::LogicalSize::new(target_w, target_h));
    let _ = window.center();
}

/// Pure sizing maths (returns LOGICAL px), split out so it can be unit-tested without a real
/// window. Given a monitor's physical work-area size and DPI scale, returns ~92% of the work area,
/// clamped to never exceed the design size nor fall below the min size.
fn fitted_window_size(area_phys_w: u32, area_phys_h: u32, scale: f64) -> (f64, f64) {
    // Keep in sync with tauri.conf.json → app.windows[0].
    const DESIGN_W: f64 = 1480.0;
    const DESIGN_H: f64 = 936.0;
    const MIN_W: f64 = 860.0;
    const MIN_H: f64 = 560.0;
    const MARGIN: f64 = 0.92; // breathing room around the window on smaller screens

    let area_w = area_phys_w as f64 / scale;
    let area_h = area_phys_h as f64 / scale;
    (
        (area_w * MARGIN).clamp(MIN_W, DESIGN_W),
        (area_h * MARGIN).clamp(MIN_H, DESIGN_H),
    )
}

/// Makes `tracing` write to `%LOCALAPPDATA%\AuraScribe\aurascribe.log`. A release build has no
/// console (`windows_subsystem = "windows"`), so without this every log line vanished — which is
/// why "why isn't the model transcribing?" was undiagnosable. Shares an `Arc<Mutex<File>>` so a
/// new writer is handed out per event without reopening the file. No new dependency.
#[derive(Clone)]
struct FileMaker(Arc<std::sync::Mutex<std::fs::File>>);

struct FileHandle(Arc<std::sync::Mutex<std::fs::File>>);

impl std::io::Write for FileHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().map_err(|_| std::io::ErrorKind::Other)?.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().map_err(|_| std::io::ErrorKind::Other)?.flush()
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for FileMaker {
    type Writer = FileHandle;
    fn make_writer(&'a self) -> Self::Writer {
        FileHandle(self.0.clone())
    }
}

/// Open the log file, truncating it first if the previous session left it large so it can't grow
/// without bound. Returns `None` if the data directory can't be created (logging then falls back
/// to stdout only).
fn open_log_file() -> Option<std::fs::File> {
    use std::io::Write as _;
    let dir = dirs::data_local_dir()?.join("AuraScribe");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("aurascribe.log");
    let too_big = std::fs::metadata(&path).map(|m| m.len() > 5_000_000).unwrap_or(false);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(!too_big)
        .write(true)
        .truncate(too_big)
        .open(&path)
        .ok()?;
    let _ = writeln!(
        file,
        "\n==== AuraScribe {} started {} ====",
        env!("CARGO_PKG_VERSION"),
        chrono::Utc::now().to_rfc3339()
    );
    Some(file)
}

fn main() {
    use tracing_subscriber::prelude::*;

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "aurascribe=debug,tauri=info".into());

    // Always log to stdout (dev). Additionally log to a file when we can open one, so an
    // installed release build is diagnosable — the file is what the user can send back.
    let file_layer = open_log_file().map(|f| {
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(FileMaker(Arc::new(std::sync::Mutex::new(f))))
    });
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(file_layer)
        .init();
    tracing::info!("Logging to %LOCALAPPDATA%\\AuraScribe\\aurascribe.log");

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
                target_window: Arc::new(Mutex::new(0)),
                chunk_state: Arc::new(Mutex::new(Default::default())),
                chunk_task: Arc::new(Mutex::new(None)),
            };
            app.manage(state);

            tray::build(&app_handle)?;
            overlay::create(&app_handle)?;

            if settings.hotkey_enabled == 0 {
                tracing::info!("Dictation hotkey disabled in settings; not registering it.");
            } else if let Err(e) = hotkey::apply(&app_handle, &settings.hotkey, &settings.hotkey_mode) {
                tracing::warn!("Failed to register hotkey \"{}\": {}", settings.hotkey, e);
                // Don't fail silently — record it so the UI can tell the user their dictation
                // shortcut isn't active (usually another app already grabbed the same combo).
                let app2 = app_handle.clone();
                let combo = settings.hotkey.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app2.state::<AppState>();
                    let mut status = state.status.lock().await;
                    status.last_error = Some(format!(
                        "Couldn't register the {combo} hotkey — another app may be using it. Pick a different shortcut in Settings."
                    ));
                });
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
            commands::search_transcripts,
            commands::delete_transcript,
            commands::clear_transcripts,
            commands::transcript_daily_counts,
            commands::delete_transcripts_between,
            commands::get_stats,
            commands::get_streak_state,
            commands::get_year_recap,
            commands::save_share_image,
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

#[cfg(test)]
mod tests {
    use super::fitted_window_size;

    // Design 1480x936, min 860x560, margin 0.92 — kept in sync with tauri.conf.json.

    #[test]
    fn desktop_1080p_stays_at_design_size() {
        // 1080p, 100% scale, work area 1920x1032 → 0.92 exceeds the design size → clamped to it.
        assert_eq!(fitted_window_size(1920, 1032, 1.0), (1480.0, 936.0));
    }

    #[test]
    fn four_k_stays_at_design_size() {
        assert_eq!(fitted_window_size(3840, 2100, 1.0), (1480.0, 936.0));
    }

    #[test]
    fn small_laptop_shrinks_to_fit() {
        // 1366x768, 100% scale, taskbar ~40px → 1366x728 work area.
        let (w, h) = fitted_window_size(1366, 728, 1.0);
        assert!(w > 860.0 && w < 1480.0, "width {w} out of range");
        assert!(h > 560.0 && h < 936.0, "height {h} out of range");
        assert!((w - 1366.0 * 0.92).abs() < 0.5);
        assert!((h - 728.0 * 0.92).abs() < 0.5);
    }

    #[test]
    fn high_dpi_uses_logical_not_physical_pixels() {
        // 1080p laptop at 150% scale: physical work area 1920x1032 → logical 1280x688.
        // The old physical-pixels logic mis-shaped the window here; logical maths keeps it on-screen.
        let (w, h) = fitted_window_size(1920, 1032, 1.5);
        assert!((w - 1177.6).abs() < 0.5, "width {w}");
        assert!((h - 632.96).abs() < 0.5, "height {h}");
        assert!((860.0..=1480.0).contains(&w) && (560.0..=936.0).contains(&h));
    }

    #[test]
    fn tiny_screen_never_below_min() {
        // 1024x600: the height maths (552) would dip below the min, so it clamps up to 560.
        let (w, h) = fitted_window_size(1024, 600, 1.0);
        assert!((860.0..=1480.0).contains(&w), "width {w}");
        assert_eq!(h, 560.0);
    }
}
