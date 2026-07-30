// src-tauri/src/models.rs
//! Model management utilities

use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size_mb: u64,
    pub downloaded: bool,
    pub path: Option<std::path::PathBuf>,
}

/// Whisper model details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperModel {
    pub id: String,
    pub name: String,
    pub language: &'static str,
    pub size: &'static str,
    pub speed: &'static str,
    pub quality: &'static str,
    pub recommended: bool,
}

impl WhisperModel {
    pub const fn new(id: &str, name: &str, language: &'static str, size: &'static str, speed: &'static str, quality: &'static str, recommended: bool) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            language,
            size,
            speed,
            quality,
            recommended,
        }
    }
}

/// Available Whisper models (lightweight versions for production use)
pub const WHISPER_MODELS: &[WhisperModel] = &[
    WhisperModel::new("tiny.en", "Tiny English (Fastest)", "en", "39 MB", "Fastest", "Good", true),
    WhisperModel::new("tiny", "Tiny Multilingual (Fastest)", "multilingual", "41 MB", "Fastest", "Good", false),
    WhisperModel::new("base.en", "Base English (Recommended)", "en", "74 MB", "Fast", "Better", true),
    WhisperModel::new("base", "Base Multilingual (Recommended)", "multilingual", "76 MB", "Fast", "Better", true),
    WhisperModel::new("small.en", "Small English (Balanced)", "en", "244 MB", "Balanced", "Great", false),
    WhisperModel::new("small", "Small Multilingual (Balanced)", "multilingual", "247 MB", "Balanced", "Great", false),
    WhisperModel::new("medium", "Medium Multilingual (Slow)", "multilingual", "769 MB", "Slow", "Excellent", false),
];

/// Get model info by ID
pub fn get_model_info(id: &str) -> Option<&WhisperModel> {
    WHISPER_MODELS.iter().find(|m| m.id == id)
}

/// Validate model ID
pub fn validate_model_id(id: &str) -> bool {
    get_model_info(id).is_some()
}

/// Model states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelState {
    Ready,
    Loading,
    Downloading,
    Error(String),
}