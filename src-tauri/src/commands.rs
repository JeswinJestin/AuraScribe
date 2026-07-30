// src-tauri/src/commands.rs
//! Tauri command handlers for frontend IPC

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::mpsc;

use crate::{
    asr::WhisperASR,
    audio::{AudioCapture, VoiceActivityDetector, VoiceActivity},
    db::{Database, Settings as DbSettings},
    injection::TextInjector,
    llm::LLMCleanup,
    models::ModelManager,
    system::{GlobalShortcutManager, SystemTrayManager},
    vad::SileroVad,
};

// State structures
pub struct AppState {
    pub asr: Arc<Mutex<WhisperASR>>,
    pub audio_capture: Arc<Mutex<Option<AudioCapture>>>,
    pub vad: Arc<Mutex<SileroVad>>,
    pub voice_detector: Arc<Mutex<Option<VoiceActivityDetector>>>,
    pub injector: Arc<Mutex<Option<TextInjector>>>,
    pub llm_cleanup: Arc<Mutex<Option<LLMCleanup>>>,
    pub model_manager: Arc<Mutex<ModelManager>>,
    pub db: Arc<Mutex<Database>>,
    pub shortcut_manager: Arc<Mutex<GlobalShortcutManager>>,
    pub tray_manager: Arc<Mutex<SystemTrayManager>>,
    pub settings: Arc<Mutex<Settings>>,
    pub recording_state: Arc<Mutex<RecordingState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: String,
    pub hotkey_mode: String, // "press-hold" | "toggle"
    pub whisper_model: String,
    pub openrouter_key: String,
    pub openrouter_model: String,
    pub ai_cleanup_enabled: bool,
    pub auto_punctuation: bool,
    pub language: String,
    pub theme: String,
    pub start_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Space".to_string(),
            hotkey_mode: "press-hold".to_string(),
            whisper_model: "base.en".to_string(),
            openrouter_key: String::new(),
            openrouter_model: "nvidia/nemotron-3-ultra".to_string(),
            ai_cleanup_enabled: false,
            auto_punctuation: true,
            language: "en".to_string(),
            theme: "system".to_string(),
            start_at_login: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub is_recording: bool,
    pub is_processing: bool,
    pub is_model_loaded: bool,
    pub current_text: String,
    pub last_error: Option<String>,
    pub hotkey_mode: String,
    pub ai_cleanup_enabled: bool,
}

#[derive(Debug, Default)]
struct RecordingState {
    is_recording: bool,
    is_processing: bool,
    buffer: Vec<f32>,
    current_transcript: String,
    audio_receiver: Option<mpsc::UnboundedReceiver<Vec<f32>>>,
}

// Initialize state
pub fn init_state(app: &AppHandle) -> Result<AppState> {
    let db = Database::new(app)?;
    let settings = db.load_settings()?;

    let asr = WhisperASR::new()?;
    let vad = SileroVad::new()?;
    let model_manager = ModelManager::new()?;

    Ok(AppState {
        asr: Arc::new(Mutex::new(asr)),
        audio_capture: Arc::new(Mutex::new(None)),
        vad: Arc::new(Mutex::new(vad)),
        voice_detector: Arc::new(Mutex::new(None)),
        injector: Arc::new(Mutex::new(None)),
        llm_cleanup: Arc::new(Mutex::new(None)),
        model_manager: Arc::new(Mutex::new(model_manager)),
        db: Arc::new(Mutex::new(db)),
        shortcut_manager: Arc::new(Mutex::new(GlobalShortcutManager::new())),
        tray_manager: Arc::new(Mutex::new(SystemTrayManager::new(app.clone())?)),
        settings: Arc::new(Mutex::new(settings)),
        recording_state: Arc::new(Mutex::new(RecordingState::default())),
    })
}

// ===== Commands =====

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let mut s = state.settings.lock().unwrap();
    *s = settings.clone();

    // Persist to database
    state.db.lock().unwrap().save_settings(&settings)?;

    // Update hotkey if changed
    state.shortcut_manager.lock().unwrap().update_hotkey(&settings.hotkey, &settings.hotkey_mode)?;

    // Emit event
    state.app_handle().emit("settings-changed", &settings).ok();

    Ok(())
}

#[tauri::command]
pub async fn load_settings(state: State<'_, AppState>) -> Result<(), String> {
    let settings = state.db.lock().unwrap().load_settings()?;
    *state.settings.lock().unwrap() = settings.clone();
    state.app_handle().emit("settings-changed", &settings).ok();
    Ok(())
}

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<Status, String> {
    let settings = state.settings.lock().unwrap();
    let recording = state.recording_state.lock().unwrap();
    let asr = state.asr.lock().unwrap();

    Ok(Status {
        is_recording: recording.is_recording,
        is_processing: recording.is_processing,
        is_model_loaded: asr.get_current_model().is_some(),
        current_text: recording.current_transcript.clone(),
        last_error: None,
        hotkey_mode: settings.hotkey_mode.clone(),
        ai_cleanup_enabled: settings.ai_cleanup_enabled,
    })
}

#[tauri::command]
pub async fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    let mut recording = state.recording_state.lock().unwrap();

    if recording.is_recording {
        return Ok(());
    }

    // Load model if not loaded
    let model_id = {
        let settings = state.settings.lock().unwrap();
        settings.whisper_model.clone()
    };

    {
        let asr = state.asr.lock().unwrap();
        if asr.get_current_model() != Some(model_id.clone()) {
            asr.load_model(&model_id).map_err(|e| e.to_string())?;
        }
    }

    // Initialize audio capture
    let vad = state.vad.lock().unwrap().clone();
    let mut audio_capture = AudioCapture::new(vad).map_err(|e| e.to_string())?;

    let app_handle = state.app_handle().clone();
    let rx = audio_capture.start(app_handle.clone()).map_err(|e| e.to_string())?;

    // Initialize voice detector
    let vad_clone = state.vad.lock().unwrap().clone();
    let voice_detector = VoiceActivityDetector::new(vad_clone);

    recording.is_recording = true;
    recording.is_processing = false;
    recording.buffer.clear();
    recording.current_transcript.clear();
    recording.audio_receiver = Some(rx);

    *state.audio_capture.lock().unwrap() = Some(audio_capture);
    *state.voice_detector.lock().unwrap() = Some(voice_detector);

    // Initialize injector
    let injector = TextInjector::new(app_handle.clone());
    *state.injector.lock().unwrap() = Some(injector);

    // Initialize LLM cleanup if enabled
    let settings = state.settings.lock().unwrap();
    if settings.ai_cleanup_enabled && !settings.openrouter_key.is_empty() {
        let llm = LLMCleanup::new(&settings.openrouter_key, &settings.openrouter_model);
        *state.llm_cleanup.lock().unwrap() = Some(llm);
    }

    // Start processing loop
    let state_clone = state.inner().clone();
    tokio::spawn(async move {
        process_audio_loop(state_clone).await;
    });

    state.app_handle().emit("status-changed", get_status_internal(&state).await).ok();

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<(), String> {
    let mut recording = state.recording_state.lock().unwrap();

    if !recording.is_recording {
        return Ok(());
    }

    recording.is_recording = false;
    recording.is_processing = true;

    // Stop audio capture
    if let Some(mut capture) = state.audio_capture.lock().unwrap().take() {
        capture.stop();
    }

    // Process remaining buffer
    if !recording.buffer.is_empty() {
        recording.is_processing = true;
        let buffer = std::mem::take(&mut recording.buffer);
        let asr = state.asr.lock().unwrap();

        match asr.transcribe(&buffer, Some("en")) {
            Ok(text) => {
                recording.current_transcript = text.clone();

                // Apply AI cleanup if enabled
                let settings = state.settings.lock().unwrap();
                let final_text = if settings.ai_cleanup_enabled {
                    if let Some(llm) = state.llm_cleanup.lock().unwrap().as_ref() {
                        llm.cleanup(&text, &settings.language).await.unwrap_or(text)
                    } else {
                        text
                    }
                } else {
                    text
                };

                // Inject into active app
                if let Some(injector) = state.injector.lock().unwrap().as_ref() {
                    injector.inject(&final_text).ok();
                }

                // Save transcript
                state.db.lock().unwrap().save_transcript(&text, &final_text, "base.en", 0).ok();
            }
            Err(e) => {
                tracing::error!("Transcription failed: {}", e);
            }
        }
    }

    recording.is_processing = false;

    state.app_handle().emit("status-changed", get_status_internal(&state).await).ok();
    state.app_handle().emit("transcript-received", &recording.current_transcript).ok();

    Ok(())
}

async fn process_audio_loop(state: Arc<AppState>) {
    let mut rx = {
        let mut recording = state.recording_state.lock().unwrap();
        recording.audio_receiver.take()
    };

    if rx.is_none() {
        return;
    }

    let mut rx = rx.unwrap();

    while let Some(chunk) = rx.recv().await {
        let should_continue = {
            let recording = state.recording_state.lock().unwrap();
            recording.is_recording
        };

        if !should_continue {
            break;
        }

        // Add to buffer
        {
            let mut recording = state.recording_state.lock().unwrap();
            recording.buffer.extend_from_slice(&chunk);
        }

        // Process through VAD
        let voice_activity = {
            let mut detector = state.voice_detector.lock().unwrap();
            if let Some(detector) = detector.as_mut() {
                detector.process(&chunk)
            } else {
                VoiceActivity::Speech
            }
        };

        match voice_activity {
            VoiceActivity::Speech => {
                // Continue accumulating
            }
            VoiceActivity::EndOfSpeech(speech_audio) => {
                // Transcribe the speech segment
                let asr = state.asr.lock().unwrap();
                if let Ok(text) = asr.transcribe(&speech_audio, Some("en")) {
                    let mut recording = state.recording_state.lock().unwrap();
                    recording.current_transcript.push_str(&text);
                    recording.current_transcript.push(' ');

                    // Emit partial transcript
                    state.app_handle().emit("transcript-received", &recording.current_transcript).ok();
                }
            }
            _ => {}
        }
    }
}

async fn get_status_internal(state: &State<'_, AppState>) -> Status {
    let settings = state.settings.lock().unwrap();
    let recording = state.recording_state.lock().unwrap();
    let asr = state.asr.lock().unwrap();

    Status {
        is_recording: recording.is_recording,
        is_processing: recording.is_processing,
        is_model_loaded: asr.get_current_model().is_some(),
        current_text: recording.current_transcript.clone(),
        last_error: None,
        hotkey_mode: settings.hotkey_mode.clone(),
        ai_cleanup_enabled: settings.ai_cleanup_enabled,
    }
}

// Model management
#[tauri::command]
pub async fn download_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let asr = state.asr.lock().unwrap();
    asr.download_model(&model_id).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn load_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let asr = state.asr.lock().unwrap();
    asr.load_model(&model_id).map_err(|e| e.to_string())?;

    // Update settings
    let mut settings = state.settings.lock().unwrap();
    settings.whisper_model = model_id;
    state.db.lock().unwrap().save_settings(&settings)?;

    Ok(())
}

#[tauri::command]
pub async fn list_models(state: State<'_, AppState>) -> Result<Vec<crate::asr::ModelInfo>, String> {
    let asr = state.asr.lock().unwrap();
    Ok(asr.list_available_models())
}

#[tauri::command]
pub async fn delete_model(state: State<'_, AppState>, model_id: String) -> Result<(), String> {
    let asr = state.asr.lock().unwrap();
    asr.delete_model(&model_id).map_err(|e| e.to_string())
}

// Dictionary
#[tauri::command]
pub async fn get_dictionary(state: State<'_, AppState>) -> Result<Vec<crate::db::DictionaryEntry>, String> {
    state.db.lock().unwrap().get_dictionary().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_dictionary_entry(
    state: State<'_, AppState>,
    entry: crate::db::DictionaryEntry,
) -> Result<i64, String> {
    state.db.lock().unwrap().add_dictionary_entry(entry).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_dictionary_entry(
    state: State<'_, AppState>,
    id: i64,
    entry: crate::db::DictionaryEntry,
) -> Result<(), String> {
    state.db.lock().unwrap().update_dictionary_entry(id, entry).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_dictionary_entry(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.db.lock().unwrap().delete_dictionary_entry(id).map_err(|e| e.to_string())
}

// Snippets
#[tauri::command]
pub async fn get_snippets(state: State<'_, AppState>) -> Result<Vec<crate::db::SnippetEntry>, String> {
    state.db.lock().unwrap().get_snippets().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_snippet(
    state: State<'_, AppState>,
    snippet: crate::db::SnippetEntry,
) -> Result<i64, String> {
    state.db.lock().unwrap().add_snippet(snippet).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_snippet(
    state: State<'_, AppState>,
    id: i64,
    snippet: crate::db::SnippetEntry,
) -> Result<(), String> {
    state.db.lock().unwrap().update_snippet(id, snippet).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_snippet(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.db.lock().unwrap().delete_snippet(id).map_err(|e| e.to_string())
}

// App Profiles
#[tauri::command]
pub async fn get_app_profiles(state: State<'_, AppState>) -> Result<Vec<crate::db::AppProfile>, String> {
    state.db.lock().unwrap().get_app_profiles().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_app_profile(
    state: State<'_, AppState>,
    profile: crate::db::AppProfile,
) -> Result<i64, String> {
    state.db.lock().unwrap().add_app_profile(profile).map_err(|e| e.to_string())
}

// Transcripts
#[tauri::command]
pub async fn get_transcripts(
    state: State<'_, AppState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<crate::db::TranscriptEntry>, String> {
    state.db.lock().unwrap()
        .get_transcripts(limit.unwrap_or(100), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_transcripts(state: State<'_, AppState>) -> Result<(), String> {
    state.db.lock().unwrap().clear_transcripts().map_err(|e| e.to_string())
}

// AI Cleanup
#[tauri::command]
pub async fn cleanup_with_ai(
    state: State<'_, AppState>,
    text: String,
    style: String,
    openrouter_key: String,
    openrouter_model: String,
) -> Result<String, String> {
    let llm = crate::llm::LLMCleanup::new(&openrouter_key, &openrouter_model);
    llm.cleanup(&text, &style).await.map_err(|e| e.to_string())
}

// System
#[tauri::command]
pub async fn set_start_at_login(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.start_at_login = enabled;
    state.db.lock().unwrap().save_settings(&settings)?;
    state.shortcut_manager.lock().unwrap().set_autostart(enabled)?;
    Ok(())
}

#[tauri::command]
pub async fn open_settings_folder(state: State<'_, AppState>) -> Result<(), String> {
    let path = state.db.lock().unwrap().get_data_dir()?;
    tauri::plugin::opener::open_path(path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_microphone_permission() -> Result<bool, String> {
    // Platform-specific check
    Ok(true) // Simplified
}

#[tauri::command]
pub async fn request_microphone_permission() -> Result<bool, String> {
    Ok(true)
}

#[tauri::command]
pub async fn check_accessibility_permission() -> Result<bool, String> {
    Ok(true)
}

#[tauri::command]
pub async fn request_accessibility_permission() -> Result<(), String> {
    Ok(())
}