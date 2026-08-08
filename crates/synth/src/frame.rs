//! Frame stream exchanged between the prosody frame builder and the Klatt
//! renderer. One frame covers `FRAME_SAMPLES` output samples; the renderer
//! interpolates per sample between the current and the next frame, so the
//! effective parameter rate is one frame per 4.99 ms.

/// Output samples per frame (4.99 ms at 22050 Hz).
pub const FRAME_SAMPLES: usize = 110;

/// One synthesis parameter frame.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    /// Cascade formant frequencies F1..F5, Hz.
    pub f: [f32; 5],
    /// Cascade formant bandwidths, Hz.
    pub bw: [f32; 5],
    /// Parallel branch formants (frication/burst), Hz.
    pub fp: [f32; 4],
    /// Parallel branch bandwidths, Hz.
    pub bp: [f32; 4],
    /// Parallel branch amplitudes 0..1.
    pub ap: [f32; 4],
    /// Nasal branch amount 0..1.
    pub an: f32,
    /// Voicing amplitude 0..1.
    pub av: f32,
    /// Frication amplitude 0..1.
    pub af: f32,
    /// Aspiration amplitude 0..1.
    pub ah: f32,
    /// Parallel bypass 0..1.
    pub ab: f32,
    /// Open quotient 0.30..0.80.
    pub oq: f32,
    /// Pitch, Hz (already quantized by the prosody Step 3e transform).
    pub f0: f32,
}

impl Frame {
    /// A silent, neutral frame (formants zeroed, pitch at the base).
    pub fn new() -> Self {
        Self {
            f: [0.0; 5],
            bw: [0.0; 5],
            fp: [0.0; 4],
            bp: [0.0; 4],
            ap: [0.0; 4],
            an: 0.0,
            av: 0.0,
            af: 0.0,
            ah: 0.0,
            ab: 0.0,
            oq: 0.60,
            f0: 118.0,
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}
