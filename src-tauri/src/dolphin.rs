// src-tauri/src/dolphin.rs
//! Dolphin ASR — a fast **multilingual CTC** engine (DataoceanAI/Tsinghua Dolphin via sherpa-onnx).
//!
//! This is the pragmatic answer to "fast local **Indian-language** dictation". Dolphin is a
//! multilingual CTC model covering ~40 Eastern languages — including **Hindi, Tamil, Telugu,
//! Bengali, Urdu, Marathi, Gujarati, Punjabi, Odia** and more — with **automatic language
//! detection**. It runs on the sherpa-onnx engine we already ship, as a single `model.int8.onnx`
//! + `tokens.txt` (~105 MB), so it stays light and downloads on demand like the other models.
//!
//! **Honest scope:** Dolphin does **not** cover **Malayalam or Kannada** (see DataoceanAI's
//! `languages.md`). Those two remain the gap that only AI4Bharat's IndicConformer fills — which is
//! still blocked on a NeMo→sherpa-onnx export (see `docs/INDIC-CONFORMER.md`,
//! `docs/CONTRIB-indicconformer-sherpa-onnx.md`). So Dolphin gives most Indian languages *now*;
//! Malayalam is tracked separately.
//!
//! Gated behind the `moonshine` Cargo feature (which is really "the sherpa-onnx engine set").

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Mutex;

use sherpa_rs::dolphin::{DolphinConfig, DolphinRecognizer};

use crate::asr::{EngineKind, ModelInfo};

/// Dolphin is a CTC model: one ONNX graph + a tokens file. sherpa-onnx publishes them flat.
const MODEL_FILES: [(&str, u64); 2] = [
    ("model.int8.onnx", 104_000_000),
    ("tokens.txt", 505_000),
];

/// The Dolphin line-up. `base` (int8) is small and fast; a larger `small` tier could be added
/// later for more accuracy. `cpu_cost` is a conservative estimate (CTC is cheap; the base model
/// is ~105 MB) pending a real benchmark. `accuracy` is mid — good enough for dictation, and the
/// only *fast local* option for these languages today.
const DOLPHIN_MODELS: &[(&str, &str, u64, u8, u8, f32)] = &[
    // (id, hf_repo, size_mb, speed(1=fastest), accuracy(5=best), cpu_cost estimate)
    (
        "dolphin-base-multilang",
        "csukuangfj/sherpa-onnx-dolphin-base-ctc-multi-lang-int8-2025-04-02",
        105,
        2,
        3,
        0.3,
    ),
];

fn hf_base_url(hf_repo: &str) -> String {
    format!("https://huggingface.co/{hf_repo}/resolve/main")
}

fn dolphin_row(model_id: &str) -> Option<&'static (&'static str, &'static str, u64, u8, u8, f32)> {
    DOLPHIN_MODELS.iter().find(|(id, ..)| *id == model_id)
}

/// Whether `model_id` names a Dolphin model.
pub fn is_dolphin_model(model_id: &str) -> bool {
    dolphin_row(model_id).is_some()
}

/// A Dolphin engine. The native handle holds state and `transcribe` takes `&mut self`, so it
/// lives behind a `Mutex` and is `Send + Sync` for the Tauri state.
pub struct DolphinASR {
    recognizer: Mutex<Option<DolphinRecognizer>>,
    models_dir: PathBuf,
}

impl DolphinASR {
    pub fn new(models_dir: PathBuf) -> Self {
        Self {
            recognizer: Mutex::new(None),
            models_dir,
        }
    }

    pub fn model_dir(&self, model_id: &str) -> PathBuf {
        self.models_dir.join(model_id)
    }

    /// Whether both files the recognizer needs are present.
    pub fn is_present(&self, model_id: &str) -> bool {
        let dir = self.model_dir(model_id);
        MODEL_FILES.iter().all(|(f, _)| dir.join(f).exists())
    }

    pub fn list_available_models(&self) -> Vec<ModelInfo> {
        DOLPHIN_MODELS
            .iter()
            .map(|(id, _repo, size_mb, speed, accuracy, cpu_cost)| {
                let downloaded = self.is_present(id);
                ModelInfo {
                    id: id.to_string(),
                    name: id.to_string(),
                    engine: EngineKind::Dolphin,
                    multilingual: true, // ~40 languages incl. Hindi/Tamil/Telugu, auto-detected
                    size_mb: *size_mb,
                    speed: *speed,
                    accuracy: *accuracy,
                    recommended: false, // decided by the facade across all engines
                    downloaded,
                    path: downloaded.then(|| self.model_dir(id)),
                    realtime_factor: *cpu_cost,
                    warning: None,
                }
            })
            .collect()
    }

    /// Download a Dolphin model's files, byte-weighted progress. Files land in a `.part` sibling
    /// first and are only promoted once complete.
    pub async fn download_model(
        &self,
        model_id: &str,
        mut on_progress: impl FnMut(f32) + Send,
    ) -> Result<PathBuf> {
        let (_, repo, ..) =
            dolphin_row(model_id).with_context(|| format!("Unknown Dolphin model: {model_id}"))?;

        let dir = self.model_dir(model_id);
        if self.is_present(model_id) {
            on_progress(1.0);
            return Ok(dir);
        }
        tokio::fs::create_dir_all(&dir)
            .await
            .context("Failed to create Dolphin model directory")?;

        let base = hf_base_url(repo);
        let client = reqwest::Client::new();
        let total_bytes: u64 = MODEL_FILES.iter().map(|(_, b)| *b).sum();
        let mut done_bytes: u64 = 0;

        use futures_util::StreamExt;
        for (file, approx) in MODEL_FILES.iter() {
            let dest = dir.join(file);
            if dest.exists() {
                done_bytes += *approx;
                on_progress((done_bytes as f32 / total_bytes as f32).min(1.0));
                continue;
            }

            let url = format!("{base}/{file}");
            let response = client
                .get(&url)
                .send()
                .await
                .with_context(|| format!("Failed to start download of {file}"))?
                .error_for_status()
                .with_context(|| format!("Download of {file} rejected by server"))?;

            let tmp = dest.with_extension("part");
            let mut out = tokio::fs::File::create(&tmp)
                .await
                .with_context(|| format!("Failed to create {file}"))?;
            let mut got: u64 = 0;
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.with_context(|| format!("Download error on {file}"))?;
                tokio::io::AsyncWriteExt::write_all(&mut out, &chunk)
                    .await
                    .with_context(|| format!("Write error on {file}"))?;
                got += chunk.len() as u64;
                let within = (got.min(*approx)) as f32;
                on_progress(((done_bytes as f32 + within) / total_bytes as f32).min(1.0));
            }
            tokio::io::AsyncWriteExt::flush(&mut out)
                .await
                .context("Failed to flush file")?;
            drop(out);
            tokio::fs::rename(&tmp, &dest)
                .await
                .with_context(|| format!("Failed to finalize {file}"))?;

            done_bytes += *approx;
            on_progress((done_bytes as f32 / total_bytes as f32).min(1.0));
        }

        on_progress(1.0);
        Ok(dir)
    }

    /// Load a downloaded Dolphin model into memory.
    pub fn load_model(&self, model_id: &str, num_threads: usize) -> Result<()> {
        let dir = self.model_dir(model_id);
        if !self.is_present(model_id) {
            anyhow::bail!(
                "Dolphin model '{model_id}' is not fully downloaded (looked in {}).",
                dir.display()
            );
        }

        let config = DolphinConfig {
            model: dir.join("model.int8.onnx").to_string_lossy().into_owned(),
            tokens: dir.join("tokens.txt").to_string_lossy().into_owned(),
            decoding_method: "greedy_search".to_string(),
            provider: Some("cpu".to_string()),
            num_threads: Some(num_threads.max(1) as i32),
            debug: false,
        };

        tracing::info!(model = %model_id, threads = num_threads, "Loading Dolphin model (sherpa-onnx)");
        let recognizer = DolphinRecognizer::new(config)
            .map_err(|e| anyhow::anyhow!("Failed to load Dolphin model '{model_id}': {e}"))?;

        *self.recognizer.lock().unwrap() = Some(recognizer);
        tracing::info!(model = %model_id, "Dolphin model loaded");
        Ok(())
    }

    /// Transcribe 16 kHz mono `f32` samples. Dolphin auto-detects the language, so `language` is
    /// accepted for a uniform surface but ignored.
    pub fn transcribe(&self, audio: &[f32], _language: Option<&str>) -> Result<String> {
        let mut guard = self.recognizer.lock().unwrap();
        let recognizer = guard.as_mut().context("Dolphin model not loaded")?;
        let started = std::time::Instant::now();
        let result = recognizer.transcribe(16_000, audio);
        let text = result.text.trim().to_string();
        tracing::info!(
            samples = audio.len(),
            lang = %result.lang,
            out_chars = text.len(),
            took_ms = started.elapsed().as_millis() as u64,
            "Dolphin transcribed a chunk"
        );
        Ok(text)
    }
}
