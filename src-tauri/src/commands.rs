use crate::app_state::AppState;
use crate::cleanup::{self, CleanupOptions};
use crate::db::Database;
use crate::injection::TextInjector;
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: String,
    pub hotkey_mode: String,
    pub whisper_model: String,
    pub mic_device: Option<String>,
    pub ai_cleanup_enabled: bool,
    pub remove_fillers: bool,
    pub language: String,
    pub theme: String,
    pub start_at_login: bool,
    pub sound_cues: bool,
    /// True once the first-run walkthrough has been dismissed. The UI shows onboarding while
    /// this is false; completing (or skipping) it saves `true`.
    pub onboarded: bool,
    /// When false, the global dictation hotkey is not registered — the app effectively "sleeps"
    /// and no keypress triggers recording until the user re-enables it in Settings. Default true.
    pub hotkey_enabled: bool,
}

/// Platform-appropriate default global shortcut. Always a modifier + a non-alphabet key (a bare
/// letter would hijack that key while typing), chosen to dodge each OS's reserved combos:
/// Windows/Linux use `Ctrl+Shift+Space`; macOS uses `Super+Shift+Space` (Cmd+Shift+Space), steering
/// clear of Cmd+Space (Spotlight) and Ctrl+Space (input-source switch). "Super" is how this app's
/// hotkey format spells the Cmd/Win key (see `HotkeyCapture`), and tauri's shortcut parser maps it
/// to Cmd on macOS. NOTE: the macOS default has not been validated on a real Mac yet — do that when
/// the macOS build is exercised in CI/on-target.
pub fn default_hotkey() -> String {
    #[cfg(target_os = "macos")]
    {
        "Super+Shift+Space".to_string()
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Ctrl+Shift+Space".to_string()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
            hotkey_mode: "toggle".to_string(),
            // Moonshine is the shipped default engine (v0.4.x bundles it). The Whisper catalogue
            // is now empty, so a Whisper-only dev build ships no built-in model — the default
            // simply won't resolve there, which is fine (that build is not the product).
            whisper_model: "moonshine-base-en".to_string(),
            mic_device: None,
            ai_cleanup_enabled: true,
            remove_fillers: true,
            language: "en".to_string(),
            theme: "glass".to_string(),
            start_at_login: false,
            sound_cues: true,
            // Safe fallback if the DB read ever fails: don't nag. A genuine fresh install gets
            // onboarded = 0 from the database (see `Database::new`), which is authoritative.
            onboarded: true,
            hotkey_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub is_recording: bool,
    pub is_processing: bool,
    pub is_model_loaded: bool,
    /// Which model is actually in memory right now. The frontend must trust this rather
    /// than the saved setting — the two diverge whenever a load fails or is in flight.
    pub loaded_model: Option<String>,
    pub current_text: String,
    pub last_error: Option<String>,
    pub hotkey_mode: String,
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
        sound_cues: row.sound_cues != 0,
        onboarded: row.onboarded != 0,
        hotkey_enabled: row.hotkey_enabled != 0,
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
    // Register the hotkey only when it's enabled; when disabled, tear it down so no keypress
    // triggers dictation ("sleep" the app from Settings).
    if settings.hotkey_enabled {
        crate::hotkey::apply(&app, &settings.hotkey, &settings.hotkey_mode)
            .map_err(|e| format!("Invalid hotkey \"{}\": {}", settings.hotkey, e))?;
    } else {
        crate::hotkey::disable(&app);
    }

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
            sound_cues: settings.sound_cues as i32,
            onboarded: settings.onboarded as i32,
            hotkey_enabled: settings.hotkey_enabled as i32,
        })
        .await
        .map_err(|e| e.to_string())?;
    }

    crate::sound::set_enabled(settings.sound_cues);

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

/// Transcribes the recording in pieces while it is still being made.
///
/// Started by `start_recording`, awaited by `stop_recording`. On seeing the stop flag it
/// transcribes whatever is left and exits, so awaiting it is what guarantees the tail of the
/// recording is not dropped.
///
/// **The audio buffer lock is held for microseconds only.** The cpal capture callback takes
/// it with `try_lock` and silently discards samples when it cannot — so holding it across a
/// transcription (seconds) would punch holes in the user's recording. Samples are drained
/// into a local buffer and the lock released before any work happens.
#[allow(clippy::too_many_arguments)]
async fn run_chunker(
    app: AppHandle,
    asr: std::sync::Arc<crate::engine::Asr>,
    audio_buffer: std::sync::Arc<tokio::sync::Mutex<Vec<f32>>>,
    sample_rate: std::sync::Arc<tokio::sync::Mutex<u32>>,
    stop_flag: std::sync::Arc<tokio::sync::Mutex<bool>>,
    status: std::sync::Arc<tokio::sync::Mutex<Status>>,
    chunk_state: std::sync::Arc<tokio::sync::Mutex<crate::app_state::ChunkState>>,
    language: String,
    // Only split during recording when the model can keep pace with speech. For a slower
    // model, splitting cannot drain the backlog and each small chunk costs an extra
    // whisper 30-second window, so the whole recording is transcribed once at stop instead.
    live_chunking: bool,
) {
    // Pending audio stays at the device's rate; resampling happens once per chunk on a
    // contiguous block, rather than repeatedly across arbitrary boundaries.
    let mut pending: Vec<f32> = Vec::new();
    let mut rate: u32 = 16000;

    loop {
        let drained: Vec<f32> = {
            let mut buf = audio_buffer.lock().await;
            std::mem::take(&mut *buf)
        };
        if !drained.is_empty() {
            rate = *sample_rate.lock().await;
            pending.extend_from_slice(&drained);
            let mut cs = chunk_state.lock().await;
            cs.raw_samples += drained.len();
            cs.sample_rate = rate;
        }

        let stopping = *stop_flag.lock().await;

        // Cut and transcribe every complete piece available right now — but only if live
        // chunking helps this model. Otherwise everything falls through to the single
        // transcription at stop below.
        while live_chunking {
            let Some(idx) = crate::chunking::find_split_point(&pending, rate) else {
                break;
            };
            let chunk: Vec<f32> = pending.drain(..idx).collect();
            transcribe_chunk(&app, &asr, &status, &chunk_state, chunk, rate, &language).await;
        }

        if stopping {
            if !pending.is_empty() {
                let tail = std::mem::take(&mut pending);
                transcribe_chunk(&app, &asr, &status, &chunk_state, tail, rate, &language).await;
            }
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

/// Transcribe one piece and append it to the running transcript.
async fn transcribe_chunk(
    app: &AppHandle,
    asr: &std::sync::Arc<crate::engine::Asr>,
    status: &std::sync::Arc<tokio::sync::Mutex<Status>>,
    chunk_state: &std::sync::Arc<tokio::sync::Mutex<crate::app_state::ChunkState>>,
    chunk: Vec<f32>,
    rate: u32,
    language: &str,
) {
    let resampled = crate::audio::resample_linear(&chunk, rate, 16000);
    if resampled.is_empty() {
        return;
    }

    // Strip dead air before Whisper sees it. Whisper is charged for silence exactly like
    // speech, so cutting the pauses and the gaps at each end is a real speed win on every
    // model and every machine, at no accuracy cost.
    let trimmed = crate::chunking::trim_silence(&resampled, 16000);
    if trimmed.is_empty() {
        return;
    }

    // Boost a soft/quiet recording toward a healthy level so a soft-spoken user transcribes as well
    // as a loud one. Conservative: only amplifies, capped, leaves already-loud audio untouched.
    let trimmed = crate::chunking::normalize_gain(&trimmed);

    let asr = asr.clone();
    let lang = language.to_string();
    let result = tokio::task::spawn_blocking(move || {
        let l = if lang == "auto" { None } else { Some(lang.as_str()) };
        asr.transcribe(&trimmed, l)
    })
    .await;

    let text = match result {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => {
            tracing::warn!("Chunk transcription failed: {}", e);
            return;
        }
        Err(e) => {
            tracing::warn!("Chunk transcription task panicked: {}", e);
            return;
        }
    };

    if text.trim().is_empty() {
        return;
    }

    let running = {
        let mut cs = chunk_state.lock().await;
        cs.texts.push(text);
        cs.texts.join(" ")
    };
    tracing::debug!(chars = running.len(), "Chunk transcribed");

    // Show the transcript building up while the user is still talking. Without this the
    // work is invisible and the app looks idle right up until the text lands.
    let mut st = status.lock().await;
    st.current_text = running;
    emit_status(app, &st).await;
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

    // Remember the window the user is dictating into, captured now (before the overlay appears),
    // so the transcript can be pasted back into it even if stopping via the overlay moved focus.
    *state.target_window.lock().await = crate::system::capture_foreground_window();

    // Cue plays before the mic stream opens below, so the tone is never captured into the
    // recording. This is also the fastest possible acknowledgement of the hotkey — it fires
    // even when no window is open.
    crate::sound::play_start();

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
    {
        let mut cs = state.chunk_state.lock().await;
        *cs = Default::default();
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

    // Transcribe while the user speaks, so the wait after they stop is one chunk rather
    // than the whole recording. Works on any hardware - unlike GPU offload, which only
    // helps machines that have one.
    let language = {
        let db = state.db.lock().await;
        load_settings_from_db(&db)
            .await
            .map(|s| s.language)
            .unwrap_or_else(|_| "en".to_string())
    };
    // Live chunking only pays off when the model keeps up with speech. Decide from the model
    // actually loaded, so a slow model transcribes once at stop rather than fighting a
    // backlog it can never win — and without paying the extra-window penalty of many chunks.
    let live_chunking = {
        let status = state.status.lock().await;
        status
            .loaded_model
            .as_deref()
            .map(|m| state.asr.should_chunk(m))
            .unwrap_or(false)
    };
    tracing::info!(live_chunking, "Recording started");
    let task = tokio::spawn(run_chunker(
        app.clone(),
        state.asr.clone(),
        state.audio_buffer.clone(),
        state.audio_sample_rate.clone(),
        state.stop_flag.clone(),
        state.status.clone(),
        state.chunk_state.clone(),
        language,
        live_chunking,
    ));
    {
        let mut t = state.chunk_task.lock().await;
        *t = Some(task);
    }

    Ok(())
}

#[command]
pub async fn stop_recording(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    crate::sound::play_stop();

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

    // Most of the recording was transcribed while it was being made. Awaiting the chunker
    // is what flushes the tail - it sees the stop flag, transcribes whatever is left, and
    // exits. This await is therefore the only remaining wait the user experiences, and it
    // is bounded by one chunk rather than by the length of the recording.
    let wait_started = std::time::Instant::now();
    if let Some(task) = state.chunk_task.lock().await.take() {
        if let Err(e) = task.await {
            tracing::warn!("Chunker task failed: {}", e);
        }
    }

    let (raw_text, audio_ms) = {
        let mut cs = state.chunk_state.lock().await;
        let text = std::mem::take(&mut cs.texts).join(" ");
        let ms = if cs.sample_rate > 0 {
            (cs.raw_samples as f64 * 1000.0 / cs.sample_rate as f64).round() as i64
        } else {
            0
        };
        *cs = Default::default();
        (text, ms)
    };

    tracing::info!(
        wait_ms = wait_started.elapsed().as_millis() as u64,
        audio_ms,
        "Recording finished"
    );

    if audio_ms == 0 {
        let mut status = state.status.lock().await;
        status.is_processing = false;
        status.last_error = Some("No audio captured".to_string());
        emit_status(&app, &status).await;
        return Ok(());
    }

    let (settings, dictionary, snippets) = {
        let db = state.db.lock().await;
        let settings = load_settings_from_db(&db).await?;
        // Loaded once per dictation, off the hot path. Empty lists make `expand::apply` a no-op,
        // so users who never open Words/Snippets pay nothing.
        let dictionary = db.list_dictionary().await.unwrap_or_default();
        let snippets = db.list_snippets().await.unwrap_or_default();
        (settings, dictionary, snippets)
    };

    let db_arc = state.db.clone();
    let status_arc = state.status.clone();
    let app_clone = app.clone();
    // The window to paste back into (see start_recording). Read now; used just before injection.
    let target_window = *state.target_window.lock().await;

    tokio::spawn(async move {
        // Now measures the wait the user actually experienced, not total transcription
        // work - most of that already happened while they were speaking.
        let start = wait_started;

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

        // Apply the personal dictionary and snippet expansions to the final text. This is what
        // makes the Words and Snippets screens actually affect dictation — before this they were
        // stored and displayed but never touched the transcript. Runs even when cleanup is off,
        // so corrections and canned phrases work regardless of that toggle.
        let cleaned = crate::expand::apply(&cleaned, &dictionary, &snippets);

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

        // Restore focus to the app the user was dictating into before pasting. This is the fix
        // for click-to-stop: the overlay click can steal foreground, and injection targets the
        // focused window — so without this the paste went nowhere (the hotkey-stop path, which
        // never moved focus, always worked). A no-op when focus never moved.
        crate::system::focus_window(target_window);

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
    state.asr.delete_model(&model_id).map_err(|e| e.to_string())?;
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
pub async fn search_transcripts(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<crate::db::TranscriptRow>, String> {
    let db = state.db.lock().await;
    db.search_transcripts(&query, limit, offset)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn delete_transcript(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_transcript(id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn clear_transcripts(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().await;
    db.clear_transcripts().await.map_err(|e| e.to_string())
}

/// Per-day dictation counts over the last `days` days, for the History usage heatmap.
#[command]
pub async fn transcript_daily_counts(
    state: tauri::State<'_, AppState>,
    days: i64,
) -> Result<Vec<crate::db::DailyCount>, String> {
    let since = chrono::Utc::now().timestamp() - days.max(1) * 86_400;
    let db = state.db.lock().await;
    db.daily_counts(since).await.map_err(|e| e.to_string())
}

/// Delete every transcript whose timestamp is within `[start, end]` (unix seconds, inclusive).
/// Returns how many were removed so the UI can report it.
#[command]
pub async fn delete_transcripts_between(
    state: tauri::State<'_, AppState>,
    start: i64,
    end: i64,
) -> Result<u64, String> {
    let db = state.db.lock().await;
    db.delete_transcripts_between(start, end)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_stats(state: tauri::State<'_, AppState>) -> Result<crate::db::UsageStats, String> {
    let db = state.db.lock().await;
    db.stats().await.map_err(|e| e.to_string())
}

/// The dictation streak + freeze economy for the Insights page. Reconciles on every read (cheap and
/// idempotent within a day), which is what makes the streak self-healing across app restarts and
/// missed days without needing a background job — see `streaks.rs` and the design spec.
#[command]
pub async fn get_streak_state(
    state: tauri::State<'_, AppState>,
) -> Result<crate::streaks::StreakInfo, String> {
    use crate::streaks::{StreakState, MIN_WORDS_PER_DAY};
    use std::collections::BTreeSet;

    let db = state.db.lock().await;
    let day_data = db
        .streak_day_data(MIN_WORDS_PER_DAY)
        .await
        .map_err(|e| e.to_string())?;
    let row = db.load_streak_state().await.map_err(|e| e.to_string())?;

    let prior = StreakState {
        current_streak: row.current_streak,
        longest_streak: row.longest_streak,
        freezes: row.freezes,
        earn_progress: row.earn_progress,
        last_reconciled_day: row.last_reconciled_day,
        backfilled: row.backfilled != 0,
    };

    let counted: BTreeSet<i64> = day_data.counted.iter().copied().collect();
    let next = prior.reconcile(&counted, day_data.today_ordinal);

    // Only write when the finalized state actually changed, so a plain read costs no DB write.
    if next != prior {
        let updated = crate::db::StreakStateRow {
            current_streak: next.current_streak,
            longest_streak: next.longest_streak,
            freezes: next.freezes,
            earn_progress: next.earn_progress,
            last_reconciled_day: next.last_reconciled_day,
            backfilled: next.backfilled as i64,
        };
        db.save_streak_state(&updated)
            .await
            .map_err(|e| e.to_string())?;
    }

    let today_counted = counted.contains(&day_data.today_ordinal);
    Ok(next.to_info(today_counted, day_data.words_today))
}

/// The "Your Year" recap for the Insights page. `year` defaults to the current local year; the
/// frontend passes an explicit year (e.g. last year in January, when that recap is the interesting one).
#[command]
pub async fn get_year_recap(
    state: tauri::State<'_, AppState>,
    year: Option<i32>,
) -> Result<crate::db::YearRecap, String> {
    use chrono::Datelike;
    let year = year.unwrap_or_else(|| chrono::Local::now().year());
    let db = state.db.lock().await;
    db.year_recap(year).await.map_err(|e| e.to_string())
}

/// Save a shareable PNG (a streak or recap card rendered on the frontend canvas) to the user's
/// Pictures folder and reveal it in Explorer. Stays dependency-free — no dialog/fs plugin — and the
/// image never leaves the machine; the user chooses whether to share the file. Returns the path.
#[command]
pub async fn save_share_image(name: String, bytes: Vec<u8>) -> Result<String, String> {
    if bytes.is_empty() {
        return Err("nothing to save".into());
    }
    let dir = dirs::picture_dir()
        .or_else(dirs::desktop_dir)
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    // Sanitise the filename to a safe stem.
    let stem: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let stem = stem.trim_matches('-');
    let stem = if stem.is_empty() { "aurascribe-share" } else { stem };
    let file = dir.join(format!("{stem}.png"));
    std::fs::write(&file, &bytes).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&file)
            .spawn();
    }
    Ok(file.display().to_string())
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
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(data_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(data_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
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
    // macOS gates synthetic keyboard events AND global hotkeys behind the Accessibility permission;
    // Windows (SendInput) and Linux/X11 (XTEST) need no such grant.
    #[cfg(target_os = "macos")]
    {
        Ok(macos_accessibility_client::accessibility::application_is_trusted())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

#[command]
pub async fn request_accessibility_permission() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Prompts the user and deep-links to System Settings → Privacy & Security → Accessibility.
        // Without this grant, dictation cannot type into other apps and the hotkey won't fire.
        macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
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

#[cfg(test)]
mod wire_format_tests {
    use super::*;

    /// The frontend reads these field names literally (`src/lib/ipc.ts`), and `invoke()`
    /// returns an unchecked shape — TypeScript asserts the type rather than verifying it.
    /// So a rename here fails silently at runtime instead of at compile time.
    ///
    /// It already happened: `Status` carried `#[serde(rename = "isModelLoaded")]` while the
    /// UI read `is_model_loaded`, so the field was permanently `undefined`. The Dictate
    /// screen showed "Add a voice model to begin" no matter what was loaded, the sidebar
    /// never showed recording state, and `save_settings` could not deserialize what the UI
    /// sent. One mismatch, four broken features, and nothing anywhere reported an error.
    ///
    /// These tests are the contract. If one fails, update `src/lib/ipc.ts` in the same
    /// commit — do not just change the expected string.
    #[test]
    fn status_serializes_with_the_field_names_the_ui_reads() {
        let json = serde_json::to_value(Status::default()).unwrap();
        for field in [
            "is_recording",
            "is_processing",
            "is_model_loaded",
            "loaded_model",
            "current_text",
            "last_error",
            "hotkey_mode",
            "ai_cleanup_enabled",
        ] {
            assert!(
                json.get(field).is_some(),
                "Status is missing `{field}` on the wire; src/lib/ipc.ts reads that name"
            );
        }
    }

    #[test]
    fn settings_round_trip_through_the_wire_shape() {
        let json = serde_json::to_value(Settings::default()).unwrap();
        for field in [
            "hotkey",
            "hotkey_mode",
            "whisper_model",
            "mic_device",
            "ai_cleanup_enabled",
            "remove_fillers",
            "language",
            "theme",
            "start_at_login",
            "sound_cues",
            "onboarded",
            "hotkey_enabled",
        ] {
            assert!(
                json.get(field).is_some(),
                "Settings is missing `{field}` on the wire; src/lib/ipc.ts sends that name"
            );
        }

        // save_settings deserializes what the UI sends, so the reverse must hold too.
        let back: Settings = serde_json::from_value(json).expect(
            "Settings must deserialize from its own serialized form — save_settings depends on it",
        );
        assert_eq!(back.hotkey, Settings::default().hotkey);
    }
}
