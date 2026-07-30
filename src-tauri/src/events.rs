// src-tauri/src/events.rs
//! Event management for AuraScribe

use tauri::{AppHandle, Emitter};
use serde::{Serialize, Deserialize};

/// Supported events that can be emitted from Rust backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuraEvent {
    /// Model download progress
    ModelDownloadProgress {
        model_id: String,
        progress: f32,
        downloaded: u64,
        total: u64,
    },
    /// Transcription completed
    TranscriptionCompleted {
        text: String,
        raw_text: String,
    },
    /// Error occurred
    ErrorOccurred {
        message: String,
        detail: Option<String>,
    },
    /// Whisper model loaded
    ModelLoaded {
        model_id: String,
        model_name: String,
    },
    /// Whisper model unloaded
    ModelUnloaded,
    /// Recording status changed
    RecordingChanged {
        is_recording: bool,
    },
    /// Settings changed
    SettingsChanged {
        setting: String,
        value: String,
    },
    /// Text injection into application
    TextInjected {
        success: bool,
        text: String,
    },
}

impl AuraEvent {
    /// Emit event to frontend
    pub fn emit(&self, app_handle: &AppHandle) -> anyhow::Result<()> {
        let (event_name, payload) = match self {
            AuraEvent::ModelDownloadProgress { model_id, progress, downloaded, total } => {
                let payload = serde_json::json!({
                    "model": model_id,
                    "progress": progress,
                    "downloaded": downloaded,
                    "total": total,
                });
                ("model-download-progress", payload)
            },
            AuraEvent::TranscriptionCompleted { text, raw_text } => {
                let payload = serde_json::json!({
                    "text": text,
                    "raw_text": raw_text,
                });
                ("transcription-completed", payload)
            },
            AuraEvent::ErrorOccurred { message, detail } => {
                let payload = serde_json::json!({
                    "message": message,
                    "detail": detail,
                });
                ("error-occurred", payload)
            },
            AuraEvent::ModelLoaded { model_id, model_name } => {
                let payload = serde_json::json!({
                    "model_id": model_id,
                    "model_name": model_name,
                });
                ("model-loaded", payload)
            },
            AuraEvent::ModelUnloaded => {
                ("model-unloaded", serde_json::json!({}))
            },
            AuraEvent::RecordingChanged { is_recording } => {
                let payload = serde_json::json!({
                    "is_recording": is_recording,
                });
                ("recording-changed", payload)
            },
            AuraEvent::SettingsChanged { setting, value } => {
                let payload = serde_json::json!({
                    "setting": setting,
                    "value": value,
                });
                ("settings-changed", payload)
            },
            AuraEvent::TextInjected { success, text } => {
                let payload = serde_json::json!({
                    "success": success,
                    "text": text,
                });
                ("text-injected", payload)
            },
        };

        app_handle.emit(event_name, payload)
            .context("Failed to emit event")
    }
}