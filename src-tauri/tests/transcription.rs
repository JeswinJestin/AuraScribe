//! End-to-end check of the real ASR path: load a Whisper model, transcribe a WAV,
//! and confirm recognizable words come back.
//!
//! Ignored by default because it needs a downloaded model and a sample file. Run with:
//!
//! ```text
//! cargo test --test transcription -- --ignored --nocapture
//! ```
//!
//! Set `AURASCRIBE_TEST_WAV` to a 16 kHz mono 16-bit WAV, and optionally
//! `AURASCRIBE_TEST_MODEL` to a model id (defaults to `base.en`).

use std::path::PathBuf;

fn models_dir() -> PathBuf {
    dirs::data_local_dir()
        .expect("local data dir")
        .join("AuraScribe")
        .join("models")
}

/// Minimal 16-bit PCM WAV reader — enough for the fixture this test uses.
fn read_wav_mono_16k(path: &PathBuf) -> Vec<f32> {
    let bytes = std::fs::read(path).expect("read wav");
    assert_eq!(&bytes[0..4], b"RIFF", "not a RIFF file");
    assert_eq!(&bytes[8..12], b"WAVE", "not a WAVE file");

    let mut pos = 12;
    let mut data: Option<&[u8]> = None;
    let mut channels = 1u16;
    let mut sample_rate = 16000u32;

    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body_start = pos + 8;
        let body_end = (body_start + size).min(bytes.len());

        match id {
            b"fmt " => {
                channels = u16::from_le_bytes(bytes[body_start + 2..body_start + 4].try_into().unwrap());
                sample_rate =
                    u32::from_le_bytes(bytes[body_start + 4..body_start + 8].try_into().unwrap());
            }
            b"data" => data = Some(&bytes[body_start..body_end]),
            _ => {}
        }
        pos = body_start + size + (size & 1);
    }

    assert_eq!(sample_rate, 16000, "fixture must be 16 kHz");
    let data = data.expect("wav has no data chunk");

    let samples: Vec<f32> = data
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
        .collect();

    if channels > 1 {
        samples
            .chunks(channels as usize)
            .map(|f| f.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        samples
    }
}

#[test]
#[ignore = "requires a downloaded Whisper model and AURASCRIBE_TEST_WAV"]
fn transcribes_known_audio() {
    let wav_path = PathBuf::from(
        std::env::var("AURASCRIBE_TEST_WAV").expect("set AURASCRIBE_TEST_WAV to a 16kHz mono wav"),
    );
    let model_id = std::env::var("AURASCRIBE_TEST_MODEL").unwrap_or_else(|_| "base.en".into());
    let model_path = models_dir().join(format!("ggml-{model_id}.bin"));
    assert!(model_path.exists(), "model not downloaded: {}", model_path.display());

    let audio = read_wav_mono_16k(&wav_path);
    println!("audio: {} samples ({:.2}s)", audio.len(), audio.len() as f32 / 16000.0);

    let load_start = std::time::Instant::now();
    let mut ctx_params = whisper_rs::WhisperContextParameters::default();
    ctx_params.use_gpu(true);
    let ctx = whisper_rs::WhisperContext::new_with_params(&model_path, ctx_params)
        .expect("load model");
    println!("model load: {:?}", load_start.elapsed());

    let transcribe_start = std::time::Instant::now();
    let mut state = ctx.create_state().expect("create state");
    let mut params =
        whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_realtime(false);
    params.set_print_progress(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);
    params.set_translate(false);
    params.set_no_context(true);
    params.set_language(Some("en"));

    // Must mirror asr.rs::transcribe, or this benchmark measures something the app never
    // runs. It previously left n_threads at whisper.cpp's default of 4 while the app used
    // every core, so the numbers it produced were not the app's numbers.
    let threads = std::env::var("AURASCRIBE_TEST_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(16)
        });
    params.set_n_threads(threads as i32);

    // Temperature fallback re-runs the decoder for a window when whisper.cpp decides the
    // decode looked wrong. Off by default here so the two configurations are comparable.
    let fallback = std::env::var("AURASCRIBE_TEST_FALLBACK").is_ok();
    if !fallback {
        params.set_temperature_inc(0.0);
    }
    println!("threads: {threads}, temperature fallback: {fallback}");

    state.full(params, &audio).expect("transcribe");

    let n = state.full_n_segments();
    let mut text = String::new();
    for i in 0..n {
        if let Some(segment) = state.get_segment(i) {
            if let Ok(s) = segment.to_str_lossy() {
                text.push_str(&s);
            }
        }
    }
    let elapsed = transcribe_start.elapsed();

    println!("transcribe: {elapsed:?}");
    println!("RAW   : {}", text.trim());

    let lower = text.to_lowercase();
    assert!(
        lower.contains("quick brown fox") || lower.contains("lazy dog"),
        "transcript did not contain expected phrase: {text:?}"
    );
}
