// src-tauri/src/moonshine.rs
//! Moonshine ASR — the newer, faster, lower-latency engine (ONNX via sherpa-onnx).
//!
//! Whisper (see `asr.rs`) stays the v1 accuracy engine. Moonshine is added on top as the
//! advanced option: ~5x faster than Whisper on a CPU, WER better than `tiny.en`/`base.en`,
//! and — unlike Whisper's fixed 30-second window — its compute scales with clip length, so
//! it *compounds* with the chunking and silence-trimming already in the pipeline.
//!
//! This whole module is gated behind the `moonshine` Cargo feature so the shippable Whisper
//! build is untouched until the download + transcribe path is proven on a real machine.
//!
//! A Moonshine model is not a single file but a small bundle of ONNX graphs plus a token
//! table. The download + extraction of that bundle lands in a later commit; this module owns
//! the config, load, and transcribe surface so the engine seam can be wired next.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Mutex;

use sherpa_rs::moonshine::{MoonshineConfig, MoonshineRecognizer};

use crate::asr::{EngineKind, ModelInfo};

/// The ONNX graphs + token table that make up a Moonshine model on disk. sherpa-onnx names
/// them exactly this inside its published archives, so we keep the same names on extraction.
const MODEL_FILES: [&str; 5] = [
    "preprocess.onnx",
    "encode.int8.onnx",
    "uncached_decode.int8.onnx",
    "cached_decode.int8.onnx",
    "tokens.txt",
];

/// The Moonshine line-up. Distributed as int8 ONNX so it stays CPU-friendly.
///
/// `cpu_cost` (processing time as a multiple of speech length) is a **conservative estimate**
/// until the benchmark in a later commit measures it on real hardware — Moonshine is
/// documented at ~5x faster than Whisper, which on this project's `base.en` (~0.5x) puts base
/// near ~0.15x. `accuracy` reflects Moonshine's published WER, which beats Whisper `base.en`.
///
/// Sizes are the sum of the five int8 files (English-only models today).
const MOONSHINE_MODELS: &[(&str, &str, u64, u8, u8, f32)] = &[
    // (id, hf_variant, size_mb, speed(1=fastest), accuracy(5=best), cpu_cost estimate)
    // `moonshine-tiny-en` ("AuraScribe English Mini") is listed first — the genuinely-light option
    // (110 MB) that tops the model list. `moonshine-base-en` ("AuraScribe English") is the
    // recommended English default (286 MB, accuracy 4) — the badge comes from `engine.rs`, and
    // this ordering matches the Settings list the owner designed.
    ("moonshine-tiny-en", "tiny", 110, 1, 3, 0.10),
    ("moonshine-base-en", "base", 286, 1, 4, 0.15),
];

/// HuggingFace repo that hosts a model's five files flat (no archive to unpack).
fn hf_base_url(hf_variant: &str) -> String {
    format!(
        "https://huggingface.co/csukuangfj/sherpa-onnx-moonshine-{hf_variant}-en-int8/resolve/main"
    )
}

/// Look up a Moonshine model's catalog row by id.
fn moonshine_row(model_id: &str) -> Option<&'static (&'static str, &'static str, u64, u8, u8, f32)> {
    MOONSHINE_MODELS.iter().find(|(id, ..)| *id == model_id)
}

/// Whether `model_id` names a Moonshine model at all.
#[allow(dead_code)] // used by the engine facade in a following commit
pub fn is_moonshine_model(model_id: &str) -> bool {
    moonshine_row(model_id).is_some()
}

/// A Moonshine engine. The native recognizer holds decoder state and its `transcribe` takes
/// `&mut self`, so — exactly like `WhisperASR`'s context — it lives behind a `Mutex` and the
/// whole engine is `Send + Sync` for the Tauri state.
#[allow(dead_code)] // wired into the engine seam in a following commit
pub struct MoonshineASR {
    recognizer: Mutex<Option<MoonshineRecognizer>>,
    /// Root that holds one sub-directory per Moonshine model id.
    models_dir: PathBuf,
}

#[allow(dead_code)] // wired into the engine seam in a following commit
impl MoonshineASR {
    pub fn new(models_dir: PathBuf) -> Self {
        Self {
            recognizer: Mutex::new(None),
            models_dir,
        }
    }

    /// Where a given model's bundle lives, e.g. `<models>/moonshine-base-en/`.
    pub fn model_dir(&self, model_id: &str) -> PathBuf {
        self.models_dir.join(model_id)
    }

    /// Whether every file the recognizer needs is present for this model.
    pub fn is_present(&self, model_id: &str) -> bool {
        let dir = self.model_dir(model_id);
        MODEL_FILES.iter().all(|f| dir.join(f).exists())
    }

    /// The Moonshine models, described for the shared model list the UI reads. The engine
    /// facade concatenates these with Whisper's so both appear as one catalogue.
    pub fn list_available_models(&self) -> Vec<ModelInfo> {
        MOONSHINE_MODELS
            .iter()
            .map(|(id, _variant, size_mb, speed, accuracy, cpu_cost)| {
                let downloaded = self.is_present(id);
                ModelInfo {
                    id: id.to_string(),
                    name: id.to_string(),
                    engine: EngineKind::Moonshine,
                    size_mb: *size_mb,
                    multilingual: false, // English-only models today; Parakeet covers the rest
                    speed: *speed,
                    accuracy: *accuracy,
                    // Recommendation is decided against Whisper by the facade once the
                    // benchmark has measured Moonshine's real speed — not here.
                    recommended: false,
                    downloaded,
                    path: downloaded.then(|| self.model_dir(id)),
                    realtime_factor: *cpu_cost,
                    warning: None,
                }
            })
            .collect()
    }

    /// Download a Moonshine model's five files into its directory, reporting 0.0..=1.0
    /// progress across the whole set. Files land in a `.part` sibling first and are only
    /// promoted once complete, so an interrupted download never looks "present".
    pub async fn download_model(
        &self,
        model_id: &str,
        mut on_progress: impl FnMut(f32) + Send,
    ) -> Result<PathBuf> {
        let (_, variant, ..) = moonshine_row(model_id)
            .with_context(|| format!("Unknown Moonshine model: {model_id}"))?;

        let dir = self.model_dir(model_id);
        if self.is_present(model_id) {
            on_progress(1.0);
            return Ok(dir);
        }
        tokio::fs::create_dir_all(&dir)
            .await
            .context("Failed to create Moonshine model directory")?;

        let base = hf_base_url(variant);
        let client = reqwest::Client::new();
        let total = MODEL_FILES.len() as f32;

        use futures_util::StreamExt;
        for (i, file) in MODEL_FILES.iter().enumerate() {
            let dest = dir.join(file);
            if dest.exists() {
                on_progress((i as f32 + 1.0) / total);
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
            let file_total = response.content_length().unwrap_or(0);

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

                let frac_in_file = if file_total > 0 {
                    got as f32 / file_total as f32
                } else {
                    0.0
                };
                on_progress((i as f32 + frac_in_file) / total);
            }
            tokio::io::AsyncWriteExt::flush(&mut out)
                .await
                .context("Failed to flush file")?;
            drop(out);
            tokio::fs::rename(&tmp, &dest)
                .await
                .with_context(|| format!("Failed to finalize {file}"))?;
            on_progress((i as f32 + 1.0) / total);
        }

        on_progress(1.0);
        Ok(dir)
    }

    /// Load a downloaded Moonshine model into memory. `num_threads` should be physical cores
    /// (see `asr::worker_threads`) — Moonshine is SIMD-bound the same way Whisper is.
    pub fn load_model(&self, model_id: &str, num_threads: usize) -> Result<()> {
        let dir = self.model_dir(model_id);
        if !self.is_present(model_id) {
            anyhow::bail!(
                "Moonshine model '{model_id}' is not fully downloaded (looked in {}).",
                dir.display()
            );
        }

        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let config = MoonshineConfig {
            preprocessor: file("preprocess.onnx"),
            encoder: file("encode.int8.onnx"),
            uncached_decoder: file("uncached_decode.int8.onnx"),
            cached_decoder: file("cached_decode.int8.onnx"),
            tokens: file("tokens.txt"),
            provider: Some("cpu".to_string()),
            num_threads: Some(num_threads.max(1) as i32),
            debug: false,
            ..Default::default()
        };

        // sherpa-rs returns an eyre::Result; bridge it into anyhow with the message preserved.
        let recognizer = MoonshineRecognizer::new(config)
            .map_err(|e| anyhow::anyhow!("Failed to load Moonshine model '{model_id}': {e}"))?;

        *self.recognizer.lock().unwrap() = Some(recognizer);
        Ok(())
    }

    /// Transcribe 16 kHz mono `f32` samples. Moonshine's shipped models are English-only, so
    /// `language` is accepted for a uniform engine surface but ignored here.
    pub fn transcribe(&self, audio: &[f32], _language: Option<&str>) -> Result<String> {
        let mut guard = self.recognizer.lock().unwrap();
        let recognizer = guard.as_mut().context("Moonshine model not loaded")?;
        let started = std::time::Instant::now();
        let text = recognizer.transcribe(16_000, audio).text.trim().to_string();
        tracing::info!(
            samples = audio.len(),
            out_chars = text.len(),
            took_ms = started.elapsed().as_millis() as u64,
            "Moonshine transcribed a chunk"
        );
        Ok(text)
    }
}
