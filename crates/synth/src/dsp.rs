//! Post chain: presence shelf, ring modulator, make-up gain, fades.
//! Streaming-safe: `PostState` carries the ring carrier phase and the
//! presence shelf state across chunks, and nothing needs the whole buffer.

use crate::character;
use crate::prosody::ChunkPos;
const SYNTH_HEADROOM_CEILING: f32 = 0.95;

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
/// 3. fixed make-up gain + clamped user volume + soft knee + final hard
///    headroom ceiling (no peak normalisation and no `tanh`),
/// 4. 6 ms fades at the global utterance edges only.
pub fn post_chain(
    samples: &mut [f32],
    depth: f32,
    volume: f32,
    state: &mut PostState,
    pos: ChunkPos,
) {
    let depth = depth.clamp(0.0, 1.0);
    let volume = volume.clamp(0.0, 2.0);
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

    // 3. Make-up gain and user volume feed one final safety stage.
    for s in samples.iter_mut() {
        let v = *s * character::OUT_GAIN * volume;
        if !v.is_finite() {
            *s = 0.0;
            continue;
        }
        let a = v.abs();
        let limited = if a > 0.9 {
            v.signum() * (0.9 + (a - 0.9) * 0.3)
        } else {
            v
        };
        *s = limited.clamp(-SYNTH_HEADROOM_CEILING, SYNTH_HEADROOM_CEILING);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_output() {
        let mut samples: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.001).sin() * 0.25).collect();
        post_chain(
            &mut samples,
            0.6,
            1.0,
            &mut PostState::default(),
            ChunkPos {
                first: true,
                last: true,
            },
        );
        assert!(samples
            .iter()
            .all(|s| s.is_finite() && s.abs() <= SYNTH_HEADROOM_CEILING));
        let peak = samples.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
        assert!(peak > 0.10 * character::OUT_GAIN, "peak={peak}");
    }

    #[test]
    fn fades_edges() {
        let mut samples = vec![0.5f32; 1000];
        post_chain(
            &mut samples,
            0.5,
            1.0,
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
        let mut faded = vec![0.5f32; 1000];
        post_chain(
            &mut faded,
            0.5,
            1.0,
            &mut PostState::default(),
            ChunkPos {
                first: true,
                last: true,
            },
        );
        let mut plain = vec![0.5f32; 1000];
        post_chain(
            &mut plain,
            0.5,
            1.0,
            &mut PostState::default(),
            ChunkPos {
                first: false,
                last: true,
            },
        );
        assert!(faded[0].abs() < 0.01, "leading fade must apply when first");
        assert!(
            plain[0].abs() > 0.4 * character::OUT_GAIN,
            "leading samples must not be faded: {}",
            plain[0]
        );
        assert!(plain[999].abs() < 0.01, "trailing fade stays");
    }

    #[test]
    fn ring_phase_advances_when_skipped() {
        let mut state = PostState::default();
        let mut a = vec![0.1f32; 500];
        let mut b = vec![0.1f32; 500];
        post_chain(
            &mut a,
            0.0,
            1.0,
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
    #[test]
    fn final_barrier_silences_non_finite_samples() {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut samples = [invalid];
            post_chain(
                &mut samples,
                0.55,
                1.0,
                &mut PostState::default(),
                ChunkPos {
                    first: false,
                    last: false,
                },
            );
            assert_eq!(samples, [0.0]);
        }
    }

    #[test]
    fn volume_precedes_the_final_headroom_ceiling() {
        let mut samples = [10.0, -10.0];
        post_chain(
            &mut samples,
            0.0,
            2.0,
            &mut PostState::default(),
            ChunkPos {
                first: false,
                last: false,
            },
        );
        assert_eq!(samples, [SYNTH_HEADROOM_CEILING, -SYNTH_HEADROOM_CEILING]);
    }
}
