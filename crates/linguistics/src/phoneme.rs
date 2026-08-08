#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PhonemeKind {
    IY,
    IH,
    EH,
    EY,
    AE,
    AA,
    AH,
    AO,
    UH,
    UW,
    UX,
    AX,
    ER,
    UE,
    OE,
    OEU,
    EN,
    AN,
    ON,
    UN,
    EI,
    AI,
    OI,
    OW,
    AU,
    IA,
    EA,
    UA,
    P,
    B,
    T,
    D,
    K,
    G,
    F,
    V,
    TH,
    DH,
    S,
    Z,
    SH,
    ZH,
    CH,
    JH,
    H,
    M,
    N,
    NG,
    NY,
    L,
    R,
    RR,
    W,
    Y,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stress {
    None,
    Secondary,
    Primary,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Boundary {
    None,
    Word,
    Clause,
    Sentence,
    Question,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Phoneme {
    pub kind: PhonemeKind,
    pub stress: Stress,
    pub boundary_after: Boundary,
}

impl Phoneme {
    pub fn new(kind: PhonemeKind) -> Self {
        Self {
            kind,
            stress: Stress::None,
            boundary_after: Boundary::None,
        }
    }

    pub fn is_vowel(&self) -> bool {
        matches!(
            self.kind,
            PhonemeKind::IY
                | PhonemeKind::IH
                | PhonemeKind::EH
                | PhonemeKind::EY
                | PhonemeKind::AE
                | PhonemeKind::AA
                | PhonemeKind::AH
                | PhonemeKind::AO
                | PhonemeKind::UH
                | PhonemeKind::UW
                | PhonemeKind::UX
                | PhonemeKind::AX
                | PhonemeKind::ER
                | PhonemeKind::UE
                | PhonemeKind::OE
                | PhonemeKind::OEU
                | PhonemeKind::EN
                | PhonemeKind::AN
                | PhonemeKind::ON
                | PhonemeKind::UN
                | PhonemeKind::EI
                | PhonemeKind::AI
                | PhonemeKind::OI
                | PhonemeKind::OW
                | PhonemeKind::AU
                | PhonemeKind::IA
                | PhonemeKind::EA
                | PhonemeKind::UA
        )
    }
}
