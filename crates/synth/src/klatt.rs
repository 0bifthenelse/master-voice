//! Stateful Klatt-style cascade + parallel formant renderer with a
//! replicant twin glottal source.
//!
//! The renderer is a **struct** so an utterance can be rendered in pieces
//! (chunked streaming) without resetting filter state, glottal phase or the
//! LCGs: `render` interpolates per sample between the current and the next
//! frame and pushes frequencies into the resonators via `set`, which never
//! touches filter state (V1).

use crate::character;
use crate::frame::{Frame, FRAME_SAMPLES};
use crate::params::SAMPLE_RATE;

/// Two-pole resonator (Klatt 1980). `set` assigns only the coefficients —
/// `y1`/`y2` state survives, so formant movement is continuous.
pub struct Resonator {
    a: f32,
    b: f32,
    c: f32,
    y1: f32,
    y2: f32,
}

impl Resonator {
    fn new() -> Self {
        // Identity: y = x. Used as pass-through until `set` is called.
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    pub fn set(&mut self, freq: f32, bw: f32, sr: f32) {
        let r = (-std::f32::consts::PI * bw.max(20.0) / sr).exp();
        self.c = -(r * r);
        self.b = 2.0 * r * (2.0 * std::f32::consts::PI * freq.clamp(80.0, sr * 0.47) / sr).cos();
        self.a = 1.0 - self.b - self.c;
    }
    fn reset_output(&mut self, sample: f32) {
        self.y1 = sample;
        self.y2 = sample;
    }

    fn tick(&mut self, x: f32) -> f32 {
        let y = self.a * x + self.b * self.y1 + self.c * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// FIR anti-resonator (nasal zero). `set` installs the bounded canonical
/// zero coefficients; state is reset only at nasal/oral transitions.
struct AntiResonator {
    a: f32,
    b: f32,
    c: f32,
    x1: f32,
    x2: f32,
}

impl AntiResonator {
    fn new() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            x1: 0.0,
            x2: 0.0,
        }
    }

    fn set(&mut self, freq: f32, bw: f32, sr: f32) {
        let r = (-std::f32::consts::PI * bw.max(20.0) / sr).exp();
        let omega = 2.0 * std::f32::consts::PI * freq.clamp(80.0, sr * 0.47) / sr;
        self.a = 1.0;
        self.b = -2.0 * r * omega.cos();
        self.c = r * r;
    }

    fn reset_input(&mut self, sample: f32) {
        self.x1 = sample;
        self.x2 = sample;
    }

    fn tick(&mut self, x: f32) -> f32 {
        let y = self.a * x + self.b * self.x1 + self.c * self.x2;
        self.x2 = self.x1;
        self.x1 = x;
        y
    }
}

/// One-pole lowpass.
struct OnePole {
    coeff: f32,
    state: f32,
}

impl OnePole {
    fn new(cutoff: f32, sr: f32) -> Self {
        let coeff = 1.0 - (-2.0 * std::f32::consts::PI * cutoff.max(1.0) / sr).exp();
        Self { coeff, state: 0.0 }
    }

    fn tick(&mut self, x: f32) -> f32 {
        self.state += self.coeff * (x - self.state);
        self.state
    }
}

/// Deterministic LCG noise, `-1..1`.
struct Lcg {
    state: u32,
}

impl Lcg {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn unit(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.state >> 8) as f32 / 8_388_608.0
    }

    fn next(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

/// One glottal pulse train at a fixed frequency.
struct Pulse {
    phase: f32,
    period: f32,
    t1: f32,
    t2: f32,
}

impl Pulse {
    fn new() -> Self {
        Self {
            phase: 0.0,
            period: 0.0,
            t1: 0.0,
            t2: 0.0,
        }
    }

    fn tick(&mut self, sr: f32, f0: f32, oq: f32, jitter: f32, lcg: &mut Lcg) -> f32 {
        if self.phase >= self.period {
            // Period recomputed at every pulse start (no hysteresis gate).
            let mut t0 = sr / f0.max(40.0);
            t0 *= 1.0 + jitter * (lcg.unit() - 0.5);
            self.period = t0.max(2.0);
            // Rosenberg two-cosine pulse. The opening/closing split is
            // 0.82/0.18 of the open phase (not 0.71/0.29): the shape's
            // first null sits at 1/(open phase) = f0/OQ and the closure
            // kink at 1/t2; with the 0.29 split both cannot be right at
            // once (null inside the vowel range when the kink clears F1).
            self.t1 = oq * 0.82 * self.period;
            self.t2 = oq * 0.18 * self.period;
            self.phase = 0.0;
        }
        // Rosenberg two-cosine pulse.
        let v = if self.phase < self.t1 {
            0.5 * (1.0 - (std::f32::consts::PI * self.phase / self.t1.max(0.5)).cos())
        } else if self.phase < self.t1 + self.t2 {
            (std::f32::consts::PI * (self.phase - self.t1) / (2.0 * self.t2.max(0.5))).cos()
        } else {
            0.0
        };
        self.phase += 1.0;
        v
    }
}

/// Voicing source: the replicant twin. Two Rosenberg trains detuned by
/// `DETUNE_CENTS`, beating slowly; the twin passes through the same
/// formant filters, so it cannot move a formant.
struct Voice {
    primary: Pulse,
    twin: Pulse,
    tilt: OnePole,
    /// Source-balance shelf: y = x + GAIN * (x - lp(x, corner)). Lifts the
    /// pulse's 1/f^2 rolloff above the corner so F2/F3 harmonics are not
    /// buried 25-40 dB below F1 (D3: vowel F2s measured -10..-38 dB).
    shelf: OnePole,
    lcg: Lcg,
    /// First-order pre-emphasis state: y = x + K*(x - prev). Together with
    /// the shelf this restores the classic Klatt +6 dB/oct source tilt so
    /// the upper formants carry the vowel identity.
    preem_prev: f32,
}

impl Voice {
    fn new(sr: f32) -> Self {
        Self {
            primary: Pulse::new(),
            twin: Pulse::new(),
            tilt: OnePole::new(character::TILT_HZ, sr),
            shelf: OnePole::new(character::SHELF_HZ, sr),
            lcg: Lcg::new(0x9E37_79B9),
            preem_prev: 0.0,
        }
    }

    fn tick(&mut self, sr: f32, f0: f32, oq: f32, depth: f32) -> f32 {
        let mix = (character::DETUNE_MIX_MAX * depth).min(0.50);
        let twin_f0 = f0 * 2f32.powf(character::DETUNE_CENTS / 1200.0);
        let jitter = character::JITTER_MAX * (1.0 - depth);
        let primary = self.primary.tick(sr, f0, oq, jitter, &mut self.lcg);
        let twin = self.twin.tick(sr, twin_f0, oq, jitter, &mut self.lcg);
        let mut voiced = primary * (1.0 - mix) + twin * mix;
        let lp = self.shelf.tick(voiced);
        voiced += character::SHELF_GAIN * (voiced - lp);
        let tilted = self.tilt.tick(voiced);
        let out = tilted - character::PREEMPH_GAIN * self.preem_prev;
        self.preem_prev = tilted;
        out
    }
}

/// The resumable renderer: an utterance can be rendered in pieces without
/// resetting filters, glottal phase or the LCGs (streaming prerequisite).
pub struct Renderer {
    cascade: [Resonator; 5],
    nasal_zero: AntiResonator,
    nasal_pole: Resonator,
    nasal_active: bool,
    parallel: [Resonator; 4],
    voice: Voice,
    noise: Lcg,
    noise_hp: OnePole,
    depth: f32,
    last_freq: [f32; 5],
    last_par: [f32; 4],
    primed: bool,
    prev_frame: Option<Frame>,
    /// Radiation filter state: three cascaded one-pole highpasses at
    /// `character::RAD_HZ` (third-order radiation, -18 dB/oct below the
    /// corner). A single differencer (+6 dB/oct from DC) leaves the
    /// glottal 1/f region ~25 dB above F2/F3, which buries the upper
    /// formants and makes the voice boomy (V2 measures the result).
    rad_lp1: f32,
    rad_lp2: f32,
    rad_lp3: f32,
    rad_a: f32,
}

impl Renderer {
    pub fn new(depth: f32) -> Self {
        let sr = SAMPLE_RATE as f32;
        Self {
            cascade: std::array::from_fn(|_| Resonator::new()),
            nasal_zero: AntiResonator::new(),
            nasal_pole: Resonator::new(),
            nasal_active: false,
            parallel: std::array::from_fn(|_| Resonator::new()),
            voice: Voice::new(sr),
            noise: Lcg::new(0x9E37_79B9),
            noise_hp: OnePole::new(300.0, sr),
            depth: depth.clamp(0.0, 1.0),
            last_freq: [0.0; 5],
            last_par: [0.0; 4],
            primed: false,
            prev_frame: None,
            rad_lp1: 0.0,
            rad_lp2: 0.0,
            rad_lp3: 0.0,
            rad_a: 1.0 - (-2.0 * std::f32::consts::PI * character::RAD_HZ / sr).exp(),
        }
    }

    /// Render `FRAME_SAMPLES` output samples per frame. Each frame's
    /// samples interpolate from the *previous* frame's values (retained
    /// across calls, so chunk boundaries land exactly on a frame's
    /// values) to this frame's values; the first frame of the first call
    /// holds its own values. Calling `render` again continues from the
    /// retained filter state — chunked rendering is bit-identical to a
    /// single call (V4).
    pub fn render(&mut self, frames: &[Frame]) -> Vec<f32> {
        for f in frames {
            debug_assert!(f.f.iter().chain(f.bw.iter()).all(|v| v.is_finite()));
            debug_assert!(f
                .fp
                .iter()
                .chain(f.bp.iter())
                .chain(f.ap.iter())
                .all(|v| v.is_finite()));
            debug_assert!(f.an.is_finite() && f.av.is_finite() && f.af.is_finite());
            debug_assert!(
                f.ah.is_finite() && f.ab.is_finite() && f.oq.is_finite() && f.f0.is_finite()
            );
        }
        if frames.is_empty() {
            return Vec::new();
        }
        if !self.primed {
            self.last_freq = frames[0].f;
            self.last_par = frames[0].fp;
            self.primed = true;
        }

        let sr = SAMPLE_RATE as f32;
        let mut out = Vec::with_capacity(frames.len() * FRAME_SAMPLES);
        for cur in frames {
            let prev = self.prev_frame.unwrap_or(*cur);
            for j in 0..FRAME_SAMPLES {
                // Ends exactly at `cur`'s values, so a chunk boundary is a
                // point where the signal holds the boundary frame's values.
                let u = (j as f32 + 1.0) / FRAME_SAMPLES as f32;
                let l = |a: f32, b: f32| a + (b - a) * u;
                let mut f = Frame::new();
                for k in 0..5 {
                    f.f[k] = l(prev.f[k], cur.f[k]);
                    f.bw[k] = l(prev.bw[k], cur.bw[k]);
                }
                for k in 0..4 {
                    f.fp[k] = l(prev.fp[k], cur.fp[k]);
                    f.bp[k] = l(prev.bp[k], cur.bp[k]);
                    f.ap[k] = l(prev.ap[k], cur.ap[k]);
                }
                f.an = l(prev.an, cur.an);
                f.av = l(prev.av, cur.av);
                f.af = l(prev.af, cur.af);
                f.ah = l(prev.ah, cur.ah);
                f.ab = l(prev.ab, cur.ab);
                f.oq = l(prev.oq, cur.oq);
                f.f0 = l(prev.f0, cur.f0);
                out.push(self.tick(&f, sr));
            }
            self.prev_frame = Some(*cur);
        }
        out
    }

    fn tick(&mut self, f: &Frame, sr: f32) -> f32 {
        // Rate-limit formant movement (guards against table typos); first
        // use of a branch adopts its target directly so onsets stay crisp.
        for k in 0..5 {
            let prev = self.last_freq[k];
            let freq = if prev <= 0.0 {
                f.f[k]
            } else {
                prev + (f.f[k] - prev).clamp(-18.0, 18.0)
            };
            self.last_freq[k] = freq;
            self.cascade[k].set(freq, f.bw[k], sr);
        }
        for k in 0..4 {
            let prev = self.last_par[k];
            let freq = if prev <= 0.0 {
                f.fp[k]
            } else {
                prev + (f.fp[k] - prev).clamp(-18.0, 18.0)
            };
            self.last_par[k] = freq;
            self.parallel[k].set(freq, f.bp[k], sr);
        }

        // Voicing + aspiration into the cascade.
        let voiced = self.voice.tick(sr, f.f0.max(40.0), f.oq, self.depth) * f.av;
        let n = self.noise.next();
        let aspiration = self.noise_hp.tick(n) * f.ah * 0.20;
        let mut x = voiced + aspiration;

        // Nasal pair: pass-through when an == 0, else the nasal pole/zero.
        // The anti-resonator (zero) pair attenuates everything below its
        // notch by ~1 - 2r·cos(ω0) + r² (≈ -35 dB for a near-unit-circle
        // zero at 450 Hz), so routing the whole murmur through it silenced
        // M/N/NG and the French nasal vowels. The murmur therefore comes
        // from the pole on the *direct* input, the oral vowel is kept via
        // the (1 - an) bypass, and the zero is mixed in lightly to shape
        // the antiformant without starving the murmur band.
        let nasal_active = f.an > 0.0;
        if nasal_active {
            self.nasal_zero.set(950.0, 800.0, sr);
            self.nasal_pole.set(280.0, 400.0, sr);
            if !self.nasal_active {
                self.nasal_zero.reset_input(x);
                self.nasal_pole.reset_output(x);
            }
            let z = self.nasal_zero.tick(x);
            let p = self.nasal_pole.tick(x);
            // Murmur (pole) + direct oral + light antiformant; the
            // antiformant must never starve the murmur band (the zero's
            // below-notch shelf would silence it). The differencer cuts
            // the 250-400 Hz murmur ~20 dB, so the pole path is
            // re-amplified — strongly for nasal consonants (an=1) and
            // mildly for nasal vowels (an=0.6), which must not exceed the
            // oral vowels in level.
            x = p * f.an * (1.0 + 2.5 * f.an) + x * (1.0 - f.an) + z * 0.35 * f.an * f.an;
        } else {
            if self.nasal_active {
                self.nasal_zero.reset_input(0.0);
                self.nasal_pole.reset_output(0.0);
            }
            // Bypass both filters while oral.
        }
        self.nasal_active = nasal_active;

        // Cascade R5 -> R4 -> R3 -> R2 -> R1.
        for k in (0..5).rev() {
            x = self.cascade[k].tick(x);
        }

        // Parallel frication branch, alternating-sign phase convention.
        let fric_in = n * f.af;
        let mut para = 0.0;
        for k in 0..4 {
            if f.fp[k] > 0.0 {
                let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
                para += sign * self.parallel[k].tick(fric_in) * f.ap[k];
            }
        }
        para += fric_in * f.ab;

        // Radiation: third-order highpass (three cascaded one-pole
        // highpasses at `character::RAD_HZ`), then the output trim.
        let out = x + para;
        let hp1 = out - self.rad_lp1;
        self.rad_lp1 += self.rad_a * (out - self.rad_lp1);
        let hp2 = hp1 - self.rad_lp2;
        self.rad_lp2 += self.rad_a * (hp1 - self.rad_lp2);
        // Radiation output (no trim: the OUT_GAIN in the post chain is the
        // single level control; vowels land at ~0.6-0.9 after it).
        let y = hp2 - self.rad_lp3;
        self.rad_lp3 += self.rad_a * (hp2 - self.rad_lp3);
        y
    }
}

/// Debug: the raw voiced source (pulse train through tilt/shelf), for
/// spectrum inspection.
pub fn debug_voice(f0: f32, oq: f32, depth: f32, samples: usize) -> Vec<f32> {
    let sr = SAMPLE_RATE as f32;
    let mut voice = Voice::new(sr);
    (0..samples)
        .map(|_| voice.tick(sr, f0, oq, depth))
        .collect()
}

/// Convenience wrapper for non-streaming callers.
pub fn render_frames(frames: &[Frame], depth: f32) -> Vec<f32> {
    Renderer::new(depth).render(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prosody::{build_frames, SynthOptions};
    use master_voice_linguistics::phoneme::{Phoneme, PhonemeKind};

    #[test]
    fn resonator_keeps_state_and_rings() {
        let mut r = Resonator::new();
        r.set(500.0, 50.0, SAMPLE_RATE as f32);
        let mut samples = [0.0f32; 400];
        samples[0] = r.tick(1.0);
        for s in samples.iter_mut().take(20).skip(1) {
            *s = r.tick(0.0);
        }
        let y1_before = r.y1;
        let y2_before = r.y2;
        r.set(600.0, 60.0, SAMPLE_RATE as f32);
        assert_eq!(r.y1, y1_before, "set must not touch y1");
        assert_eq!(r.y2, y2_before, "set must not touch y2");
        for s in samples.iter_mut().skip(20) {
            *s = r.tick(0.0);
        }
        let sign_changes = samples
            .windows(2)
            .filter(|w| (w[0] > 0.0) != (w[1] > 0.0))
            .count();
        assert!(sign_changes >= 6, "only {sign_changes} sign changes");
        assert!(
            samples[200].abs() < samples[20].abs(),
            "decay: {} vs {}",
            samples[200].abs(),
            samples[20].abs()
        );
    }

    #[test]
    fn renders_finite_samples() {
        let frames = build_frames(
            &[
                Phoneme::new(PhonemeKind::H),
                Phoneme::new(PhonemeKind::EH),
                Phoneme::new(PhonemeKind::L),
                Phoneme::new(PhonemeKind::OW),
            ],
            &SynthOptions::default(),
        );
        let samples = render_frames(&frames, 0.0);
        assert!(samples.len() > 1000);
        assert!(samples.iter().all(|s| s.is_finite()));
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        assert!(rms > 0.01, "rms={rms}");
    }

    #[test]
    fn renders_deterministic() {
        let phonemes = [
            Phoneme::new(PhonemeKind::H),
            Phoneme::new(PhonemeKind::EH),
            Phoneme::new(PhonemeKind::L),
            Phoneme::new(PhonemeKind::OW),
        ];
        let frames = build_frames(&phonemes, &SynthOptions::default());
        let a = render_frames(&frames, 0.55);
        let b = render_frames(&frames, 0.55);
        assert_eq!(a, b);
    }

    fn master_voice_online() -> Vec<Phoneme> {
        [
            PhonemeKind::M,
            PhonemeKind::AE,
            PhonemeKind::S,
            PhonemeKind::T,
            PhonemeKind::ER,
            PhonemeKind::V,
            PhonemeKind::OI,
            PhonemeKind::S,
            PhonemeKind::AH,
            PhonemeKind::N,
            PhonemeKind::L,
            PhonemeKind::AI,
            PhonemeKind::N,
        ]
        .into_iter()
        .map(Phoneme::new)
        .collect()
    }

    #[test]
    fn chunked_render_is_bit_identical() {
        let frames = build_frames(&master_voice_online(), &SynthOptions::default());
        assert!(frames.len() > 19, "need ≥20 frames, got {}", frames.len());

        let whole = render_frames(&frames, 0.55);

        let mut r = Renderer::new(0.55);
        let mut chunked = Vec::new();
        chunked.extend(r.render(&frames[..7]));
        chunked.extend(r.render(&frames[7..19]));
        chunked.extend(r.render(&frames[19..]));

        assert_eq!(whole, chunked);
    }

    #[test]
    fn chunked_post_chain_is_continuous() {
        use crate::dsp::{post_chain, PostState};
        use crate::prosody::ChunkPos;

        let frames = build_frames(&master_voice_online(), &SynthOptions::default());
        let mut whole = render_frames(&frames, 0.55);
        post_chain(
            &mut whole,
            0.55,
            1.0,
            &mut PostState::default(),
            ChunkPos {
                first: true,
                last: true,
            },
        );

        let mut chunks = render_frames(&frames, 0.55);
        let split = (whole.len() / 3).max(1);
        let mut part: Vec<f32> = chunks.drain(split..).collect();
        let rest: Vec<f32> = part.split_off(split);
        let (mut a, mut b) = (chunks, part);
        let mut c = rest;
        let mut state = PostState::default();
        post_chain(
            &mut a,
            0.55,
            1.0,
            &mut state,
            ChunkPos {
                first: true,
                last: false,
            },
        );
        post_chain(
            &mut b,
            0.55,
            1.0,
            &mut state,
            ChunkPos {
                first: false,
                last: false,
            },
        );
        post_chain(
            &mut c,
            0.55,
            1.0,
            &mut state,
            ChunkPos {
                first: false,
                last: true,
            },
        );
        let mut chunked = a;
        chunked.extend(b);
        chunked.extend(c);

        // Fades apply only at the global utterance edges, so every sample
        // must match the single-shot call.
        assert_eq!(whole, chunked);
    }
    #[test]
    fn nasal_oral_transitions_keep_raw_renderer_bounded() {
        let mut oral = Frame::new();
        oral.f = [500.0, 1_500.0, 2_500.0, 3_500.0, 4_500.0];
        oral.bw = [60.0, 90.0, 120.0, 180.0, 250.0];
        oral.av = 0.8;

        let mut nasal = oral;
        nasal.an = 0.8;
        let trace = [
            ("oral onset", oral),
            ("nasal onset", nasal),
            ("nasal sustain", nasal),
            ("oral release", oral),
            ("nasal re-entry", nasal),
        ];

        let mut renderer = Renderer::new(0.55);
        for (label, frame) in trace {
            let samples = renderer.render(&[frame]);
            if let Some((index, sample)) = samples
                .iter()
                .copied()
                .enumerate()
                .find(|(_, sample)| !sample.is_finite() || sample.abs() >= 4.0)
            {
                eprintln!(
                    "{label}: first offending operation Renderer::render/tick at sample \
                     {index}: {sample}"
                );
                panic!("{label}: raw renderer exceeded safety bound");
            }
            let peak = samples
                .iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
            assert!(peak < 4.0, "{label}: raw peak={peak}");
        }
    }

    #[test]
    fn legal_filter_grid_stays_finite_and_bounded() {
        let sr = SAMPLE_RATE as f32;
        for (freq, bandwidth) in [
            (80.0, 50.0),
            (450.0, 300.0),
            (1_500.0, 135.0),
            (3_300.0, 232.0),
            (3_750.0, 250.0),
            (5_000.0, 800.0),
            (sr * 0.47, 800.0),
        ] {
            let mut resonator = Resonator::new();
            resonator.set(freq, bandwidth, sr);
            let mut anti_resonator = AntiResonator::new();
            anti_resonator.set(freq, bandwidth, sr);

            for sample_index in 0..2_048 {
                let input = if sample_index == 0 { 1.0 } else { 0.0 };
                let pole = resonator.tick(input);
                let zero = anti_resonator.tick(input);
                assert!(
                    pole.is_finite() && pole.abs() < 16.0,
                    "pole freq={freq}, bandwidth={bandwidth}, sample={sample_index}, value={pole}"
                );
                assert!(
                    zero.is_finite() && zero.abs() < 4.0,
                    "zero freq={freq}, bandwidth={bandwidth}, sample={sample_index}, value={zero}"
                );
            }
        }
    }
}
