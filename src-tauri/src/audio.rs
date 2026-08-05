use cpal::traits::{DeviceTrait, HostTrait};

pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

/// Simple linear-interpolation resampler — cheap and good enough for speech
/// recognition input; avoids pulling in a heavier resampling library for a
/// one-shot mono conversion.
pub fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = ((input.len() as f64) / ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = input.get(idx).copied().unwrap_or(0.0);
        let b = input.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_rates_match() {
        let input = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_linear(&input, 16000, 16000), input);
    }

    #[test]
    fn downsamples_to_expected_length() {
        let input = vec![0.0; 48000];
        let out = resample_linear(&input, 48000, 16000);
        assert_eq!(out.len(), 16000);
    }

    #[test]
    fn handles_empty_input() {
        assert!(resample_linear(&[], 48000, 16000).is_empty());
    }
}
