// src-tauri/src/vad.rs
//! Voice Activity Detection using Silero VAD

use anyhow::{Context, Result};
use silero_vad::{SileroVad, VadConfig};
use std::sync::{Arc, Mutex};

const VAD_SAMPLE_RATE: i32 = 16000;
const VAD_WINDOW_SIZE: usize = 512; // 32ms at 16kHz
const VAD_THRESHOLD: f32 = 0.5;
const MIN_SPEECH_DURATION_MS: i64 = 250;
const MIN_SILENCE_DURATION_MS: i64 = 100;
const SPEECH_PAD_MS: i64 = 30;

pub struct VoiceActivityDetector {
    vad: SileroVad,
    speech_buffer: Vec<f32>,
    is_speaking: bool,
    silence_frames: usize,
    speech_frames: usize,
    sample_count: usize,
}

impl VoiceActivityDetector {
    pub fn new() -> Result<Self> {
        let config = VadConfig {
            sample_rate: VAD_SAMPLE_RATE,
            window_size: VAD_WINDOW_SIZE,
            threshold: VAD_THRESHOLD,
            min_speech_duration_ms: MIN_SPEECH_DURATION_MS,
            min_silence_duration_ms: MIN_SILENCE_DURATION_MS,
            speech_pad_ms: SPEECH_PAD_MS,
        };

        let vad = SileroVad::new(config).context("Failed to initialize Silero VAD")?;

        Ok(Self {
            vad,
            speech_buffer: Vec::with_capacity(VAD_SAMPLE_RATE as usize * 30), // 30 seconds max
            is_speaking: false,
            silence_frames: 0,
            speech_frames: 0,
            sample_count: 0,
        })
    }

    /// Process audio samples and return speech segments
    /// Returns (is_speech_end, speech_audio)
    pub fn process(&mut self, samples: &[f32]) -> Result<Option<Vec<f32>>> {
        let mut speech_segments = Vec::new();
        let mut current_segment = Vec::new();
        let mut in_speech = false;

        // Process in windows
        for chunk in samples.chunks(VAD_WINDOW_SIZE) {
            if chunk.len() < VAD_WINDOW_SIZE {
                // Pad with zeros if needed
                let mut padded = chunk.to_vec();
                padded.resize(VAD_WINDOW_SIZE, 0.0);
                let prob = self.vad.process(&padded).context("VAD processing failed")?;

                if prob > VAD_THRESHOLD {
                    in_speech = true;
                    current_segment.extend_from_slice(chunk);
                } else if in_speech {
                    // End of speech
                    if current_segment.len() >= (VAD_SAMPLE_RATE as usize * MIN_SPEECH_DURATION_MS as usize / 1000) {
                        speech_segments.push(current_segment);
                    }
                    current_segment = Vec::new();
                    in_speech = false;
                }
            } else {
                let prob = self.vad.process(chunk).context("VAD processing failed")?;

                if prob > VAD_THRESHOLD {
                    in_speech = true;
                    current_segment.extend_from_slice(chunk);
                } else if in_speech {
                    if current_segment.len() >= (VAD_SAMPLE_RATE as usize * MIN_SPEECH_DURATION_MS as usize / 1000) {
                        speech_segments.push(current_segment);
                    }
                    current_segment = Vec::new();
                    in_speech = false;
                }
            }
        }

        // Handle ongoing speech
        if in_speech && !current_segment.is_empty() {
            // Keep buffering for potential continuation
            self.speech_buffer.extend(current_segment);
            return Ok(None);
        }

        // If we have completed segments, return the first one and buffer the rest
        if let Some(first) = speech_segments.first() {
            let result = first.clone();
            // Buffer remaining segments for next call
            self.speech_buffer = speech_segments.into_iter().skip(1).flatten().collect();
            Ok(Some(result))
        } else if !self.speech_buffer.is_empty() {
            // Flush buffered speech
            let result = std::mem::take(&mut self.speech_buffer);
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    pub fn reset(&mut self) {
        self.vad.reset();
        self.speech_buffer.clear();
        self.is_speaking = false;
        self.silence_frames = 0;
        self.speech_frames = 0;
    }

    pub fn flush(&mut self) -> Option<Vec<f32>> {
        if !self.speech_buffer.is_empty() {
            Some(std::mem::take(&mut self.speech_buffer))
        } else {
            None
        }
    }
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new().expect("Failed to create VAD")
    }
}