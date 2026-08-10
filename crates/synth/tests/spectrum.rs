use master_voice_linguistics::phoneme::{Boundary, Phoneme, PhonemeKind, Stress};
use master_voice_synth::{synthesize, SynthOptions, SAMPLE_RATE};

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

fn goertzel(samples: &[f32], frequency: f32) -> f32 {
    let omega = std::f32::consts::TAU * frequency / SAMPLE_RATE as f32;
    let coefficient = 2.0 * omega.cos();
    let mut previous = 0.0;
    let mut previous_two = 0.0;
    for sample in samples {
        let current = sample + coefficient * previous - previous_two;
        previous_two = previous;
        previous = current;
    }
    (previous_two * previous_two + previous * previous - coefficient * previous * previous_two)
        / samples.len().max(1) as f32
}

fn band_power(samples: &[f32], low: f32, high: f32) -> f32 {
    let mut sum = 0.0;
    let mut count = 0usize;
    let mut frequency = low;
    while frequency <= high {
        sum += goertzel(samples, frequency);
        count += 1;
        frequency += 25.0;
    }
    sum / count.max(1) as f32
}

fn estimate_f0(samples: &[f32]) -> f32 {
    let mean = samples.iter().sum::<f32>() / samples.len().max(1) as f32;
    let minimum_lag = (SAMPLE_RATE as f32 / 170.0) as usize;
    let maximum_lag = (SAMPLE_RATE as f32 / 75.0) as usize;
    let mut best_lag = minimum_lag;
    let mut best = f32::NEG_INFINITY;
    for lag in minimum_lag..=maximum_lag.min(samples.len().saturating_sub(1)) {
        let score = samples[..samples.len() - lag]
            .iter()
            .zip(&samples[lag..])
            .map(|(left, right)| (left - mean) * (right - mean))
            .sum::<f32>();
        if score > best {
            best = score;
            best_lag = lag;
        }
    }
    SAMPLE_RATE as f32 / best_lag as f32
}

fn steady_vowel(kind: PhonemeKind, depth: f32, pitch: f32) -> Vec<f32> {
    let buffer = synthesize(
        &[phone(kind, Boundary::None)],
        &SynthOptions {
            pitch,
            robotic_depth: depth,
            ..SynthOptions::default()
        },
    );
    let edge = (0.020 * SAMPLE_RATE as f32) as usize;
    buffer.samples[edge..buffer.samples.len() - edge].to_vec()
}

fn formant_centroid(samples: &[f32], target: f32) -> f32 {
    let low = target * 0.85;
    let high = target * 1.15;
    let mut weighted = 0.0;
    let mut total = 0.0;
    let mut frequency = low;
    while frequency <= high {
        let power = goertzel(samples, frequency).max(0.0);
        weighted += frequency * power;
        total += power;
        frequency += 10.0;
    }
    weighted / total.max(f32::MIN_POSITIVE)
}

#[test]
fn default_f0_and_user_pitch_follow_public_contract() {
    let baseline = steady_vowel(PhonemeKind::IY, 0.0, 1.0);
    let center = &baseline[baseline.len() / 3..baseline.len() * 2 / 3];
    let f0 = estimate_f0(center);
    assert!((105.0..=120.0).contains(&f0), "default F0 {f0}");

    let raised = steady_vowel(PhonemeKind::IY, 0.0, 1.25);
    let raised = estimate_f0(&raised[raised.len() / 3..raised.len() * 2 / 3]);
    assert!(
        (1.18..1.32).contains(&(raised / f0)),
        "pitch ratio {}",
        raised / f0
    );
}

#[test]
fn question_tail_rises_twenty_percent_above_declarative() {
    let kinds = [
        PhonemeKind::DH,
        PhonemeKind::AX,
        PhonemeKind::V,
        PhonemeKind::OI,
    ];
    let render = |boundary, pause_samples: usize| {
        let mut phones: Vec<_> = kinds
            .into_iter()
            .map(|kind| phone(kind, Boundary::None))
            .collect();
        phones.last_mut().expect("phone").boundary_after = boundary;
        let buffer = synthesize(&phones, &SynthOptions::default());
        let speech_end = buffer.samples.len() - pause_samples;
        estimate_f0(&buffer.samples[speech_end - 1400..speech_end - 200])
    };
    let declarative = render(Boundary::Sentence, 5280);
    let question = render(Boundary::Question, 4320);
    assert!(
        question >= declarative * 1.20,
        "declarative {declarative}, question {question}"
    );
}

#[test]
fn vowel_formants_are_realized_and_character_invariant() {
    for (kind, first, second) in [
        (PhonemeKind::IY, 300.0, 2300.0),
        (PhonemeKind::AA, 720.0, 1100.0),
    ] {
        let plain = steady_vowel(kind, 0.0, 1.0);
        let robotic = steady_vowel(kind, 1.0, 1.0);
        let plain_first = formant_centroid(&plain, first);
        let plain_second = formant_centroid(&plain, second);
        let robot_first = formant_centroid(&robotic, first);
        let robot_second = formant_centroid(&robotic, second);
        assert!(
            (plain_first / first - 1.0).abs() < 0.15,
            "{kind:?} F1 {plain_first}"
        );
        assert!(
            (plain_second / second - 1.0).abs() < 0.15,
            "{kind:?} F2 {plain_second}"
        );
        assert!(
            (robot_first / plain_first - 1.0).abs() < 0.05,
            "{kind:?} F1 moved: {plain_first} to {robot_first}"
        );
        assert!(
            (robot_second / plain_second - 1.0).abs() < 0.05,
            "{kind:?} F2 moved: {plain_second} to {robot_second}"
        );
        let second_band = band_power(&plain, second * 0.9, second * 1.1);
        let valley = band_power(&plain, second * 0.65, second * 0.75);
        let minimum_ratio = if kind == PhonemeKind::IY { 0.35 } else { 0.20 };
        assert!(
            second_band > valley * minimum_ratio,
            "{kind:?} F2 is starved: band {second_band}, valley {valley}, ratio {}",
            second_band / valley
        );
    }
}

#[test]
fn average_voiced_spectral_tilt_is_between_three_and_seven_db_per_octave() {
    let mut octave_powers = [0.0f32; 3];
    for kind in [PhonemeKind::IY, PhonemeKind::AA, PhonemeKind::UW] {
        let samples = steady_vowel(kind, 0.0, 1.0);
        octave_powers[0] += band_power(&samples, 300.0, 600.0);
        octave_powers[1] += band_power(&samples, 600.0, 1200.0);
        octave_powers[2] += band_power(&samples, 1200.0, 2400.0);
    }
    let low_to_mid = 10.0 * (octave_powers[1] / octave_powers[0]).log10();
    let mid_to_high = 10.0 * (octave_powers[2] / octave_powers[1]).log10();
    let tilt = (low_to_mid + mid_to_high) * 0.5;
    assert!(
        (-7.0..=-3.0).contains(&tilt),
        "spectral tilt {tilt} dB/octave"
    );
}
