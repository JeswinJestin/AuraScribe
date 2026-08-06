// src-tauri/src/asr.rs
//! Automatic Speech Recognition using Whisper.cpp via whisper-rs

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const MODELS_DIR: &str = "models";

/// Which speech-recognition backend runs a given model.
///
/// Whisper (via whisper.cpp) is the original v1 engine and stays the accuracy fallback.
/// Moonshine is the newer, faster, lower-latency engine (ONNX via sherpa-onnx); Parakeet is
/// the multilingual/accelerated tier. Serialised lowercase so the frontend can badge models
/// by engine ("New · Moonshine") without a lookup table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
// Variants are populated per build: Moonshine only under the `moonshine` feature, Parakeet
// is a declared roadmap engine not yet wired — so some are "never constructed" in some builds.
#[allow(dead_code)]
pub enum EngineKind {
    Whisper,
    Moonshine,
    Parakeet,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub id: String,
    /// Filename stem on HuggingFace, e.g. `large-v3-turbo-q5_0`.
    pub name: String,
    /// The backend that runs this model — see `EngineKind`.
    pub engine: EngineKind,
    pub size_mb: u64,
    pub multilingual: bool,
    /// Rough speed rank, 1 = fastest. Used for ordering and UI hints.
    pub speed: u8,
    /// Rough accuracy rank, 5 = best.
    pub accuracy: u8,
    /// True for the best choice **on this machine**, not in the abstract. Computed, because
    /// a static "recommended" flag told a CPU-only laptop to run a 3.1 GB model.
    pub recommended: bool,
    pub downloaded: bool,
    pub path: Option<PathBuf>,
    /// Estimated processing time as a multiple of the length of speech, on this hardware.
    /// Below 1.0 means it finishes faster than you spoke.
    pub realtime_factor: f32,
    /// Set when this model would be a bad experience here. Shown in the UI *before* the
    /// user commits to a multi-gigabyte download.
    pub warning: Option<String>,
}

/// The curated model line-up: **only models that run faster than speech on a CPU.**
///
/// The `large-v3` family was removed. On a CPU those models run at 2.5x–15x the length of the
/// speech — a 30-second dictation became minutes of waiting with the fan screaming — and this
/// is a free, local, CPU-first product, so a model that needs a GPU to be usable does not
/// belong in the list. Keeping it only invited the failure it kept causing: a user reaches
/// for "most accurate" and gets a terrible experience. Accuracy on clean dictation is
/// improved far more cheaply by silence trimming and (future) a personal dictionary than by a
/// model the machine cannot run.
///
/// `base` is the multilingual counterpart of `base.en` — same size and speed, tuned for
/// languages beyond English — so dictating in other languages stays possible without the
/// heavy models. The `.en` variants are more accurate for English specifically.
///
/// If GPU offload lands (see `build-vulkan.bat`), the large models become viable again and
/// can be reinstated behind a `gpu_enabled()` check.
///
/// `cpu_cost` is processing time as a multiple of the speech length, CPU-only, anchored on
/// real measurements from the owner's 8-core Ryzen rather than on guesswork:
///
/// | model | measured |
/// |---|---|
/// | `base.en` | 0.36-0.62x over 7 real dictations |
/// | `large-v3` | 8.8x, 16.6x, 27.2x over 3 real dictations |
///
/// The `large-v3` numbers are why this table exists. A user reasonably picked the model
/// labelled most accurate and waited **7.9 minutes for 17 seconds of speech** while their
/// laptop's fan screamed. Nothing in the UI had warned them.
const MODELS: &[(&str, EngineKind, u64, bool, u8, u8, f32)] = &[
    // (id, engine, size_mb, multilingual, speed(1=fastest), accuracy(5=best), cpu_cost)
    ("tiny.en", EngineKind::Whisper, 75, false, 1, 2, 0.2), // English · fastest, weak machines
    ("base.en", EngineKind::Whisper, 142, false, 2, 3, 0.5), // English · Whisper recommended
    ("base", EngineKind::Whisper, 142, true, 2, 2, 0.6),    // multilingual · same speed
    // NOTE: heavier Whisper models (small, large-v3-turbo) are deliberately NOT offered. On a
    // CPU they are far too slow (minutes per clip) — the opposite of what this app is for. The
    // path to better accuracy is a light, fast engine (Moonshine), not a bigger Whisper model.
];

/// Whether a GPU backend is compiled in. `use_gpu(true)` is requested unconditionally, but
/// it is a no-op unless whisper-rs was built with `vulkan`/`cuda`/`metal` — so this must key
/// off the build, never off the presence of a GPU in the machine.
pub fn gpu_enabled() -> bool {
    cfg!(any(feature = "vulkan", feature = "cuda", feature = "metal"))
}

/// How much faster a GPU makes inference. Conservative: the point of the estimate is to stop
/// recommending something unusable, so erring low is the safe direction.
const GPU_SPEEDUP: f32 = 8.0;

/// Estimated processing time as a multiple of speech length on this machine.
fn realtime_factor(cpu_cost: f32) -> f32 {
    if gpu_enabled() {
        cpu_cost / GPU_SPEEDUP
    } else {
        cpu_cost
    }
}

/// Estimated processing time for a model, as a multiple of the speech length, on this
/// machine. Unknown ids fall back to a slow guess so an unfamiliar model is treated as
/// "cannot keep up" rather than optimistically chunked.
pub fn realtime_factor_for(model_id: &str) -> f32 {
    MODELS
        .iter()
        .find(|(id, ..)| *id == model_id)
        .map(|(_, _, _, _, _, _, cpu_cost)| realtime_factor(*cpu_cost))
        .unwrap_or(5.0)
}

/// Whether live chunking helps this model here. It only does when the model transcribes
/// faster than speech arrives — otherwise the backlog grows during the recording and is
/// paid back afterwards, and the extra 30-second windows each small chunk costs make the
/// total *slower*. The 0.8 margin leaves headroom for scheduling and thermal throttling.
pub fn should_chunk(model_id: &str) -> bool {
    realtime_factor_for(model_id) <= 0.8
}

/// Threads to give whisper.cpp: **physical** cores, not logical.
///
/// whisper.cpp defaults to 4 regardless of the machine, so using the real core count is a
/// large free speedup. But using *logical* cores is a mistake on an SMT machine: inference
/// is SIMD-bound, and the two threads sharing a core contend for the same vector units. You
/// pay the extra heat and power for little or no throughput — which matters on a laptop
/// that is already thermally limited. whisper.cpp's own guidance is physical cores.
///
/// On the owner's 8-core / 16-thread Ryzen this asks for 8 instead of 16.
pub fn worker_threads() -> usize {
    physical_cores()
        .or_else(|| std::thread::available_parallelism().ok().map(|n| n.get()))
        .unwrap_or(4)
        .clamp(1, 16)
}

#[cfg(target_os = "windows")]
fn physical_cores() -> Option<usize> {
    use windows::Win32::System::SystemInformation::{
        GetLogicalProcessorInformation, RelationProcessorCore, SYSTEM_LOGICAL_PROCESSOR_INFORMATION,
    };

    unsafe {
        // First call reports the buffer size it needs.
        let mut bytes: u32 = 0;
        let _ = GetLogicalProcessorInformation(None, &mut bytes);
        if bytes == 0 {
            return None;
        }

        let entry = std::mem::size_of::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION>() as u32;
        let count = (bytes / entry) as usize;
        let mut buf: Vec<SYSTEM_LOGICAL_PROCESSOR_INFORMATION> = vec![std::mem::zeroed(); count];

        GetLogicalProcessorInformation(Some(buf.as_mut_ptr()), &mut bytes).ok()?;

        let cores = buf
            .iter()
            .take(count)
            .filter(|i| i.Relationship == RelationProcessorCore)
            .count();

        (cores > 0).then_some(cores)
    }
}

#[cfg(not(target_os = "windows"))]
fn physical_cores() -> Option<usize> {
    None
}

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

    /// The shared models directory. The Moonshine engine keeps its bundles in sub-directories
    /// of this same folder, so the facade hands it this path.
    #[cfg_attr(not(feature = "moonshine"), allow(dead_code))]
    pub fn models_dir(&self) -> &std::path::Path {
        &self.models_dir
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

        params.set_n_threads(worker_threads() as i32);

        // Temperature fallback is deliberately left ON. Disabling it looked like a 12% win
        // when measured immediately after a full rebuild, but that was thermal throttling:
        // on an idle machine, fallback OFF is consistently *slower* (~31s vs ~26s over
        // three runs each, 64.65s clip). Benchmark this project on an idle CPU or the
        // numbers are fiction.

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
        // Pick the recommendation for *this* machine: the most accurate model that still
        // finishes faster than the user spoke. On a CPU-only laptop that is base.en; with a
        // GPU backend compiled in, the turbo tiers come into range.
        let best = MODELS
            .iter()
            .filter(|(_, _, _, _, _, _, cost)| realtime_factor(*cost) <= 1.0)
            .max_by_key(|(_, _, _, _, _, accuracy, _)| *accuracy)
            .map(|(id, ..)| *id)
            .unwrap_or("tiny.en");

        MODELS
            .iter()
            .map(|(id, engine, size, multilingual, speed, accuracy, cpu_cost)| {
                let path = self.get_model_path(id);
                let downloaded = path.exists();
                let factor = realtime_factor(*cpu_cost);

                // Concrete numbers, not adjectives: what a real 30-second dictation costs
                // on this machine. A user who reached for the most accurate model waited
                // ~9 minutes with the fan screaming; the warning has to make that unmissable
                // *before* the download, not after.
                let wait_30s = 30.0 * factor;
                let warning = if factor > 2.0 {
                    Some(format!(
                        "Too slow on this computer — about {factor:.0}× the length of what you \
                         say. 30 seconds of speech would take ~{:.0} minutes to process, with \
                         every CPU core loaded the whole time. Live processing is turned off \
                         for models this slow, so you wait for the whole recording at the end.",
                        wait_30s / 60.0
                    ))
                } else if factor > 1.0 {
                    Some(format!(
                        "Slower than real time here — about {factor:.1}×, so 30 seconds of \
                         speech means roughly a {wait_30s:.0}-second wait after you stop."
                    ))
                } else {
                    None
                };

                ModelInfo {
                    id: id.to_string(),
                    name: id.to_string(),
                    engine: *engine,
                    size_mb: *size,
                    multilingual: *multilingual,
                    speed: *speed,
                    accuracy: *accuracy,
                    recommended: *id == best,
                    downloaded,
                    path: downloaded.then_some(path),
                    realtime_factor: factor,
                    warning,
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

#[cfg(test)]
mod thread_tests {
    use super::*;

    /// Guards the SMT mistake: asking for logical cores on an 8C/16T laptop meant 16 threads
    /// fighting over 8 sets of vector units — more heat, no more throughput.
    #[test]
    fn worker_threads_is_sane_and_not_logical_count() {
        let t = worker_threads();
        assert!(t >= 1 && t <= 16, "thread count out of range: {t}");

        if let (Some(phys), Ok(logical)) = (physical_cores(), std::thread::available_parallelism())
        {
            assert_eq!(t, phys.clamp(1, 16), "should use physical cores");
            if phys < logical.get() {
                assert!(
                    t < logical.get(),
                    "SMT machine: {t} threads should be below the {} logical cores",
                    logical.get()
                );
            }
        }
    }
}

#[cfg(test)]
mod model_selection_tests {
    use super::*;

    fn models() -> Vec<ModelInfo> {
        WhisperASR::new().unwrap().list_available_models()
    }

    /// The bug this guards: `large-v3` shipped with a static `recommended`-adjacent 5/5
    /// accuracy badge and no warning, so a CPU-only laptop was invited to run a model that
    /// measured 27x the length of the speech — 7.9 minutes for 17 seconds of audio.
    #[test]
    fn unusable_models_carry_a_warning() {
        for m in models() {
            if m.realtime_factor > 1.0 {
                assert!(
                    m.warning.is_some(),
                    "{} runs at {:.1}x here but warns the user about nothing",
                    m.id,
                    m.realtime_factor
                );
            }
        }
    }

    /// Exactly one recommendation, and it must actually keep up with speech.
    #[test]
    fn the_recommended_model_is_faster_than_speaking() {
        let all = models();
        let rec: Vec<_> = all.iter().filter(|m| m.recommended).collect();
        assert_eq!(rec.len(), 1, "expected exactly one recommended model");

        let r = rec[0];
        assert!(
            r.realtime_factor <= 1.0,
            "recommended {} runs at {:.1}x — slower than speaking",
            r.id,
            r.realtime_factor
        );
        assert!(r.warning.is_none(), "recommended model should not carry a warning");
    }

    /// Without a GPU backend compiled in, the heavy tiers must never be the recommendation.
    #[test]
    fn cpu_only_does_not_recommend_a_large_model() {
        if gpu_enabled() {
            return;
        }
        let rec = models().into_iter().find(|m| m.recommended).unwrap();
        assert!(
            !rec.id.starts_with("large"),
            "CPU-only build recommended {}, which cannot keep up",
            rec.id
        );
    }
}
