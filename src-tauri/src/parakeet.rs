// src-tauri/src/parakeet.rs
//! The **transducer engine** — fast multilingual ASR via sherpa-onnx offline transducers
//! (NeMo FastConformer family), ONNX Runtime, CPU. This one engine serves two things:
//!
//! 1. **Parakeet-TDT-0.6b-v3** (built-in, downloadable): NVIDIA's multilingual model — 25
//!    European languages with automatic language detection, accuracy at/above Whisper large-v3,
//!    fast on CPU (~5× real time on a mid-range machine). Same public model Handy/OpenWhispr use.
//!
//! 2. **Custom / bring-your-own transducer bundles** (discovered on disk): *any* sherpa-onnx
//!    offline transducer placed in the models directory is auto-listed and usable. This is how
//!    languages that have no ready-made fast model — **Hindi, Malayalam, and the other Indian
//!    languages via AI4Bharat's IndicConformer** — get in **without a cloud call and without
//!    bundling a heavyweight Python server**: you export the model once to sherpa-onnx format
//!    (see `docs/INDIC-CONFORMER.md`) and drop the folder in. It never leaves your machine.
//!
//! **Honest scope of the built-in:** Parakeet v3 covers 25 *European* languages only. It does
//! **not** cover Hindi/Malayalam/CJK — those are served by a custom IndicConformer bundle (path
//! 2). There is no free model today that matches Moonshine's speed across those languages; a
//! transducer like IndicConformer is faster-than-real-time on a good CPU, not Moonshine-instant.
//!
//! Gated behind the `moonshine` Cargo feature (which is really "the sherpa-onnx engine set").

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};

use crate::asr::{EngineKind, ModelInfo};

/// The built-in, downloadable Parakeet line-up.
///
/// `cpu_cost` is a **conservative estimate** pending a real benchmark — Handy measures ~5× real
/// time (≈0.2×) on a mid-range i5, but the 0.6B encoder is heavier than Moonshine's, so 0.5× is
/// a safe headline that still beats real time. `accuracy` reflects the published WER (≥ large-v3).
const PARAKEET_MODELS: &[(&str, &str, u64, u8, u8, f32)] = &[
    // (id, hf_repo, size_mb, speed(1=fastest), accuracy(5=best), cpu_cost estimate)
    (
        "parakeet-v3-multilingual",
        "csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8",
        671,
        3,
        5,
        0.5,
    ),
];

/// The four files (by *stem*) that make up a sherpa-onnx offline transducer bundle. Each may be
/// full-precision (`<stem>.onnx`) or quantised (`<stem>.int8.onnx`); `resolve` prefers int8.
/// tokens.txt is exact. This set is what distinguishes a transducer bundle from a Moonshine
/// bundle (which has `preprocess`/`encode`/`*_decode`, no `joiner`) or a Whisper `.bin` file.
const TRANSDUCER_STEMS: [&str; 3] = ["encoder", "decoder", "joiner"];

/// Per-file byte estimates for the built-in Parakeet download, used only to weight progress
/// (the encoder is ~98% of the bytes, so equal per-file weighting would make the bar misbehave).
const PARAKEET_DOWNLOAD_FILES: [(&str, u64); 4] = [
    ("encoder.int8.onnx", 652_000_000),
    ("decoder.int8.onnx", 12_000_000),
    ("joiner.int8.onnx", 6_400_000),
    ("tokens.txt", 94_000),
];

fn hf_base_url(hf_repo: &str) -> String {
    format!("https://huggingface.co/{hf_repo}/resolve/main")
}

fn parakeet_row(model_id: &str) -> Option<&'static (&'static str, &'static str, u64, u8, u8, f32)> {
    PARAKEET_MODELS.iter().find(|(id, ..)| *id == model_id)
}

/// Resolve one component of a transducer bundle inside `dir`, preferring the int8 quantised file.
fn resolve(dir: &Path, stem: &str) -> Option<PathBuf> {
    let int8 = dir.join(format!("{stem}.int8.onnx"));
    if int8.exists() {
        return Some(int8);
    }
    let full = dir.join(format!("{stem}.onnx"));
    full.exists().then_some(full)
}

/// Whether `dir` holds a complete sherpa-onnx transducer bundle (encoder+decoder+joiner+tokens).
fn is_transducer_bundle(dir: &Path) -> bool {
    dir.is_dir()
        && TRANSDUCER_STEMS.iter().all(|s| resolve(dir, s).is_some())
        && dir.join("tokens.txt").exists()
}

/// Whether `model_id` names a Parakeet built-in **or** a custom transducer bundle on disk. Used
/// by the engine facade to route this id to the transducer engine.
pub fn is_transducer_model(models_dir: &Path, model_id: &str) -> bool {
    parakeet_row(model_id).is_some() || is_transducer_bundle(&models_dir.join(model_id))
}

/// A transducer engine. The native handle holds state and `transcribe` takes `&mut self`, so it
/// lives behind a `Mutex` and the whole engine is `Send + Sync` for the Tauri state.
pub struct ParakeetASR {
    recognizer: Mutex<Option<TransducerRecognizer>>,
    models_dir: PathBuf,
}

impl ParakeetASR {
    pub fn new(models_dir: PathBuf) -> Self {
        Self {
            recognizer: Mutex::new(None),
            models_dir,
        }
    }

    pub fn model_dir(&self, model_id: &str) -> PathBuf {
        self.models_dir.join(model_id)
    }

    /// Whether this engine handles `model_id` (built-in Parakeet or a custom bundle on disk).
    pub fn owns(&self, model_id: &str) -> bool {
        is_transducer_model(&self.models_dir, model_id)
    }

    /// Whether every file the recognizer needs is present for this model.
    pub fn is_present(&self, model_id: &str) -> bool {
        is_transducer_bundle(&self.model_dir(model_id))
    }

    /// Total size of a bundle on disk, in MB (for the UI). Best-effort.
    fn dir_size_mb(dir: &Path) -> u64 {
        let bytes: u64 = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.metadata().ok().map(|m| m.len()))
            .sum();
        (bytes / 1_000_000).max(1)
    }

    /// Custom (user-provided) transducer bundles discovered in the models directory: any
    /// sub-directory that is a complete transducer bundle and is not a built-in Parakeet id.
    fn custom_models(&self) -> Vec<ModelInfo> {
        let Ok(entries) = std::fs::read_dir(&self.models_dir) else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter_map(|e| {
                let dir = e.path();
                let id = dir.file_name()?.to_string_lossy().into_owned();
                if parakeet_row(&id).is_some() || !is_transducer_bundle(&dir) {
                    return None;
                }
                Some(ModelInfo {
                    id: id.clone(),
                    name: id,
                    engine: EngineKind::Parakeet,
                    // A bundle the user deliberately added for their languages — assume
                    // multilingual and reasonably accurate; speed is unknown until benchmarked.
                    multilingual: true,
                    size_mb: Self::dir_size_mb(&dir),
                    speed: 3,
                    accuracy: 4,
                    recommended: false,
                    downloaded: true,
                    path: Some(dir),
                    realtime_factor: 0.5,
                    warning: None,
                })
            })
            .collect()
    }

    /// The built-in Parakeet models plus any discovered custom transducer bundles.
    pub fn list_available_models(&self) -> Vec<ModelInfo> {
        let mut models: Vec<ModelInfo> = PARAKEET_MODELS
            .iter()
            .map(|(id, _repo, size_mb, speed, accuracy, cpu_cost)| {
                let downloaded = self.is_present(id);
                ModelInfo {
                    id: id.to_string(),
                    name: id.to_string(),
                    engine: EngineKind::Parakeet,
                    multilingual: true, // 25 European languages, auto-detected
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
            .collect();
        models.extend(self.custom_models());
        models
    }

    /// Download a built-in Parakeet model, byte-weighted progress. Custom bundles are not
    /// downloadable — they are placed on disk by the user — so this refuses them with guidance.
    pub async fn download_model(
        &self,
        model_id: &str,
        mut on_progress: impl FnMut(f32) + Send,
    ) -> Result<PathBuf> {
        let Some((_, repo, ..)) = parakeet_row(model_id) else {
            anyhow::bail!(
                "'{model_id}' is a custom model. Place its sherpa-onnx files in {} yourself — \
                 see docs/INDIC-CONFORMER.md.",
                self.model_dir(model_id).display()
            );
        };

        let dir = self.model_dir(model_id);
        if self.is_present(model_id) {
            on_progress(1.0);
            return Ok(dir);
        }
        tokio::fs::create_dir_all(&dir)
            .await
            .context("Failed to create Parakeet model directory")?;

        let base = hf_base_url(repo);
        let client = reqwest::Client::new();
        let total_bytes: u64 = PARAKEET_DOWNLOAD_FILES.iter().map(|(_, b)| *b).sum();
        let mut done_bytes: u64 = 0;

        use futures_util::StreamExt;
        for (file, approx) in PARAKEET_DOWNLOAD_FILES.iter() {
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

    /// Load a transducer bundle (built-in or custom) into memory. Filenames are resolved
    /// flexibly so both int8 and full-precision exports work.
    pub fn load_model(&self, model_id: &str, num_threads: usize) -> Result<()> {
        let dir = self.model_dir(model_id);
        if !is_transducer_bundle(&dir) {
            anyhow::bail!(
                "Transducer model '{model_id}' is incomplete (need encoder/decoder/joiner + \
                 tokens.txt in {}).",
                dir.display()
            );
        }

        let stem = |s: &str| resolve(&dir, s).unwrap().to_string_lossy().into_owned();
        let config = TransducerConfig {
            encoder: stem("encoder"),
            decoder: stem("decoder"),
            joiner: stem("joiner"),
            tokens: dir.join("tokens.txt").to_string_lossy().into_owned(),
            num_threads: num_threads.max(1) as i32,
            sample_rate: 16_000,
            feature_dim: 80,
            decoding_method: "greedy_search".to_string(),
            // Empty so sherpa-onnx reads the real type (e.g. "nemo_transducer"/tdt) from the
            // encoder's embedded metadata. The crate's default "transducer" would mis-decode a
            // TDT model's duration outputs.
            model_type: String::new(),
            provider: Some("cpu".to_string()),
            debug: false,
            ..Default::default()
        };

        tracing::info!(
            model = %model_id,
            encoder = %stem("encoder"),
            threads = num_threads,
            "Loading transducer model (sherpa-onnx). First inference loads the encoder into ONNX \
             Runtime, which can take several seconds for a large model."
        );
        let recognizer = TransducerRecognizer::new(config)
            .map_err(|e| anyhow::anyhow!("Failed to load transducer model '{model_id}': {e}"))?;

        *self.recognizer.lock().unwrap() = Some(recognizer);
        tracing::info!(model = %model_id, "Transducer model loaded");
        Ok(())
    }

    /// Transcribe 16 kHz mono `f32` samples. Transducer models auto-detect language, so
    /// `language` is accepted for a uniform surface but ignored.
    pub fn transcribe(&self, audio: &[f32], _language: Option<&str>) -> Result<String> {
        let mut guard = self.recognizer.lock().unwrap();
        let recognizer = guard.as_mut().context("Transducer model not loaded")?;
        let started = std::time::Instant::now();
        let text = recognizer.transcribe(16_000, audio).trim().to_string();
        tracing::info!(
            samples = audio.len(),
            secs = audio.len() as f32 / 16_000.0,
            out_chars = text.len(),
            took_ms = started.elapsed().as_millis() as u64,
            "Transducer transcribed a chunk"
        );
        if text.is_empty() {
            tracing::warn!(
                samples = audio.len(),
                "Transducer returned EMPTY text — model may be incompatible with this audio, or \
                 the clip was silence. Check the model files and that audio is 16 kHz mono."
            );
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch(path: &Path) {
        fs::write(path, b"x").unwrap();
    }

    #[test]
    fn detects_a_transducer_bundle_and_prefers_int8() {
        let tmp = std::env::temp_dir().join(format!("aura-td-{}", std::process::id()));
        let dir = tmp.join("indic-conformer-ml");
        fs::create_dir_all(&dir).unwrap();
        touch(&dir.join("encoder.int8.onnx"));
        touch(&dir.join("decoder.onnx"));
        touch(&dir.join("joiner.int8.onnx"));
        touch(&dir.join("tokens.txt"));

        assert!(is_transducer_bundle(&dir));
        // int8 is preferred when both would exist; here only int8 encoder exists.
        assert!(resolve(&dir, "encoder").unwrap().ends_with("encoder.int8.onnx"));
        assert!(resolve(&dir, "decoder").unwrap().ends_with("decoder.onnx"));

        // Discovered as a custom model by the engine.
        let engine = ParakeetASR::new(tmp.clone());
        let ids: Vec<String> = engine.list_available_models().into_iter().map(|m| m.id).collect();
        assert!(ids.iter().any(|id| id == "indic-conformer-ml"));
        assert!(engine.owns("indic-conformer-ml"));

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn a_moonshine_bundle_is_not_a_transducer_bundle() {
        // Moonshine has encode/decode graphs and no joiner — must not be mistaken for one.
        let tmp = std::env::temp_dir().join(format!("aura-ms-{}", std::process::id()));
        let dir = tmp.join("moonshine-base-en");
        fs::create_dir_all(&dir).unwrap();
        touch(&dir.join("encode.int8.onnx"));
        touch(&dir.join("uncached_decode.int8.onnx"));
        touch(&dir.join("tokens.txt"));

        assert!(!is_transducer_bundle(&dir));
        fs::remove_dir_all(&tmp).ok();
    }
}
