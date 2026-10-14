//! Notetaker — local meeting transcription.
//!
//! Records **the meeting you hear** (system output, via WASAPI loopback) **plus your microphone**,
//! transcribes the mix **entirely on-device** with the model already loaded for dictation, and
//! stores a timestamped transcript. **No cloud call, no account, no bundled summary model** — the
//! whole point of AuraScribe. Design + scope: `docs/superpowers/specs/2026-10-13-notetaker-design.md`.
//!
//! ## Isolation
//! This module never touches the dictation hot path (`commands::start_recording` / `run_chunker`).
//! Meetings have their own [`MeetingState`], their own capture threads, and their own chunker, so a
//! bug here cannot regress daily dictation.
//!
//! ## Platform honesty
//! System-audio loopback is **Windows-only** in this MVP (WASAPI). macOS has no public loopback API
//! without a virtual device; Linux needs a PulseAudio/PipeWire monitor source. Off Windows,
//! [`start_meeting`] returns an explicit error rather than silently capturing nothing — the
//! "don't fake success on unimplemented platforms" rule.
//!
//! ## Verification status
//! The orchestration, mixing, and DB writes are ordinary Rust (unit-tested where pure). The WASAPI
//! loopback COM path **cannot be exercised from CI or a headless sandbox** — it needs a real audio
//! session. It compiles; its runtime correctness is confirmed by the owner running a real call and
//! checking `aurascribe.log` (which logs the detected loopback format/rate). Treat it as unverified
//! until that run, exactly as the macOS/Linux dictation audio paths are labelled.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// How often the meeting chunker drains the captured audio and transcribes it. Short enough that
/// the live transcript feels real-time (a segment appears every few seconds), long enough that each
/// window has a usable utterance. Moonshine (the recommended engine) is short-form optimized, so a
/// few seconds per window costs little accuracy.
const CHUNK_SECS: u64 = 6;

/// Target rate for ASR (same 16 kHz mono the dictation path feeds Whisper/sherpa).
const ASR_RATE: u32 = 16_000;

/// All state for an in-progress (or idle) meeting capture. Separate from the dictation state.
#[derive(Clone)]
pub struct MeetingState {
    /// True while a meeting is being captured.
    pub recording: Arc<AtomicBool>,
    /// The DB id of the meeting currently (or last) being recorded.
    pub meeting_id: Arc<Mutex<Option<i64>>>,
    /// When the current meeting started, for elapsed/offset computation.
    pub started: Arc<Mutex<Option<std::time::Instant>>>,
    /// Native OS capture threads (loopback + mic). Joined on stop.
    pub threads: Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>,
    /// The async chunker; awaited on stop so the tail is flushed.
    pub chunk_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// System-output (loopback) audio, mono, at `system_rate`.
    pub system_buf: Arc<Mutex<Vec<f32>>>,
    pub system_rate: Arc<Mutex<u32>>,
    /// Microphone audio, mono, at `mic_rate`.
    pub mic_buf: Arc<Mutex<Vec<f32>>>,
    pub mic_rate: Arc<Mutex<u32>>,
}

impl Default for MeetingState {
    fn default() -> Self {
        Self {
            recording: Arc::new(AtomicBool::new(false)),
            meeting_id: Arc::new(Mutex::new(None)),
            started: Arc::new(Mutex::new(None)),
            threads: Arc::new(Mutex::new(Vec::new())),
            chunk_task: Arc::new(Mutex::new(None)),
            system_buf: Arc::new(Mutex::new(Vec::new())),
            system_rate: Arc::new(Mutex::new(0)),
            mic_buf: Arc::new(Mutex::new(Vec::new())),
            mic_rate: Arc::new(Mutex::new(0)),
        }
    }
}

// --- Pure helpers (unit-tested) ------------------------------------------------------------------

/// Down-mix interleaved multi-channel samples to mono by averaging each frame.
pub fn downmix_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Mix two mono streams (system + mic) into one, summing sample-by-sample and soft-clamping to
/// [-1, 1]. The shorter stream is treated as silence past its end, so one source being quiet (or
/// absent) never truncates the other. Both inputs must already be at the same sample rate.
pub fn mix_streams(a: &[f32], b: &[f32]) -> Vec<f32> {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let s = a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0);
        out.push(s.clamp(-1.0, 1.0));
    }
    out
}

/// Resample a mono stream to 16 kHz for ASR, guarding the empty / unknown-rate cases so a source
/// that never produced audio contributes nothing instead of panicking.
fn to_asr_rate(mono: &[f32], rate: u32) -> Vec<f32> {
    if mono.is_empty() || rate == 0 {
        return Vec::new();
    }
    crate::audio::resample_linear(mono, rate, ASR_RATE)
}

// --- Capture: microphone (reuses the proven cpal path) -------------------------------------------

/// Capture the default (or configured) microphone into `buf` as mono f32 at the device's native
/// rate, until `recording` goes false. Mirrors `commands::start_recording`'s format-agnostic,
/// no-drop capture, but writes into the meeting's own buffer.
fn capture_mic(
    buf: Arc<Mutex<Vec<f32>>>,
    rate_out: Arc<Mutex<u32>>,
    recording: Arc<AtomicBool>,
    mic_device: Option<String>,
) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = mic_device
        .as_deref()
        .and_then(|name| {
            host.input_devices()
                .ok()?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
        })
        .or_else(|| host.default_input_device());
    let device = match device {
        Some(d) => d,
        None => {
            tracing::warn!("Meeting: no input device for mic capture");
            return;
        }
    };
    let config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Meeting: failed to get mic config: {}", e);
            return;
        }
    };
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;
    let sample_format = config.sample_format();
    if let Ok(mut r) = rate_out.try_lock() {
        *r = sample_rate;
    }
    tracing::info!(
        "Meeting mic capture: format={:?}, {}Hz, {}ch",
        sample_format, sample_rate, channels
    );
    let stream_config: cpal::StreamConfig = config.into();

    fn build<T>(
        device: &cpal::Device,
        cfg: &cpal::StreamConfig,
        buf: Arc<Mutex<Vec<f32>>>,
        channels: usize,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        T: cpal::SizedSample,
        f32: cpal::FromSample<T>,
    {
        use cpal::traits::DeviceTrait;
        use cpal::Sample;
        let mut local: Vec<f32> = Vec::new();
        device.build_input_stream(
            cfg,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if channels > 1 {
                    for frame in data.chunks(channels) {
                        let sum: f32 = frame.iter().map(|s| f32::from_sample(*s)).sum();
                        local.push(sum / channels as f32);
                    }
                } else {
                    local.extend(data.iter().map(|s| f32::from_sample(*s)));
                }
                if let Ok(mut b) = buf.try_lock() {
                    b.append(&mut local);
                }
            },
            |err| tracing::warn!("Meeting mic stream error: {}", err),
            None,
        )
    }

    let stream = match sample_format {
        cpal::SampleFormat::F32 => build::<f32>(&device, &stream_config, buf, channels),
        cpal::SampleFormat::I16 => build::<i16>(&device, &stream_config, buf, channels),
        cpal::SampleFormat::U16 => build::<u16>(&device, &stream_config, buf, channels),
        other => {
            tracing::warn!("Meeting mic: unsupported sample format {:?}", other);
            return;
        }
    };
    let stream = match stream {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Meeting mic: failed to build stream: {}", e);
            return;
        }
    };
    if stream.play().is_err() {
        tracing::warn!("Meeting mic: failed to start stream");
        return;
    }
    while recording.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

// --- Capture: system output (WASAPI loopback, Windows only) --------------------------------------

/// Capture the default render endpoint's output (what the user hears) via WASAPI loopback into
/// `buf` as mono f32 at the endpoint's mix rate, until `recording` goes false.
///
/// **Unverified from the sandbox** — this is COM audio code that needs a real audio session. It
/// compiles and follows the documented WASAPI loopback sequence; the owner confirms it on a real
/// call (the detected format/rate is logged). A failure here is logged and leaves the meeting with
/// mic-only audio rather than crashing.
#[cfg(target_os = "windows")]
fn capture_system_loopback(
    buf: Arc<Mutex<Vec<f32>>>,
    rate_out: Arc<Mutex<u32>>,
    recording: Arc<AtomicBool>,
) {
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
        AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
        WAVEFORMATEXTENSIBLE,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
        COINIT_MULTITHREADED,
    };

    const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
    const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
    // ~200 ms buffer, in 100-ns units (REFERENCE_TIME).
    const BUFFER_HNS: i64 = 2_000_000;

    unsafe {
        // The capture runs on its own thread; initialize COM for it (MTA so the loopback service
        // is callable from this worker). Ignore RPC_E_CHANGED_MODE if COM is already up.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let result = (|| -> Result<(), String> {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| format!("CoCreateInstance(MMDeviceEnumerator) failed: {e}"))?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| format!("GetDefaultAudioEndpoint(render) failed: {e}"))?;
            let audio_client: IAudioClient = device
                .Activate(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate(IAudioClient) failed: {e}"))?;

            let pwfx = audio_client
                .GetMixFormat()
                .map_err(|e| format!("GetMixFormat failed: {e}"))?;
            if pwfx.is_null() {
                return Err("GetMixFormat returned null".into());
            }
            let wfx = *pwfx;
            let channels = wfx.nChannels as usize;
            let rate = wfx.nSamplesPerSec;
            let bits = wfx.wBitsPerSample;

            // Determine whether the mix format delivers 32-bit float (the usual case) or 16-bit PCM.
            // For WAVE_FORMAT_EXTENSIBLE, the real format is in the SubFormat GUID, whose Data1 is
            // the classic format tag (1 = PCM, 3 = IEEE float) — so we can read it without importing
            // the KSDATAFORMAT_SUBTYPE_* constants.
            let is_float = match wfx.wFormatTag {
                WAVE_FORMAT_IEEE_FLOAT => true,
                WAVE_FORMAT_EXTENSIBLE => {
                    let ext = pwfx as *const WAVEFORMATEXTENSIBLE;
                    (*ext).SubFormat.data1 == 3
                }
                _ => false, // treated as PCM below
            };

            audio_client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK,
                    BUFFER_HNS,
                    0,
                    pwfx,
                    None,
                )
                .map_err(|e| format!("IAudioClient::Initialize(loopback) failed: {e}"))?;

            let capture: IAudioCaptureClient = audio_client
                .GetService()
                .map_err(|e| format!("GetService(IAudioCaptureClient) failed: {e}"))?;

            if let Ok(mut r) = rate_out.try_lock() {
                *r = rate;
            }
            tracing::info!(
                "Meeting loopback capture: {}Hz, {}ch, {}bit, float={}",
                rate, channels, bits, is_float
            );

            audio_client
                .Start()
                .map_err(|e| format!("IAudioClient::Start failed: {e}"))?;

            let bytes_per_sample = (bits / 8) as usize;
            let mut local: Vec<f32> = Vec::new();

            while recording.load(Ordering::Relaxed) {
                // WASAPI delivers in packets; drain all available, then wait a slice of the buffer.
                loop {
                    let packet = capture
                        .GetNextPacketSize()
                        .map_err(|e| format!("GetNextPacketSize failed: {e}"))?;
                    if packet == 0 {
                        break;
                    }
                    let mut p_data: *mut u8 = std::ptr::null_mut();
                    let mut num_frames: u32 = 0;
                    let mut flags: u32 = 0;
                    capture
                        .GetBuffer(&mut p_data, &mut num_frames, &mut flags, None, None)
                        .map_err(|e| format!("GetBuffer failed: {e}"))?;

                    let frames = num_frames as usize;
                    let silent = (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0;
                    if silent || p_data.is_null() {
                        // A silent packet still advances time; push zeros so mic/system stay aligned.
                        local.extend(std::iter::repeat(0.0).take(frames));
                    } else {
                        let total = frames * channels;
                        if is_float && bytes_per_sample == 4 {
                            let samples = std::slice::from_raw_parts(p_data as *const f32, total);
                            local.extend(downmix_mono(samples, channels));
                        } else if bytes_per_sample == 2 {
                            let samples = std::slice::from_raw_parts(p_data as *const i16, total);
                            let as_f32: Vec<f32> =
                                samples.iter().map(|s| *s as f32 / 32768.0).collect();
                            local.extend(downmix_mono(&as_f32, channels));
                        } else {
                            // Unexpected format: advance time with silence rather than garbage.
                            local.extend(std::iter::repeat(0.0).take(frames));
                        }
                    }

                    capture
                        .ReleaseBuffer(num_frames)
                        .map_err(|e| format!("ReleaseBuffer failed: {e}"))?;
                }

                if !local.is_empty() {
                    if let Ok(mut b) = buf.try_lock() {
                        b.append(&mut local);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }

            let _ = audio_client.Stop();
            CoTaskMemFree(Some(pwfx as *const _ as *mut _));
            Ok(())
        })();

        if let Err(e) = result {
            tracing::warn!("Meeting loopback capture failed ({e}); meeting will have mic audio only");
        }
        CoUninitialize();
    }
}

#[cfg(not(target_os = "windows"))]
fn capture_system_loopback(
    _buf: Arc<Mutex<Vec<f32>>>,
    _rate_out: Arc<Mutex<u32>>,
    _recording: Arc<AtomicBool>,
) {
    // System-audio loopback is Windows-only in this MVP; `start_meeting` refuses off Windows, so
    // this is never reached. Present so the module compiles on all platforms.
}

// --- Orchestration -------------------------------------------------------------------------------

/// Transcribe one 16 kHz mono window with the loaded model, returning trimmed text (or None).
fn transcribe_window(asr: &crate::engine::Asr, samples: &[f32], language: &str) -> Option<String> {
    let trimmed = crate::chunking::trim_silence(samples, ASR_RATE);
    if trimmed.is_empty() {
        return None;
    }
    let trimmed = crate::chunking::normalize_gain(&trimmed);
    let lang = if language == "auto" { None } else { Some(language) };
    match asr.transcribe(&trimmed, lang) {
        Ok(t) if !t.trim().is_empty() => Some(t.trim().to_string()),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!("Meeting chunk transcription failed: {}", e);
            None
        }
    }
}

/// One drain-and-transcribe tick: take everything captured so far from both sources, resample,
/// mix, transcribe, and append a segment. Returns the segment text if any was produced.
async fn process_tick(st: &MeetingState, asr: &Arc<crate::engine::Asr>, language: &str) -> Option<String> {
    let (sys, sys_rate, mic, mic_rate) = {
        let mut sb = st.system_buf.lock().await;
        let mut mb = st.mic_buf.lock().await;
        let sr = *st.system_rate.lock().await;
        let mr = *st.mic_rate.lock().await;
        (std::mem::take(&mut *sb), sr, std::mem::take(&mut *mb), mr)
    };
    if sys.is_empty() && mic.is_empty() {
        return None;
    }
    let sys16 = to_asr_rate(&sys, sys_rate);
    let mic16 = to_asr_rate(&mic, mic_rate);
    let mixed = mix_streams(&sys16, &mic16);
    if mixed.is_empty() {
        return None;
    }
    let asr = asr.clone();
    let lang = language.to_string();
    tokio::task::spawn_blocking(move || transcribe_window(&asr, &mixed, &lang))
        .await
        .ok()
        .flatten()
}

// --- Tauri commands ------------------------------------------------------------------------------

use tauri::{command, AppHandle, Emitter, State};

/// Returned by `meeting_status` so the UI can poll authoritative state (events are the fast path,
/// not the only path — the CLAUDE.md rule).
#[derive(serde::Serialize)]
pub struct MeetingStatus {
    pub recording: bool,
    pub meeting_id: Option<i64>,
    pub elapsed_ms: i64,
}

/// Emitted (`meeting-segment`) each time a window is transcribed during capture, so the UI can grow
/// the transcript live instead of only after Stop.
#[derive(Clone, serde::Serialize)]
pub struct MeetingSegmentEvent {
    pub meeting_id: i64,
    pub offset_ms: i64,
    pub text: String,
}

/// Emitted (`meeting-finished`) after Stop once the meeting is finalized and (maybe) auto-renamed,
/// so the UI refreshes its title in the list.
#[derive(Clone, serde::Serialize)]
pub struct MeetingFinishedEvent {
    pub meeting_id: i64,
    pub title: String,
}

/// Derive a human title from the first real speech in a meeting, **locally, with no model** — the
/// first sentence (or first ~8 words), tidied and capped. Returns `None` when there is no usable
/// text (keeps the default "Untitled meeting"). A smarter, LLM-generated title is a later opt-in.
pub fn derive_title(segment_texts: &[String]) -> Option<String> {
    let joined = segment_texts.join(" ");
    let trimmed = joined.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Prefer the first sentence; fall back to the first chunk of words.
    let first = trimmed
        .split(|c| c == '.' || c == '?' || c == '!')
        .map(str::trim)
        .find(|s| !s.is_empty())
        .unwrap_or(trimmed);
    let words: Vec<&str> = first.split_whitespace().take(8).collect();
    if words.is_empty() {
        return None;
    }
    let mut title = words.join(" ");
    // Cap length so the list stays tidy; trim trailing punctuation/commas.
    const MAX: usize = 60;
    if title.chars().count() > MAX {
        title = title.chars().take(MAX).collect::<String>().trim_end().to_string();
        title.push('…');
    }
    let title = title.trim_end_matches([',', ';', ':', '-']).trim().to_string();
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

/// A meeting plus its transcript segments, for the detail view.
#[derive(serde::Serialize)]
pub struct MeetingDetail {
    pub meeting: crate::db::MeetingRow,
    pub segments: Vec<crate::db::MeetingSegmentRow>,
}

/// Start capturing + transcribing a meeting. System-audio loopback is Windows-only in this MVP.
#[command]
pub async fn start_meeting(
    state: State<'_, crate::app_state::AppState>,
    app: AppHandle,
    title: Option<String>,
) -> Result<i64, String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (&state, &title, &app);
        return Err(
            "Meeting capture records system audio (what you hear in the call), which is only \
             supported on Windows right now. It is coming to macOS/Linux later."
                .to_string(),
        );
    }

    #[cfg(target_os = "windows")]
    {
        {
            let status = state.status.lock().await;
            if !status.is_model_loaded {
                return Err("No voice model loaded. Load one in Settings first.".to_string());
            }
        }
        if state.meeting.recording.load(Ordering::Relaxed) {
            return Err("A meeting is already being recorded.".to_string());
        }

        let (mic_device, language) = {
            let db = state.db.lock().await;
            crate::commands::load_settings_from_db(&db)
                .await
                .map(|s| (s.mic_device, s.language))
                .unwrap_or((None, "en".to_string()))
        };

        let title = title.unwrap_or_else(|| "Untitled meeting".to_string());
        let meeting_id = {
            let db = state.db.lock().await;
            db.create_meeting(&title, "system+mic")
                .await
                .map_err(|e| format!("Could not create meeting: {e}"))?
        };

        // Reset buffers/state for a fresh capture.
        state.meeting.system_buf.lock().await.clear();
        state.meeting.mic_buf.lock().await.clear();
        *state.meeting.system_rate.lock().await = 0;
        *state.meeting.mic_rate.lock().await = 0;
        *state.meeting.meeting_id.lock().await = Some(meeting_id);
        let start_instant = std::time::Instant::now();
        *state.meeting.started.lock().await = Some(start_instant);
        state.meeting.recording.store(true, Ordering::Relaxed);

        // Loopback (system output) capture thread.
        {
            let buf = state.meeting.system_buf.clone();
            let rate = state.meeting.system_rate.clone();
            let rec = state.meeting.recording.clone();
            let h = std::thread::spawn(move || capture_system_loopback(buf, rate, rec));
            state.meeting.threads.lock().await.push(h);
        }
        // Microphone capture thread.
        {
            let buf = state.meeting.mic_buf.clone();
            let rate = state.meeting.mic_rate.clone();
            let rec = state.meeting.recording.clone();
            let h = std::thread::spawn(move || capture_mic(buf, rate, rec, mic_device));
            state.meeting.threads.lock().await.push(h);
        }

        // Chunker: drain + transcribe every CHUNK_SECS, flushing the tail on stop. Each saved
        // segment is also emitted so the UI can show the transcript building up live.
        let st = state.meeting.clone();
        let asr = state.asr.clone();
        let db = state.db.clone();
        let app = app.clone();
        let task = tokio::spawn(async move {
            loop {
                for _ in 0..(CHUNK_SECS * 10) {
                    if !st.recording.load(Ordering::Relaxed) {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                let still = st.recording.load(Ordering::Relaxed);
                if let Some(text) = process_tick(&st, &asr, &language).await {
                    let offset_ms = start_instant.elapsed().as_millis() as i64;
                    {
                        let db = db.lock().await;
                        if let Err(e) = db.add_meeting_segment(meeting_id, offset_ms, &text).await {
                            tracing::warn!("Meeting: failed to save segment: {}", e);
                        }
                    }
                    // Live update for the Notetaker view (events = the fast path).
                    app.emit(
                        "meeting-segment",
                        MeetingSegmentEvent { meeting_id, offset_ms, text },
                    )
                    .ok();
                }
                if !still {
                    break;
                }
            }
            tracing::info!("Meeting chunker finished (meeting {})", meeting_id);
        });
        *state.meeting.chunk_task.lock().await = Some(task);

        tracing::info!("Meeting started (id {})", meeting_id);
        Ok(meeting_id)
    }
}

/// Stop capturing, flush the tail, and finalize the meeting row.
#[command]
pub async fn stop_meeting(
    state: State<'_, crate::app_state::AppState>,
    app: AppHandle,
) -> Result<(), String> {
    if !state.meeting.recording.load(Ordering::Relaxed) {
        return Ok(());
    }
    state.meeting.recording.store(false, Ordering::Relaxed);

    // Join native capture threads off the async runtime.
    let threads: Vec<_> = std::mem::take(&mut *state.meeting.threads.lock().await);
    let _ = tokio::task::spawn_blocking(move || {
        for h in threads {
            let _ = h.join();
        }
    })
    .await;

    // Await the chunker so the final window is transcribed and saved.
    let task = state.meeting.chunk_task.lock().await.take();
    if let Some(t) = task {
        let _ = t.await;
    }

    let meeting_id = *state.meeting.meeting_id.lock().await;
    let duration_ms = state
        .meeting
        .started
        .lock()
        .await
        .map(|s| s.elapsed().as_millis() as i64)
        .unwrap_or(0);
    if let Some(id) = meeting_id {
        let db = state.db.lock().await;
        db.finish_meeting(id, duration_ms)
            .await
            .map_err(|e| format!("Could not finalize meeting: {e}"))?;

        // Auto-rename from the transcript, locally, if the user never set a title. Keeps the
        // meeting list readable without any model or network call.
        let mut final_title = db
            .get_meeting(id)
            .await
            .ok()
            .flatten()
            .map(|m| m.title)
            .unwrap_or_else(|| "Untitled meeting".to_string());
        if final_title == "Untitled meeting" {
            let segs = db.list_meeting_segments(id).await.unwrap_or_default();
            let texts: Vec<String> = segs.into_iter().map(|s| s.text).collect();
            if let Some(title) = derive_title(&texts) {
                if db.rename_meeting(id, &title).await.is_ok() {
                    final_title = title;
                }
            }
        }
        app.emit(
            "meeting-finished",
            MeetingFinishedEvent { meeting_id: id, title: final_title },
        )
        .ok();
    }
    tracing::info!("Meeting stopped");
    Ok(())
}

#[command]
pub async fn meeting_status(
    state: State<'_, crate::app_state::AppState>,
) -> Result<MeetingStatus, String> {
    let recording = state.meeting.recording.load(Ordering::Relaxed);
    let meeting_id = *state.meeting.meeting_id.lock().await;
    let elapsed_ms = if recording {
        state
            .meeting
            .started
            .lock()
            .await
            .map(|s| s.elapsed().as_millis() as i64)
            .unwrap_or(0)
    } else {
        0
    };
    Ok(MeetingStatus { recording, meeting_id, elapsed_ms })
}

#[command]
pub async fn list_meetings(
    state: State<'_, crate::app_state::AppState>,
) -> Result<Vec<crate::db::MeetingRow>, String> {
    let db = state.db.lock().await;
    db.list_meetings().await.map_err(|e| e.to_string())
}

#[command]
pub async fn get_meeting(
    state: State<'_, crate::app_state::AppState>,
    id: i64,
) -> Result<MeetingDetail, String> {
    let db = state.db.lock().await;
    let meeting = db
        .get_meeting(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Meeting not found".to_string())?;
    let segments = db.list_meeting_segments(id).await.map_err(|e| e.to_string())?;
    Ok(MeetingDetail { meeting, segments })
}

#[command]
pub async fn rename_meeting(
    state: State<'_, crate::app_state::AppState>,
    id: i64,
    title: String,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.rename_meeting(id, &title).await.map_err(|e| e.to_string())
}

#[command]
pub async fn delete_meeting(
    state: State<'_, crate::app_state::AppState>,
    id: i64,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_meeting(id).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_averages_channels() {
        let stereo = [1.0, 0.0, 0.5, 0.5];
        assert_eq!(downmix_mono(&stereo, 2), vec![0.5, 0.5]);
        assert_eq!(downmix_mono(&[0.1, 0.2], 1), vec![0.1, 0.2]);
    }

    #[test]
    fn mix_sums_and_clamps() {
        let a = [0.5, 0.9, -0.9];
        let b = [0.5, 0.9, -0.9];
        assert_eq!(mix_streams(&a, &b), vec![1.0, 1.0, -1.0]);
    }

    #[test]
    fn mix_pads_shorter_stream_with_silence() {
        let a = [0.2, 0.2, 0.2];
        let b = [0.1];
        assert_eq!(mix_streams(&a, &b), vec![0.3, 0.2, 0.2]);
        assert_eq!(mix_streams(&[], &a), vec![0.2, 0.2, 0.2]);
    }

    #[test]
    fn to_asr_rate_guards_empty_and_zero_rate() {
        assert!(to_asr_rate(&[], 48000).is_empty());
        assert!(to_asr_rate(&[0.1, 0.2], 0).is_empty());
    }

    #[test]
    fn derive_title_uses_first_sentence() {
        let segs = vec![
            "Okay let us kick off the design sync.".to_string(),
            "First item is the layout.".to_string(),
        ];
        assert_eq!(derive_title(&segs).as_deref(), Some("Okay let us kick off the design sync"));
    }

    #[test]
    fn derive_title_caps_to_eight_words() {
        let segs = vec!["one two three four five six seven eight nine ten".to_string()];
        assert_eq!(derive_title(&segs).as_deref(), Some("one two three four five six seven eight"));
    }

    #[test]
    fn derive_title_none_on_empty() {
        assert!(derive_title(&[]).is_none());
        assert!(derive_title(&["   ".to_string()]).is_none());
    }
}
