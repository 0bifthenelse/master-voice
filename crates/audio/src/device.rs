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
}

impl AudioError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AudioError::NoDevice
            | AudioError::Device(_)
            | AudioError::Stream(_)
            | AudioError::InvalidRate(_) => 5,
            AudioError::QueueFull => 7,
        }
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
    Ok(out)
}

pub fn to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * 32768.0).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
        .collect()
}

pub fn to_u16(samples: &[f32]) -> Vec<u16> {
    samples
        .iter()
        .map(|s| ((s.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16)
        .collect()
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
                        for (out, v) in data.iter_mut().zip(buf.iter()) {
                            *out = (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        }
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
                        for (out, v) in data.iter_mut().zip(buf.iter()) {
                            *out = ((v.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16;
                        }
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
    fn i16_conversion_clamps() {
        let out = to_i16(&[1.5, -1.5, 0.0]);
        assert_eq!(out, vec![i16::MAX, i16::MIN, 0]);
    }

    #[test]
    fn no_device_errors_gracefully() {
        let result = open_stream(Some("definitely-not-a-device-xyz"), |_, _| {});
        assert!(result.is_err());
        assert!(matches!(result, Err(AudioError::NoDevice)));
    }
}
