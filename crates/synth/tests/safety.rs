use master_voice_linguistics::phoneme::{Phoneme, PhonemeKind};
use master_voice_synth::SynthOptions;

const DEPTHS: [f32; 4] = [0.0, 0.55, 0.82, 1.0];
const VOLUMES: [f32; 3] = [0.0, 1.0, 2.0];
const HEADROOM_CEILING: f32 = 0.95;

fn phones(kinds: &[PhonemeKind]) -> Vec<Phoneme> {
    kinds.iter().copied().map(Phoneme::new).collect()
}

fn long_phone_run() -> Vec<Phoneme> {
    let cycle = [
        PhonemeKind::AA,
        PhonemeKind::N,
        PhonemeKind::AN,
        PhonemeKind::P,
        PhonemeKind::CH,
        PhonemeKind::S,
        PhonemeKind::IY,
        PhonemeKind::ON,
    ];
    cycle
        .into_iter()
        .cycle()
        .take(64)
        .map(Phoneme::new)
        .collect()
}

#[test]
fn full_phone_corpus_is_finite_normalized_and_audible() {
    let cases = [
        (
            "vowels",
            phones(&[
                PhonemeKind::IY,
                PhonemeKind::AE,
                PhonemeKind::AA,
                PhonemeKind::UW,
                PhonemeKind::OEU,
            ]),
        ),
        (
            "french nasals",
            phones(&[
                PhonemeKind::AA,
                PhonemeKind::AN,
                PhonemeKind::N,
                PhonemeKind::ON,
                PhonemeKind::M,
                PhonemeKind::EN,
                PhonemeKind::UN,
            ]),
        ),
        (
            "stops",
            phones(&[
                PhonemeKind::AA,
                PhonemeKind::P,
                PhonemeKind::B,
                PhonemeKind::T,
                PhonemeKind::D,
                PhonemeKind::K,
                PhonemeKind::G,
                PhonemeKind::IY,
            ]),
        ),
        (
            "fricatives",
            phones(&[
                PhonemeKind::AA,
                PhonemeKind::F,
                PhonemeKind::V,
                PhonemeKind::S,
                PhonemeKind::Z,
                PhonemeKind::SH,
                PhonemeKind::ZH,
                PhonemeKind::IY,
            ]),
        ),
        (
            "affricates",
            phones(&[
                PhonemeKind::AA,
                PhonemeKind::CH,
                PhonemeKind::JH,
                PhonemeKind::CH,
                PhonemeKind::JH,
                PhonemeKind::IY,
            ]),
        ),
        ("long run", long_phone_run()),
    ];

    for (label, phonemes) in cases {
        for depth in DEPTHS {
            for volume in VOLUMES {
                let buffer = master_voice_synth::synthesize(
                    &phonemes,
                    &SynthOptions {
                        robotic_depth: depth,
                        volume,
                        ..SynthOptions::default()
                    },
                );
                assert!(
                    !buffer.samples.is_empty(),
                    "{label}, depth={depth}, volume={volume}"
                );
                assert!(
                    buffer
                        .samples
                        .iter()
                        .all(|sample| { sample.is_finite() && sample.abs() <= HEADROOM_CEILING }),
                    "{label}, depth={depth}, volume={volume}"
                );

                let peak = buffer
                    .samples
                    .iter()
                    .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
                let rms = (buffer
                    .samples
                    .iter()
                    .map(|sample| sample * sample)
                    .sum::<f32>()
                    / buffer.samples.len() as f32)
                    .sqrt();
                if volume == 0.0 {
                    assert_eq!(peak, 0.0, "{label}, depth={depth}");
                } else if volume == 1.0 {
                    assert!(
                        peak < HEADROOM_CEILING,
                        "{label}, depth={depth}, peak={peak}"
                    );
                    assert!(rms > 0.01, "{label}, depth={depth}, rms={rms}");
                }
            }
        }
    }
}
