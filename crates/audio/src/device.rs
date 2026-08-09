use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("no audio output device available")]
    NoDevice,
    #[error("audio device error: {0}")]
    Device(String),
    #[error("stream error: {0}")]
    Stream(String),
    #[error("playback queue is full")]
    QueueFull,
    #[error("invalid sample rate {0}")]
    InvalidRate(u32),
    #[error("invalid normalized PCM sample at index {index}")]
    InvalidSamples { index: usize },
}

impl AudioError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AudioError::NoDevice
            | AudioError::Device(_)
            | AudioError::Stream(_)
            | AudioError::InvalidRate(_)
            | AudioError::InvalidSamples { .. } => 5,
            AudioError::QueueFull => 7,
        }
    }
}

pub fn validate_normalized_pcm(samples: &[f32]) -> Result<(), AudioError> {
    match samples
        .iter()
        .position(|sample| !sample.is_finite() || !(-1.0..=1.0).contains(sample))
    {
        Some(index) => Err(AudioError::InvalidSamples { index }),
        None => Ok(()),
    }
}

pub struct DeviceInfo {
    pub name: String,
    pub channels: u16,
    pub sample_rate: u32,
}

pub fn default_device_info() -> Result<DeviceInfo, AudioError> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or(AudioError::NoDevice)?;
    let config = device
        .default_output_config()
        .map_err(|e| AudioError::Device(e.to_string()))?;
    Ok(DeviceInfo {
        name: device.name().unwrap_or_else(|_| "unknown".to_string()),
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
    })
}

pub fn list_devices() -> Vec<DeviceInfo> {
    let host = cpal::default_host();
    let mut out = Vec::new();
    let Ok(devices) = host.output_devices() else {
        return out;
    };
    for device in devices {
        if let Ok(config) = device.default_output_config() {
            out.push(DeviceInfo {
                name: device.name().unwrap_or_else(|_| "unknown".to_string()),
                channels: config.channels(),
                sample_rate: config.sample_rate().0,
            });
        }
    }
    out
}

pub fn resample(input: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>, AudioError> {
    if from_rate == 0 || to_rate == 0 {
        return Err(AudioError::InvalidRate(if from_rate == 0 {
            from_rate
        } else {
            to_rate
        }));
    }
    if input.is_empty() {
        return Ok(Vec::new());
    }
    validate_normalized_pcm(input)?;
    if from_rate == to_rate {
        return Ok(input.to_vec());
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = ((input.len() as f64) / ratio).ceil() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos.floor() as usize;
        let frac = (pos - idx as f64) as f32;
        let a = input[idx.min(input.len() - 1)];
        let b = input[(idx + 1).min(input.len() - 1)];
        out.push(a + (b - a) * frac);
    }
    validate_normalized_pcm(&out)?;
    Ok(out)
}

fn sanitize_output_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn sample_to_i16(sample: f32) -> i16 {
    (sanitize_output_sample(sample) * 32768.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

fn sample_to_u16(sample: f32) -> u16 {
    ((sanitize_output_sample(sample) * 0.5 + 0.5) * u16::MAX as f32) as u16
}

fn write_f32_output(output: &mut [f32]) {
    for sample in output {
        *sample = sanitize_output_sample(*sample);
    }
}

fn write_i16_output(output: &mut [i16], input: &[f32]) {
    debug_assert_eq!(output.len(), input.len());
    for (output, input) in output.iter_mut().zip(input) {
        *output = sample_to_i16(*input);
    }
}

fn write_u16_output(output: &mut [u16], input: &[f32]) {
    debug_assert_eq!(output.len(), input.len());
    for (output, input) in output.iter_mut().zip(input) {
        *output = sample_to_u16(*input);
    }
}

pub fn to_i16(samples: &[f32]) -> Vec<i16> {
    samples.iter().copied().map(sample_to_i16).collect()
}

pub fn to_u16(samples: &[f32]) -> Vec<u16> {
    samples.iter().copied().map(sample_to_u16).collect()
}

pub fn open_stream(
    device_name: Option<&str>,
    on_data: impl FnMut(&mut [f32], &cpal::StreamConfig) + Send + 'static,
) -> Result<(cpal::Stream, cpal::StreamConfig, DeviceInfo), AudioError> {
    let host = cpal::default_host();
    let device = match device_name {
        Some(name) => {
            let mut devices = host
                .output_devices()
                .map_err(|e| AudioError::Device(e.to_string()))?;
            devices
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .ok_or(AudioError::NoDevice)?
        }
        None => host.default_output_device().ok_or(AudioError::NoDevice)?,
    };
    let config = device
        .default_output_config()
        .map_err(|e| AudioError::Device(e.to_string()))?;
    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();
    let info = DeviceInfo {
        name: device.name().unwrap_or_else(|_| "unknown".to_string()),
        channels: stream_config.channels,
        sample_rate: stream_config.sample_rate.0,
    };

    let err_fn = |err: cpal::StreamError| {
        tracing::error!("audio stream error: {err}");
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let mut callback = on_data;
            let cfg = stream_config.clone();
            device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [f32], _| {
                        callback(data, &cfg);
                        write_f32_output(data);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AudioError::Stream(e.to_string()))?
        }
        cpal::SampleFormat::I16 => {
            let mut callback = on_data;
            let cfg = stream_config.clone();
            device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _| {
                        let mut buf = vec![0.0f32; data.len()];
                        callback(&mut buf, &cfg);
                        write_i16_output(data, &buf);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AudioError::Stream(e.to_string()))?
        }
        cpal::SampleFormat::U16 => {
            let mut callback = on_data;
            let cfg = stream_config.clone();
            device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [u16], _| {
                        let mut buf = vec![0.0f32; data.len()];
                        callback(&mut buf, &cfg);
                        write_u16_output(data, &buf);
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| AudioError::Stream(e.to_string()))?
        }
        other => {
            return Err(AudioError::Stream(format!(
                "unsupported sample format {other:?}"
            )))
        }
    };
    stream
        .play()
        .map_err(|e| AudioError::Stream(e.to_string()))?;
    Ok((stream, stream_config, info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_identity() {
        let input = vec![0.0f32, 0.5, 1.0, 0.5, 0.0];
        let out = resample(&input, 22050, 22050).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn resample_upsamples() {
        let input = vec![0.0f32, 1.0, 0.0, 1.0, 0.0];
        let out = resample(&input, 22050, 44100).unwrap();
        assert!(out.len() > input.len());
        assert!(out.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn resample_rejects_zero() {
        assert!(resample(&[], 0, 48000).is_err());
        assert!(resample(&[], 22050, 0).is_err());
    }

    #[test]
    fn empty_resampling_is_valid_after_rate_validation() {
        assert_eq!(resample(&[], 22_050, 48_000).unwrap(), Vec::<f32>::new());
        assert_eq!(resample(&[], 22_050, 22_050).unwrap(), Vec::<f32>::new());
    }

    #[test]
    fn validation_reports_first_invalid_sample() {
        for (samples, index) in [
            (vec![0.0, f32::NAN], 1),
            (vec![f32::INFINITY], 0),
            (vec![f32::NEG_INFINITY], 0),
            (vec![0.0, 1.01, f32::NAN], 1),
            (vec![-1.01], 0),
        ] {
            assert!(matches!(
                validate_normalized_pcm(&samples),
                Err(AudioError::InvalidSamples { index: actual }) if actual == index
            ));
            assert!(matches!(
                resample(&samples, 22_050, 48_000),
                Err(AudioError::InvalidSamples { index: actual }) if actual == index
            ));
        }
    }

    #[test]
    fn i16_conversion_clamps() {
        let out = to_i16(&[1.5, -1.5, 0.0]);
        assert_eq!(out, vec![i16::MAX, i16::MIN, 0]);
    }

    #[test]
    fn every_device_format_sanitizes_non_finite_and_out_of_range_samples() {
        let injected = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -2.0, 2.0];

        let mut f32_output = injected;
        write_f32_output(&mut f32_output);
        assert_eq!(f32_output, [0.0, 0.0, 0.0, -1.0, 1.0]);
        assert!(f32_output
            .iter()
            .all(|sample| sample.is_finite() && (-1.0..=1.0).contains(sample)));

        let mut i16_output = [0; 5];
        write_i16_output(&mut i16_output, &injected);
        assert_eq!(i16_output, [0, 0, 0, i16::MIN, i16::MAX]);

        let mut u16_output = [0; 5];
        write_u16_output(&mut u16_output, &injected);
        assert_eq!(u16_output, [32_767, 32_767, 32_767, u16::MIN, u16::MAX]);
    }

    #[test]
    fn no_device_errors_gracefully() {
        let result = open_stream(Some("definitely-not-a-device-xyz"), |_, _| {});
        assert!(result.is_err());
        assert!(matches!(result, Err(AudioError::NoDevice)));
    }
}
