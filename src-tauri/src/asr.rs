// src-tauri/src/asr.rs
//! Automatic Speech Recognition using Whisper.cpp via whisper-rs

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const MODELS_DIR: &str = "models";

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub id: String,
    /// Filename stem on HuggingFace, e.g. `large-v3-turbo-q5_0`.
    pub name: String,
    pub size_mb: u64,
    pub multilingual: bool,
    /// Rough speed rank, 1 = fastest. Used for ordering and UI hints.
    pub speed: u8,
    /// Rough accuracy rank, 5 = best.
    pub accuracy: u8,
    pub recommended: bool,
    pub downloaded: bool,
    pub path: Option<PathBuf>,
}

/// The curated model line-up.
///
/// Deliberately *not* the full whisper.cpp catalogue. The old "small" and "medium" tiers
/// are omitted: `large-v3-turbo` quantised to q5_0 is smaller, faster *and* more accurate
/// than `small.en`, so shipping the older tiers would only invite users to pick a strictly
/// worse option. Turbo is a distilled 4-layer decoder — it breaks the usual
/// accuracy-versus-speed tradeoff rather than sitting on it.
const MODELS: &[(&str, u64, bool, u8, u8, bool)] = &[
    // (id, size_mb, multilingual, speed(1=fastest), accuracy(5=best), recommended)
    ("tiny.en", 75, false, 1, 1, false),
    ("base.en", 142, false, 2, 2, false),
    ("large-v3-turbo-q5_0", 574, true, 2, 4, true),
    ("large-v3-turbo", 1620, true, 3, 5, false),
    ("large-v3", 3100, true, 5, 5, false),
];

pub struct WhisperASR {
    context: Arc<Mutex<Option<WhisperContext>>>,
    models_dir: PathBuf,
}

impl WhisperASR {
    pub fn new() -> Result<Self> {
        // Local (not roaming) app data: models run to gigabytes and must never be
        // synced by a roaming profile. Keeps them beside the database, too.
        let models_dir = dirs::data_local_dir()
            .context("Could not find local data directory")?
            .join("AuraScribe")
            .join(MODELS_DIR);

        std::fs::create_dir_all(&models_dir).context("Failed to create models directory")?;

        Ok(Self {
            context: Arc::new(Mutex::new(None)),
            models_dir,
        })
    }

    pub fn get_model_path(&self, model_id: &str) -> PathBuf {
        self.models_dir.join(format!("ggml-{}.bin", model_id))
    }

    pub async fn download_model(
        &self,
        model_id: &str,
        mut on_progress: impl FnMut(f32) + Send,
    ) -> Result<PathBuf> {
        if !MODELS.iter().any(|(id, ..)| *id == model_id) {
            anyhow::bail!("Unknown model: {}", model_id);
        }

        let url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
            model_id
        );
        let path = self.get_model_path(model_id);

        if path.exists() {
            on_progress(1.0);
            return Ok(path);
        }

        let tmp_path = path.with_extension("bin.part");
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to start download")?
            .error_for_status()
            .context("Model download rejected by server")?;
        let total_size = response.content_length().unwrap_or(0);

        let mut file = tokio::fs::File::create(&tmp_path)
            .await
            .context("Failed to create model file")?;
        let mut downloaded: u64 = 0;

        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Download error")?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                .await
                .context("Write error")?;
            downloaded += chunk.len() as u64;

            let progress = if total_size > 0 {
                downloaded as f32 / total_size as f32
            } else {
                0.0
            };
            on_progress(progress);
        }

        tokio::io::AsyncWriteExt::flush(&mut file)
            .await
            .context("Failed to flush file")?;
        drop(file);
        tokio::fs::rename(&tmp_path, &path)
            .await
            .context("Failed to finalize model file")?;
        on_progress(1.0);
        Ok(path)
    }

    pub fn load_model(&self, model_id: &str) -> Result<()> {
        let path = self.get_model_path(model_id);

        if !path.exists() {
            anyhow::bail!("Model not found: {}. Download it first.", model_id);
        }

        let mut params = WhisperContextParameters::default();
        // Harmless when no GPU backend is compiled in — whisper.cpp falls back to CPU.
        params.use_gpu(true);

        let ctx = WhisperContext::new_with_params(&path, params)
            .context("Failed to load Whisper model")?;

        *self.context.lock().unwrap() = Some(ctx);

        Ok(())
    }

    pub fn transcribe(&self, audio: &[f32], language: Option<&str>) -> Result<String> {
        let ctx_guard = self.context.lock().unwrap();
        let ctx = ctx_guard.as_ref().context("Model not loaded")?;

        let mut state = ctx.create_state().context("Failed to create whisper state")?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_realtime(false);
        params.set_print_progress(false);
        params.set_print_timestamps(false);
        params.set_print_special(false);
        params.set_translate(false);
        params.set_no_context(true);

        // whisper.cpp defaults to 4 threads regardless of the machine; using the cores we
        // actually have is a large, free speedup on typical multi-core laptops.
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(16);
        params.set_n_threads(threads as i32);

        if let Some(lang) = language {
            params.set_language(Some(lang));
        }

        state.full(params, audio).context("Transcription failed")?;

        let num_segments = state.full_n_segments();
        let mut text = String::new();

        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                if let Ok(s) = segment.to_str_lossy() {
                    text.push_str(&s);
                }
            }
        }

        Ok(text.trim().to_string())
    }

    pub fn list_available_models(&self) -> Vec<ModelInfo> {
        MODELS
            .iter()
            .map(|(id, size, multilingual, speed, accuracy, recommended)| {
                let path = self.get_model_path(id);
                let downloaded = path.exists();
                ModelInfo {
                    id: id.to_string(),
                    name: id.to_string(),
                    size_mb: *size,
                    multilingual: *multilingual,
                    speed: *speed,
                    accuracy: *accuracy,
                    recommended: *recommended,
                    downloaded,
                    path: downloaded.then_some(path),
                }
            })
            .collect()
    }
}

impl Default for WhisperASR {
    fn default() -> Self {
        Self::new().expect("Failed to create ASR")
    }
}
