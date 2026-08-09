pub mod character;
pub mod dsp;
pub mod frame;
pub mod klatt;
pub mod params;
pub mod prosody;

pub use prosody::{ChunkPos, SynthOptions};

use master_voice_linguistics::phoneme::Phoneme;

pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

pub fn synthesize(phonemes: &[Phoneme], opts: &SynthOptions) -> AudioBuffer {
    let frames = prosody::build_frames(phonemes, opts);
    let mut samples = klatt::render_frames(&frames, opts.robotic_depth);
    let mut post = dsp::PostState::default();
    dsp::post_chain(
        &mut samples,
        opts.robotic_depth,
        opts.volume,
        &mut post,
        ChunkPos {
            first: true,
            last: true,
        },
    );
    AudioBuffer {
        samples,
        sample_rate: params::SAMPLE_RATE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use master_voice_linguistics::phoneme::PhonemeKind;

    #[test]
    fn synthesize_hello_like() {
        let phonemes = [
            Phoneme::new(PhonemeKind::H),
            Phoneme::new(PhonemeKind::EH),
            Phoneme::new(PhonemeKind::L),
            Phoneme::new(PhonemeKind::OW),
        ];
        let buffer = synthesize(&phonemes, &SynthOptions::default());
        assert!(!buffer.samples.is_empty());
        assert_eq!(buffer.sample_rate, params::SAMPLE_RATE);
        let duration = buffer.samples.len() as f32 / buffer.sample_rate as f32;
        assert!(duration > 0.15, "duration={duration}");
        let rms = (buffer.samples.iter().map(|s| s * s).sum::<f32>() / buffer.samples.len() as f32)
            .sqrt();
        assert!(rms > 0.02, "rms={rms}");
        assert!(buffer
            .samples
            .iter()
            .all(|s| s.is_finite() && s.abs() <= 1.0));
    }

    #[test]
    fn empty_phonemes_produce_empty() {
        let buffer = synthesize(&[], &SynthOptions::default());
        assert!(buffer.samples.is_empty());
    }

    #[test]
    fn deterministic_output() {
        let phonemes = [Phoneme::new(PhonemeKind::AA), Phoneme::new(PhonemeKind::N)];
        let a = synthesize(&phonemes, &SynthOptions::default()).samples;
        let b = synthesize(&phonemes, &SynthOptions::default()).samples;
        assert_eq!(a, b);
    }

    #[test]
    fn rate_changes_duration() {
        let phonemes = [Phoneme::new(PhonemeKind::AA)];
        let slow = synthesize(
            &phonemes,
            &SynthOptions {
                rate: 0.5,
                ..SynthOptions::default()
            },
        );
        let fast = synthesize(
            &phonemes,
            &SynthOptions {
                rate: 2.0,
                ..SynthOptions::default()
            },
        );
        assert!(slow.samples.len() > fast.samples.len() * 2);
    }

    #[test]
    fn french_nasals_render() {
        let phonemes = [Phoneme::new(PhonemeKind::B), Phoneme::new(PhonemeKind::ON)];
        let buffer = synthesize(&phonemes, &SynthOptions::default());
        assert!(buffer.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn robotic_depth_zero_is_plain() {
        let phonemes = [Phoneme::new(PhonemeKind::AA)];
        let plain = synthesize(
            &phonemes,
            &SynthOptions {
                robotic_depth: 0.0,
                ..SynthOptions::default()
            },
        )
        .samples;
        let character = synthesize(
            &phonemes,
            &SynthOptions {
                robotic_depth: 1.0,
                ..SynthOptions::default()
            },
        )
        .samples;
        assert_ne!(plain, character);
    }
}
