use crate::app_state::AppState;
use crate::db::Database;
use crate::injection::TextInjector;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: String,
    #[serde(rename = "hotkeyMode")]
    pub hotkey_mode: String,
    #[serde(rename = "whisperModel")]
    pub whisper_model: String,
    #[serde(rename = "openrouterKey")]
    pub openrouter_key: String,
    #[serde(rename = "openrouterModel")]
    pub openrouter_model: String,
    #[serde(rename = "aiCleanupEnabled")]
    pub ai_cleanup_enabled: bool,
    #[serde(rename = "autoPunctuation")]
    pub auto_punctuation: bool,
    pub language: String,
    pub theme: String,
    #[serde(rename = "startAtLogin")]
    pub start_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Alt".to_string(),
            hotkey_mode: "toggle".to_string(),
            whisper_model: "base.en".to_string(),
            openrouter_key: String::new(),
            openrouter_model: "nvidia/nemotron-3-ultra".to_string(),
            ai_cleanup_enabled: false,
            auto_punctuation: true,
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
            current_text: String::new(),
            last_error: None,
            hotkey_mode: "toggle".to_string(),
            ai_cleanup_enabled: false,
        }
    }
}

async fn emit_status(app: &AppHandle, status: &Status) {
    app.emit("status-changed", status.clone()).ok();
}

async fn load_settings_from_db(db: &Database) -> Result<Settings, String> {
    let row = db.load_settings().await.map_err(|e| e.to_string())?;
    Ok(Settings {
        hotkey: row.hotkey.unwrap_or_else(|| "Ctrl+Alt".to_string()),
        hotkey_mode: row
            .hotkey_mode
            .unwrap_or_else(|| "toggle".to_string()),
        whisper_model: row
            .whisper_model
            .unwrap_or_else(|| "base.en".to_string()),
        openrouter_key: row.openrouter_key.unwrap_or_default(),
        openrouter_model: row
            .openrouter_model
            .unwrap_or_else(|| "nvidia/nemotron-3-ultra".to_string()),
        ai_cleanup_enabled: row.ai_cleanup_enabled != 0,
        auto_punctuation: row.auto_punctuation != 0,
        language: row.language.unwrap_or_else(|| "en".to_string()),
        theme: row.theme.unwrap_or_else(|| "dark".to_string()),
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
    settings: Settings,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.save_settings(
        &Some(settings.hotkey.clone()),
        &Some(settings.hotkey_mode.clone()),
        &Some(settings.whisper_model.clone()),
        &Some(settings.openrouter_key.clone()),
        &Some(settings.openrouter_model.clone()),
        settings.ai_cleanup_enabled as i32,
        settings.auto_punctuation as i32,
        &Some(settings.language.clone()),
        &Some(settings.theme.clone()),
        settings.start_at_login as i32,
    )
    .await
    .map_err(|e| e.to_string())
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

    let buffer_clone = state.audio_buffer.clone();
    let stop_flag_clone = state.stop_flag.clone();
    let handle = std::thread::spawn(move || {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();
        let device = match host.default_input_device() {
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

        let stream_config: cpal::StreamConfig = config.into();

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buf) = buffer_clone.try_lock() {
                        if channels > 1 {
                            for chunk in data.chunks(channels) {
                                let mono: f32 =
                                    chunk.iter().sum::<f32>() / channels as f32;
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

    let settings = {
        let db = state.db.lock().await;
        load_settings_from_db(&db).await?
    };

    let app_clone = app.clone();
    let status_arc = state.status.clone();

    tokio::spawn(async move {
        match transcribe_audio(&audio_buffer, &settings).await {
            Ok(text) => {
                let cleaned = if settings.ai_cleanup_enabled && !settings.openrouter_key.is_empty()
                {
                    match crate::llm::chat_completion(
                        &crate::llm::LLMConfig {
                            api_key: settings.openrouter_key.clone(),
                            model: settings.openrouter_model.clone(),
                            base_url: Some("https://openrouter.ai/api/v1".to_string()),
                        },
                        &[crate::llm::LLMMessage {
                            role: "user".to_string(),
                            content: format!(
                                "Clean up this dictated text. Fix punctuation, capitalization, remove filler words:\n\n{}",
                                text
                            ),
                        }],
                        "You are a text cleanup assistant. Fix punctuation, capitalization, and remove filler words from dictated text. Return only the cleaned text, nothing else.",
                    )
                    .await
                    {
                        Ok(cleaned) => cleaned,
                        Err(_) => text,
                    }
                } else {
                    text
                };

                if let Err(e) = TextInjector::new().inject_text(&cleaned) {
                    let mut status = status_arc.lock().await;
                    status.is_processing = false;
                    status.last_error = Some(format!("Text injection failed: {}", e));
                    emit_status(&app_clone, &status).await;
                    return;
                }

                let mut status = status_arc.lock().await;
                status.is_processing = false;
                status.current_text = cleaned.clone();
                emit_status(&app_clone, &status).await;
                app_clone.emit("transcript-received", cleaned).ok();
            }
            Err(e) => {
                let mut status = status_arc.lock().await;
                status.is_processing = false;
                status.last_error = Some(format!("Transcription failed: {}", e));
                emit_status(&app_clone, &status).await;
            }
        }
    });

    Ok(())
}

#[command]
pub async fn load_model(state: tauri::State<'_, AppState>, _model_id: String) -> Result<(), String> {
    let mut status = state.status.lock().await;
    status.is_model_loaded = true;
    status.last_error = None;
    emit_status(state.app_handle(), &status).await;
    Ok(())
}

#[command]
pub async fn download_model(_model_id: String) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn get_downloaded_models() -> Result<Vec<String>, String> {
    Ok(vec![
        "tiny.en".to_string(),
        "base.en".to_string(),
        "small.en".to_string(),
    ])
}

#[command]
pub async fn delete_model(_model_id: String) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn cleanup_with_ai(
    text: String,
    style: String,
    openrouter_key: String,
    openrouter_model: String,
) -> Result<String, String> {
    if openrouter_key.is_empty() {
        return Ok(text);
    }

    crate::llm::chat_completion(
        &crate::llm::LLMConfig {
            api_key: openrouter_key,
            model: openrouter_model,
            base_url: Some("https://openrouter.ai/api/v1".to_string()),
        },
        &[crate::llm::LLMMessage {
            role: "user".to_string(),
            content: format!(
                "Clean up this dictated text. Fix punctuation, capitalization, remove filler words, make it {}:\n\n{}",
                style, text
            ),
        }],
        "You are a text cleanup assistant. Fix punctuation, capitalization, and remove filler words from dictated text. Return only the cleaned text.",
    )
    .await
}

#[command]
pub async fn get_dictionary() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

#[command]
pub async fn add_dictionary_entry(_entry: serde_json::Value) -> Result<i64, String> {
    Ok(1)
}

#[command]
pub async fn update_dictionary_entry(_id: i64, _entry: serde_json::Value) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn delete_dictionary_entry(_id: i64) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn get_snippets() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

#[command]
pub async fn add_snippet(_snippet: serde_json::Value) -> Result<i64, String> {
    Ok(1)
}

#[command]
pub async fn update_snippet(_id: i64, _snippet: serde_json::Value) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn delete_snippet(_id: i64) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn get_app_profiles() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

#[command]
pub async fn add_app_profile(_profile: serde_json::Value) -> Result<String, String> {
    Ok("default".to_string())
}

#[command]
pub async fn update_app_profile(_id: String, _profile: serde_json::Value) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn delete_app_profile(_id: String) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn get_transcripts(_limit: i64, _offset: i64) -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

#[command]
pub async fn clear_transcripts() -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn set_start_at_login(_enabled: bool) -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn open_settings_folder() -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn check_microphone_permission() -> Result<bool, String> {
    Ok(true)
}

#[command]
pub async fn request_microphone_permission() -> Result<bool, String> {
    Ok(true)
}

#[command]
pub async fn check_accessibility_permission() -> Result<bool, String> {
    Ok(true)
}

#[command]
pub async fn request_accessibility_permission() -> Result<(), String> {
    Ok(())
}

#[command]
pub async fn list_conversations(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<crate::db::ConversationRow>, String> {
    let db = state.db.lock().await;
    db.list_conversations().await.map_err(|e| e.to_string())
}

#[command]
pub async fn delete_conversation(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_conversation(&conversation_id)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn load_conversation(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<crate::db::MessageRow>, String> {
    let db = state.db.lock().await;
    db.load_conversation_messages(&conversation_id)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_log_file_path() -> Result<String, String> {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("AuraScribe");
    Ok(data_dir
        .join("aurascribe.log")
        .to_string_lossy()
        .to_string())
}

async fn transcribe_audio(audio_buffer: &[f32], settings: &Settings) -> Result<String, String> {
    let api_key = &settings.openrouter_key;
    if api_key.is_empty() {
        return Err("OpenRouter API key not configured. Go to Settings to add one.".to_string());
    }

    let client = reqwest::Client::new();

    let wav_data = audio_buffer_to_wav(audio_buffer, 16000, 1)?;

    let part = reqwest::multipart::Part::bytes(wav_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "openai/whisper-large-v3")
        .text("language", settings.language.clone());

    let response = client
        .post("https://openrouter.ai/api/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    body["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Invalid transcription response: {}", body))
}

fn audio_buffer_to_wav(buffer: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>, String> {
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_size = buffer.len() * (bits_per_sample / 8) as usize;
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + data_size);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(file_size as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());

    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_size as u32).to_le_bytes());

    for &sample in buffer {
        let s16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        wav.extend_from_slice(&s16.to_le_bytes());
    }

    Ok(wav)
}
