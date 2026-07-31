use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

pub struct AudioCapture {
    _stream: Option<cpal::Stream>,
}

impl AudioCapture {
    pub fn new() -> Self {
        Self { _stream: None }
    }

    pub fn default_input_device() -> Option<Device> {
        let host = cpal::default_host();
        host.default_input_device()
    }

    pub fn list_input_devices() -> Vec<String> {
        let host = cpal::default_host();
        host.input_devices()
            .map(|devices| {
                devices
                    .filter_map(|d| d.name().ok().map(|n| n.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn start_capture(
        &mut self,
        device_name: Option<&str>,
        audio_callback: Arc<Mutex<dyn FnMut(&[f32]) + Send>>,
    ) -> Result<(), String> {
        let host = cpal::default_host();
        let device = if let Some(name) = device_name {
            host.input_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .ok_or_else(|| format!("Device '{}' not found", name))?
        } else {
            Self::default_input_device()
                .ok_or_else(|| "No input device found".to_string())?
        };

        let config = device
            .default_input_config()
            .map_err(|e| e.to_string())?;
        let _sample_format = config.sample_format();
        let stream_config: StreamConfig = config.into();

        let channels = stream_config.channels as usize;
        let sample_rate = stream_config.sample_rate.0;
        let _frame_size = (sample_rate as f32 * 0.03) as usize;

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut cb) = audio_callback.lock() {
                        cb(data);
                    }
                },
                |err| {
                    warn!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        info!("Audio capture started ({}Hz, {}ch)", sample_rate, channels);

        self._stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) {
        self._stream = None;
        info!("Audio capture stopped");
    }
}
