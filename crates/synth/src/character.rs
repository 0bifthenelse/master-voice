//! Every tunable of the voice lives here and nowhere else.
//!
//! The REPLICANT character is a *designed layer* over an intelligible
//! formant core: a detuned twin glottal source (+6 cents), a 62 Hz ring
//! modulator and semitone-quantized pitch. No character block ever moves a
//! formant (V3 guards this).

/// Hz, phrase base pitch.
pub const BASE_F0: f32 = 118.0;
/// Shipped character depth: unmistakable character with crisp consonants.
/// The 0.55 default preserves the original formant balance.
pub const DEFAULT_ROBOTIC_DEPTH: f32 = 0.55;
/// Twin source offset, cents.
pub const DETUNE_CENTS: f32 = 6.0;
/// Twin mix, × depth; capped below equal level because 0.50 hollows F1.
pub const DETUNE_MIX_MAX: f32 = 0.45;
/// Ring modulator rate, Hz.
pub const RING_HZ: f32 = 62.0;
/// Ring wet, × depth; hard cap 0.25. 0.16 keeps the sidebands weak
/// enough that the measured formant positions stay within V3's 5%
/// (0.22 pushed AA's F1-window peak onto the F1 skirt); 0.16 is the
/// strongest wet that passed V3 during the 0.82-depth rebalance.
pub const RING_WET_MAX: f32 = 0.16;
/// Glottal period jitter, × (1.0 - depth).
pub const JITTER_MAX: f32 = 0.010;
/// Open quotient at depth 0. The pulse's spectral nulls sit at
/// (f0/OQ)*k; a rational OQ with a small denominator puts a harmonic
/// exactly on a null (0.60 = 3/5 nulls the 5th harmonic always; 0.625 =
/// 5/8 nulls the 8th). 0.64 = 16/25 pushes the first line-null
/// coincidence to the 25th harmonic, so every formant is carried by a
/// live harmonic at any pitch.
pub const OQ_BASE: f32 = 0.64;
/// oq = OQ_BASE - OQ_DEPTH * depth (0: the OQ is depth-independent —
/// varying it moves the pulse nulls across the F1 region and shifts the
/// measured formant positions, breaking V3).
pub const OQ_DEPTH: f32 = 0.0;
/// Source spectral tilt, Hz (one-pole lowpass on the pulse train).
pub const TILT_HZ: f32 = 3000.0;
/// Source-balance shelf corner, Hz: y = x + SHELF_GAIN*(x - lp(x)).
/// Currently unused (gain 0): the OQ-based kink placement balances the
/// source; the shelf remains as a calibration lever.
pub const SHELF_HZ: f32 = 800.0;
/// Source-balance shelf gain (0 = off).
pub const SHELF_GAIN: f32 = 0.0;
/// Radiation corner, Hz: third-order highpass (18 dB/oct below the
/// corner). A single differencer (6 dB/oct from DC) leaves the glottal
/// 1/f region ~25 dB above F2/F3; the extra orders restore the formant
/// balance (V2) and remove the boom.
pub const RAD_HZ: f32 = 250.0;
/// Consonant presence shelf, Hz (always on, never depth-scaled).
pub const PRESENCE_HZ: f32 = 1500.0;
/// Presence shelf gain (≈ +1.7 dB above `PRESENCE_HZ`). The level
/// calibration puts fricatives at the vowel peak already; the shelf adds
/// a gentle lift above the corner without clamping.
pub const PRESENCE_GAIN: f32 = 0.2;
/// Fixed make-up gain calibrated so the default corpus stays below 0.75.
pub const OUT_GAIN: f32 = 0.64;
