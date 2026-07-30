// src-tauri/src/audio.rs
//! Audio capture using CPAL with Silero VAD integration

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SampleRate, Stream, StreamConfig};
use parking_lot::Mutex;
use rubato::{FftFixedInOut, Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::vad::SileroVad;

const TARGET_SAMPLE_RATE: u32 = 16000;
const CHUNK_DURATION_MS: u64 = 100; // 100ms chunks for VAD
const CHUNK_SIZE: usize = (TARGET_SAMPLE_RATE as usize * CHUNK_DURATION_MS) / 1000;
const VAD_CHUNK_SIZE: usize = 512; // Silero VAD expects 512 samples at 16kHz

pub struct AudioCapture {
    stream: Option<Stream>,
    is_recording: Arc<AtomicBool>,
    audio_sender: Option<mpsc::UnboundedSender<Vec<f32>>>,
    vad: Arc<Mutex<SileroVad>>,
    resampler: Option<Arc<Mutex<FftFixedInOut<f32>>>>,
}

impl AudioCapture {
    pub fn new(vad: SileroVad) -> Result<Self> {
        Ok(Self {
            stream: None,
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_sender: None,
            vad: Arc::new(Mutex::new(vad)),
            resampler: None,
        })
    }

    pub fn start(&mut self, app_handle: tauri::AppHandle) -> Result<mpsc::UnboundedReceiver<Vec<f32>>> {
        if self.is_recording.load(Ordering::Relaxed) {
            anyhow::bail!("Already recording");
        }

        let (tx, rx) = mpsc::unbounded_channel();
        self.audio_sender = Some(tx);

        let host = cpal::default_host();
        let device = host.default_input_device().context("No input device found")?;
        let config = device.default_input_config().context("Failed to get default input config")?;

        info!("Audio device: {}", device.name()?);
        info!("Input config: {:?}", config);

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        // Set up resampler if needed
        if sample_rate != TARGET_SAMPLE_RATE {
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };
            let resampler = SincFixedIn::<f32>::new(
                sample_rate as f64 / TARGET_SAMPLE_RATE as f64,
                2.0,
                params,
                CHUNK_SIZE,
                channels,
            ).context("Failed to create resampler")?;
            self.resampler = Some(Arc::new(Mutex::new(resampler)));
        }

        let is_recording = self.is_recording.clone();
        let audio_sender = self.audio_sender.clone();
        let vad = self.vad.clone();
        let resampler = self.resampler.clone();

        let err_fn = move |err| error!("Audio stream error: {}", err);

        let stream = match config.sample_format() {
            SampleFormat::F32 => Self::build_stream::<f32>(
                &device, &config.into(), is_recording, audio_sender, vad, resampler, err_fn
            )?,
            SampleFormat::I16 => Self::build_stream::<i16>(
                &device, &config.into(), is_recording, audio_sender, vad, resampler, err_fn
            )?,
            SampleFormat::U16 => Self::build_stream::<u16>(
                &device, &config.into(), is_recording, audio_sender, vad, resampler, err_fn
            )?,
            _ => anyhow::bail!("Unsupported sample format"),
        };

        stream.play().context("Failed to start audio stream")?;
        self.stream = Some(stream);
        self.is_recording.store(true, Ordering::Relaxed);

        info!("Audio capture started");
        Ok(rx)
    }

    fn build_stream<T>(
        device: &Device,
        config: &StreamConfig,
        is_recording: Arc<AtomicBool>,
        audio_sender: Option<mpsc::UnboundedSender<Vec<f32>>>,
        vad: Arc<Mutex<SileroVad>>,
        resampler: Option<Arc<Mutex<SincFixedIn<f32>>>>,
        err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
    ) -> Result<Stream>
    where
        T: cpal::Sample + cpal::SizedSample + Send + 'static,
        f32: From<T>,
    {
        let channels = config.channels as usize;
        let mut buffer = Vec::with_capacity(CHUNK_SIZE * channels);

        let stream = device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if !is_recording.load(Ordering::Relaxed) {
                    return;
                }

                // Convert to f32 mono
                buffer.clear();
                for frame in data.chunks(channels) {
                    let sample: f32 = frame[0].into(); // Take first channel
                    buffer.push(sample);
                }

                // Resample if needed
                let resampled = if let Some(ref resampler) = resampler {
                    let mut resampler = resampler.lock();
                    let mut output = vec![0.0f32; CHUNK_SIZE];
                    resampler.process(&[&buffer], &mut [&mut output]).ok();
                    output
                } else {
                    buffer.clone()
                };

                // Process through VAD
                let mut vad = vad.lock();
                let is_speech = vad.process_chunk(&resampled);

                // Send audio if we're recording and there's speech activity
                if let Some(ref sender) = audio_sender {
                    if is_speech || !resampled.iter().all(|&x| x.abs() < 0.001) {
                        let _ = sender.send(resampled);
                    }
                }
            },
            err_fn,
            None,
        ).context("Failed to build input stream")?;

        Ok(stream)
    }

    pub fn stop(&mut self) {
        self.is_recording.store(false, Ordering::Relaxed);
        self.stream = None;
        self.audio_sender = None;
        info!("Audio capture stopped");
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn get_input_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let devices = host.input_devices().context("Failed to enumerate input devices")?;
        Ok(devices.filter_map(|d| d.name().ok()).collect())
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Voice Activity Detection using Silero VAD
pub struct VoiceActivityDetector {
    vad: Arc<Mutex<SileroVad>>,
    speech_buffer: Mutex<Vec<f32>>,
    silence_buffer: Mutex<Vec<f32>>,
    is_speech: AtomicBool,
    speech_started: AtomicBool,
    min_speech_duration_ms: u64,
    max_silence_duration_ms: u64,
}

impl VoiceActivityDetector {
    pub fn new(vad: SileroVad) -> Self {
        Self {
            vad: Arc::new(Mutex::new(vad)),
            speech_buffer: Mutex::new(Vec::new()),
            silence_buffer: Mutex::new(Vec::new()),
            is_speech: AtomicBool::new(false),
            speech_started: AtomicBool::new(false),
            min_speech_duration_ms: 250,
            max_silence_duration_ms: 800,
        }
    }

    pub fn process(&self, audio: &[f32]) -> VoiceActivity {
        let mut vad = self.vad.lock();
        let is_speech = vad.process_chunk(audio);

        if is_speech {
            self.speech_started.store(true, Ordering::Relaxed);
            self.is_speech.store(true, Ordering::Relaxed);
            self.speech_buffer.lock().extend_from_slice(audio);
            self.silence_buffer.lock().clear();
            VoiceActivity::Speech
        } else if self.speech_started.load(Ordering::Relaxed) {
            self.silence_buffer.lock().extend_from_slice(audio);
            let silence_duration = (self.silence_buffer.lock().len() as f32 / TARGET_SAMPLE_RATE as f32 * 1000.0) as u64;

            if silence_duration > self.max_silence_duration_ms {
                let speech = self.speech_buffer.lock().clone();
                self.speech_buffer.lock().clear();
                self.silence_buffer.lock().clear();
                self.speech_started.store(false, Ordering::Relaxed);
                self.is_speech.store(false, Ordering::Relaxed);
                VoiceActivity::EndOfSpeech(speech)
            } else {
                VoiceActivity::SilenceDuringSpeech
            }
        } else {
            VoiceActivity::Silence
        }
    }

    pub fn reset(&self) {
        self.speech_buffer.lock().clear();
        self.silence_buffer.lock().clear();
        self.speech_started.store(false, Ordering::Relaxed);
        self.is_speech.store(false, Ordering::Relaxed);
        self.vad.lock().reset();
    }
}

#[derive(Debug)]
pub enum VoiceActivity {
    Speech,
    Silence,
    SilenceDuringSpeech,
    EndOfSpeech(Vec<f32>),
}