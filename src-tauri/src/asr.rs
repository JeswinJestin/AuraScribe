// src-tauri/src/asr.rs
//! Automatic Speech Recognition using Whisper.cpp via whisper-rs

use anyhow::{Context, Result};
use std::path::PathBuf;

const MODELS_DIR: &str = "models";

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size_mb: u64,
    pub downloaded: bool,
    pub path: Option<PathBuf>,
}

#[cfg(feature = "whisper")]
mod whisper_impl {
    use super::*;
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    pub struct WhisperASR {
        context: Arc<Mutex<Option<WhisperContext>>>,
        current_model: Arc<Mutex<Option<String>>>,
        models_dir: PathBuf,
    }

    impl WhisperASR {
        pub fn new() -> Result<Self> {
            let models_dir = dirs::data_dir()
                .context("Could not find data directory")?
                .join("AuraScribe")
                .join(MODELS_DIR);

            std::fs::create_dir_all(&models_dir).context("Failed to create models directory")?;

            Ok(Self {
                context: Arc::new(Mutex::new(None)),
                current_model: Arc::new(Mutex::new(None)),
                models_dir,
            })
        }

        pub fn get_model_path(&self, model_id: &str) -> PathBuf {
            self.models_dir.join(format!("ggml-{}.bin", model_id))
        }

        pub async fn download_model(&self, model_id: &str) -> Result<PathBuf> {
            let url = format!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
                model_id
            );
            let path = self.get_model_path(model_id);

            if path.exists() {
                return Ok(path);
            }

            let client = reqwest::Client::new();
            let response = client.get(&url).send().await.context("Failed to start download")?;
            let total_size = response.content_length().unwrap_or(0);

            let mut file = tokio::fs::File::create(&path)
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
                tracing::debug!(
                    model = model_id,
                    progress,
                    downloaded,
                    total = total_size,
                    "Model download progress"
                );
            }

            tokio::io::AsyncWriteExt::flush(&mut file)
                .await
                .context("Failed to flush file")?;
            Ok(path)
        }

        pub fn load_model(&self, model_id: &str) -> Result<()> {
            let path = self.get_model_path(model_id);

            if !path.exists() {
                anyhow::bail!("Model not found: {}. Download it first.", model_id);
            }

            let params = WhisperContextParameters::default();
            let ctx = WhisperContext::new_with_params(&path, params)
                .context("Failed to load Whisper model")?;

            *self.context.lock().unwrap() = Some(ctx);
            *self.current_model.lock().unwrap() = Some(model_id.to_string());

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
            params.set_single_segment(false);

            if let Some(lang) = language {
                params.set_language(Some(lang));
            }

            state.full(params, audio).context("Transcription failed")?;

            let num_segments = state.full_n_segments().context("Failed to get segments")?;
            let mut text = String::new();

            for i in 0..num_segments {
                if let Ok(segment) = state.full_get_segment_text(i) {
                    text.push_str(&segment);
                }
            }

            Ok(text.trim().to_string())
        }

        pub fn transcribe_streaming(
            &self,
            audio: &[f32],
            language: Option<&str>,
        ) -> Result<Vec<String>> {
            let ctx_guard = self.context.lock().unwrap();
            let ctx = ctx_guard.as_ref().context("Model not loaded")?;

            let mut state = ctx.create_state().context("Failed to create whisper state")?;

            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_print_realtime(false);
            params.set_print_progress(false);
            params.set_print_timestamps(true);
            params.set_print_special(false);
            params.set_translate(false);
            params.set_no_context(false);
            params.set_single_segment(true);

            if let Some(lang) = language {
                params.set_language(Some(lang));
            }

            state.full(params, audio).context("Streaming transcription failed")?;

            let num_segments = state.full_n_segments().context("Failed to get segments")?;
            let mut segments = Vec::new();

            for i in 0..num_segments {
                if let Ok(segment) = state.full_get_segment_text(i) {
                    if !segment.trim().is_empty() {
                        segments.push(segment.trim().to_string());
                    }
                }
            }

            Ok(segments)
        }

        pub fn list_available_models(&self) -> Vec<ModelInfo> {
            vec![
                ("tiny.en", "Tiny English", 39),
                ("base.en", "Base English", 74),
                ("small.en", "Small English", 244),
                ("medium", "Medium Multilingual", 769),
                ("large-v3", "Large v3 Multilingual", 1550),
            ]
            .into_iter()
            .map(|(id, name, size)| ModelInfo {
                id: id.to_string(),
                name: name.to_string(),
                size_mb: size,
                downloaded: self.get_model_path(id).exists(),
                path: if self.get_model_path(id).exists() {
                    Some(self.get_model_path(id))
                } else {
                    None
                },
            })
            .collect()
        }

        pub fn get_current_model(&self) -> Option<String> {
            self.current_model.lock().unwrap().clone()
        }

        pub fn unload_model(&self) {
            *self.context.lock().unwrap() = None;
            *self.current_model.lock().unwrap() = None;
        }
    }

    impl Default for WhisperASR {
        fn default() -> Self {
            Self::new().expect("Failed to create ASR")
        }
    }
}

#[cfg(feature = "whisper")]
pub use whisper_impl::WhisperASR;

#[cfg(not(feature = "whisper"))]
pub struct WhisperASR {
    models_dir: PathBuf,
}

#[cfg(not(feature = "whisper"))]
impl WhisperASR {
    pub fn new() -> Result<Self> {
        let models_dir = dirs::data_dir()
            .context("Could not find data directory")?
            .join("AuraScribe")
            .join(MODELS_DIR);

        std::fs::create_dir_all(&models_dir).context("Failed to create models directory")?;

        Ok(Self { models_dir })
    }

    pub fn get_model_path(&self, model_id: &str) -> PathBuf {
        self.models_dir.join(format!("ggml-{}.bin", model_id))
    }

    pub async fn download_model(&self, model_id: &str) -> Result<PathBuf> {
        let url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
            model_id
        );
        let path = self.get_model_path(model_id);

        if path.exists() {
            return Ok(path);
        }

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await.context("Failed to start download")?;
        let total_size = response.content_length().unwrap_or(0);

        let mut file = tokio::fs::File::create(&path)
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
            tracing::debug!(model = model_id, downloaded, total = total_size, "Download progress");
        }

        tokio::io::AsyncWriteExt::flush(&mut file)
            .await
            .context("Failed to flush file")?;
        Ok(path)
    }

    pub fn load_model(&self, _model_id: &str) -> Result<()> {
        anyhow::bail!("Whisper support not compiled. Enable the 'whisper' feature to use local ASR.")
    }

    pub fn transcribe(&self, _audio: &[f32], _language: Option<&str>) -> Result<String> {
        anyhow::bail!("Whisper support not compiled. Enable the 'whisper' feature to use local ASR.")
    }

    pub fn transcribe_streaming(&self, _audio: &[f32], _language: Option<&str>) -> Result<Vec<String>> {
        anyhow::bail!("Whisper support not compiled. Enable the 'whisper' feature to use local ASR.")
    }

    pub fn list_available_models(&self) -> Vec<ModelInfo> {
        vec![
            ("tiny.en", "Tiny English", 39),
            ("base.en", "Base English", 74),
            ("small.en", "Small English", 244),
            ("medium", "Medium Multilingual", 769),
            ("large-v3", "Large v3 Multilingual", 1550),
        ]
        .into_iter()
        .map(|(id, name, size)| ModelInfo {
            id: id.to_string(),
            name: name.to_string(),
            size_mb: size,
            downloaded: self.get_model_path(id).exists(),
            path: if self.get_model_path(id).exists() {
                Some(self.get_model_path(id))
            } else {
                None
            },
        })
        .collect()
    }

    pub fn get_current_model(&self) -> Option<String> {
        None
    }

    pub fn unload_model(&self) {}
}

#[cfg(not(feature = "whisper"))]
impl Default for WhisperASR {
    fn default() -> Self {
        Self::new().expect("Failed to create ASR")
    }
}
