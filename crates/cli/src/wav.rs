//! Minimal 16-bit mono PCM WAV writer (44-byte RIFF header, no
//! dependencies).

use std::io::Write;

pub fn write_wav(path: &std::path::Path, samples: &[f32], sample_rate: u32) -> std::io::Result<()> {
    let data_len = (samples.len() * 2) as u32;
    let mut file = std::fs::File::create(path)?;
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_len).to_le_bytes())?;
    file.write_all(b"WAVE")?;
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?; // PCM
    file.write_all(&1u16.to_le_bytes())?; // mono
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&(sample_rate * 2).to_le_bytes())?; // byte rate
    file.write_all(&2u16.to_le_bytes())?; // block align
    file.write_all(&16u16.to_le_bytes())?; // bits per sample
    file.write_all(b"data")?;
    file.write_all(&data_len.to_le_bytes())?;
    let mut pcm = Vec::with_capacity(samples.len());
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        pcm.extend_from_slice(&((clamped * 32767.0) as i16).to_le_bytes());
    }
    file.write_all(&pcm)?;
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_valid_header() {
        let path = std::env::temp_dir().join(format!("mv-wav-test-{}", std::process::id()));
        let samples: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        write_wav(&path, &samples, 22050).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 22050);
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            (samples.len() * 2) as u32
        );
        assert_eq!(bytes.len(), 44 + samples.len() * 2);
        std::fs::remove_file(&path).unwrap();
    }
}
