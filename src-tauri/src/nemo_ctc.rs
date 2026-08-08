// src-tauri/src/nemo_ctc.rs
//! NeMo Conformer-CTC engine — runs **AI4Bharat IndicConformer** (the accurate Indian-language
//! models) via sherpa-onnx's offline NeMo-CTC recognizer.
//!
//! This is the path to accurate **Malayalam and Kannada** (which Dolphin lacks) without the
//! blocked NeMo→sherpa export: the community already published the IndicConformer weights as
//! plain CTC ONNX (`trysem/indicconformer-120m-onnx`, CC-BY-4.0) — one `model.onnx` + `vocab.json`
//! per language. Its interface (`audio_signal [B,80,T]` + `length` → CTC logits) is exactly the
//! standard NeMo Conformer-CTC that sherpa-onnx supports natively, so we load it through the
//! `nemo_ctc` model config. sherpa-rs has no safe wrapper for that config, so this module builds
//! it with the raw `sherpa-rs-sys` FFI (same pattern sherpa-rs uses internally for Dolphin).
//!
//! **Unverified end-to-end** (needs a real Malayalam mic test + the 493 MB download). The two
//! risks, both visible in the log if they bite: sherpa may need extra metadata on the ONNX, or the
//! blank-token position in `tokens.txt` may need adjusting. Gated behind the `moonshine` feature.

use anyhow::{Context, Result};
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::asr::{EngineKind, ModelInfo};

/// The files a NeMo-CTC bundle needs on disk. `model.onnx` and `vocab.json` are downloaded;
/// `tokens.txt` is generated from `vocab.json` at download time (sherpa reads tokens.txt, not
/// the JSON vocab).
const MODEL_FILE: &str = "model.onnx";
const VOCAB_FILE: &str = "vocab.json";
const TOKENS_FILE: &str = "tokens.txt";

/// The IndicConformer line-up we expose. Each is a `<lang>/model.onnx` + `<lang>/vocab.json`
/// inside the source repo. Malayalam and Kannada first — the two the fast engines can't do.
/// `size_mb` is the fp32 model (~493 MB); `cpu_cost` is an estimate (120M Conformer-CTC is
/// moderate on CPU) pending a real benchmark.
const NEMO_MODELS: &[(&str, &str, &str, u64, u8, u8, f32)] = &[
    // (id, hf_repo, lang_subdir, size_mb, speed(1=fastest), accuracy(5=best), cpu_cost)
    //
    // Re-enabled in Round 27. The raw `trysem/indicconformer-120m-onnx` ONNX lacks the sherpa-onnx
    // metadata sherpa needs, and sherpa *aborts the process* on such a model (Round 26b crash). The
    // fix, verified with the sherpa-onnx Python API on this exact model: append the six metadata
    // entries sherpa's own exporter sets (`ensure_packaged` / `append_sherpa_metadata` below), then
    // it loads cleanly and outputs Malayalam. So the app now packages the model itself before load.
    ("indicconformer-ml", "trysem/indicconformer-120m-onnx", "ml", 494, 3, 4, 0.6),
    ("indicconformer-kn", "trysem/indicconformer-120m-onnx", "kn", 494, 3, 4, 0.6),
];

fn nemo_row(model_id: &str) -> Option<&'static (&'static str, &'static str, &'static str, u64, u8, u8, f32)> {
    NEMO_MODELS.iter().find(|(id, ..)| *id == model_id)
}

pub fn is_nemo_ctc_model(model_id: &str) -> bool {
    nemo_row(model_id).is_some()
}

/// Owns the raw sherpa-onnx offline recognizer pointer. Not `Send`/`Sync` by default (raw ptr),
/// so we assert it — the C object is used only behind our `Mutex`.
struct Recognizer(*const sherpa_rs_sys::SherpaOnnxOfflineRecognizer);
unsafe impl Send for Recognizer {}
unsafe impl Sync for Recognizer {}
impl Drop for Recognizer {
    fn drop(&mut self) {
        unsafe { sherpa_rs_sys::SherpaOnnxDestroyOfflineRecognizer(self.0) }
    }
}

pub struct NemoCtcASR {
    recognizer: Mutex<Option<Recognizer>>,
    models_dir: PathBuf,
}

impl NemoCtcASR {
    pub fn new(models_dir: PathBuf) -> Self {
        Self {
            recognizer: Mutex::new(None),
            models_dir,
        }
    }

    pub fn model_dir(&self, model_id: &str) -> PathBuf {
        self.models_dir.join(model_id)
    }

    /// Present once the model and the generated tokens.txt are both on disk.
    pub fn is_present(&self, model_id: &str) -> bool {
        let dir = self.model_dir(model_id);
        dir.join(MODEL_FILE).exists() && dir.join(TOKENS_FILE).exists()
    }

    pub fn list_available_models(&self) -> Vec<ModelInfo> {
        NEMO_MODELS
            .iter()
            .map(|(id, _repo, _lang, size_mb, speed, accuracy, cpu_cost)| {
                let downloaded = self.is_present(id);
                ModelInfo {
                    id: id.to_string(),
                    name: id.to_string(),
                    engine: EngineKind::NemoCtc,
                    multilingual: false, // one language per model (ml, kn, ...)
                    size_mb: *size_mb,
                    speed: *speed,
                    accuracy: *accuracy,
                    recommended: false,
                    downloaded,
                    path: downloaded.then(|| self.model_dir(id)),
                    realtime_factor: *cpu_cost,
                    warning: None,
                }
            })
            .collect()
    }

    /// Download `model.onnx` + `vocab.json` for the language, then generate `tokens.txt` from the
    /// vocab (sherpa reads tokens.txt). Byte-weighted progress dominated by the ~493 MB model.
    pub async fn download_model(
        &self,
        model_id: &str,
        mut on_progress: impl FnMut(f32) + Send,
    ) -> Result<PathBuf> {
        let (_, repo, lang, ..) =
            nemo_row(model_id).with_context(|| format!("Unknown NeMo-CTC model: {model_id}"))?;

        let dir = self.model_dir(model_id);
        if self.is_present(model_id) {
            on_progress(1.0);
            return Ok(dir);
        }
        tokio::fs::create_dir_all(&dir)
            .await
            .context("Failed to create model directory")?;

        let base = format!("https://huggingface.co/{repo}/resolve/main/{lang}");
        // (filename, approx bytes) — model dominates.
        let files = [(MODEL_FILE, 493_000_000u64), (VOCAB_FILE, 70_000u64)];
        let total: u64 = files.iter().map(|(_, b)| *b).sum();
        let mut done: u64 = 0;

        let client = reqwest::Client::new();
        use futures_util::StreamExt;
        for (file, approx) in files.iter() {
            let dest = dir.join(file);
            if dest.exists() {
                done += *approx;
                on_progress((done as f32 / total as f32).min(1.0));
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
                on_progress(((done as f32 + within) / total as f32).min(1.0));
            }
            tokio::io::AsyncWriteExt::flush(&mut out)
                .await
                .context("flush")?;
            drop(out);
            tokio::fs::rename(&tmp, &dest)
                .await
                .with_context(|| format!("Failed to finalize {file}"))?;
            done += *approx;
            on_progress((done as f32 / total as f32).min(1.0));
        }

        // Package for sherpa-onnx right away (tokens.txt + appended metadata), so the very first
        // load succeeds instead of aborting.
        ensure_packaged(&dir).context("Failed to package the model for sherpa-onnx")?;

        on_progress(1.0);
        Ok(dir)
    }

    /// Load the model into a sherpa-onnx offline NeMo-CTC recognizer (raw FFI).
    pub fn load_model(&self, model_id: &str, num_threads: usize) -> Result<()> {
        let dir = self.model_dir(model_id);
        if !self.is_present(model_id) {
            anyhow::bail!("NeMo-CTC model '{model_id}' is not fully downloaded ({}).", dir.display());
        }
        // Make sure the ONNX carries the sherpa-onnx metadata (else sherpa ABORTS the process, not
        // a catchable error). Idempotent via a `.packaged` marker, and it repairs a raw download
        // left over from an earlier build. This is the fix that stops the crash.
        ensure_packaged(&dir).context("Failed to package the model for sherpa-onnx")?;

        let model = CString::new(dir.join(MODEL_FILE).to_string_lossy().into_owned())?;
        let tokens = CString::new(dir.join(TOKENS_FILE).to_string_lossy().into_owned())?;
        let provider = CString::new("cpu")?;
        let decoding = CString::new("greedy_search")?;

        tracing::info!(model = %model_id, threads = num_threads, "Loading NeMo-CTC model (sherpa-onnx)");
        let recognizer = unsafe {
            // Zero everything, then set only the NeMo-CTC path, tokens, threads and provider.
            let mut model_config: sherpa_rs_sys::SherpaOnnxOfflineModelConfig = std::mem::zeroed();
            model_config.nemo_ctc.model = model.as_ptr();
            model_config.tokens = tokens.as_ptr();
            model_config.num_threads = num_threads.max(1) as i32;
            model_config.debug = 0;
            model_config.provider = provider.as_ptr();

            let mut config: sherpa_rs_sys::SherpaOnnxOfflineRecognizerConfig = std::mem::zeroed();
            config.model_config = model_config;
            config.decoding_method = decoding.as_ptr();
            config.feat_config.sample_rate = 16000;
            config.feat_config.feature_dim = 80;

            sherpa_rs_sys::SherpaOnnxCreateOfflineRecognizer(&config)
        };
        if recognizer.is_null() {
            anyhow::bail!(
                "sherpa-onnx could not create a NeMo-CTC recognizer for '{model_id}'. The ONNX may \
                 need sherpa metadata added, or the model/tokens are mismatched."
            );
        }

        *self.recognizer.lock().unwrap() = Some(Recognizer(recognizer));
        tracing::info!(model = %model_id, "NeMo-CTC model loaded");
        Ok(())
    }

    /// Transcribe 16 kHz mono `f32` samples via the offline stream API.
    pub fn transcribe(&self, audio: &[f32], _language: Option<&str>) -> Result<String> {
        let guard = self.recognizer.lock().unwrap();
        let rec = guard.as_ref().context("NeMo-CTC model not loaded")?.0;
        let started = std::time::Instant::now();
        let text = unsafe {
            let stream = sherpa_rs_sys::SherpaOnnxCreateOfflineStream(rec);
            sherpa_rs_sys::SherpaOnnxAcceptWaveformOffline(
                stream,
                16000,
                audio.as_ptr(),
                audio.len() as i32,
            );
            sherpa_rs_sys::SherpaOnnxDecodeOfflineStream(rec, stream);
            let result_ptr = sherpa_rs_sys::SherpaOnnxGetOfflineStreamResult(stream);
            let text = if result_ptr.is_null() {
                String::new()
            } else {
                let raw = result_ptr.read();
                let s = if raw.text.is_null() {
                    String::new()
                } else {
                    std::ffi::CStr::from_ptr(raw.text).to_string_lossy().into_owned()
                };
                sherpa_rs_sys::SherpaOnnxDestroyOfflineRecognizerResult(result_ptr);
                s
            };
            sherpa_rs_sys::SherpaOnnxDestroyOfflineStream(stream);
            text
        };
        let text = text.trim().to_string();
        tracing::info!(
            samples = audio.len(),
            out_chars = text.len(),
            took_ms = started.elapsed().as_millis() as u64,
            "NeMo-CTC transcribed a chunk"
        );
        Ok(text)
    }
}

/// Ensure the downloaded ONNX is packaged for sherpa-onnx: append the metadata sherpa's loader
/// requires (verified against the sherpa-onnx Python API on this exact model) and (re)generate
/// tokens.txt. Idempotent — a `.packaged` marker means it's already done. Without this, sherpa-onnx
/// calls `exit()` and takes the whole app down.
fn ensure_packaged(dir: &std::path::Path) -> Result<()> {
    let marker = dir.join(".packaged");
    if marker.exists() {
        return Ok(());
    }
    // vocab_size = number of BPE tokens; the CTC blank sits at this index.
    let raw = std::fs::read_to_string(dir.join(VOCAB_FILE)).context("read vocab.json")?;
    let vocab: Vec<String> = serde_json::from_str(&raw).context("vocab.json is not a JSON array")?;
    write_tokens_from_vocab(dir)?;
    append_sherpa_metadata(&dir.join(MODEL_FILE), vocab.len())?;
    std::fs::write(&marker, b"1").ok();
    tracing::info!(dir = %dir.display(), vocab = vocab.len(), "Packaged IndicConformer ONNX for sherpa-onnx");
    Ok(())
}

/// Append the six ONNX `metadata_props` entries sherpa-onnx's NeMo-CTC loader reads. ONNX is a
/// protobuf; `metadata_props` is field 14 (repeated `StringStringEntryProto{key=1,value=2}`), and
/// repeated fields merge no matter where they appear — so appending encoded entries to the end of
/// the file adds metadata without parsing the 493 MB model. Matches what sherpa's own exporter sets.
fn append_sherpa_metadata(model_path: &std::path::Path, vocab_size: usize) -> Result<()> {
    fn varint(mut n: u64, out: &mut Vec<u8>) {
        loop {
            let b = (n & 0x7f) as u8;
            n >>= 7;
            if n != 0 {
                out.push(b | 0x80);
            } else {
                out.push(b);
                break;
            }
        }
    }
    fn entry(key: &str, value: &str, out: &mut Vec<u8>) {
        let mut inner = Vec::new();
        inner.push(0x0a); // field 1 (key), wire type 2
        varint(key.len() as u64, &mut inner);
        inner.extend_from_slice(key.as_bytes());
        inner.push(0x12); // field 2 (value), wire type 2
        varint(value.len() as u64, &mut inner);
        inner.extend_from_slice(value.as_bytes());
        out.push(0x72); // field 14 (metadata_props), wire type 2
        varint(inner.len() as u64, out);
        out.extend_from_slice(&inner);
    }

    let vs = vocab_size.to_string();
    let meta: [(&str, &str); 6] = [
        ("vocab_size", &vs),
        ("normalize_type", "per_feature"),
        ("subsampling_factor", "8"),
        ("model_type", "EncDecHybridRNNTCTCBPEModel"),
        ("version", "1"),
        ("model_author", "AI4Bharat"),
    ];
    let mut extra = Vec::new();
    for (k, v) in meta {
        entry(k, v, &mut extra);
    }

    use std::io::Write as _;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(model_path)
        .context("open model.onnx for append")?;
    f.write_all(&extra).context("append metadata")?;
    Ok(())
}

/// Convert `vocab.json` (a JSON array of BPE token strings) into sherpa-onnx `tokens.txt`
/// (`<symbol> <id>` per line), appending a final blank token at id = len(vocab).
fn write_tokens_from_vocab(dir: &std::path::Path) -> Result<()> {
    let raw = std::fs::read_to_string(dir.join(VOCAB_FILE)).context("read vocab.json")?;
    let vocab: Vec<String> = serde_json::from_str(&raw).context("vocab.json is not a JSON array of strings")?;

    let mut out = String::with_capacity(vocab.len() * 8);
    for (id, tok) in vocab.iter().enumerate() {
        // Guard the two-column format: a token can't be empty or contain whitespace. NeMo BPE uses
        // the metasymbol U+2581 for spaces, so real tokens are safe; substitute defensively.
        let sym = if tok.is_empty() || tok.chars().any(|c| c.is_whitespace()) {
            format!("<u{id}>")
        } else {
            tok.clone()
        };
        out.push_str(&sym);
        out.push(' ');
        out.push_str(&id.to_string());
        out.push('\n');
    }
    // Blank last (IndicConformer CTC: blank id == len(vocab)).
    out.push_str(&format!("<blk> {}\n", vocab.len()));

    std::fs::write(dir.join(TOKENS_FILE), out).context("write tokens.txt")?;
    Ok(())
}
