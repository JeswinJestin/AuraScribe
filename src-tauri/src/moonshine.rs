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

/// The ONNX graphs + token table that make up a Moonshine model on disk. sherpa-onnx names
/// them exactly this inside its published archives, so we keep the same names on extraction.
const MODEL_FILES: [&str; 5] = [
    "preprocess.onnx",
    "encode.int8.onnx",
    "uncached_decode.int8.onnx",
    "cached_decode.int8.onnx",
    "tokens.txt",
];

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
        let result = recognizer.transcribe(16_000, audio);
        Ok(result.text.trim().to_string())
    }
}
