use master_voice_linguistics::phoneme::{Boundary, Phoneme, Stress};

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
            robotic_depth: 0.6,
        }
    }
}

#[derive(Clone)]
pub struct Segment {
    pub f1: f32,
    pub f2: f32,
    pub f3: f32,
    pub b1: f32,
    pub b2: f32,
    pub b3: f32,
    pub voicing: f32,
    pub voice_amp: f32,
    pub noise_amp: f32,
    pub noise_freq: f32,
    pub nasal: f32,
    pub f0: f32,
    pub duration_s: f32,
    pub burst: Option<f32>,
}

pub struct ProsodyResult {
    pub segments: Vec<Segment>,
    pub total_s: f32,
}

const BASE_VOWEL_S: f32 = 0.095;
const BASE_CONSONANT_S: f32 = 0.070;
const WORD_PAUSE_S: f32 = 0.030;
const CLAUSE_PAUSE_S: f32 = 0.180;
const SENTENCE_PAUSE_S: f32 = 0.320;
const BASE_F0: f32 = 118.0;

fn pause_for(boundary: Boundary) -> f32 {
    match boundary {
        Boundary::None => 0.0,
        Boundary::Word => WORD_PAUSE_S,
        Boundary::Clause => CLAUSE_PAUSE_S,
        Boundary::Sentence => SENTENCE_PAUSE_S,
        Boundary::Question => SENTENCE_PAUSE_S,
    }
}

pub fn build_prosody(phonemes: &[Phoneme], opts: &SynthOptions) -> ProsodyResult {
    let rate = opts.rate.clamp(0.5, 2.0);
    let pitch = opts.pitch.clamp(0.5, 1.5);
    let robotic = opts.robotic_depth.clamp(0.0, 1.0);
    let pitch_range = 1.0 - 0.55 * robotic;

    let mut segments = Vec::with_capacity(phonemes.len() * 2);
    let mut phrase_f0 = BASE_F0 * pitch;
    let mut phrase_start = 0usize;

    for (idx, phoneme) in phonemes.iter().enumerate() {
        let is_phrase_start = idx == 0
            || matches!(
                phonemes[idx - 1].boundary_after,
                Boundary::Clause | Boundary::Sentence | Boundary::Question
            );
        if is_phrase_start {
            phrase_start = idx;
            phrase_f0 = BASE_F0 * pitch;
        }

        let is_vowel = phoneme.is_vowel();
        let base = if is_vowel {
            BASE_VOWEL_S
        } else {
            BASE_CONSONANT_S
        };
        let mut dur = base / rate;
        match phoneme.stress {
            Stress::Primary => dur *= 1.28,
            Stress::Secondary => dur *= 1.12,
            Stress::None => {}
        }

        let phrase_len = (idx - phrase_start + 1).max(1) as f32;
        let declination = 1.0 - 0.06 * (phrase_len / 24.0).min(1.0);
        let mut f0 = phrase_f0 * declination;

        match phoneme.stress {
            Stress::Primary => f0 *= 1.0 + 0.14 * pitch_range,
            Stress::Secondary => f0 *= 1.0 + 0.07 * pitch_range,
            Stress::None => {}
        }

        let phrase_end = matches!(
            phoneme.boundary_after,
            Boundary::Sentence | Boundary::Question
        );
        if phrase_end {
            let question = phoneme.boundary_after == Boundary::Question;
            f0 *= if question { 1.35 } else { 0.90 };
        }

        if is_vowel {
            dur += 0.012;
        }

        let target = super::params::target_for(phoneme.kind);
        let mut segment = Segment {
            f1: target.f1,
            f2: target.f2,
            f3: target.f3,
            b1: target.b1,
            b2: target.b2,
            b3: target.b3,
            voicing: target.voicing,
            voice_amp: target.voice_amp,
            noise_amp: target.noise_amp,
            noise_freq: target.noise_freq,
            nasal: target.nasal,
            f0,
            duration_s: dur,
            burst: None,
        };

        let is_stop = matches!(
            phoneme.kind,
            master_voice_linguistics::phoneme::PhonemeKind::P
                | master_voice_linguistics::phoneme::PhonemeKind::B
                | master_voice_linguistics::phoneme::PhonemeKind::T
                | master_voice_linguistics::phoneme::PhonemeKind::D
                | master_voice_linguistics::phoneme::PhonemeKind::K
                | master_voice_linguistics::phoneme::PhonemeKind::G
                | master_voice_linguistics::phoneme::PhonemeKind::CH
                | master_voice_linguistics::phoneme::PhonemeKind::JH
        );
        if is_stop {
            let closure = 0.055 / rate;
            let burst = 0.014;
            let mut closure_segment = segment.clone();
            closure_segment.voice_amp = 0.0;
            closure_segment.noise_amp = 0.0;
            closure_segment.duration_s = closure;
            segments.push(closure_segment);
            segment.burst = Some(segment.noise_freq);
            segment.voice_amp *= 0.4;
            segment.duration_s = burst;
            segment.noise_amp = 0.6;
        }

        segments.push(segment.clone());

        if phoneme.boundary_after != Boundary::None {
            let pause = pause_for(phoneme.boundary_after);
            if pause > 0.0 {
                let mut silence = segment;
                silence.voice_amp = 0.0;
                silence.noise_amp = 0.0;
                silence.duration_s = pause;
                segments.push(silence);
            }
        }
    }

    let total_s = segments.iter().map(|s| s.duration_s).sum();
    ProsodyResult { segments, total_s }
}

#[cfg(test)]
mod tests {
    use super::*;
    use master_voice_linguistics::phoneme::{Phoneme, PhonemeKind};

    #[test]
    fn builds_segments_with_pauses() {
        let phonemes = [
            Phoneme::new(PhonemeKind::H),
            Phoneme::new(PhonemeKind::EH),
            Phoneme::new(PhonemeKind::L),
            Phoneme::new(PhonemeKind::OW),
        ];
        let result = build_prosody(&phonemes, &SynthOptions::default());
        assert!(result.total_s > 0.2);
        assert!(result.segments.iter().all(|s| s.duration_s > 0.0));
    }

    #[test]
    fn question_rises() {
        let mut phoneme = Phoneme::new(PhonemeKind::IY);
        phoneme.boundary_after = Boundary::Question;
        let result = build_prosody(&[phoneme], &SynthOptions::default());
        assert!(result.segments[0].f0 > BASE_F0);
    }

    #[test]
    fn robotic_flattens_pitch() {
        let mut stressed = Phoneme::new(PhonemeKind::AE);
        stressed.stress = Stress::Primary;
        let plain = Phoneme::new(PhonemeKind::AE);
        let robotic = build_prosody(
            &[stressed, plain],
            &SynthOptions {
                robotic_depth: 1.0,
                ..SynthOptions::default()
            },
        );
        let natural = build_prosody(
            &[stressed, plain],
            &SynthOptions {
                robotic_depth: 0.0,
                ..SynthOptions::default()
            },
        );
        let robotic_spread = (robotic.segments[0].f0 - robotic.segments[1].f0).abs();
        let natural_spread = (natural.segments[0].f0 - natural.segments[1].f0).abs();
        assert!(robotic_spread < natural_spread);
    }
}
