use crate::prosody::Segment;

struct Bandpass {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Bandpass {
    fn new(freq: f32, bw: f32, sr: f32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * freq / sr;
        let q = (freq / bw.max(1.0)).clamp(0.5, 20.0);
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self {
            b0: alpha / a0,
            b1: 0.0,
            b2: -alpha / a0,
            a1: -2.0 * w0.cos() / a0,
            a2: (1.0 - alpha) / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn tick(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

struct Lowpass {
    coeff: f32,
    state: f32,
}

impl Lowpass {
    fn new(cutoff: f32, sr: f32) -> Self {
        let dt = 1.0 / sr;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff.max(1.0));
        Self {
            coeff: dt / (rc + dt),
            state: 0.0,
        }
    }

    fn tick(&mut self, x: f32) -> f32 {
        self.state += self.coeff * (x - self.state);
        self.state
    }
}

struct Noise {
    state: u32,
}

impl Noise {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> f32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        ((self.state >> 8) as f32 / 8_388_608.0) * 2.0 - 1.0
    }
}

struct Glottal {
    phase: f32,
    period: f32,
    pulse_left: f32,
    pulse_samples: f32,
}

impl Glottal {
    fn new() -> Self {
        Self {
            phase: 0.0,
            period: 0.0,
            pulse_left: 0.0,
            pulse_samples: 0.0,
        }
    }

    fn set_f0(&mut self, f0: f32, sr: f32) {
        let new_period = (sr / f0.max(40.0)).max(2.0);
        if (new_period - self.period).abs() > 0.5 {
            self.period = new_period;
        }
    }

    fn tick(&mut self, sr: f32) -> f32 {
        if self.pulse_left > 0.0 {
            self.pulse_left -= 1.0;
            let t = 1.0 - self.pulse_left / self.pulse_samples;
            return (t * std::f32::consts::PI).sin().max(0.0);
        }
        self.phase += 1.0;
        if self.phase >= self.period {
            self.phase = 0.0;
            self.pulse_samples = (0.0012 * sr).max(1.0);
            self.pulse_left = self.pulse_samples;
            return 1.0;
        }
        0.0
    }
}

struct RenderState {
    f1: Bandpass,
    f2: Bandpass,
    f3: Bandpass,
    fric: Bandpass,
    glottal_lp: Lowpass,
    nasal_lp: Lowpass,
    fric_lp: Lowpass,
    noise: Noise,
    glottal: Glottal,
    cur_f1: f32,
    cur_f2: f32,
    cur_f3: f32,
    cur_b1: f32,
    cur_b2: f32,
    cur_b3: f32,
    cur_voice_amp: f32,
    cur_noise_amp: f32,
    cur_noise_freq: f32,
    cur_nasal: f32,
}

impl RenderState {
    fn new(sr: f32) -> Self {
        Self {
            f1: Bandpass::new(500.0, 100.0, sr),
            f2: Bandpass::new(1500.0, 150.0, sr),
            f3: Bandpass::new(2500.0, 200.0, sr),
            fric: Bandpass::new(2500.0, 800.0, sr),
            glottal_lp: Lowpass::new(400.0, sr),
            nasal_lp: Lowpass::new(1400.0, sr),
            fric_lp: Lowpass::new(9000.0, sr),
            noise: Noise::new(0x5eed_1234),
            glottal: Glottal::new(),
            cur_f1: 500.0,
            cur_f2: 1500.0,
            cur_f3: 2500.0,
            cur_b1: 100.0,
            cur_b2: 150.0,
            cur_b3: 200.0,
            cur_voice_amp: 0.0,
            cur_noise_amp: 0.0,
            cur_noise_freq: 2500.0,
            cur_nasal: 0.0,
        }
    }

    fn update_targets(&mut self, seg: &Segment, blend: f32) {
        let lerp = |a: f32, b: f32| a + (b - a) * blend;
        self.cur_f1 = lerp(self.cur_f1, seg.f1);
        self.cur_f2 = lerp(self.cur_f2, seg.f2);
        self.cur_f3 = lerp(self.cur_f3, seg.f3);
        self.cur_b1 = lerp(self.cur_b1, seg.b1);
        self.cur_b2 = lerp(self.cur_b2, seg.b2);
        self.cur_b3 = lerp(self.cur_b3, seg.b3);
        self.cur_voice_amp = lerp(self.cur_voice_amp, seg.voice_amp);
        self.cur_noise_amp = lerp(self.cur_noise_amp, seg.noise_amp);
        self.cur_noise_freq = lerp(self.cur_noise_freq, seg.noise_freq);
        self.cur_nasal = lerp(self.cur_nasal, seg.nasal);
        self.f1 = Bandpass::new(self.cur_f1, self.cur_b1, crate::params::SAMPLE_RATE as f32);
        self.f2 = Bandpass::new(self.cur_f2, self.cur_b2, crate::params::SAMPLE_RATE as f32);
        self.f3 = Bandpass::new(self.cur_f3, self.cur_b3, crate::params::SAMPLE_RATE as f32);
        self.fric = Bandpass::new(
            self.cur_noise_freq.max(200.0),
            800.0,
            crate::params::SAMPLE_RATE as f32,
        );
    }

    fn tick(&mut self, sr: f32, seg: &Segment, t: f32) -> f32 {
        self.update_targets(seg, t);
        self.glottal.set_f0(seg.f0, sr);

        let mut voice = self.glottal.tick(sr);
        voice = self.glottal_lp.tick(voice);
        let mut voiced = self.f1.tick(voice * self.cur_voice_amp);
        voiced = self.f2.tick(voiced);
        voiced = self.f3.tick(voiced);

        if self.cur_nasal > 0.3 {
            voiced = self.nasal_lp.tick(voiced) * 0.9 + voiced * 0.35;
        }

        let noise_in = self.noise.next();
        let noise_f = self.fric.tick(noise_in);
        let noise_f = self.fric_lp.tick(noise_f);
        let breath = noise_in * 0.02;

        let burst = seg
            .burst
            .map(|freq| {
                let mut bp = Bandpass::new(freq, 700.0, sr);
                let b = bp.tick(noise_in) * self.cur_noise_amp.max(0.4);
                b * (1.0 - t).max(0.0)
            })
            .unwrap_or(0.0);

        voiced * 0.55 + noise_f * self.cur_noise_amp * 0.5 + breath + burst
    }
}

pub fn render_segments(segments: &[Segment]) -> Vec<f32> {
    let sr = crate::params::SAMPLE_RATE as f32;
    let total: usize = segments.iter().map(|s| (s.duration_s * sr) as usize).sum();
    let mut out = Vec::with_capacity(total);
    let mut state = RenderState::new(sr);

    for seg in segments {
        let n = (seg.duration_s * sr) as usize;
        let transition = 0.025;
        for i in 0..n {
            let t_in = i as f32 / n as f32;
            let t = if seg.duration_s < transition * 2.0 {
                t_in
            } else {
                (t_in * seg.duration_s / transition).min(1.0)
            };
            let sample = state.tick(sr, seg, t);
            out.push(sample);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params;
    use crate::prosody::{build_prosody, Segment, SynthOptions};
    use master_voice_linguistics::phoneme::{Phoneme, PhonemeKind};

    fn vowel_segment() -> Segment {
        let target = params::target_for(PhonemeKind::AA);
        Segment {
            f1: target.f1,
            f2: target.f2,
            f3: target.f3,
            b1: target.b1,
            b2: target.b2,
            b3: target.b3,
            voicing: 1.0,
            voice_amp: 1.0,
            noise_amp: 0.0,
            noise_freq: 0.0,
            nasal: 0.0,
            f0: 118.0,
            duration_s: 0.2,
            burst: None,
        }
    }

    #[test]
    fn renders_finite_samples() {
        let samples = render_segments(&[vowel_segment()]);
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
        let prosody = build_prosody(&phonemes, &SynthOptions::default());
        let a = render_segments(&prosody.segments);
        let b = render_segments(&prosody.segments);
        assert_eq!(a, b);
    }
}
