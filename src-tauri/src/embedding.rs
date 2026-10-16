//! Layer B — speaker **voice fingerprints** for guest diarization, fully on-device.
//!
//! A small (~40 MB) speaker-embedding model turns a window of guest audio into a vector; the pure
//! [`crate::diarize::OnlineClusterer`] then groups those vectors into Speaker 1/2/3… It is an
//! **opt-in download** (the same one-time pattern as the ASR models — the only network use the app
//! ever makes), so until the model is present the Notetaker falls back to a single "Guest" cluster.
//! No cloud, no account.
//!
//! The extraction itself is sherpa-onnx FFI and can only be verified on real audio/hardware, like
//! the loopback capture path; the clustering it feeds is unit-tested in `diarize`.

use std::path::{Path, PathBuf};

/// NeMo TitaNet-small — a compact English speaker-embedding model published by the sherpa-onnx
/// project. 16 kHz mono input; L2-normalized output. (Tag is sherpa's own "recongition" spelling.)
pub const MODEL_FILE: &str = "nemo_en_titanet_small.onnx";
const MODEL_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/nemo_en_titanet_small.onnx";
/// Known size, used for the progress bar when the server doesn't send Content-Length.
pub const MODEL_SIZE_BYTES: u64 = 40_257_283;

pub fn model_path(models_dir: &Path) -> PathBuf {
    models_dir.join(MODEL_FILE)
}

pub fn is_downloaded(models_dir: &Path) -> bool {
    model_path(models_dir).exists()
}

/// Download the speaker-embedding model to the models dir (stream to a `.part`, then rename), the
/// same safe pattern the ASR downloader uses.
pub async fn download(
    models_dir: &Path,
    mut on_progress: impl FnMut(f32) + Send,
) -> anyhow::Result<PathBuf> {
    use anyhow::Context;
    let path = model_path(models_dir);
    if path.exists() {
        on_progress(1.0);
        return Ok(path);
    }
    tokio::fs::create_dir_all(models_dir).await.ok();
    let tmp = path.with_extension("onnx.part");
    let client = reqwest::Client::new();
    let resp = client
        .get(MODEL_URL)
        .send()
        .await
        .context("Failed to start speaker-model download")?
        .error_for_status()
        .context("Speaker-model download rejected by server")?;
    let total = resp.content_length().unwrap_or(MODEL_SIZE_BYTES);

    let mut file = tokio::fs::File::create(&tmp)
        .await
        .context("Failed to create speaker-model file")?;
    let mut downloaded: u64 = 0;
    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Speaker-model download error")?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .context("Write error")?;
        downloaded += chunk.len() as u64;
        on_progress(if total > 0 { downloaded as f32 / total as f32 } else { 0.0 });
    }
    tokio::io::AsyncWriteExt::flush(&mut file).await.ok();
    drop(file);
    tokio::fs::rename(&tmp, &path)
        .await
        .context("Failed to finalize speaker-model file")?;
    on_progress(1.0);
    Ok(path)
}

/// L2-normalize so cosine similarity in the clusterer is just a dot product and thresholds are
/// scale-independent.
fn l2_normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

#[cfg(feature = "moonshine")]
pub struct SpeakerEmbedder {
    inner: sherpa_rs::speaker_id::EmbeddingExtractor,
}

#[cfg(feature = "moonshine")]
impl SpeakerEmbedder {
    /// Load the extractor from the downloaded model. Errors if the model isn't on disk.
    pub fn load(models_dir: &Path) -> anyhow::Result<Self> {
        let path = model_path(models_dir);
        if !path.exists() {
            anyhow::bail!("speaker-embedding model not downloaded");
        }
        let config = sherpa_rs::speaker_id::ExtractorConfig {
            model: path.to_string_lossy().to_string(),
            provider: None,
            num_threads: Some(1),
            debug: false,
        };
        let inner = sherpa_rs::speaker_id::EmbeddingExtractor::new(config)
            .map_err(|e| anyhow::anyhow!("failed to load speaker-embedding model: {e}"))?;
        Ok(Self { inner })
    }

    /// Fingerprint one 16 kHz mono window, or `None` when the window is too short/silent to embed.
    pub fn compute(&mut self, samples16k: &[f32]) -> Option<Vec<f32>> {
        match self.inner.compute_speaker_embedding(samples16k.to_vec(), 16_000) {
            Ok(v) => Some(l2_normalize(v)),
            Err(e) => {
                tracing::debug!("speaker embedding skipped: {e}");
                None
            }
        }
    }
}

// Fallback for a build without the sherpa engine: speaker separation simply stays off.
#[cfg(not(feature = "moonshine"))]
pub struct SpeakerEmbedder;

#[cfg(not(feature = "moonshine"))]
impl SpeakerEmbedder {
    pub fn load(_models_dir: &Path) -> anyhow::Result<Self> {
        anyhow::bail!("speaker separation needs the moonshine engine")
    }
    pub fn compute(&mut self, _samples16k: &[f32]) -> Option<Vec<f32>> {
        None
    }
}
