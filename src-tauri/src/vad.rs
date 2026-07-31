/// Energy-based Voice Activity Detector.
///
/// A lightweight, dependency-free VAD that detects speech by comparing
/// the RMS energy of audio frames against a configurable threshold.
pub struct VoiceActivityDetector {
    threshold: f32,
    frame_size: usize,
    speaking: bool,
}

impl VoiceActivityDetector {
    pub fn new(threshold: f32, sample_rate: u32, frame_duration_ms: u32) -> Self {
        let frame_size = (sample_rate as f32 * frame_duration_ms as f32 / 1000.0) as usize;
        Self {
            threshold,
            frame_size,
            speaking: false,
        }
    }

    pub fn process_chunk(&mut self, samples: &[f32]) -> bool {
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

        self.speaking = rms > self.threshold;
        self.speaking
    }

    pub fn reset(&mut self) {
        self.speaking = false;
    }

    pub fn is_speaking(&self) -> bool {
        self.speaking
    }

    pub fn frame_size(&self) -> usize {
        self.frame_size
    }
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new(0.01, 16000, 30)
    }
}
