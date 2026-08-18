//! Deciding where to cut a live recording so it can be transcribed while the user is still
//! speaking.
//!
//! Transcribing only after the user stops means the wait scales with both how long they
//! spoke and how slow their machine is. A 30-second dictation at 0.5x realtime is a
//! 15-second wait; on a weaker laptop it is worse. Transcribing completed pieces *during*
//! the recording makes the remaining wait roughly one chunk — a second or two — on any
//! hardware. That is the property that makes dictation feel instant, and unlike GPU
//! offload it costs the user nothing and requires no particular machine.
//!
//! The only hard part is where to cut. Splitting mid-word gives Whisper half a word on each
//! side and it mis-transcribes both, so cuts are placed in silence.
//!
//! Chunking hides latency **only for models that keep up with real time.** A model that runs
//! slower than the speech (large-v3 on CPU is ~15x) falls behind while you talk and the
//! backlog is paid back after you stop, so splitting buys nothing there — and because
//! whisper.cpp processes in fixed 30-second windows, many small chunks cost *more* total
//! windows than one pass. The pipeline therefore only chunks when the model is fast enough;
//! see `run_chunker`.

/// Don't cut before this much audio has accumulated. Whisper pads short input up to its
/// 30-second window internally, so very small chunks waste work for no latency gain.
pub const MIN_CHUNK_SECS: f32 = 6.0;

/// Cut by this point even without silence, rather than letting a chunk grow unbounded when
/// someone talks continuously. This bounds the *final* chunk's transcription — the only wait
/// a keeping-up model leaves the user — so it is kept well under whisper's 30s window to
/// keep that tail short.
pub const MAX_CHUNK_SECS: f32 = 15.0;

/// A gap must be at least this long to count as a sentence/phrase boundary. Shorter gaps
/// are the pauses inside normal speech, and cutting there still splits a phrase.
const MIN_SILENCE_MS: f32 = 250.0;

/// Energy is measured over frames this long.
const FRAME_MS: f32 = 20.0;

/// Absolute floor for "silent", so a recording of near-total silence doesn't get split at
/// arbitrary points by the relative threshold alone.
const ABSOLUTE_SILENCE_RMS: f32 = 0.005;

/// Fraction of the clip's own loudness below which a frame counts as silence. Relative so
/// that quiet microphones and loud ones behave the same.
const RELATIVE_SILENCE: f32 = 0.15;

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

/// Where to cut `samples`, or `None` to keep accumulating.
///
/// Returns an index into `samples`. Everything before it is ready to transcribe; everything
/// from it onward stays pending. Prefers the **latest** usable silence so each chunk carries
/// as much context as possible — Whisper is more accurate with more context.
pub fn find_split_point(samples: &[f32], sample_rate: u32) -> Option<usize> {
    if sample_rate == 0 {
        return None;
    }
    let sr = sample_rate as f32;
    let min_samples = (MIN_CHUNK_SECS * sr) as usize;
    let max_samples = (MAX_CHUNK_SECS * sr) as usize;

    if samples.len() < min_samples {
        return None;
    }

    let frame = ((FRAME_MS / 1000.0) * sr) as usize;
    let min_silence_frames = ((MIN_SILENCE_MS / FRAME_MS) as usize).max(1);
    if frame == 0 {
        return None;
    }

    // Only consider cuts at or after min_samples, so no chunk is uselessly small.
    let search_start = min_samples;
    let search_end = samples.len().min(max_samples);

    if search_end > search_start {
        let threshold = (rms(&samples[..search_end]) * RELATIVE_SILENCE).max(ABSOLUTE_SILENCE_RMS);

        // Walk frames in the search window, tracking runs of silence. Take the last
        // qualifying run and cut in its middle, which keeps a little trailing air on the
        // chunk and a little leading air on the remainder.
        let mut run_start: Option<usize> = None;
        let mut best: Option<usize> = None;

        let mut pos = search_start;
        while pos + frame <= search_end {
            let quiet = rms(&samples[pos..pos + frame]) < threshold;
            if quiet {
                run_start.get_or_insert(pos);
            } else if let Some(start) = run_start.take() {
                let frames = (pos - start) / frame;
                if frames >= min_silence_frames {
                    best = Some(start + (pos - start) / 2);
                }
            }
            pos += frame;
        }

        // A run still open at the end of the window counts too.
        if let Some(start) = run_start {
            let frames = (search_end - start) / frame;
            if frames >= min_silence_frames {
                best = Some(start + (search_end - start) / 2);
            }
        }

        if best.is_some() {
            return best;
        }
    }

    // No silence found. Cut anyway once the chunk reaches the ceiling: a word may be split,
    // but letting one chunk grow forever defeats the point of chunking at all.
    if samples.len() >= max_samples {
        return Some(max_samples);
    }

    None
}

/// Internal silence longer than this is collapsed. Kept above natural speech pauses so real
/// sentence rhythm survives; only genuine dead air (thinking, hesitation, room tone) is cut.
const TRIM_SILENCE_MS: f32 = 600.0;

/// What a collapsed silence is replaced with. A little pause preserves the sentence boundary
/// for Whisper without feeding it the full dead air.
const KEPT_PAUSE_MS: f32 = 150.0;

/// Remove dead air before transcription. Whisper is charged for silence the same as speech —
/// it processes fixed 30-second windows regardless of content — so trimming the pauses, the
/// hesitation, and the gap at each end is a real speed win on *every* model and every
/// machine, with no accuracy cost. This is the cheapest lever there is: less audio in, less
/// compute, less heat.
///
/// Returns the input unchanged when there is nothing worth trimming.
pub fn trim_silence(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if sample_rate == 0 || samples.is_empty() {
        return samples.to_vec();
    }
    let sr = sample_rate as f32;
    let frame = ((FRAME_MS / 1000.0) * sr) as usize;
    if frame == 0 {
        return samples.to_vec();
    }

    let threshold = (rms(samples) * RELATIVE_SILENCE).max(ABSOLUTE_SILENCE_RMS);
    let trim_frames = ((TRIM_SILENCE_MS / FRAME_MS) as usize).max(1);
    let kept = ((KEPT_PAUSE_MS / 1000.0) * sr) as usize;

    // Classify each frame as loud/quiet, then walk the clip copying speech and collapsing any
    // quiet run longer than the threshold down to a short pause. Leading and trailing quiet
    // collapse to nothing.
    let mut out: Vec<f32> = Vec::with_capacity(samples.len());
    let mut run_start: Option<usize> = None;
    let mut seen_speech = false;

    let mut pos = 0;
    while pos < samples.len() {
        let end = (pos + frame).min(samples.len());
        let quiet = rms(&samples[pos..end]) < threshold;

        if quiet {
            run_start.get_or_insert(pos);
        } else {
            if let Some(start) = run_start.take() {
                let len = pos - start;
                // A leading silence (before any speech) is dropped entirely; an internal one
                // longer than the limit collapses to a short pause; a short one is kept as-is.
                if seen_speech {
                    if len > trim_frames * frame {
                        out.extend(std::iter::repeat(0.0).take(kept.min(len)));
                    } else {
                        out.extend_from_slice(&samples[start..pos]);
                    }
                }
            }
            out.extend_from_slice(&samples[pos..end]);
            seen_speech = true;
        }
        pos = end;
    }
    // Trailing silence run is simply dropped (never flushed).

    if out.is_empty() {
        // Everything was silence; hand back something rather than an empty buffer.
        return samples.to_vec();
    }
    out
}

/// Target peak a quiet recording is boosted toward.
pub const NORMALIZE_TARGET_PEAK: f32 = 0.9;
/// Ceiling on the boost, so a near-silent buffer of room tone isn't blasted up into noise.
pub const NORMALIZE_MAX_GAIN: f32 = 10.0;
/// Below this peak the buffer is effectively silent and left alone — amplifying it would only
/// raise the noise floor, not reveal speech.
const NORMALIZE_MIN_PEAK: f32 = 1e-3;

/// Boost a quiet recording toward a healthy level before transcription, so a soft-spoken user is
/// heard as well as a loud one. Deliberately conservative and low-risk: it **only amplifies**
/// (never attenuates), **caps** the gain, leaves an **already-loud** clip untouched, and skips a
/// **near-silent** buffer entirely. Pure DSP — no dependency, negligible cost. This is the
/// low-voice half of the audio-pickup work; robust noisy-room suppression is a separate, larger
/// effort (a VAD/denoise model + on-device tuning) tracked for its own release.
pub fn normalize_gain(samples: &[f32]) -> Vec<f32> {
    let peak = samples.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
    if peak < NORMALIZE_MIN_PEAK {
        return samples.to_vec();
    }
    let gain = (NORMALIZE_TARGET_PEAK / peak).min(NORMALIZE_MAX_GAIN);
    if gain <= 1.0 {
        // Already at or above target — don't touch a recording that's already loud enough.
        return samples.to_vec();
    }
    samples.iter().map(|&s| (s * gain).clamp(-1.0, 1.0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 16000;

    fn tone(secs: f32) -> Vec<f32> {
        let n = (secs * SR as f32) as usize;
        (0..n)
            .map(|i| (i as f32 * 0.05).sin() * 0.4)
            .collect()
    }

    fn silence(secs: f32) -> Vec<f32> {
        vec![0.0; (secs * SR as f32) as usize]
    }

    #[test]
    fn does_not_split_before_the_minimum() {
        let audio = tone(3.0);
        assert_eq!(find_split_point(&audio, SR), None);
    }

    #[test]
    fn splits_inside_the_silence_not_the_speech() {
        // 10s speech, 1s gap, 3s speech. The cut belongs in the gap.
        let mut audio = tone(10.0);
        let gap_start = audio.len();
        audio.extend(silence(1.0));
        let gap_end = audio.len();
        audio.extend(tone(3.0));

        let split = find_split_point(&audio, SR).expect("should split at the gap");
        assert!(
            split >= gap_start && split <= gap_end,
            "split at {split} is outside the silence ({gap_start}..{gap_end})"
        );
    }

    #[test]
    fn ignores_gaps_that_are_too_short_to_be_boundaries() {
        // A 60ms blip is a pause within a phrase, not a boundary; with nothing else to go
        // on the chunk should keep growing rather than cut mid-sentence.
        let mut audio = tone(9.0);
        audio.extend(silence(0.06));
        audio.extend(tone(2.0));
        assert_eq!(find_split_point(&audio, SR), None);
    }

    #[test]
    fn force_splits_at_the_ceiling_when_speech_never_pauses() {
        let audio = tone(MAX_CHUNK_SECS + 2.0);
        let split = find_split_point(&audio, SR).expect("must cut rather than grow forever");
        assert!(split <= (MAX_CHUNK_SECS * SR as f32) as usize + 1);
    }

    #[test]
    fn prefers_the_latest_boundary_for_maximum_context() {
        // Two candidate gaps; the later one gives Whisper more context in the chunk.
        let mut audio = tone(9.0);
        audio.extend(silence(0.5));
        audio.extend(tone(3.0));
        let late_start = audio.len();
        audio.extend(silence(0.5));
        audio.extend(tone(2.0));

        let split = find_split_point(&audio, SR).unwrap();
        assert!(split >= late_start, "expected the later gap, got {split}");
    }

    #[test]
    fn quiet_recordings_still_find_their_boundaries() {
        // Scaling everything down must not turn the whole clip into "silence".
        let quiet: Vec<f32> = tone(10.0).iter().map(|s| s * 0.05).collect();
        let mut audio = quiet.clone();
        let gap_start = audio.len();
        audio.extend(silence(0.6));
        audio.extend(quiet.iter().take(SR as usize * 2).copied());

        let split = find_split_point(&audio, SR).expect("relative threshold should still work");
        assert!(split >= gap_start, "cut at {split} landed in speech");
    }

    #[test]
    fn zero_sample_rate_is_not_a_panic() {
        assert_eq!(find_split_point(&tone(10.0), 0), None);
    }

    #[test]
    fn trims_leading_and_trailing_silence() {
        let mut audio = silence(1.5);
        audio.extend(tone(2.0));
        audio.extend(silence(1.5));
        let trimmed = trim_silence(&audio, SR);
        // The 3s of dead air at the ends should be gone, leaving roughly the speech.
        assert!(trimmed.len() < audio.len(), "nothing was trimmed");
        let speech = (2.0 * SR as f32) as usize;
        assert!(
            trimmed.len() <= speech + SR as usize / 2,
            "kept too much: {} samples for 2s of speech",
            trimmed.len()
        );
    }

    #[test]
    fn collapses_long_internal_pauses_but_keeps_speech() {
        let mut audio = tone(2.0);
        audio.extend(silence(2.0)); // a long "thinking" gap
        audio.extend(tone(2.0));
        let trimmed = trim_silence(&audio, SR);
        // ~4s of speech plus one short kept pause, not the full 6s.
        let four_s = (4.0 * SR as f32) as usize;
        assert!(trimmed.len() >= four_s, "speech was lost: {}", trimmed.len());
        assert!(
            trimmed.len() < (5.0 * SR as f32) as usize,
            "long pause not collapsed: {}",
            trimmed.len()
        );
    }

    #[test]
    fn keeps_natural_short_pauses() {
        // A 200ms pause between words is normal speech, not dead air; it should survive.
        let mut audio = tone(1.0);
        audio.extend(silence(0.2));
        audio.extend(tone(1.0));
        let trimmed = trim_silence(&audio, SR);
        let expected = (2.15 * SR as f32) as usize;
        assert!(
            trimmed.len() >= expected,
            "a natural pause was over-trimmed: {} < {}",
            trimmed.len(),
            expected
        );
    }

    #[test]
    fn trim_handles_all_silence_and_empty() {
        assert!(!trim_silence(&silence(2.0), SR).is_empty());
        assert!(trim_silence(&[], SR).is_empty());
        assert_eq!(trim_silence(&tone(1.0), 0).len(), tone(1.0).len());
    }

    #[test]
    fn boosts_a_quiet_clip_toward_the_target() {
        let quiet = vec![0.1, -0.1, 0.05, -0.08, 0.1];
        let out = normalize_gain(&quiet);
        let peak = out.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!((peak - NORMALIZE_TARGET_PEAK).abs() < 0.01, "peak {peak} not near target");
    }

    #[test]
    fn leaves_an_already_loud_clip_untouched() {
        let loud = vec![0.95, -0.9, 0.5, -0.95];
        assert_eq!(normalize_gain(&loud), loud);
    }

    #[test]
    fn caps_the_gain_on_a_very_quiet_clip() {
        // peak 0.01 → 0.9/0.01 = 90, capped to NORMALIZE_MAX_GAIN (10) → peak ~0.1, not blasted.
        let very_quiet = vec![0.01, -0.01, 0.008];
        let peak = normalize_gain(&very_quiet).iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!((peak - 0.1).abs() < 0.005, "peak {peak} should be ~0.1 (10x cap)");
    }

    #[test]
    fn leaves_near_silence_alone() {
        let hush = vec![0.0002, -0.0003, 0.0001];
        assert_eq!(normalize_gain(&hush), hush);
    }

    #[test]
    fn never_exceeds_full_scale_and_handles_empty() {
        let out = normalize_gain(&[0.2, -0.3, 0.25]);
        assert!(out.iter().all(|&s| s.abs() <= 1.0));
        assert!(normalize_gain(&[]).is_empty());
    }
}
