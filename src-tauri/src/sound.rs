//! Short audible cues for dictation start and stop.
//!
//! The cue has to come from the backend, not the UI: the hotkey fires when no window is open,
//! so there is no page to play a sound. It uses `cpal` — already a dependency for audio
//! capture, so no new weight — to synthesize a soft sine tone rather than call the harsh
//! Win32 console beep, which the owner explicitly did not want to be irritating.
//!
//! - **Start**: two quick rising notes — "I'm listening."
//! - **Stop**: two quick falling notes — "done, processing."
//!
//! Best-effort. If there is no output device, or the tone can't play, dictation is
//! unaffected — a missing cue is never allowed to break recording.

use std::sync::atomic::{AtomicBool, Ordering};

/// Lets a user turn the cue off. Defaults on. Set from settings at startup and on change.
static ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// A rising two-note chirp. Played the instant recording begins.
pub fn play_start() {
    play(&[(660.0, 70), (880.0, 90)]);
}

/// A falling two-note chirp. Played when recording stops.
pub fn play_stop() {
    play(&[(880.0, 70), (620.0, 90)]);
}

/// Play a sequence of `(frequency_hz, duration_ms)` notes on a detached thread, so the caller
/// never blocks on audio. Silently does nothing when cues are disabled.
fn play(notes: &'static [(f32, u64)]) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(move || {
        if let Err(e) = play_blocking(notes) {
            tracing::debug!("Cue tone skipped: {}", e);
        }
    });
}

fn play_blocking(notes: &[(f32, u64)]) -> Result<(), String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("no output device")?;
    let config = device
        .default_output_config()
        .map_err(|e| e.to_string())?;

    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;

    // Build the whole waveform up front: each note is a sine with a short linear fade in/out
    // so there is no click at the edges. Low amplitude — this is a cue, not an alert.
    const AMPLITUDE: f32 = 0.14;
    let mut samples: Vec<f32> = Vec::new();
    for &(freq, ms) in notes {
        let n = ((ms as f32 / 1000.0) * sample_rate) as usize;
        let fade = (n / 8).max(1);
        for i in 0..n {
            let t = i as f32 / sample_rate;
            let env = if i < fade {
                i as f32 / fade as f32
            } else if i > n - fade {
                (n - i) as f32 / fade as f32
            } else {
                1.0
            };
            samples.push((2.0 * std::f32::consts::PI * freq * t).sin() * AMPLITUDE * env);
        }
    }

    let total = samples.len();
    let played = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let done = std::sync::Arc::new(AtomicBool::new(false));

    let played_cb = played.clone();
    let done_cb = done.clone();
    let stream = device
        .build_output_stream(
            &config.into(),
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in out.chunks_mut(channels) {
                    let idx = played_cb.fetch_add(1, Ordering::Relaxed);
                    let s = samples.get(idx).copied().unwrap_or(0.0);
                    for ch in frame.iter_mut() {
                        *ch = s;
                    }
                }
                if played_cb.load(Ordering::Relaxed) >= total {
                    done_cb.store(true, Ordering::Relaxed);
                }
            },
            |e| tracing::debug!("Cue stream error: {}", e),
            None,
        )
        .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;

    // Wait for the buffer to drain (plus a small tail), then drop the stream.
    let ms: u64 = notes.iter().map(|(_, d)| d).sum();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms + 120);
    while !done.load(Ordering::Relaxed) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(())
}
