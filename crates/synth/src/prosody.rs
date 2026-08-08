//! Prosody: syllable structure, durations, intonation tunes and the frame
//! builder. Turns a phoneme stream into a parameter frame stream for the
//! Klatt renderer (Step 3 of the uplift plan).

use crate::character;
use crate::frame::{Frame, FRAME_SAMPLES};
use crate::params::{self, Manner, PhoneSpec, Place};
use master_voice_linguistics::phoneme::{Boundary, Phoneme, PhonemeKind, Stress};

/// User-facing synthesis options. `robotic_depth` is the replicant
/// character amount (0.0 = plain speech, 1.0 = full replicant).
pub struct SynthOptions {
    pub rate: f32,
    pub pitch: f32,
    pub volume: f32,
    pub robotic_depth: f32,
}

impl Default for SynthOptions {
    fn default() -> Self {
        Self {
            rate: 1.0,
            pitch: 1.0,
            volume: 1.0,
            robotic_depth: character::DEFAULT_ROBOTIC_DEPTH,
        }
    }
}

/// Position of a chunk within its utterance: `first`/`last` are used by
/// the post chain for fades; `last == false` also forces the continuation
/// tune and suppresses phrase-final lengthening and the trailing pause.
#[derive(Clone, Copy, Debug)]
pub struct ChunkPos {
    pub first: bool,
    pub last: bool,
}

impl Default for ChunkPos {
    fn default() -> Self {
        Self {
            first: true,
            last: true,
        }
    }
}

/// One syllable: phoneme indices `start..end`, nucleus = the vowel.
#[derive(Clone, Copy, Debug)]
pub struct Syllable {
    pub start: usize,
    pub nucleus: usize,
    pub end: usize,
    pub stress: Stress,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tune {
    Declarative,
    Continuation,
    Question,
    Exclamation,
}

struct TuneSpec {
    prehead: f32,
    head_start: f32,
    head_end: f32,
    nuc_start: f32,
    nuc_end: f32,
    tail: f32,
}

const DECLARATIVE: TuneSpec = TuneSpec {
    prehead: 0.95,
    head_start: 1.14,
    head_end: 1.00,
    nuc_start: 1.05,
    nuc_end: 0.72,
    tail: 0.72,
};
const CONTINUATION: TuneSpec = TuneSpec {
    prehead: 0.96,
    head_start: 1.10,
    head_end: 1.02,
    nuc_start: 0.98,
    nuc_end: 1.12,
    tail: 1.12,
};
const QUESTION: TuneSpec = TuneSpec {
    prehead: 0.95,
    head_start: 1.08,
    head_end: 1.00,
    nuc_start: 0.95,
    nuc_end: 1.45,
    tail: 1.45,
};
const EXCLAMATION: TuneSpec = TuneSpec {
    prehead: 1.00,
    head_start: 1.25,
    head_end: 1.08,
    nuc_start: 1.25,
    nuc_end: 0.80,
    tail: 0.80,
};

fn tune_spec(tune: Tune) -> &'static TuneSpec {
    match tune {
        Tune::Declarative => &DECLARATIVE,
        Tune::Continuation => &CONTINUATION,
        Tune::Question => &QUESTION,
        Tune::Exclamation => &EXCLAMATION,
    }
}

fn tune_for(boundary: Boundary) -> Tune {
    match boundary {
        Boundary::Clause => Tune::Continuation,
        Boundary::Question => Tune::Question,
        Boundary::Exclaim => Tune::Exclamation,
        _ => Tune::Declarative,
    }
}

fn is_phrase_boundary(b: Boundary) -> bool {
    matches!(
        b,
        Boundary::Clause | Boundary::Sentence | Boundary::Question | Boundary::Exclaim
    )
}

fn is_word_boundary(b: Boundary) -> bool {
    matches!(
        b,
        Boundary::Word
            | Boundary::Clause
            | Boundary::Sentence
            | Boundary::Question
            | Boundary::Exclaim
    )
}

fn pause_for(boundary: Boundary) -> f32 {
    match boundary {
        Boundary::None => 0.0,
        Boundary::Word => 0.020,
        Boundary::Clause => 0.160,
        Boundary::Sentence => 0.300,
        Boundary::Question => 0.300,
        Boundary::Exclaim => 0.280,
    }
}

fn is_stop(k: PhonemeKind) -> bool {
    matches!(
        k,
        PhonemeKind::P
            | PhonemeKind::B
            | PhonemeKind::T
            | PhonemeKind::D
            | PhonemeKind::K
            | PhonemeKind::G
    )
}

fn is_voiceless_obstruent(k: PhonemeKind) -> bool {
    matches!(
        k,
        PhonemeKind::P
            | PhonemeKind::T
            | PhonemeKind::K
            | PhonemeKind::F
            | PhonemeKind::TH
            | PhonemeKind::S
            | PhonemeKind::SH
            | PhonemeKind::CH
            | PhonemeKind::H
    )
}

/// Maximal-onset syllabification. A `Boundary::Word` or higher on a
/// phoneme always breaks the syllable.
pub fn syllabify(phonemes: &[Phoneme]) -> Vec<Syllable> {
    let mut out: Vec<Syllable> = Vec::new();
    let mut pending: Vec<usize> = Vec::new();
    let mut nucleus: Option<usize> = None;
    let mut start = 0usize;

    for (i, p) in phonemes.iter().enumerate() {
        if p.is_vowel() {
            match nucleus.take() {
                Some(n) => {
                    // Split the pending consonant run: longest legal onset
                    // goes to this syllable, the rest stays as coda.
                    let onset = longest_legal_onset(phonemes, &pending);
                    let coda = pending.len() - onset;
                    let end = if coda > 0 {
                        pending[coda - 1] + 1
                    } else {
                        pending.first().copied().unwrap_or(i)
                    };
                    out.push(Syllable {
                        start,
                        nucleus: n,
                        end,
                        stress: phonemes[n].stress,
                    });
                    start = if coda > 0 {
                        pending[coda]
                    } else {
                        pending.first().copied().unwrap_or(i)
                    };
                }
                None => {
                    // No previous nucleus: the pending run is the onset.
                    start = pending.first().copied().unwrap_or(i);
                }
            }
            pending.clear();
            nucleus = Some(i);
            if is_word_boundary(p.boundary_after) {
                out.push(Syllable {
                    start,
                    nucleus: i,
                    end: i + 1,
                    stress: p.stress,
                });
                nucleus = None;
                start = i + 1;
            }
        } else {
            pending.push(i);
            if is_word_boundary(p.boundary_after) {
                if let Some(n) = nucleus {
                    out.push(Syllable {
                        start,
                        nucleus: n,
                        end: i + 1,
                        stress: phonemes[n].stress,
                    });
                    nucleus = None;
                    start = i + 1;
                }
                pending.clear();
            }
        }
    }
    if let Some(n) = nucleus {
        out.push(Syllable {
            start,
            nucleus: n,
            end: phonemes.len(),
            stress: phonemes[n].stress,
        });
    }
    out
}

fn legal_onset(kinds: &[PhonemeKind]) -> bool {
    match kinds.len() {
        1 => true,
        2 => {
            let (a, b) = (kinds[0], kinds[1]);
            (is_stop(a) || a == PhonemeKind::F || a == PhonemeKind::TH || a == PhonemeKind::SH)
                && matches!(
                    b,
                    PhonemeKind::L | PhonemeKind::R | PhonemeKind::W | PhonemeKind::Y
                )
                || a == PhonemeKind::S
                    && (is_stop(b)
                        || matches!(
                            b,
                            PhonemeKind::M | PhonemeKind::N | PhonemeKind::L | PhonemeKind::W
                        ))
        }
        3 => {
            kinds[0] == PhonemeKind::S
                && is_stop(kinds[1])
                && matches!(kinds[2], PhonemeKind::L | PhonemeKind::R | PhonemeKind::W)
        }
        _ => false,
    }
}

/// Longest legal-onset suffix length of the pending consonant run.
fn longest_legal_onset(phonemes: &[Phoneme], pending: &[usize]) -> usize {
    let max = pending.len().min(3);
    for len in (1..=max).rev() {
        let kinds: Vec<PhonemeKind> = pending[pending.len() - len..]
            .iter()
            .map(|&i| phonemes[i].kind)
            .collect();
        if legal_onset(&kinds) {
            return len;
        }
    }
    0
}

/// Per-phoneme durations in seconds (Step 3b rules 1-6).
fn phoneme_durations(
    phonemes: &[Phoneme],
    syllables: &[Syllable],
    opts: &SynthOptions,
    final_allowed: bool,
) -> Vec<f32> {
    let rate = opts.rate.clamp(0.5, 2.0);
    let n = phonemes.len();
    let mut mult = vec![1.0f32; n];

    // Rule 1: vowels by stress.
    for s in syllables {
        let m = match phonemes[s.nucleus].stress {
            Stress::Primary => 1.30,
            Stress::Secondary => 1.12,
            Stress::None => 0.85,
        };
        mult[s.nucleus] *= m;
    }

    // Rule 2: phrase-final syllable (nucleus and coda), only when final.
    if final_allowed {
        for s in syllables {
            let last = s.end - 1;
            if is_phrase_boundary(phonemes[last].boundary_after) {
                for m in &mut mult[s.nucleus..s.end] {
                    *m *= 1.35;
                }
            }
        }
    }

    // Rule 3: word-final consonant before a Word boundary.
    for i in 0..n {
        if !phonemes[i].is_vowel() && phonemes[i].boundary_after == Boundary::Word {
            mult[i] *= 1.15;
        }
    }

    // Rule 4: vowel whose coda holds a voiceless obstruent (pre-fortis).
    for s in syllables {
        if s.nucleus + 1 < s.end
            && (s.nucleus + 1..s.end).any(|i| is_voiceless_obstruent(phonemes[i].kind))
        {
            mult[s.nucleus] *= 0.85;
        }
    }

    // Rule 5: consonant inside a cluster of >= 3.
    let mut run = 0usize;
    for i in 0..n {
        if phonemes[i].is_vowel() {
            run = 0;
        } else {
            run += 1;
            if run >= 3 {
                mult[i] *= 0.80;
            }
        }
    }

    // Rule 6: divide by rate.
    let mut d = vec![0.0f32; n];
    for i in 0..n {
        d[i] = params::spec_for(phonemes[i].kind).base_ms / 1000.0 * mult[i] / rate;
    }
    d
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PhaseKind {
    Main,
    Closure,
    Burst,
    Frication,
}

fn phases_for(spec: &PhoneSpec, d: f32) -> Vec<(PhaseKind, f32)> {
    match spec.manner {
        Manner::Stop => {
            let total = spec.base_ms + spec.burst_ms;
            vec![
                (PhaseKind::Closure, d * spec.base_ms / total),
                (PhaseKind::Burst, d * spec.burst_ms / total),
            ]
        }
        Manner::Affricate => {
            let total = spec.base_ms + spec.burst_ms + 55.0;
            vec![
                (PhaseKind::Closure, d * spec.base_ms / total),
                (PhaseKind::Burst, d * spec.burst_ms / total),
                (PhaseKind::Frication, d * 55.0 / total),
            ]
        }
        _ => vec![(PhaseKind::Main, d)],
    }
}

enum TransKind {
    Cosine,
    Linear,
}

struct Trans {
    len: f32,
    from: [f32; 3],
    to: [f32; 3],
    kind: TransKind,
}

/// Coarticulation transitions for a vowel: at a C->V or V->C junction the
/// vowel's span starts/ends at the consonant locus (or the neighbour
/// vowel's target) and moves to/from its own target.
fn vowel_transitions(
    phonemes: &[Phoneme],
    i: usize,
    d: f32,
    phrase_of: &[usize],
) -> (Option<Trans>, Option<Trans>) {
    let spec = params::spec_for(phonemes[i].kind);
    let head = if i > 0 && phrase_of[i - 1] == phrase_of[i] {
        let prev = params::spec_for(phonemes[i - 1].kind);
        if phonemes[i - 1].is_vowel() {
            let len = (0.030f32).min(0.4 * d);
            Some(Trans {
                len,
                from: prev.end,
                to: [0.0; 3],
                kind: TransKind::Linear,
            })
        } else if prev.place != Place::Glottal {
            let len = (0.045f32).min(0.4 * d);
            Some(Trans {
                len,
                from: params::locus(prev.place, spec.start[1]),
                to: [0.0; 3],
                kind: TransKind::Cosine,
            })
        } else {
            None
        }
    } else {
        None
    };
    let tail = if i + 1 < phonemes.len() && phrase_of[i + 1] == phrase_of[i] {
        let next = params::spec_for(phonemes[i + 1].kind);
        if phonemes[i + 1].is_vowel() {
            let len = (0.030f32).min(0.4 * d);
            Some(Trans {
                len,
                from: [0.0; 3],
                to: next.start,
                kind: TransKind::Linear,
            })
        } else if next.place != Place::Glottal {
            let len = (0.045f32).min(0.4 * d);
            Some(Trans {
                len,
                from: [0.0; 3],
                to: params::locus(next.place, spec.start[1]),
                kind: TransKind::Cosine,
            })
        } else {
            None
        }
    } else {
        None
    };
    (head, tail)
}

fn lerp3(a: [f32; 3], b: [f32; 3], u: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * u,
        a[1] + (b[1] - a[1]) * u,
        a[2] + (b[2] - a[2]) * u,
    ]
}

/// Diphthong base path: hold start 35 %, glide 45 %, hold end 20 %.
fn diphthong_base(spec: &PhoneSpec, u: f32) -> [f32; 3] {
    if u < 0.35 {
        spec.start
    } else if u < 0.80 {
        lerp3(spec.start, spec.end, (u - 0.35) / 0.45)
    } else {
        spec.end
    }
}

/// F1-F3 at frame fraction `u` of vowel `i` (loci transitions applied).
fn vowel_formants_at(
    phonemes: &[Phoneme],
    i: usize,
    u: f32,
    d: f32,
    phrase_of: &[usize],
) -> [f32; 3] {
    let spec = params::spec_for(phonemes[i].kind);
    let (head, tail) = vowel_transitions(phonemes, i, d, phrase_of);
    let base = |u: f32| diphthong_base(spec, u);
    let mut v = base(u);
    if let Some(h) = &head {
        if u * d < h.len {
            let target = base(h.len / d);
            let w = (u * d) / h.len;
            v = match h.kind {
                TransKind::Cosine => lerp3(
                    h.from,
                    target,
                    0.5 * (1.0 - (std::f32::consts::PI * w).cos()),
                ),
                TransKind::Linear => lerp3(h.from, target, w),
            };
        }
    }
    if let Some(t) = &tail {
        let frac = t.len / d;
        if u > 1.0 - frac {
            let target = base(1.0 - frac);
            let w = (u - (1.0 - frac)) / frac;
            v = match t.kind {
                TransKind::Cosine => {
                    lerp3(target, t.to, 0.5 * (1.0 - (std::f32::consts::PI * w).cos()))
                }
                TransKind::Linear => lerp3(target, t.to, w),
            };
        }
    }
    v
}

struct Tagged {
    frame: Frame,
    phoneme: usize,
    pause: bool,
    sample: f32,
}

/// Build the frame stream for a whole utterance.
pub fn build_frames(phonemes: &[Phoneme], opts: &SynthOptions) -> Vec<Frame> {
    build_frames_chunk(
        phonemes,
        opts,
        ChunkPos {
            first: true,
            last: true,
        },
    )
}

/// Build the frame stream for one chunk of an utterance. `pos.first` /
/// `pos.last` mark chunk position; when `pos.last == false` the chunk is
/// mid-utterance: continuation tune, no phrase-final lengthening, no
/// trailing pause.
pub fn build_frames_chunk(phonemes: &[Phoneme], opts: &SynthOptions, pos: ChunkPos) -> Vec<Frame> {
    let pitch = opts.pitch.clamp(0.5, 1.5);
    let depth = opts.robotic_depth.clamp(0.0, 1.0);
    if phonemes.is_empty() {
        return Vec::new();
    }

    let syllables = syllabify(phonemes);
    let durations = phoneme_durations(phonemes, &syllables, opts, pos.last);

    // Phrases: maximal runs ending at a phrase boundary.
    let mut phrase_of = vec![0usize; phonemes.len()];
    let mut phrase_boundary: Vec<Boundary> = Vec::new();
    let mut pid = 0usize;
    for (i, p) in phonemes.iter().enumerate() {
        phrase_of[i] = pid;
        if is_phrase_boundary(p.boundary_after) {
            phrase_boundary.push(p.boundary_after);
            pid += 1;
        }
    }
    if !is_phrase_boundary(phonemes.last().unwrap().boundary_after) {
        phrase_boundary.push(Boundary::None);
    }
    let phrase_count = phrase_boundary.len();
    let mut phrase_first = vec![0usize; phrase_count];
    let mut phrase_last = vec![0usize; phrase_count];
    for i in 0..phonemes.len() {
        let p = phrase_of[i];
        if i == 0 || phrase_of[i - 1] != p {
            phrase_first[p] = i;
        }
        phrase_last[p] = i;
    }

    // OQ floor 0.55: the pulse's spectral nulls sit at (f0/OQ)*k; below
    // 0.55 the 2nd/3rd nulls sweep into the F1 region and silence
    // low-F1 vowels at the default pitch.
    let oq = (character::OQ_BASE - character::OQ_DEPTH * depth).max(0.55);
    let sr = params::SAMPLE_RATE as f32;

    let mut tagged: Vec<Tagged> = Vec::new();
    let mut cursor = 0usize;
    let mut phoneme_start = vec![0usize; phonemes.len()];
    let mut phoneme_end = vec![0usize; phonemes.len()];
    let mut last_formants: ([f32; 5], [f32; 5]) = ([0.0; 5], [0.0; 5]);

    for (i, p) in phonemes.iter().enumerate() {
        phoneme_start[i] = cursor;
        let spec = params::spec_for(p.kind);
        let d = durations[i];
        for (phase_kind, phase_d) in phases_for(spec, d) {
            let n_frames = ((phase_d * sr) / FRAME_SAMPLES as f32).round().max(1.0) as usize;
            for k in 0..n_frames {
                let u = (k as f32 + 0.5) / n_frames as f32;
                let mut frame = Frame::new();
                frame.oq = oq;
                let f123 = match spec.manner {
                    Manner::Vowel | Manner::Diphthong => {
                        vowel_formants_at(phonemes, i, u, d, &phrase_of)
                    }
                    _ => spec.start,
                };
                frame.f = [f123[0], f123[1], f123[2], params::F4, params::F5];
                frame.bw = [
                    params::b1(f123[0], spec.nasal),
                    params::b2(f123[1]),
                    params::b3(f123[2]),
                    250.0,
                    300.0,
                ];
                match (phase_kind, spec.manner) {
                    (PhaseKind::Main, Manner::Vowel | Manner::Diphthong) => {
                        frame.av = 1.0;
                        frame.an = if spec.nasal { 0.6 } else { 0.0 };
                    }
                    (PhaseKind::Main, Manner::Nasal) => {
                        frame.av = spec.av;
                        frame.an = 1.0;
                    }
                    (PhaseKind::Main, _) => {
                        frame.av = spec.av;
                        frame.af = spec.af;
                        frame.ah = spec.ah;
                        if spec.af > 0.0 {
                            frame.fp = spec.fric;
                            frame.bp = [params::PARALLEL_BW; 4];
                            frame.ap = spec.fric_a;
                            frame.ab = 0.25;
                        }
                    }
                    (PhaseKind::Closure, _) => {}
                    (PhaseKind::Burst, _) => {
                        frame.av = spec.av;
                        frame.af = spec.af;
                        frame.ah = spec.ah;
                        frame.fp = spec.fric;
                        frame.bp = [params::PARALLEL_BW; 4];
                        frame.ap = spec.fric_a;
                        frame.ab = 0.25;
                    }
                    (PhaseKind::Frication, _) => {
                        frame.av = spec.av;
                        frame.af = spec.af;
                        frame.fp = spec.fric;
                        frame.bp = [params::PARALLEL_BW; 4];
                        frame.ap = spec.fric_a;
                        frame.ab = 0.25;
                    }
                }
                last_formants = (frame.f, frame.bw);
                tagged.push(Tagged {
                    frame,
                    phoneme: i,
                    pause: false,
                    sample: cursor as f32,
                });
                cursor += FRAME_SAMPLES;
            }
        }
        phoneme_end[i] = cursor;

        // Pauses (none trailing a non-final chunk).
        let trailing = !pos.last && i + 1 == phonemes.len();
        if p.boundary_after != Boundary::None && !trailing {
            let pause_s = pause_for(p.boundary_after);
            if pause_s > 0.0 {
                let n = ((pause_s * sr) / FRAME_SAMPLES as f32).round().max(1.0) as usize;
                for _ in 0..n {
                    let mut frame = Frame::new();
                    frame.f = last_formants.0;
                    frame.bw = last_formants.1;
                    frame.oq = oq;
                    frame.f0 = character::BASE_F0 * pitch * 0.5;
                    tagged.push(Tagged {
                        frame,
                        phoneme: i,
                        pause: true,
                        sample: cursor as f32,
                    });
                    cursor += FRAME_SAMPLES;
                }
            }
        }
    }

    // --- f0 contour: tune anchors per phrase, interpolated across frames.
    let dev_scale = 1.0 - 0.55 * depth;
    let mut anchors: Vec<Vec<(f32, f32)>> = vec![Vec::new(); phrase_count];
    for p in 0..phrase_count {
        let tune = if pos.last {
            tune_for(phrase_boundary[p])
        } else {
            Tune::Continuation
        };
        let ts = tune_spec(tune);
        let syls: Vec<&Syllable> = syllables
            .iter()
            .filter(|s| s.nucleus >= phrase_first[p] && s.nucleus <= phrase_last[p])
            .collect();
        let first_stressed = syls.iter().position(|s| s.stress != Stress::None);
        let last_stressed = syls.iter().rposition(|s| s.stress != Stress::None);
        let a = &mut anchors[p];
        match (first_stressed, last_stressed) {
            (Some(fs), Some(ls)) => {
                for (si, s) in syls.iter().enumerate() {
                    let t = phoneme_start[s.nucleus] as f32;
                    let m = if si < fs {
                        ts.prehead
                    } else if si == ls {
                        continue; // nucleus handled below
                    } else if si < ls {
                        // head: linear over span
                        let h = ls - fs;
                        let frac = if h <= 1 {
                            0.0
                        } else {
                            (si - fs) as f32 / (h - 1) as f32
                        };
                        let mut m = ts.head_start + (ts.head_end - ts.head_start) * frac;
                        m += match s.stress {
                            Stress::Primary => 0.06,
                            Stress::Secondary => 0.03,
                            Stress::None => 0.0,
                        };
                        m
                    } else {
                        ts.tail
                    };
                    if si != ls {
                        a.push((t, 1.0 + (m - 1.0) * dev_scale));
                    }
                }
                let ns = &syls[ls];
                let t0 = phoneme_start[ns.nucleus] as f32;
                let t1 = phoneme_end[ns.end - 1] as f32;
                a.push((t0, 1.0 + (ts.nuc_start - 1.0) * dev_scale));
                a.push((t1, 1.0 + (ts.nuc_end - 1.0) * dev_scale));
                a.push((
                    phoneme_end[phrase_last[p]] as f32,
                    1.0 + (ts.tail - 1.0) * dev_scale,
                ));
            }
            _ => {
                for s in syls.iter() {
                    a.push((phoneme_start[s.nucleus] as f32, 1.0));
                }
                if let Some(s) = syls.last() {
                    let t0 = phoneme_start[s.nucleus] as f32;
                    let t1 = phoneme_end[s.end - 1] as f32;
                    a.push((t0, 1.0 + (ts.nuc_start - 1.0) * dev_scale));
                    a.push((t1, 1.0 + (ts.nuc_end - 1.0) * dev_scale));
                }
                a.push((phoneme_end[phrase_last[p]] as f32, 1.0));
            }
        }
        // Anchors are pushed per-syllable, then the nucleus pair — sort by
        // time so the per-frame interpolation is monotonic.
        a.sort_by(|x, y| x.0.total_cmp(&y.0));
    }

    let interp = |a: &[(f32, f32)], t: f32| -> f32 {
        if a.is_empty() {
            return 1.0;
        }
        if t <= a[0].0 {
            return a[0].1;
        }
        for w in a.windows(2) {
            if t <= w[1].0 {
                let u = (t - w[0].0) / (w[1].0 - w[0].0).max(1.0);
                return w[0].1 + (w[1].1 - w[0].1) * u;
            }
        }
        a.last().unwrap().1
    };

    let mut frames = Vec::with_capacity(tagged.len());
    for tag in tagged {
        let mut frame = tag.frame;
        if tag.pause {
            frame.f0 = character::BASE_F0 * pitch * 0.5;
        } else {
            let t = tag.sample + FRAME_SAMPLES as f32 * 0.5;
            let m = interp(&anchors[phrase_of[tag.phoneme]], t);
            frame.f0 = character::BASE_F0 * pitch * m;
            // Step 3d: per-phoneme pitch shift (after the tune contour).
            frame.f0 *= 1.0 + phonemes[tag.phoneme].pitch_shift;
        }
        // Step 3e: semitone-quantized pitch (replicant character).
        let semis = (12.0 * (frame.f0 / character::BASE_F0).log2()).round();
        let stepped = character::BASE_F0 * 2f32.powf(semis / 12.0);
        frame.f0 = (frame.f0 + (stepped - frame.f0) * depth).max(40.0);
        frames.push(frame);
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;
    use master_voice_linguistics::phoneme::Phoneme;

    #[test]
    fn frames_cover_expected_duration() {
        let phonemes = [
            Phoneme::new(PhonemeKind::H),
            Phoneme::new(PhonemeKind::EH),
            Phoneme::new(PhonemeKind::L),
            Phoneme::new(PhonemeKind::OW),
        ];
        let frames = build_frames(&phonemes, &SynthOptions::default());
        assert!(frames.len() as f32 * FRAME_SAMPLES as f32 / params::SAMPLE_RATE as f32 > 0.2);
        for f in &frames {
            assert!(f.f.iter().chain(f.bw.iter()).all(|v| v.is_finite()));
            assert!(f
                .fp
                .iter()
                .chain(f.bp.iter())
                .chain(f.ap.iter())
                .all(|v| v.is_finite()));
            assert!(f.an.is_finite() && f.av.is_finite() && f.af.is_finite());
            assert!(f.ah.is_finite() && f.ab.is_finite() && f.oq.is_finite() && f.f0.is_finite());
        }
    }

    fn stressed(kind: PhonemeKind) -> Phoneme {
        Phoneme {
            stress: Stress::Primary,
            ..Phoneme::new(kind)
        }
    }

    #[test]
    fn question_rises() {
        let mut p = stressed(PhonemeKind::AH);
        p.boundary_after = Boundary::Question;
        let frames = build_frames(
            &[p],
            &SynthOptions {
                robotic_depth: 0.0,
                ..SynthOptions::default()
            },
        );
        // Last non-pause frame = the end of the nucleus (the rise peak).
        let pause_frames = (pause_for(Boundary::Question) * params::SAMPLE_RATE as f32
            / FRAME_SAMPLES as f32)
            .round() as usize;
        let nuc_last = &frames[frames.len() - pause_frames - 1];
        assert!(nuc_last.f0 > character::BASE_F0, "f0={}", nuc_last.f0);
    }

    #[test]
    fn exclaim_and_question_both_rise_but_differ() {
        let mut a = stressed(PhonemeKind::AE);
        a.boundary_after = Boundary::None;
        let mut q = stressed(PhonemeKind::AH);
        q.boundary_after = Boundary::Question;
        let mut e = stressed(PhonemeKind::AH);
        e.boundary_after = Boundary::Exclaim;
        let opts = SynthOptions {
            robotic_depth: 0.0,
            ..SynthOptions::default()
        };
        let question = build_frames(&[a, q], &opts);
        let exclaim = build_frames(&[a, e], &opts);

        // Head peak: max f0 over the first syllable's frames (AE spans
        // 34 frames; the first 10 frames are all inside it).
        let head_peak = |frames: &[Frame]| {
            frames[..10.min(frames.len())]
                .iter()
                .map(|f| f.f0)
                .fold(0.0f32, f32::max)
        };
        let q_peak = head_peak(&question);
        let e_peak = head_peak(&exclaim);
        assert!(q_peak > character::BASE_F0);
        assert!(e_peak > character::BASE_F0);
        assert!(e_peak > q_peak, "exclaim {e_peak} vs question {q_peak}");

        let pause_frames = (pause_for(Boundary::Question) * params::SAMPLE_RATE as f32
            / FRAME_SAMPLES as f32)
            .round() as usize;
        let q_tail = question[question.len() - pause_frames - 1].f0;
        let e_tail = exclaim[exclaim.len() - pause_frames - 1].f0;
        assert!(e_tail < q_tail, "exclaim {e_tail} vs question {q_tail}");
    }

    #[test]
    fn pitch_shift_lowers_f0() {
        let plain = Phoneme::new(PhonemeKind::AA);
        let shifted = Phoneme {
            pitch_shift: -0.15,
            ..plain
        };
        let opts = SynthOptions {
            robotic_depth: 0.0,
            ..SynthOptions::default()
        };
        let a = build_frames(&[plain], &opts);
        let b = build_frames(&[shifted], &opts);
        assert!(b[0].f0 < a[0].f0);
        assert!((a[0].f0 - b[0].f0 - 0.15 * a[0].f0).abs() < 0.001);
    }

    #[test]
    fn robotic_flattens_pitch() {
        let opts_natural = SynthOptions {
            robotic_depth: 0.0,
            ..SynthOptions::default()
        };
        let opts_robotic = SynthOptions {
            robotic_depth: 1.0,
            ..SynthOptions::default()
        };
        let phonemes = [stressed(PhonemeKind::AE), Phoneme::new(PhonemeKind::AE)];
        let natural = build_frames(&phonemes, &opts_natural);
        let robotic = build_frames(&phonemes, &opts_robotic);

        let first_peak =
            |frames: &[Frame]| frames.iter().take(5).map(|f| f.f0).fold(0.0f32, f32::max);
        let natural_spread = first_peak(&natural) - natural.last().unwrap().f0;
        let robotic_spread = first_peak(&robotic) - robotic.last().unwrap().f0;
        assert!(robotic_spread < natural_spread);
    }

    #[test]
    fn non_final_chunk_has_no_trailing_pause() {
        let mut p = Phoneme::new(PhonemeKind::S);
        p.boundary_after = Boundary::Sentence;
        let opts = SynthOptions::default();
        let final_chunk = build_frames(&[p], &opts);
        let mid_chunk = build_frames_chunk(
            &[p],
            &opts,
            ChunkPos {
                first: true,
                last: false,
            },
        );
        assert!(final_chunk.len() > mid_chunk.len());
        assert!(final_chunk.last().unwrap().av == 0.0);
    }

    #[test]
    fn syllabifies_maximal_onsets() {
        // "string": S T R IH NG -> [S T R] onset of IH.
        let phonemes = [
            Phoneme::new(PhonemeKind::S),
            Phoneme::new(PhonemeKind::T),
            Phoneme::new(PhonemeKind::R),
            Phoneme::new(PhonemeKind::IH),
            Phoneme::new(PhonemeKind::NG),
        ];
        let syls = syllabify(&phonemes);
        assert_eq!(syls.len(), 1);
        assert_eq!(syls[0].start, 0);
        assert_eq!(syls[0].nucleus, 3);
        assert_eq!(syls[0].end, 5);

        // "atlas": AE T L AX S -> T L onset of AX? "at-las": T is coda.
        let phonemes2 = [
            Phoneme::new(PhonemeKind::AE),
            Phoneme::new(PhonemeKind::T),
            Phoneme::new(PhonemeKind::L),
            Phoneme::new(PhonemeKind::AX),
            Phoneme::new(PhonemeKind::S),
        ];
        let syls2 = syllabify(&phonemes2);
        assert_eq!(syls2.len(), 2);
        // T L is a legal onset, so "a-tlas".
        assert_eq!(syls2[0].nucleus, 0);
        assert_eq!(syls2[0].end, 1);
        assert_eq!(syls2[1].start, 1);
    }

    #[test]
    fn word_boundary_breaks_syllable() {
        let mut p = Phoneme::new(PhonemeKind::T);
        p.boundary_after = Boundary::Word;
        let phonemes = [
            Phoneme::new(PhonemeKind::AE),
            p,
            Phoneme::new(PhonemeKind::R),
            Phoneme::new(PhonemeKind::IY),
        ];
        let syls = syllabify(&phonemes);
        assert_eq!(syls.len(), 2);
        assert_eq!(syls[0].end, 2); // "at"
        assert_eq!(syls[1].start, 2); // "ree"
    }
}
