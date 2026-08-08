use master_voice_linguistics::phoneme::PhonemeKind::{self, *};

pub const SAMPLE_RATE: u32 = 22_050;

/// Derived (not tabulated) upper formants, Hz.
pub const F4: f32 = 3300.0;
pub const F5: f32 = 3750.0;

/// Derived bandwidth formulas (public vowel-chart values).
pub fn b1(f1: f32, nasal: bool) -> f32 {
    if nasal {
        200.0
    } else {
        50.0 + 0.06 * f1
    }
}
pub fn b2(f2: f32) -> f32 {
    60.0 + 0.05 * f2
}
pub fn b3(f3: f32) -> f32 {
    100.0 + 0.04 * f3
}

/// Parallel branch resonator bandwidth, Hz (frication/burst). 800 Hz
/// keeps the frication resonators' Q at ~5-10 so fricatives sit near the
/// vowel level (the DC-normalized peak gain is ~Q, and the vowels' F1
/// rings are ~5x the source).
pub const PARALLEL_BW: f32 = 800.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Manner {
    Vowel,
    Diphthong,
    Nasal,
    Stop,
    Fricative,
    Affricate,
    Approximant,
    Lateral,
    Rhotic,
    Aspirate,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    Labial,
    LabioDental,
    Dental,
    Alveolar,
    PostAlveolar,
    Palatal,
    Velar,
    Uvular,
    Glottal,
}

pub struct PhoneSpec {
    pub kind: PhonemeKind,
    pub manner: Manner,
    pub place: Place,
    pub voiced: bool,
    pub nasal: bool,
    /// F1,F2,F3 at onset, Hz.
    pub start: [f32; 3],
    /// F1,F2,F3 at offset, Hz (== start for monophthongs).
    pub end: [f32; 3],
    pub base_ms: f32,
    /// 0.0 for non-stops.
    pub burst_ms: f32,
    pub af: f32,
    pub ah: f32,
    pub av: f32,
    /// Parallel frication frequencies, Hz (zero-fill unused slots).
    pub fric: [f32; 4],
    /// Parallel frication amplitudes.
    pub fric_a: [f32; 4],
}

const fn vowel(
    kind: PhonemeKind,
    start: [f32; 3],
    end: [f32; 3],
    base_ms: f32,
    nasal: bool,
    diphthong: bool,
) -> PhoneSpec {
    PhoneSpec {
        kind,
        manner: if diphthong {
            Manner::Diphthong
        } else {
            Manner::Vowel
        },
        place: Place::Glottal,
        voiced: true,
        nasal,
        start,
        end,
        base_ms,
        burst_ms: 0.0,
        af: 0.0,
        ah: 0.0,
        av: 1.0,
        fric: [0.0; 4],
        fric_a: [0.0; 4],
    }
}

/// A `PhoneSpec` row for consonants (table constructor — arity is the
/// spec's own).
#[allow(clippy::too_many_arguments)]
const fn consonant(
    kind: PhonemeKind,
    manner: Manner,
    place: Place,
    voiced: bool,
    nasal: bool,
    f: [f32; 3],
    base_ms: f32,
    burst_ms: f32,
    av: f32,
    af: f32,
    ah: f32,
    fric: [f32; 4],
    fric_a: [f32; 4],
) -> PhoneSpec {
    PhoneSpec {
        kind,
        manner,
        place,
        voiced,
        nasal,
        start: f,
        end: f,
        base_ms,
        burst_ms,
        af,
        ah,
        av,
        fric,
        fric_a,
    }
}

pub static SPECS: [PhoneSpec; 54] = [
    // --- Vowels (monophthongs) ---
    vowel(
        IY,
        [300.0, 2300.0, 3000.0],
        [300.0, 2300.0, 3000.0],
        150.0,
        false,
        false,
    ),
    vowel(
        IH,
        [400.0, 1950.0, 2600.0],
        [400.0, 1950.0, 2600.0],
        105.0,
        false,
        false,
    ),
    vowel(
        EH,
        [550.0, 1800.0, 2500.0],
        [550.0, 1800.0, 2500.0],
        105.0,
        false,
        false,
    ),
    vowel(
        EY,
        [480.0, 1900.0, 2500.0],
        [330.0, 2200.0, 2900.0],
        195.0,
        false,
        true,
    ),
    vowel(
        AE,
        [660.0, 1700.0, 2400.0],
        [660.0, 1700.0, 2400.0],
        130.0,
        false,
        false,
    ),
    vowel(
        AA,
        [720.0, 1100.0, 2500.0],
        [720.0, 1100.0, 2500.0],
        150.0,
        false,
        false,
    ),
    vowel(
        AH,
        [620.0, 1200.0, 2550.0],
        [620.0, 1200.0, 2550.0],
        105.0,
        false,
        false,
    ),
    vowel(
        AO,
        [570.0, 850.0, 2400.0],
        [570.0, 850.0, 2400.0],
        150.0,
        false,
        false,
    ),
    vowel(
        UH,
        [450.0, 1100.0, 2350.0],
        [450.0, 1100.0, 2350.0],
        105.0,
        false,
        false,
    ),
    vowel(
        UW,
        [320.0, 900.0, 2300.0],
        [320.0, 900.0, 2300.0],
        150.0,
        false,
        false,
    ),
    vowel(
        UX,
        [380.0, 1600.0, 2400.0],
        [380.0, 1600.0, 2400.0],
        130.0,
        false,
        false,
    ),
    vowel(
        AX,
        [500.0, 1450.0, 2450.0],
        [500.0, 1450.0, 2450.0],
        65.0,
        false,
        false,
    ),
    vowel(
        ER,
        [470.0, 1350.0, 1600.0],
        [470.0, 1350.0, 1600.0],
        150.0,
        false,
        false,
    ),
    vowel(
        UE,
        [300.0, 1750.0, 2200.0],
        [300.0, 1750.0, 2200.0],
        140.0,
        false,
        false,
    ),
    vowel(
        OE,
        [400.0, 1550.0, 2200.0],
        [400.0, 1550.0, 2200.0],
        140.0,
        false,
        false,
    ),
    vowel(
        OEU,
        [500.0, 1450.0, 2200.0],
        [500.0, 1450.0, 2200.0],
        140.0,
        false,
        false,
    ),
    vowel(
        EN,
        [550.0, 1650.0, 2500.0],
        [550.0, 1650.0, 2500.0],
        165.0,
        true,
        false,
    ),
    vowel(
        AN,
        [620.0, 1050.0, 2550.0],
        [620.0, 1050.0, 2550.0],
        165.0,
        true,
        false,
    ),
    vowel(
        ON,
        [480.0, 900.0, 2400.0],
        [480.0, 900.0, 2400.0],
        165.0,
        true,
        false,
    ),
    vowel(
        UN,
        [500.0, 1450.0, 2250.0],
        [500.0, 1450.0, 2250.0],
        165.0,
        true,
        false,
    ),
    vowel(
        EI,
        [380.0, 2100.0, 2600.0],
        [380.0, 2100.0, 2600.0],
        140.0,
        false,
        false,
    ),
    vowel(
        AI,
        [730.0, 1200.0, 2450.0],
        [350.0, 2100.0, 2800.0],
        200.0,
        false,
        true,
    ),
    vowel(
        OI,
        [570.0, 850.0, 2400.0],
        [350.0, 2100.0, 2800.0],
        210.0,
        false,
        true,
    ),
    vowel(
        OW,
        [500.0, 950.0, 2400.0],
        [350.0, 800.0, 2300.0],
        195.0,
        false,
        true,
    ),
    vowel(
        AU,
        [720.0, 1200.0, 2450.0],
        [350.0, 800.0, 2300.0],
        205.0,
        false,
        true,
    ),
    vowel(
        IA,
        [400.0, 1950.0, 2600.0],
        [500.0, 1450.0, 2450.0],
        195.0,
        false,
        true,
    ),
    vowel(
        EA,
        [550.0, 1800.0, 2500.0],
        [500.0, 1450.0, 2450.0],
        195.0,
        false,
        true,
    ),
    vowel(
        UA,
        [450.0, 1100.0, 2350.0],
        [500.0, 1450.0, 2450.0],
        195.0,
        false,
        true,
    ),
    // --- Stops ---
    consonant(
        P,
        Manner::Stop,
        Place::Labial,
        false,
        false,
        [300.0, 900.0, 2200.0],
        55.0,
        12.0,
        0.0,
        0.5,
        0.55,
        [900.0, 1800.0, 3000.0, 0.0],
        [0.9, 0.3, 0.2, 0.0],
    ),
    consonant(
        B,
        Manner::Stop,
        Place::Labial,
        true,
        false,
        [250.0, 900.0, 2200.0],
        45.0,
        10.0,
        0.35,
        0.5,
        0.10,
        [900.0, 1800.0, 3000.0, 0.0],
        [0.7, 0.2, 0.1, 0.0],
    ),
    consonant(
        T,
        Manner::Stop,
        Place::Alveolar,
        false,
        false,
        [300.0, 1750.0, 2600.0],
        55.0,
        12.0,
        0.0,
        0.2,
        0.60,
        [3500.0, 4800.0, 6500.0, 0.0],
        [0.9, 0.6, 0.4, 0.0],
    ),
    consonant(
        D,
        Manner::Stop,
        Place::Alveolar,
        true,
        false,
        [250.0, 1750.0, 2600.0],
        45.0,
        10.0,
        0.35,
        0.24,
        0.10,
        [3500.0, 4800.0, 6500.0, 0.0],
        [0.7, 0.4, 0.2, 0.0],
    ),
    consonant(
        K,
        Manner::Stop,
        Place::Velar,
        false,
        false,
        [300.0, 1800.0, 2400.0],
        55.0,
        14.0,
        0.0,
        0.25,
        0.65,
        [1900.0, 2600.0, 3800.0, 0.0],
        [0.8, 0.9, 0.4, 0.0],
    ),
    consonant(
        G,
        Manner::Stop,
        Place::Velar,
        true,
        false,
        [250.0, 1800.0, 2400.0],
        45.0,
        12.0,
        0.35,
        0.3,
        0.10,
        [1900.0, 2600.0, 3800.0, 0.0],
        [0.6, 0.7, 0.3, 0.0],
    ),
    // --- Fricatives ---
    consonant(
        F,
        Manner::Fricative,
        Place::LabioDental,
        false,
        false,
        [300.0, 1200.0, 2400.0],
        110.0,
        0.0,
        0.0,
        0.105,
        0.0,
        [1200.0, 4500.0, 7000.0, 0.0],
        [0.3, 0.4, 0.3, 0.0],
    ),
    consonant(
        V,
        Manner::Fricative,
        Place::LabioDental,
        true,
        false,
        [280.0, 1200.0, 2400.0],
        75.0,
        0.0,
        0.5,
        0.13,
        0.0,
        [1200.0, 4500.0, 7000.0, 0.0],
        [0.2, 0.25, 0.2, 0.0],
    ),
    consonant(
        TH,
        Manner::Fricative,
        Place::Dental,
        false,
        false,
        [300.0, 1600.0, 2600.0],
        110.0,
        0.0,
        0.0,
        0.08,
        0.0,
        [1400.0, 5500.0, 8000.0, 0.0],
        [0.25, 0.35, 0.25, 0.0],
    ),
    consonant(
        DH,
        Manner::Fricative,
        Place::Dental,
        true,
        false,
        [280.0, 1600.0, 2600.0],
        75.0,
        0.0,
        0.5,
        0.1,
        0.0,
        [1400.0, 5500.0, 8000.0, 0.0],
        [0.18, 0.25, 0.18, 0.0],
    ),
    consonant(
        S,
        Manner::Fricative,
        Place::Alveolar,
        false,
        false,
        [300.0, 1750.0, 2600.0],
        115.0,
        0.0,
        0.0,
        0.03,
        0.0,
        [4000.0, 6500.0, 8200.0, 0.0],
        [0.5, 1.0, 0.7, 0.0],
    ),
    consonant(
        Z,
        Manner::Fricative,
        Place::Alveolar,
        true,
        false,
        [280.0, 1750.0, 2600.0],
        80.0,
        0.0,
        0.5,
        0.04,
        0.0,
        [4000.0, 6500.0, 8200.0, 0.0],
        [0.35, 0.7, 0.5, 0.0],
    ),
    consonant(
        SH,
        Manner::Fricative,
        Place::PostAlveolar,
        false,
        false,
        [300.0, 1900.0, 2500.0],
        120.0,
        0.0,
        0.0,
        0.11,
        0.0,
        [2200.0, 3400.0, 4800.0, 0.0],
        [1.0, 0.8, 0.4, 0.0],
    ),
    consonant(
        ZH,
        Manner::Fricative,
        Place::PostAlveolar,
        true,
        false,
        [280.0, 1900.0, 2500.0],
        80.0,
        0.0,
        0.5,
        0.1,
        0.0,
        [2200.0, 3400.0, 4800.0, 0.0],
        [0.7, 0.55, 0.3, 0.0],
    ),
    // --- Affricates ---
    consonant(
        CH,
        Manner::Affricate,
        Place::PostAlveolar,
        false,
        false,
        [300.0, 1900.0, 2500.0],
        50.0,
        10.0,
        0.0,
        0.11,
        0.3,
        [2200.0, 3400.0, 4800.0, 0.0],
        [1.0, 0.8, 0.4, 0.0],
    ),
    consonant(
        JH,
        Manner::Affricate,
        Place::PostAlveolar,
        true,
        false,
        [280.0, 1900.0, 2500.0],
        45.0,
        10.0,
        0.35,
        0.11,
        0.1,
        [2200.0, 3400.0, 4800.0, 0.0],
        [0.7, 0.55, 0.3, 0.0],
    ),
    // --- Aspirate ---
    consonant(
        H,
        Manner::Aspirate,
        Place::Glottal,
        false,
        false,
        [500.0, 1450.0, 2450.0],
        55.0,
        0.0,
        0.0,
        0.0,
        0.75,
        [0.0; 4],
        [0.0; 4],
    ),
    // --- Nasals ---
    consonant(
        M,
        Manner::Nasal,
        Place::Labial,
        true,
        true,
        [250.0, 1100.0, 2200.0],
        65.0,
        0.0,
        0.55,
        0.0,
        0.0,
        [0.0; 4],
        [0.0; 4],
    ),
    consonant(
        N,
        Manner::Nasal,
        Place::Alveolar,
        true,
        true,
        [250.0, 1600.0, 2600.0],
        65.0,
        0.0,
        0.55,
        0.0,
        0.0,
        [0.0; 4],
        [0.0; 4],
    ),
    consonant(
        NG,
        Manner::Nasal,
        Place::Velar,
        true,
        true,
        [250.0, 2000.0, 2400.0],
        70.0,
        0.0,
        0.55,
        0.0,
        0.0,
        [0.0; 4],
        [0.0; 4],
    ),
    consonant(
        NY,
        Manner::Nasal,
        Place::Palatal,
        true,
        true,
        [250.0, 2100.0, 2900.0],
        70.0,
        0.0,
        0.55,
        0.0,
        0.0,
        [0.0; 4],
        [0.0; 4],
    ),
    // --- Lateral / rhotics ---
    consonant(
        L,
        Manner::Lateral,
        Place::Alveolar,
        true,
        false,
        [350.0, 1050.0, 2900.0],
        55.0,
        0.0,
        0.85,
        0.0,
        0.0,
        [0.0; 4],
        [0.0; 4],
    ),
    consonant(
        R,
        Manner::Rhotic,
        Place::Alveolar,
        true,
        false,
        [400.0, 1150.0, 1600.0],
        60.0,
        0.0,
        0.85,
        0.0,
        0.0,
        [0.0; 4],
        [0.0; 4],
    ),
    consonant(
        RR,
        Manner::Rhotic,
        Place::Uvular,
        true,
        false,
        [350.0, 1250.0, 2200.0],
        60.0,
        0.0,
        0.6,
        0.43,
        0.0,
        [1300.0, 2200.0, 3200.0, 0.0],
        [0.4, 0.3, 0.15, 0.0],
    ),
    // --- Approximants ---
    consonant(
        W,
        Manner::Approximant,
        Place::Labial,
        true,
        false,
        [300.0, 700.0, 2300.0],
        55.0,
        0.0,
        0.75,
        0.0,
        0.0,
        [0.0; 4],
        [0.0; 4],
    ),
    consonant(
        Y,
        Manner::Approximant,
        Place::Palatal,
        true,
        false,
        [280.0, 2200.0, 3000.0],
        55.0,
        0.0,
        0.75,
        0.0,
        0.0,
        [0.0; 4],
        [0.0; 4],
    ),
];

/// Spec lookup, O(1), ordered by `PhonemeKind` declaration order.
pub fn spec_for(kind: PhonemeKind) -> &'static PhoneSpec {
    &SPECS[kind as usize]
}

/// Coarticulation locus for a consonant place: the formant position the
/// vocal tract is in *at* the consonant, used as the transition endpoint
/// inside neighbouring vowels. `neighbour_f2` resolves the velar
/// front/back split; Glottal has no locus (the caller skips the
/// transition — H copies the neighbouring vowel's targets exactly).
pub fn locus(place: Place, neighbour_f2: f32) -> [f32; 3] {
    match place {
        Place::Labial => [250.0, 800.0, 2200.0],
        Place::LabioDental => [280.0, 1000.0, 2400.0],
        Place::Dental => [280.0, 1600.0, 2600.0],
        Place::Alveolar => [280.0, 1750.0, 2600.0],
        Place::PostAlveolar => [280.0, 1900.0, 2500.0],
        Place::Palatal => [280.0, 2200.0, 2900.0],
        Place::Velar => {
            if neighbour_f2 >= 1500.0 {
                [280.0, 2100.0, 2400.0]
            } else {
                [280.0, 1200.0, 2400.0]
            }
        }
        Place::Uvular => [300.0, 1300.0, 1900.0],
        Place::Glottal => [0.0, 0.0, 0.0], // no locus; caller skips transitions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs_cover_all_kinds_in_declaration_order() {
        assert_eq!(SPECS.len(), 54);
        for (i, spec) in SPECS.iter().enumerate() {
            assert_eq!(spec.kind as usize, i, "drift at SPECS[{i}]");
        }
    }

    #[test]
    fn spec_kind_matches_spec_for() {
        for spec in SPECS.iter() {
            assert!(std::ptr::eq(spec_for(spec.kind), spec));
        }
    }

    #[test]
    fn velar_locus_splits_on_f2() {
        let front = locus(Place::Velar, 2000.0);
        let back = locus(Place::Velar, 800.0);
        assert!(front[1] > back[1]);
        assert_eq!(front[0], back[0]);
    }

    #[test]
    fn derived_bandwidths_are_positive() {
        for spec in SPECS.iter() {
            for i in 0..3 {
                assert!(
                    spec.start[i].is_finite() && spec.start[i] >= 0.0,
                    "{:?}",
                    spec.kind
                );
            }
            assert!(b1(spec.start[0], spec.nasal) > 0.0);
            assert!(b2(spec.start[1]) > 0.0);
            assert!(b3(spec.start[2]) > 0.0);
        }
    }
}
