// src-tauri/src/engine.rs
//! The engine seam: one `Asr` facade over the two speech engines.
//!
//! Whisper (`asr::WhisperASR`) is the v1 accuracy engine and is always present. Moonshine
//! (`moonshine::MoonshineASR`) is the newer, faster engine, compiled in only under the
//! `moonshine` feature. Everything the app calls — list, download, load, transcribe, delete —
//! goes through this facade, which routes each call to the right engine by the model's
//! `EngineKind`. The rest of the app never needs to know which engine is active.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::asr::{EngineKind, ModelInfo, WhisperASR};

pub struct Asr {
    whisper: WhisperASR,
    #[cfg(feature = "moonshine")]
    moonshine: crate::moonshine::MoonshineASR,
    /// The engine of the model currently loaded, so `transcribe` (which is not given a model
    /// id) routes to the same engine that was loaded.
    loaded_engine: Mutex<Option<EngineKind>>,
}

impl Asr {
    pub fn new() -> Result<Self> {
        let whisper = WhisperASR::new()?;
        #[cfg(feature = "moonshine")]
        let moonshine = crate::moonshine::MoonshineASR::new(whisper.models_dir().to_path_buf());
        Ok(Self {
            #[cfg(feature = "moonshine")]
            moonshine,
            whisper,
            loaded_engine: Mutex::new(None),
        })
    }

    /// Which engine runs a given model id. Whisper is the default for anything not recognised
    /// as another engine's model, so an unknown id never silently routes to Moonshine.
    fn engine_of(model_id: &str) -> EngineKind {
        #[cfg(feature = "moonshine")]
        if crate::moonshine::is_moonshine_model(model_id) {
            return EngineKind::Moonshine;
        }
        let _ = model_id;
        EngineKind::Whisper
    }

    /// Every model from every compiled-in engine, as one catalogue for the UI.
    pub fn list_available_models(&self) -> Vec<ModelInfo> {
        // `mut` is only used when the moonshine extend below is compiled in.
        #[cfg_attr(not(feature = "moonshine"), allow(unused_mut))]
        let mut models = self.whisper.list_available_models();
        #[cfg(feature = "moonshine")]
        models.extend(self.moonshine.list_available_models());
        models
    }

    /// Whether a model is fully present and usable (a whole Moonshine bundle, not a partial one).
    pub fn is_downloaded(&self, model_id: &str) -> bool {
        match Self::engine_of(model_id) {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => self.moonshine.is_present(model_id),
            _ => self.whisper.get_model_path(model_id).exists(),
        }
    }

    /// The on-disk location of a model — a file for Whisper, a directory for Moonshine.
    pub fn get_model_path(&self, model_id: &str) -> PathBuf {
        match Self::engine_of(model_id) {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => self.moonshine.model_dir(model_id),
            _ => self.whisper.get_model_path(model_id),
        }
    }

    /// Whether live chunking helps this model. Moonshine is fast enough that it always does;
    /// for Whisper it depends on whether the model keeps pace with speech on this machine.
    pub fn should_chunk(&self, model_id: &str) -> bool {
        match Self::engine_of(model_id) {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => true,
            _ => crate::asr::should_chunk(model_id),
        }
    }

    pub fn load_model(&self, model_id: &str) -> Result<()> {
        let engine = Self::engine_of(model_id);
        match engine {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => self
                .moonshine
                .load_model(model_id, crate::asr::worker_threads())?,
            _ => self.whisper.load_model(model_id)?,
        }
        *self.loaded_engine.lock().unwrap() = Some(engine);
        Ok(())
    }

    pub async fn download_model(
        &self,
        model_id: &str,
        on_progress: impl FnMut(f32) + Send,
    ) -> Result<PathBuf> {
        match Self::engine_of(model_id) {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => self.moonshine.download_model(model_id, on_progress).await,
            _ => self.whisper.download_model(model_id, on_progress).await,
        }
    }

    pub fn transcribe(&self, audio: &[f32], language: Option<&str>) -> Result<String> {
        let engine = (*self.loaded_engine.lock().unwrap()).unwrap_or(EngineKind::Whisper);
        match engine {
            #[cfg(feature = "moonshine")]
            EngineKind::Moonshine => self.moonshine.transcribe(audio, language),
            _ => self.whisper.transcribe(audio, language),
        }
    }

    /// Remove a downloaded model — a single file for Whisper, the whole bundle for Moonshine.
    pub fn delete_model(&self, model_id: &str) -> std::io::Result<()> {
        let path = self.get_model_path(model_id);
        if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else if path.exists() {
            std::fs::remove_file(path)
        } else {
            Ok(())
        }
    }
}
