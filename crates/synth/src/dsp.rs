pub fn robotic_chain(samples: &mut [f32], depth: f32) {
    if samples.is_empty() {
        return;
    }
    let depth = depth.clamp(0.0, 1.0);
    let sr = crate::params::SAMPLE_RATE as f32;

    let ring_depth = 0.02 + 0.10 * depth;
    let ring_freq = 38.0 + 14.0 * depth;
    let ring_phase = 2.0 * std::f32::consts::PI * ring_freq / sr;

    let mut phase = 0.0f32;
    for sample in samples.iter_mut() {
        let am = 1.0 + ring_depth * phase.sin();
        *sample *= am;
        phase += ring_phase;
        if phase > std::f32::consts::TAU {
            phase -= std::f32::consts::TAU;
        }
    }

    let peak = samples
        .iter()
        .fold(0.0f32, |acc, s| acc.max(s.abs()))
        .max(1e-6);
    let gain = 1.35 / peak;
    let drive = 1.0 + 0.6 * depth;
    for sample in samples.iter_mut() {
        *sample = (*sample * gain * drive).tanh();
    }

    let peak2 = samples
        .iter()
        .fold(0.0f32, |acc, s| acc.max(s.abs()))
        .max(1e-6);
    let norm = 0.82 / peak2;
    for sample in samples.iter_mut() {
        *sample *= norm;
    }

    let fade = (0.006 * sr) as usize;
    let len = samples.len();
    for (i, sample) in samples.iter_mut().enumerate() {
        if i < fade {
            *sample *= i as f32 / fade as f32;
        }
        let from_end = len - 1 - i;
        if from_end < fade {
            *sample *= from_end as f32 / fade as f32;
        }
    }
}

pub fn apply_volume(samples: &mut [f32], volume: f32) {
    let gain = volume.clamp(0.0, 2.0);
    for sample in samples.iter_mut() {
        *sample *= gain;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_output() {
        let mut samples = vec![0.0f32; 1000];
        for (i, s) in samples.iter_mut().enumerate() {
            *s = (i as f32 * 0.001).sin() * 0.8;
        }
        robotic_chain(&mut samples, 0.6);
        assert!(samples.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
        let peak = samples.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
        assert!(peak <= 0.85, "peak={peak}");
        assert!(peak >= 0.70, "peak={peak}");
    }

    #[test]
    fn fades_edges() {
        let mut samples = vec![0.5f32; 1000];
        robotic_chain(&mut samples, 0.5);
        assert!(samples[0].abs() < 0.01);
        assert!(samples[999].abs() < 0.01);
        assert!(samples[500].abs() > 0.1);
    }
}
