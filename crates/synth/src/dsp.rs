//! Post chain: presence shelf, ring modulator, make-up gain, fades.
//! Streaming-safe: `PostState` carries the ring carrier phase and the
//! presence shelf state across chunks, and nothing needs the whole buffer.

use crate::character;
use crate::prosody::ChunkPos;

/// State carried across chunks so the 62 Hz carrier and the shelf filter
/// stay continuous — without it every chunk boundary clicks.
#[derive(Default)]
pub struct PostState {
    ring_phase: f32,
    presence_lp: f32,
}

/// Apply the replicant post chain. Order of operations, exactly:
/// 1. presence shelf (always on, NOT depth-scaled — intelligibility),
/// 2. ring modulation (character, skipped below depth 0.02 but the phase
///    still advances so toggling depth never clicks),
/// 3. fixed make-up gain + soft knee + hard clamp (no peak normalisation,
///    no `tanh` — normalisation needs the whole buffer, and `tanh` was the
///    main consonant killer),
/// 4. 6 ms fades at the global utterance edges only.
pub fn post_chain(samples: &mut [f32], depth: f32, state: &mut PostState, pos: ChunkPos) {
    let depth = depth.clamp(0.0, 1.0);
    if samples.is_empty() {
        return;
    }
    let sr = crate::params::SAMPLE_RATE as f32;

    // 1. Presence shelf: y = x + PRESENCE_GAIN * (x - lp(x)).
    let lp_a = 1.0 - (-2.0 * std::f32::consts::PI * character::PRESENCE_HZ / sr).exp();
    for s in samples.iter_mut() {
        state.presence_lp += lp_a * (*s - state.presence_lp);
        *s = *s + character::PRESENCE_GAIN * (*s - state.presence_lp);
    }

    // 2. Ring modulation.
    let wet = (character::RING_WET_MAX * depth).min(0.25);
    let inc = std::f32::consts::TAU * character::RING_HZ / sr;
    if depth < 0.02 {
        samples.iter_mut().for_each(|_| {
            state.ring_phase += inc;
            if state.ring_phase >= std::f32::consts::TAU {
                state.ring_phase -= std::f32::consts::TAU;
            }
        });
    } else {
        for s in samples.iter_mut() {
            let carrier = state.ring_phase.sin();
            *s = *s * (1.0 - wet) + *s * carrier * wet;
            state.ring_phase += inc;
            if state.ring_phase >= std::f32::consts::TAU {
                state.ring_phase -= std::f32::consts::TAU;
            }
        }
    }

    // 3. Make-up gain, soft knee, hard clamp.
    for s in samples.iter_mut() {
        let v = *s * character::OUT_GAIN;
        let a = v.abs();
        *s = if a > 0.9 {
            v.signum() * (0.9 + (a - 0.9) * 0.3)
        } else {
            v
        };
        *s = s.clamp(-1.0, 1.0);
    }

    // 4. Fades: 6 ms at the global utterance edges only.
    let fade = (0.006 * sr) as usize;
    let len = samples.len();
    if pos.first {
        for (i, s) in samples.iter_mut().enumerate() {
            if i < fade {
                *s *= i as f32 / fade as f32;
            }
        }
    }
    if pos.last {
        for (i, s) in samples.iter_mut().enumerate() {
            let from_end = len - 1 - i;
            if from_end < fade {
                *s *= from_end as f32 / fade as f32;
            }
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
        let mut samples: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.001).sin() * 0.25).collect();
        post_chain(
            &mut samples,
            0.6,
            &mut PostState::default(),
            ChunkPos {
                first: true,
                last: true,
            },
        );
        assert!(samples.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
        let peak = samples.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
        assert!(peak >= 0.10, "peak={peak}");
    }

    #[test]
    fn fades_edges() {
        let mut samples = vec![0.5f32; 1000];
        post_chain(
            &mut samples,
            0.5,
            &mut PostState::default(),
            ChunkPos {
                first: true,
                last: true,
            },
        );
        assert!(samples[0].abs() < 0.01);
        assert!(samples[999].abs() < 0.01);
        assert!(samples[500].abs() > 0.1);
    }

    #[test]
    fn no_fade_when_not_first() {
        let mut samples = vec![0.5f32; 1000];
        post_chain(
            &mut samples,
            0.5,
            &mut PostState::default(),
            ChunkPos {
                first: false,
                last: true,
            },
        );
        assert!(samples[0].abs() > 0.3, "leading samples must not be faded");
        assert!(samples[999].abs() < 0.01, "trailing fade stays");
    }

    #[test]
    fn ring_phase_advances_when_skipped() {
        let mut state = PostState::default();
        let mut a = vec![0.1f32; 500];
        let mut b = vec![0.1f32; 500];
        post_chain(
            &mut a,
            0.0,
            &mut state,
            ChunkPos {
                first: true,
                last: false,
            },
        );
        let phase_after = state.ring_phase;
        post_chain(
            &mut b,
            1.0,
            &mut state,
            ChunkPos {
                first: false,
                last: true,
            },
        );
        // The phase must have advanced through the first chunk, so the
        // second chunk's carrier is not phase-0 (no click when depth jumps).
        assert!(phase_after > 0.0);
    }
}
