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

/// Radix-2 FFT magnitude spectrum of a windowed segment (test-only; the
/// production path stays dependency-free).
fn fft_magnitude(samples: &[f32]) -> Vec<f32> {
    let n = samples.len().next_power_of_two();
    let mut x: Vec<f32> = samples.to_vec();
    x.resize(n, 0.0);
    // Hann window.
    for (i, v) in x.iter_mut().enumerate().take(samples.len()) {
        *v *=
            0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (samples.len() - 1) as f32).cos();
    }
    // Iterative radix-2 FFT.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            x.swap(i, j);
        }
    }
    let mut re: Vec<f32> = x;
    let mut im = vec![0.0f32; n];
    let mut len = 2usize;
    while len <= n {
        let ang = -2.0 * std::f32::consts::PI / len as f32;
        let (wre, wim) = (ang.cos(), ang.sin());
        let mut half = 0usize;
        while half < n {
            let (mut curre, mut curim) = (1.0f32, 0.0f32);
            for i in 0..len / 2 {
                let (a, b) = (re[half + i], im[half + i]);
                let (c, d) = (re[half + i + len / 2], im[half + i + len / 2]);
                let (xr, xi) = (c * curre - d * curim, c * curim + d * curre);
                re[half + i] = a + xr;
                im[half + i] = b + xi;
                re[half + i + len / 2] = a - xr;
                im[half + i + len / 2] = b - xi;
                let next = curre * wre - curim * wim;
                curim = curre * wim + curim * wre;
                curre = next;
            }
            half += len;
        }
        len <<= 1;
    }
    (0..n / 2)
        .map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt())
        .collect()
}

/// Sample range (inclusive) for the middle fraction of phoneme `index` in
/// a `synthesize`d buffer, computed from the same duration rule as the
/// prosody frame builder (base_ms, no stress, rate 1).
fn mid_window(phonemes: &[Phoneme], index: usize, frac: f32) -> (usize, usize) {
    let sr = master_voice_synth::params::SAMPLE_RATE as f32;
    let frames_per_s = sr / master_voice_synth::frame::FRAME_SAMPLES as f32;
    let mut start = 0usize;
    for (i, p) in phonemes.iter().enumerate() {
        let ms = master_voice_synth::params::spec_for(p.kind).base_ms;
        let n_frames = ((ms / 1000.0) * frames_per_s).round().max(1.0) as usize;
        let n_samples = n_frames * master_voice_synth::frame::FRAME_SAMPLES;
        if i == index {
            let mid = start + n_samples / 2;
            let half = (n_samples as f32 * frac / 2.0) as usize;
            return (mid - half, mid + half);
        }
        start += n_samples;
    }
    (start, start)
}

fn band_energy(mag: &[f32], sr: f32, lo: f32, hi: f32) -> f32 {
    let n = mag.len() * 2;
    let k0 = (lo * n as f32 / sr) as usize;
    let k1 = (hi * n as f32 / sr) as usize;
    mag[k0..k1.min(mag.len())].iter().map(|m| m * m).sum()
}

#[test]
fn vowel_f2_is_realized_not_starved() {
    // /i/-like F2 (2300 Hz) must carry at least 5% of the F1-band energy.
    // Regression: before the source differencer + F1-bandwidth fix the
    // measured F2 was ~38 dB below F1 (-0.15% energy), collapsing all
    // vowels toward "uh".
    let opts = SynthOptions {
        robotic_depth: 0.0,
        ..SynthOptions::default()
    };
    let buffer = master_voice_synth::synthesize(&[Phoneme::new(PhonemeKind::IY)], &opts);
    let (a, b) = mid_window(&[Phoneme::new(PhonemeKind::IY)], 0, 0.5);
    let mag = fft_magnitude(&buffer.samples[a..b]);
    let sr = buffer.sample_rate as f32;
    let f1 = band_energy(&mag, sr, 150.0, 600.0);
    let f2 = band_energy(&mag, sr, 1700.0, 2800.0);
    assert!(f1 > 0.0 && f2 > 0.0, "f1={f1} f2={f2}");
    assert!(
        f2 / f1 > 0.05,
        "IY F2 band only {:.1}% of F1 band (starved F2)",
        100.0 * f2 / f1
    );
}

#[test]
fn nasals_stay_audible() {
    // M's murmur must stay within 24 dB of a neighbouring vowel.
    // Regression: the nasal zero's below-notch shelf silenced M (~-52 dB).
    let opts = SynthOptions {
        robotic_depth: 0.0,
        ..SynthOptions::default()
    };
    let phonemes = [
        Phoneme::new(PhonemeKind::AE),
        Phoneme::new(PhonemeKind::M),
        Phoneme::new(PhonemeKind::AE),
    ];
    let buffer = master_voice_synth::synthesize(&phonemes, &opts);
    let (va, vb) = mid_window(&phonemes, 0, 0.4);
    let (ma, mb) = mid_window(&phonemes, 1, 0.4);
    let rms = |a: usize, b: usize| {
        let s = &buffer.samples[a..b];
        (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    };
    let vrms = rms(va, vb);
    let mrms = rms(ma, mb);
    assert!(vrms > 0.01 && mrms > 0.0);
    assert!(
        mrms / vrms > 0.06,
        "nasal murmur {:.1} dB below vowel (inaudible)",
        20.0 * (mrms / vrms).log10()
    );
}

#[test]
fn fricatives_sit_below_vowels() {
    // Frication must be present but not dominate the vowel level
    // (regression: the parallel noise branch bypassed the differencer and
    // sat 2-4 dB ABOVE the rebalanced vowels).
    let opts = SynthOptions {
        robotic_depth: 0.0,
        ..SynthOptions::default()
    };
    let phonemes = [
        Phoneme::new(PhonemeKind::AE),
        Phoneme::new(PhonemeKind::S),
        Phoneme::new(PhonemeKind::AE),
    ];
    let buffer = master_voice_synth::synthesize(&phonemes, &opts);
    let (va, vb) = mid_window(&phonemes, 0, 0.4);
    let (sa, sb) = mid_window(&phonemes, 1, 0.4);
    let rms = |a: usize, b: usize| {
        let s = &buffer.samples[a..b];
        (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    };
    let ratio = rms(sa, sb) / rms(va, vb);
    assert!(
        ratio > 0.03 && ratio < 0.8,
        "fricative/vowel RMS ratio {ratio:.3} out of [0.03, 0.8]"
    );
}

#[test]
fn vowels_are_level_equalized() {
    // The differencer cuts low-F1 vowels ~15 dB harder than high-F1 ones;
    // per-vowel amplitude equalization must keep the inventory within an
    // 8 dB spread (regression: IY was 15 dB below AA).
    let opts = SynthOptions {
        robotic_depth: 0.0,
        ..SynthOptions::default()
    };
    let mut rmses = Vec::new();
    for kind in [
        PhonemeKind::IY,
        PhonemeKind::IH,
        PhonemeKind::AE,
        PhonemeKind::AA,
        PhonemeKind::UW,
        PhonemeKind::ER,
    ] {
        let phoneme = Phoneme::new(kind);
        let buffer = master_voice_synth::synthesize(&[phoneme], &opts);
        let (a, b) = mid_window(&[phoneme], 0, 0.5);
        let s = &buffer.samples[a..b];
        let rms = (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt();
        rmses.push(rms);
    }
    let min = rmses.iter().copied().fold(f32::INFINITY, f32::min);
    let max = rmses.iter().copied().fold(0.0f32, f32::max);
    assert!(
        max / min < 2.5,
        "vowel RMS spread {:.1} dB (max {max:.4}, min {min:.4})",
        20.0 * (max / min).log10()
    );
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
