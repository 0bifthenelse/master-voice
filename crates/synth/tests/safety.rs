use master_voice_linguistics::phoneme::{Boundary, Phoneme, PhonemeKind, Stress};
use master_voice_synth::{
    render_chunk, synthesize, ChunkPos, SynthOptions, SynthState, SAMPLE_RATE,
};

const ALL_KINDS: [PhonemeKind; 54] = [
    PhonemeKind::IY,
    PhonemeKind::IH,
    PhonemeKind::EH,
    PhonemeKind::EY,
    PhonemeKind::AE,
    PhonemeKind::AA,
    PhonemeKind::AH,
    PhonemeKind::AO,
    PhonemeKind::UH,
    PhonemeKind::UW,
    PhonemeKind::UX,
    PhonemeKind::AX,
    PhonemeKind::ER,
    PhonemeKind::UE,
    PhonemeKind::OE,
    PhonemeKind::OEU,
    PhonemeKind::EN,
    PhonemeKind::AN,
    PhonemeKind::ON,
    PhonemeKind::UN,
    PhonemeKind::EI,
    PhonemeKind::AI,
    PhonemeKind::OI,
    PhonemeKind::OW,
    PhonemeKind::AU,
    PhonemeKind::IA,
    PhonemeKind::EA,
    PhonemeKind::UA,
    PhonemeKind::P,
    PhonemeKind::B,
    PhonemeKind::T,
    PhonemeKind::D,
    PhonemeKind::K,
    PhonemeKind::G,
    PhonemeKind::F,
    PhonemeKind::V,
    PhonemeKind::TH,
    PhonemeKind::DH,
    PhonemeKind::S,
    PhonemeKind::Z,
    PhonemeKind::SH,
    PhonemeKind::ZH,
    PhonemeKind::CH,
    PhonemeKind::JH,
    PhonemeKind::H,
    PhonemeKind::M,
    PhonemeKind::N,
    PhonemeKind::NG,
    PhonemeKind::NY,
    PhonemeKind::L,
    PhonemeKind::R,
    PhonemeKind::RR,
    PhonemeKind::W,
    PhonemeKind::Y,
];

fn phone(kind: PhonemeKind, boundary: Boundary) -> Phoneme {
    let mut phone = Phoneme::new(kind);
    phone.stress = if phone.is_vowel() {
        Stress::Primary
    } else {
        Stress::None
    };
    phone.boundary_after = boundary;
    phone
}

fn speech_rms(samples: &[f32], trailing_pause: usize) -> f32 {
    let speech = &samples[..samples.len().saturating_sub(trailing_pause)];
    (speech.iter().map(|sample| sample * sample).sum::<f32>() / speech.len().max(1) as f32).sqrt()
}

#[test]
fn all_fifty_four_phones_are_finite_bounded_and_audible() {
    let mut levels = Vec::new();
    for kind in ALL_KINDS {
        let buffer = synthesize(&[phone(kind, Boundary::Sentence)], &SynthOptions::default());
        assert_eq!(buffer.sample_rate, SAMPLE_RATE);
        assert!(
            buffer.samples.iter().all(|sample| sample.is_finite()),
            "{kind:?}"
        );
        let peak = buffer
            .samples
            .iter()
            .copied()
            .map(f32::abs)
            .fold(0.0, f32::max);
        assert!(peak < 0.95, "{kind:?}: peak {peak}");
        assert_eq!(
            buffer
                .samples
                .iter()
                .filter(|sample| sample.abs() >= 0.94)
                .count(),
            0,
            "{kind:?}: ceiling hit"
        );
        let rms = speech_rms(&buffer.samples, (0.220 * SAMPLE_RATE as f32) as usize);
        assert!(rms > 0.000_01, "{kind:?}: rms {rms}");
        assert!(rms < 0.4, "{kind:?}: compressed rms {rms}");
        levels.push(rms);
    }
    let minimum = levels.iter().copied().fold(f32::INFINITY, f32::min);
    let maximum = levels.iter().copied().fold(0.0, f32::max);
    assert!(
        maximum / minimum < 250.0,
        "level spread {}",
        maximum / minimum
    );
}

#[test]
fn render_is_deterministic_and_rate_changes_duration() {
    let phones = [
        phone(PhonemeKind::M, Boundary::None),
        phone(PhonemeKind::AE, Boundary::None),
        phone(PhonemeKind::S, Boundary::Sentence),
    ];
    let baseline = synthesize(&phones, &SynthOptions::default());
    let repeated = synthesize(&phones, &SynthOptions::default());
    assert_eq!(baseline, repeated);

    let slow = synthesize(
        &phones,
        &SynthOptions {
            rate: 0.7,
            ..SynthOptions::default()
        },
    );
    let fast = synthesize(
        &phones,
        &SynthOptions {
            rate: 1.5,
            ..SynthOptions::default()
        },
    );
    assert!(slow.samples.len() > baseline.samples.len());
    assert!(baseline.samples.len() > fast.samples.len());
}

#[test]
fn nasals_are_audible_and_fricatives_remain_below_vowels() {
    let vowel_levels: Vec<f32> = [PhonemeKind::IY, PhonemeKind::AA, PhonemeKind::UW]
        .into_iter()
        .map(|kind| {
            let buffer = synthesize(&[phone(kind, Boundary::None)], &SynthOptions::default());
            speech_rms(&buffer.samples, 0)
        })
        .collect();
    let vowel_mean = vowel_levels.iter().sum::<f32>() / vowel_levels.len() as f32;

    for kind in [
        PhonemeKind::EN,
        PhonemeKind::AN,
        PhonemeKind::ON,
        PhonemeKind::UN,
    ] {
        let buffer = synthesize(&[phone(kind, Boundary::None)], &SynthOptions::default());
        assert!(speech_rms(&buffer.samples, 0) > 0.001, "{kind:?}");
    }
    for kind in [PhonemeKind::F, PhonemeKind::S, PhonemeKind::SH] {
        let buffer = synthesize(&[phone(kind, Boundary::None)], &SynthOptions::default());
        let rms = speech_rms(&buffer.samples, 0);
        assert!(
            rms < vowel_mean,
            "{kind:?}: fricative {rms}, vowel {vowel_mean}"
        );
    }
}

#[test]
fn streaming_state_keeps_phrase_join_continuous() {
    let first = [
        phone(PhonemeKind::DH, Boundary::None),
        phone(PhonemeKind::AX, Boundary::Word),
        phone(PhonemeKind::V, Boundary::None),
        phone(PhonemeKind::OI, Boundary::Word),
    ];
    let second = [
        phone(PhonemeKind::S, Boundary::None),
        phone(PhonemeKind::IH, Boundary::None),
        phone(PhonemeKind::S, Boundary::Sentence),
    ];
    let mut state = SynthState::new();
    let first = render_chunk(
        &mut state,
        &first,
        &SynthOptions::default(),
        ChunkPos {
            first: true,
            last: false,
        },
    );
    let second = render_chunk(
        &mut state,
        &second,
        &SynthOptions::default(),
        ChunkPos {
            first: false,
            last: true,
        },
    );
    let discontinuity = (first.samples.last().copied().unwrap_or(0.0)
        - second.samples.first().copied().unwrap_or(0.0))
    .abs();
    assert!(discontinuity < 0.02, "join discontinuity {discontinuity}");
}

#[test]
fn dense_phone_transitions_bound_sample_steps() {
    let mut phones = Vec::with_capacity(ALL_KINDS.len() * 2);
    for kind in ALL_KINDS.into_iter().chain(ALL_KINDS.into_iter().rev()) {
        phones.push(phone(kind, Boundary::None));
    }
    phones.last_mut().expect("phone").boundary_after = Boundary::Sentence;

    let buffer = synthesize(&phones, &SynthOptions::default());
    let max_step = buffer
        .samples
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f32::max);
    assert!(max_step <= 1.0, "maximum sample step {max_step}");
}
