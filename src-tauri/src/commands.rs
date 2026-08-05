use crate::app_state::AppState;
use crate::cleanup::{self, CleanupOptions};
use crate::db::Database;
use crate::injection::TextInjector;
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: String,
    #[serde(rename = "hotkeyMode")]
    pub hotkey_mode: String,
    #[serde(rename = "whisperModel")]
    pub whisper_model: String,
    #[serde(rename = "micDevice")]
    pub mic_device: Option<String>,
    #[serde(rename = "aiCleanupEnabled")]
    pub ai_cleanup_enabled: bool,
    #[serde(rename = "removeFillers")]
    pub remove_fillers: bool,
    pub language: String,
    pub theme: String,
    #[serde(rename = "startAtLogin")]
    pub start_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Shift+Space".to_string(),
            hotkey_mode: "toggle".to_string(),
            whisper_model: "base.en".to_string(),
            mic_device: None,
            ai_cleanup_enabled: true,
            remove_fillers: true,
            language: "en".to_string(),
            theme: "dark".to_string(),
            start_at_login: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    #[serde(rename = "isRecording")]
    pub is_recording: bool,
    #[serde(rename = "isProcessing")]
    pub is_processing: bool,
    #[serde(rename = "isModelLoaded")]
    pub is_model_loaded: bool,
    /// Which model is actually in memory right now. The frontend must trust this rather
    /// than the saved setting — the two diverge whenever a load fails or is in flight.
    #[serde(rename = "loadedModel")]
    pub loaded_model: Option<String>,
    #[serde(rename = "currentText")]
    pub current_text: String,
    #[serde(rename = "lastError")]
    pub last_error: Option<String>,
    #[serde(rename = "hotkeyMode")]
    pub hotkey_mode: String,
    #[serde(rename = "aiCleanupEnabled")]
    pub ai_cleanup_enabled: bool,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            is_recording: false,
            is_processing: false,
            is_model_loaded: false,
            loaded_model: None,
            current_text: String::new(),
            last_error: None,
            hotkey_mode: "toggle".to_string(),
            ai_cleanup_enabled: true,
        }
    }
}

pub async fn emit_status(app: &AppHandle, status: &Status) {
    app.emit("status-changed", status.clone()).ok();
    crate::tray::update_icon(app, status);

    if status.is_recording || status.is_processing {
        crate::overlay::show(app);
    } else {
        crate::overlay::hide(app);
    }
}

async fn load_settings_from_db(db: &Database) -> Result<Settings, String> {
    let row = db.load_settings().await.map_err(|e| e.to_string())?;
    Ok(Settings {
        hotkey: row.hotkey,
        hotkey_mode: row.hotkey_mode,
        whisper_model: row.whisper_model,
        mic_device: row.mic_device,
        ai_cleanup_enabled: row.ai_cleanup_enabled != 0,
        remove_fillers: row.remove_fillers != 0,
        language: row.language,
        theme: row.theme,
        start_at_login: row.start_at_login != 0,
    })
}

#[command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    let db = state.db.lock().await;
    load_settings_from_db(&db).await
}

#[command]
pub async fn save_settings(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    settings: Settings,
) -> Result<(), String> {
    crate::hotkey::apply(&app, &settings.hotkey, &settings.hotkey_mode)
        .map_err(|e| format!("Invalid hotkey \"{}\": {}", settings.hotkey, e))?;

    {
        let db = state.db.lock().await;
        db.save_settings(&crate::db::SettingsRow {
            id: 1,
            hotkey: settings.hotkey.clone(),
            hotkey_mode: settings.hotkey_mode.clone(),
            whisper_model: settings.whisper_model.clone(),
            mic_device: settings.mic_device.clone(),
            ai_cleanup_enabled: settings.ai_cleanup_enabled as i32,
            remove_fillers: settings.remove_fillers as i32,
            language: settings.language.clone(),
            theme: settings.theme.clone(),
            start_at_login: settings.start_at_login as i32,
        })
        .await
        .map_err(|e| e.to_string())?;
    }

    if let Err(e) = crate::system::set_startup(settings.start_at_login) {
        tracing::warn!("Failed to update startup registration: {}", e);
    }

    {
        let mut status = state.status.lock().await;
        status.hotkey_mode = settings.hotkey_mode.clone();
        status.ai_cleanup_enabled = settings.ai_cleanup_enabled;
    }

    app.emit("settings-changed", settings).ok();
    Ok(())
}

#[command]
pub async fn get_status(state: tauri::State<'_, AppState>) -> Result<Status, String> {
    let status = state.status.lock().await;
    Ok(status.clone())
}

#[command]
pub async fn start_recording(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let status = state.status.lock().await;
        if !status.is_model_loaded {
            return Err("No Whisper model loaded. Load one in Settings first.".to_string());
        }
    }

    {
        let mut status = state.status.lock().await;
        status.is_recording = true;
        status.current_text.clear();
        status.last_error = None;
        emit_status(&app, &status).await;
    }

    {
        let mut buffer = state.audio_buffer.lock().await;
        buffer.clear();
    }
    {
        let mut stop = state.stop_flag.lock().await;
        *stop = false;
    }

    let mic_device = {
        let db = state.db.lock().await;
        load_settings_from_db(&db).await.ok().and_then(|s| s.mic_device)
    };

    let buffer_clone = state.audio_buffer.clone();
    let stop_flag_clone = state.stop_flag.clone();
    let sample_rate_clone = state.audio_sample_rate.clone();

    let handle = std::thread::spawn(move || {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();
        let device = mic_device
            .as_deref()
            .and_then(|name| {
                host.input_devices()
                    .ok()?
                    .find(|d| d.name().map(|n| n == name).unwrap_or(false))
            })
            .or_else(|| host.default_input_device());
        let device = match device {
            Some(d) => d,
            None => {
                tracing::warn!("No input device found");
                return;
            }
        };

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to get input config: {}", e);
                return;
            }
        };

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        tauri::async_runtime::block_on(async {
            *sample_rate_clone.lock().await = sample_rate;
        });

        let stream_config: cpal::StreamConfig = config.into();

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = buffer_clone.try_lock() {
                        if channels > 1 {
                            for chunk in data.chunks(channels) {
                                let mono: f32 = chunk.iter().sum::<f32>() / channels as f32;
                                buf.push(mono);
                            }
                        } else {
                            buf.extend_from_slice(data);
                        }
                    }
                },
                |err| {
                    tracing::warn!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| e.to_string())
            .ok();

        if let Some(stream) = stream {
            stream.play().ok();

            loop {
                if let Ok(stop) = stop_flag_clone.try_lock() {
                    if *stop {
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            tracing::info!("Audio capture stopped ({}Hz, {}ch)", sample_rate, channels);
        }
    });

    {
        let mut h = state.recording_handle.lock().await;
        *h = Some(handle);
    }

    Ok(())
}

#[command]
pub async fn stop_recording(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut status = state.status.lock().await;
        status.is_recording = false;
        status.is_processing = true;
        emit_status(&app, &status).await;
    }

    {
        let mut stop = state.stop_flag.lock().await;
        *stop = true;
    }

    if let Some(handle) = state.recording_handle.lock().await.take() {
        let _ = handle.join();
    }

    let audio_buffer: Vec<f32> = {
        let mut buf = state.audio_buffer.lock().await;
        std::mem::take(&mut *buf)
    };

    if audio_buffer.is_empty() {
        let mut status = state.status.lock().await;
        status.is_processing = false;
        status.last_error = Some("No audio captured".to_string());
        emit_status(&app, &status).await;
        return Ok(());
    }

    let sample_rate = *state.audio_sample_rate.lock().await;
    let settings = {
        let db = state.db.lock().await;
        load_settings_from_db(&db).await?
    };

    let asr = state.asr.clone();
    let db_arc = state.db.clone();
    let status_arc = state.status.clone();
    let app_clone = app.clone();

    tokio::spawn(async move {
        let start = std::time::Instant::now();
        let resampled = crate::audio::resample_linear(&audio_buffer, sample_rate, 16000);
        let audio_ms = (resampled.len() as f64 / 16.0).round() as i64;
        let language = settings.language.clone();

        let transcribe_result = tokio::task::spawn_blocking(move || {
            let lang = if language == "auto" { None } else { Some(language.as_str()) };
            asr.transcribe(&resampled, lang)
        })
        .await;

        let raw_text = match transcribe_result {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => {
                let mut status = status_arc.lock().await;
                status.is_processing = false;
                status.last_error = Some(format!("Transcription failed: {}", e));
                emit_status(&app_clone, &status).await;
                return;
            }
            Err(e) => {
                let mut status = status_arc.lock().await;
                status.is_processing = false;
                status.last_error = Some(format!("Transcription task panicked: {}", e));
                emit_status(&app_clone, &status).await;
                return;
            }
        };

        if raw_text.trim().is_empty() {
            let mut status = status_arc.lock().await;
            status.is_processing = false;
            status.last_error = Some("No speech detected".to_string());
            emit_status(&app_clone, &status).await;
            return;
        }

        let cleaned = if settings.ai_cleanup_enabled {
            cleanup::clean(
                &raw_text,
                CleanupOptions {
                    remove_fillers: settings.remove_fillers,
                    auto_punctuation: true,
                },
            )
        } else {
            raw_text.clone()
        };

        // Recording silence or background noise leaves nothing but Whisper's non-speech
        // annotations, which cleanup strips. Injecting the empty remainder — or a lone
        // stray "." — would type junk wherever the user's cursor happens to be.
        if cleaned.trim().is_empty() || cleaned.trim() == "." {
            tracing::info!(raw = %raw_text, "No speech in recording; nothing to insert");
            let mut status = status_arc.lock().await;
            status.is_processing = false;
            status.last_error = Some("No speech detected".to_string());
            emit_status(&app_clone, &status).await;
            return;
        }

        if let Err(e) = TextInjector::new().inject_text(&cleaned) {
            let mut status = status_arc.lock().await;
            status.is_processing = false;
            status.last_error = Some(e);
            status.current_text = cleaned.clone();
            emit_status(&app_clone, &status).await;
            return;
        }

        {
            let db = db_arc.lock().await;
            let duration_ms = start.elapsed().as_millis() as i64;
            if let Err(e) = db
                .add_transcript(
                    &raw_text,
                    &cleaned,
                    &None,
                    duration_ms,
                    audio_ms,
                    &settings.whisper_model,
                )
                .await
            {
                tracing::warn!("Failed to save transcript: {}", e);
            }
        }

        let mut status = status_arc.lock().await;
        status.is_processing = false;
        status.current_text = cleaned.clone();
        emit_status(&app_clone, &status).await;
        app_clone.emit("transcript-received", cleaned).ok();
    });

    Ok(())
}

#[command]
pub async fn load_model(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    model_id: String,
) -> Result<(), String> {
    tracing::info!(model = %model_id, "Loading Whisper model");

    let asr = state.asr.clone();
    let mid = model_id.clone();
    let result = tokio::task::spawn_blocking(move || asr.load_model(&mid))
        .await
        .map_err(|e| e.to_string())?;

    let outcome = {
        let mut status = state.status.lock().await;
        match result {
            Ok(()) => {
                status.is_model_loaded = true;
                status.loaded_model = Some(model_id.clone());
                status.last_error = None;
                tracing::info!(model = %model_id, "Model loaded");
                Ok(())
            }
            Err(e) => {
                let msg = format!("Failed to load model \"{}\": {}", model_id, e);
                status.is_model_loaded = false;
                status.loaded_model = None;
                status.last_error = Some(msg.clone());
                tracing::error!("{}", msg);
                Err(msg)
            }
        }
    };

    {
        let status = state.status.lock().await;
        emit_status(&app, &status).await;
    }

    // Propagate failure to the caller: returning Ok here would leave the UI's
    // error handling silent, so a failed load looked like nothing happening.
    outcome
}

#[command]
pub async fn download_model(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    model_id: String,
) -> Result<(), String> {
    tracing::info!(model = %model_id, "Downloading Whisper model");

    let asr = state.asr.clone();
    let app_clone = app.clone();
    let event_model_id = model_id.clone();

    // Emitting an event per network chunk floods the IPC channel — thousands of messages a
    // second, which makes the progress bar jitter and stutter instead of advancing. Only
    // report when the figure has moved a visible amount.
    let mut last_reported = -1.0f32;

    asr.download_model(&model_id, move |progress| {
        if progress < 1.0 && (progress - last_reported) < 0.005 {
            return;
        }
        last_reported = progress;
        app_clone
            .emit(
                "model-download-progress",
                serde_json::json!({ "modelId": event_model_id, "progress": progress }),
            )
            .ok();
    })
    .await
    .map_err(|e| {
        let msg = format!("Failed to download model \"{}\": {}", model_id, e);
        tracing::error!("{}", msg);
        msg
    })?;

    tracing::info!(model = %model_id, "Model download complete");
    Ok(())
}

#[command]
pub async fn get_downloaded_models(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let asr = state.asr.clone();
    let models = tokio::task::spawn_blocking(move || asr.list_available_models())
        .await
        .map_err(|e| e.to_string())?;
    Ok(models.into_iter().filter(|m| m.downloaded).map(|m| m.id).collect())
}

#[command]
pub async fn get_available_models(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<crate::asr::ModelInfo>, String> {
    let asr = state.asr.clone();
    tokio::task::spawn_blocking(move || asr.list_available_models())
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn delete_model(state: tauri::State<'_, AppState>, model_id: String) -> Result<(), String> {
    let path = state.asr.get_model_path(&model_id);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ---- Dictionary ----

#[derive(Debug, Deserialize)]
pub struct NewDictionaryEntry {
    pub word: String,
    pub replacement: String,
    #[serde(rename = "caseSensitive", default)]
    pub case_sensitive: bool,
    #[serde(rename = "wholeWord", default = "default_true")]
    pub whole_word: bool,
}

fn default_true() -> bool {
    true
}

#[command]
pub async fn get_dictionary(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<crate::db::DictionaryRow>, String> {
    let db = state.db.lock().await;
    db.list_dictionary().await.map_err(|e| e.to_string())
}

#[command]
pub async fn add_dictionary_entry(
    state: tauri::State<'_, AppState>,
    entry: NewDictionaryEntry,
) -> Result<i64, String> {
    let db = state.db.lock().await;
    db.add_dictionary_entry(&entry.word, &entry.replacement, entry.case_sensitive, entry.whole_word)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn delete_dictionary_entry(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_dictionary_entry(id).await.map_err(|e| e.to_string())
}

// ---- Snippets ----

#[command]
pub async fn get_snippets(state: tauri::State<'_, AppState>) -> Result<Vec<crate::db::SnippetRow>, String> {
    let db = state.db.lock().await;
    db.list_snippets().await.map_err(|e| e.to_string())
}

#[command]
pub async fn add_snippet(
    state: tauri::State<'_, AppState>,
    trigger: String,
    expansion: String,
    description: Option<String>,
) -> Result<i64, String> {
    let db = state.db.lock().await;
    db.add_snippet(&trigger, &expansion, &description)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn delete_snippet(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_snippet(id).await.map_err(|e| e.to_string())
}

// ---- App profiles ----

#[command]
pub async fn get_app_profiles(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<crate::db::AppProfileRow>, String> {
    let db = state.db.lock().await;
    db.list_app_profiles().await.map_err(|e| e.to_string())
}

#[command]
pub async fn add_app_profile(
    state: tauri::State<'_, AppState>,
    app_name: String,
    app_identifier: Option<String>,
    style: String,
    ai_cleanup: bool,
    auto_punctuation: bool,
) -> Result<i64, String> {
    let db = state.db.lock().await;
    db.add_app_profile(&app_name, &app_identifier, &style, ai_cleanup, auto_punctuation)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn delete_app_profile(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_app_profile(id).await.map_err(|e| e.to_string())
}

// ---- Transcripts ----

#[command]
pub async fn get_transcripts(
    state: tauri::State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<crate::db::TranscriptRow>, String> {
    let db = state.db.lock().await;
    db.list_transcripts(limit, offset).await.map_err(|e| e.to_string())
}

#[command]
pub async fn clear_transcripts(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().await;
    db.clear_transcripts().await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_stats(state: tauri::State<'_, AppState>) -> Result<crate::db::UsageStats, String> {
    let db = state.db.lock().await;
    db.stats().await.map_err(|e| e.to_string())
}

// ---- System ----

#[command]
pub async fn set_start_at_login(enabled: bool) -> Result<(), String> {
    crate::system::set_startup(enabled)
}

#[command]
pub async fn open_settings_folder() -> Result<(), String> {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("AuraScribe");

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(data_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Opening the settings folder is not yet implemented on this platform".to_string())
    }
}

#[command]
pub async fn list_audio_devices() -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(crate::audio::list_input_devices)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn check_microphone_permission() -> Result<bool, String> {
    tokio::task::spawn_blocking(|| {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => return false,
        };
        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let stream_config: cpal::StreamConfig = config.into();

        let result = device.build_input_stream(
            &stream_config,
            move |_data: &[f32], _: &cpal::InputCallbackInfo| {},
            |_err| {},
            None,
        );

        match result {
            Ok(stream) => stream.play().is_ok(),
            Err(_) => false,
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[command]
pub async fn request_microphone_permission() -> Result<bool, String> {
    check_microphone_permission().await
}

#[command]
pub async fn check_accessibility_permission() -> Result<bool, String> {
    // SendInput on Windows does not require the Accessibility permission that
    // macOS gates synthetic keyboard events behind.
    #[cfg(target_os = "windows")]
    {
        Ok(true)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}

#[command]
pub async fn request_accessibility_permission() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Accessibility permission handling is not yet implemented on this platform".to_string())
    }
}

#[command]
pub async fn get_log_file_path() -> Result<String, String> {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("AuraScribe");
    Ok(data_dir.join("aurascribe.log").to_string_lossy().to_string())
}

/// Called by the overlay page once it has mounted. Until this arrives the overlay is never
/// shown, so a page that failed to load cannot park an error box on the user's screen.
#[command]
pub fn overlay_ready() {
    crate::overlay::mark_ready();
}
