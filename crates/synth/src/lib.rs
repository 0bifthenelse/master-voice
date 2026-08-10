mod ffi;

use master_voice_linguistics::phoneme::Phoneme;

pub const SAMPLE_RATE: u32 = 24_000;
pub const DEFAULT_ROBOTIC_DEPTH: f32 = 0.22;

#[derive(Clone, Debug, PartialEq)]
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SynthOptions {
    pub rate: f32,
    pub pitch: f32,
    pub volume: f32,
    pub robotic_depth: f32,
}

impl Default for SynthOptions {
    fn default() -> Self {
        Self {
            rate: 1.0,
            pitch: 1.0,
            volume: 1.0,
            robotic_depth: DEFAULT_ROBOTIC_DEPTH,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkPos {
    pub first: bool,
    pub last: bool,
}

impl Default for ChunkPos {
    fn default() -> Self {
        Self {
            first: true,
            last: true,
        }
    }
}

pub struct SynthState {
    engine: ffi::EngineState,
}

impl SynthState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: ffi::EngineState::new()
                .unwrap_or_else(|error| panic!("failed to initialize master assembly: {error}")),
        }
    }
}

impl Default for SynthState {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
pub fn synthesize(phonemes: &[Phoneme], options: &SynthOptions) -> AudioBuffer {
    let mut state = SynthState::new();
    render_chunk(
        &mut state,
        phonemes,
        options,
        ChunkPos {
            first: true,
            last: true,
        },
    )
}

#[must_use]
pub fn render_chunk(
    state: &mut SynthState,
    phonemes: &[Phoneme],
    options: &SynthOptions,
    position: ChunkPos,
) -> AudioBuffer {
    let phones = ffi::encode_phones(phonemes);
    let options = ffi::options(
        options.rate,
        options.pitch,
        options.volume,
        options.robotic_depth,
    );
    let sample_count = ffi::measure(&phones, &options, position.first, position.last)
        .unwrap_or_else(|error| panic!("master assembly measurement failed: {error}"));
    let mut samples = vec![0.0; sample_count];
    let result = ffi::render(
        &mut state.engine,
        &phones,
        &options,
        &mut samples,
        position.first,
        position.last,
    )
    .unwrap_or_else(|error| panic!("master assembly rendering failed: {error}"));
    assert_eq!(
        usize::try_from(result.written).expect("assembly sample count exceeds usize"),
        sample_count,
        "master assembly measure/render count mismatch"
    );
    AudioBuffer {
        samples,
        sample_rate: SAMPLE_RATE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use master_voice_linguistics::phoneme::{Boundary, PhonemeKind, Stress};

    fn vowel() -> Vec<Phoneme> {
        let mut phoneme = Phoneme::new(PhonemeKind::IY);
        phoneme.stress = Stress::Primary;
        phoneme.boundary_after = Boundary::Sentence;
        vec![phoneme]
    }

    #[test]
    fn public_renderer_emits_fixed_rate_finite_audio() {
        let buffer = synthesize(&vowel(), &SynthOptions::default());
        assert_eq!(buffer.sample_rate, SAMPLE_RATE);
        assert!(!buffer.samples.is_empty());
        assert!(buffer.samples.iter().all(|sample| sample.is_finite()));
        let peak = buffer
            .samples
            .iter()
            .copied()
            .map(f32::abs)
            .fold(0.0, f32::max);
        assert!((0.001..0.95).contains(&peak), "peak {peak}");
    }

    #[test]
    fn public_renderer_is_deterministic() {
        let first = synthesize(&vowel(), &SynthOptions::default());
        let second = synthesize(&vowel(), &SynthOptions::default());
        assert_eq!(first, second);
    }

    #[test]
    fn empty_phone_stream_is_empty() {
        let buffer = synthesize(&[], &SynthOptions::default());
        assert!(buffer.samples.is_empty());
        assert_eq!(buffer.sample_rate, SAMPLE_RATE);
    }
}
