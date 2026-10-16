//! Layer D — **word-by-word streaming** ASR for the Notetaker, on-device.
//!
//! sherpa-rs has no safe wrapper for the *online* (streaming) recognizer, so this builds it with the
//! raw `sherpa-rs-sys` FFI — the same pattern `nemo_ctc.rs` uses for its offline recognizer. The
//! model is a small streaming zipformer transducer (~45 MB, opt-in download). Unlike the chunked
//! offline path, an online recognizer accepts audio continuously and exposes a *growing* hypothesis,
//! which is what lets the transcript fill in word by word and finalize on a speech endpoint.
//!
//! Verification: the unsafe FFI (load + decode) is smoke-tested by [`tests`] against a real
//! downloaded model (run with `--ignored`); accuracy and the live wiring still need the owner's
//! hardware, like every other audio path here. Gated behind the `moonshine` feature.

use std::path::{Path, PathBuf};

/// Model subdirectory under the models dir, and the four files a streaming zipformer needs.
pub const MODEL_DIR: &str = "streaming-zipformer-en-20M";
const ENCODER: &str = "encoder-epoch-99-avg-1.int8.onnx";
const DECODER: &str = "decoder-epoch-99-avg-1.onnx";
const JOINER: &str = "joiner-epoch-99-avg-1.int8.onnx";
const TOKENS: &str = "tokens.txt";
const BASE_URL: &str =
    "https://huggingface.co/csukuangfj/sherpa-onnx-streaming-zipformer-en-20M-2023-02-17/resolve/main";
/// Approx bytes per file, for a byte-weighted progress bar (encoder dominates).
const FILES: &[(&str, u64)] = &[
    (ENCODER, 42_845_182),
    (DECODER, 2_092_272),
    (JOINER, 259_572),
    (TOKENS, 5_048),
];
/// Total download size in MB, for the UI.
pub const MODEL_SIZE_MB: u64 = 45;

pub fn model_dir(models_dir: &Path) -> PathBuf {
    models_dir.join(MODEL_DIR)
}

pub fn is_downloaded(models_dir: &Path) -> bool {
    let d = model_dir(models_dir);
    FILES.iter().all(|(f, _)| d.join(f).exists())
}

/// Download the four model files into the models dir (each streamed to a `.part`, then renamed).
pub async fn download(
    models_dir: &Path,
    mut on_progress: impl FnMut(f32) + Send,
) -> anyhow::Result<PathBuf> {
    use anyhow::Context;
    let dir = model_dir(models_dir);
    if is_downloaded(models_dir) {
        on_progress(1.0);
        return Ok(dir);
    }
    tokio::fs::create_dir_all(&dir).await.ok();
    let total: u64 = FILES.iter().map(|(_, b)| *b).sum();
    let mut done: u64 = 0;
    let client = reqwest::Client::new();
    use futures_util::StreamExt;
    for (file, _) in FILES {
        let dest = dir.join(file);
        if dest.exists() {
            done += FILES.iter().find(|(f, _)| f == file).map(|(_, b)| *b).unwrap_or(0);
            continue;
        }
        let tmp = dest.with_extension("part");
        let resp = client
            .get(format!("{BASE_URL}/{file}"))
            .send()
            .await
            .with_context(|| format!("start download of {file}"))?
            .error_for_status()
            .with_context(|| format!("download of {file} rejected"))?;
        let mut out = tokio::fs::File::create(&tmp).await.context("create file")?;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("download error")?;
            tokio::io::AsyncWriteExt::write_all(&mut out, &chunk).await.context("write")?;
            done += chunk.len() as u64;
            on_progress(if total > 0 { (done as f32 / total as f32).min(0.999) } else { 0.0 });
        }
        tokio::io::AsyncWriteExt::flush(&mut out).await.ok();
        drop(out);
        tokio::fs::rename(&tmp, &dest).await.context("finalize file")?;
    }
    on_progress(1.0);
    Ok(dir)
}

#[cfg(feature = "moonshine")]
mod ffi_engine {
    use super::*;
    use anyhow::Result;
    use std::ffi::{CStr, CString};
    use std::sync::Mutex;

    /// Owns the raw online-recognizer + its persistent stream. The C objects are only touched
    /// behind our `Mutex`, so the raw pointers are safe to send across threads.
    struct Inner {
        rec: *const sherpa_rs_sys::SherpaOnnxOnlineRecognizer,
        stream: *const sherpa_rs_sys::SherpaOnnxOnlineStream,
    }
    unsafe impl Send for Inner {}
    unsafe impl Sync for Inner {}
    impl Drop for Inner {
        fn drop(&mut self) {
            unsafe {
                sherpa_rs_sys::SherpaOnnxDestroyOnlineStream(self.stream);
                sherpa_rs_sys::SherpaOnnxDestroyOnlineRecognizer(self.rec);
            }
        }
    }

    /// A loaded streaming (online) transducer recognizer with one persistent stream — fed
    /// continuously while a meeting records, so the hypothesis grows word by word.
    pub struct StreamingRecognizer {
        inner: Mutex<Inner>,
    }

    impl StreamingRecognizer {
        /// Load the streaming model from `<models_dir>/streaming-zipformer-en-20M`.
        pub fn load(models_dir: &Path) -> Result<Self> {
            let dir = model_dir(models_dir);
            if !is_downloaded(models_dir) {
                anyhow::bail!("streaming model not downloaded");
            }
            let encoder = CString::new(dir.join(ENCODER).to_string_lossy().into_owned())?;
            let decoder = CString::new(dir.join(DECODER).to_string_lossy().into_owned())?;
            let joiner = CString::new(dir.join(JOINER).to_string_lossy().into_owned())?;
            let tokens = CString::new(dir.join(TOKENS).to_string_lossy().into_owned())?;
            let provider = CString::new("cpu")?;
            let decoding = CString::new("greedy_search")?;

            let rec = unsafe {
                let mut model_config: sherpa_rs_sys::SherpaOnnxOnlineModelConfig =
                    std::mem::zeroed();
                model_config.transducer.encoder = encoder.as_ptr();
                model_config.transducer.decoder = decoder.as_ptr();
                model_config.transducer.joiner = joiner.as_ptr();
                model_config.tokens = tokens.as_ptr();
                model_config.num_threads = 1;
                model_config.provider = provider.as_ptr();
                model_config.debug = 0;

                let mut config: sherpa_rs_sys::SherpaOnnxOnlineRecognizerConfig =
                    std::mem::zeroed();
                config.model_config = model_config;
                config.decoding_method = decoding.as_ptr();
                config.feat_config.sample_rate = 16000;
                config.feat_config.feature_dim = 80;
                config.enable_endpoint = 1;
                config.rule1_min_trailing_silence = 2.4;
                config.rule2_min_trailing_silence = 1.2;
                config.rule3_min_utterance_length = 20.0;

                sherpa_rs_sys::SherpaOnnxCreateOnlineRecognizer(&config)
            };
            if rec.is_null() {
                anyhow::bail!("sherpa-onnx could not create the streaming recognizer");
            }
            let stream = unsafe { sherpa_rs_sys::SherpaOnnxCreateOnlineStream(rec) };
            if stream.is_null() {
                unsafe { sherpa_rs_sys::SherpaOnnxDestroyOnlineRecognizer(rec) };
                anyhow::bail!("failed to create online stream");
            }
            Ok(Self { inner: Mutex::new(Inner { rec, stream }) })
        }

        /// Feed a chunk of 16 kHz mono audio to the running stream.
        pub fn accept(&self, samples16k: &[f32]) {
            let g = self.inner.lock().unwrap();
            unsafe {
                sherpa_rs_sys::SherpaOnnxOnlineStreamAcceptWaveform(
                    g.stream,
                    16000,
                    samples16k.as_ptr(),
                    samples16k.len() as i32,
                );
            }
        }

        /// Decode whatever is ready and return the current (growing) hypothesis plus whether the
        /// recognizer has detected a speech endpoint (end of the current utterance).
        pub fn poll(&self) -> (String, bool) {
            let g = self.inner.lock().unwrap();
            unsafe {
                while sherpa_rs_sys::SherpaOnnxIsOnlineStreamReady(g.rec, g.stream) != 0 {
                    sherpa_rs_sys::SherpaOnnxDecodeOnlineStream(g.rec, g.stream);
                }
                let result_ptr = sherpa_rs_sys::SherpaOnnxGetOnlineStreamResult(g.rec, g.stream);
                let text = if result_ptr.is_null() {
                    String::new()
                } else {
                    let raw = result_ptr.read();
                    let s = if raw.text.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(raw.text).to_string_lossy().into_owned()
                    };
                    sherpa_rs_sys::SherpaOnnxDestroyOnlineRecognizerResult(result_ptr);
                    s
                };
                let endpoint =
                    sherpa_rs_sys::SherpaOnnxOnlineStreamIsEndpoint(g.rec, g.stream) != 0;
                (text.trim().to_string(), endpoint)
            }
        }

        /// Reset the stream after an endpoint, so the next utterance starts clean.
        pub fn reset(&self) {
            let g = self.inner.lock().unwrap();
            unsafe { sherpa_rs_sys::SherpaOnnxOnlineStreamReset(g.rec, g.stream) }
        }
    }
}

#[cfg(feature = "moonshine")]
pub use ffi_engine::StreamingRecognizer;

// Fallback: no sherpa engine → streaming unavailable.
#[cfg(not(feature = "moonshine"))]
pub struct StreamingRecognizer;
#[cfg(not(feature = "moonshine"))]
impl StreamingRecognizer {
    pub fn load(_models_dir: &Path) -> anyhow::Result<Self> {
        anyhow::bail!("streaming needs the moonshine engine")
    }
    pub fn accept(&self, _samples16k: &[f32]) {}
    pub fn poll(&self) -> (String, bool) {
        (String::new(), false)
    }
    pub fn reset(&self) {}
}

#[cfg(all(test, feature = "moonshine"))]
mod tests {
    use super::*;

    /// Smoke test of the unsafe online-recognizer FFI against a REAL model. Point `AURASCRIBE_MODELS`
    /// at a dir containing `streaming-zipformer-en-20M/` and run:
    ///   cargo test --features moonshine streaming_ffi_smoke -- --ignored --nocapture
    /// Proves the recognizer loads and decodes without crashing (1s of silence → empty/benign text).
    #[test]
    #[ignore]
    fn streaming_ffi_smoke() {
        let dir = std::env::var("AURASCRIBE_MODELS").expect("set AURASCRIBE_MODELS to the models dir");
        let models_dir = std::path::Path::new(&dir);
        assert!(is_downloaded(models_dir), "model files missing under {dir}");
        let rec = StreamingRecognizer::load(models_dir).expect("load streaming recognizer");
        let silence = vec![0.0f32; 16_000]; // 1 second
        rec.accept(&silence);
        let (text, endpoint) = rec.poll();
        rec.reset();
        println!("streaming smoke decode => {text:?} (endpoint={endpoint})");
    }

    /// Fuller end-to-end check: decode a real 16 kHz mono PCM WAV in ~300 ms chunks (as the live
    /// loop does) and assert it produces actual words. Set AURASCRIBE_MODELS and AURASCRIBE_TEST_WAV.
    ///   cargo test --features moonshine streaming_decodes_speech -- --ignored --nocapture
    #[test]
    #[ignore]
    fn streaming_decodes_speech() {
        let dir = std::env::var("AURASCRIBE_MODELS").expect("set AURASCRIBE_MODELS");
        let wav = std::env::var("AURASCRIBE_TEST_WAV").expect("set AURASCRIBE_TEST_WAV");
        let rec = StreamingRecognizer::load(std::path::Path::new(&dir)).expect("load");

        // Minimal WAV read: find the `data` chunk, then interpret as i16 LE mono → f32.
        let bytes = std::fs::read(&wav).expect("read wav");
        let mut i = 12; // skip RIFF header
        let mut data: &[u8] = &[];
        while i + 8 <= bytes.len() {
            let id = &bytes[i..i + 4];
            let sz = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]]) as usize;
            if id == b"data" {
                data = &bytes[i + 8..(i + 8 + sz).min(bytes.len())];
                break;
            }
            i += 8 + sz + (sz & 1);
        }
        let samples: Vec<f32> = data
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
            .collect();
        assert!(!samples.is_empty(), "no PCM samples parsed");

        let mut finalized = String::new();
        for chunk in samples.chunks(4800) {
            rec.accept(chunk);
            let (text, endpoint) = rec.poll();
            if endpoint && !text.is_empty() {
                finalized.push_str(&text);
                finalized.push(' ');
                rec.reset();
            }
        }
        let (tail, _) = rec.poll();
        let full = format!("{finalized}{tail}").trim().to_string();
        println!("streaming speech decode => {full:?}");
        assert!(!full.is_empty(), "expected words from a speech sample, got empty");
    }
}
