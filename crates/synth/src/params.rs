use master_voice_linguistics::phoneme::PhonemeKind::{self, *};

pub const SAMPLE_RATE: u32 = 22_050;

pub struct Target {
    pub f1: f32,
    pub f2: f32,
    pub f3: f32,
    pub f4: f32,
    pub b1: f32,
    pub b2: f32,
    pub b3: f32,
    pub voicing: f32,
    pub voice_amp: f32,
    pub noise_amp: f32,
    pub noise_freq: f32,
    pub nasal: f32,
    pub fricative: f32,
}

impl Target {
    fn vowel(f1: f32, f2: f32, f3: f32) -> Self {
        Self {
            f1,
            f2,
            f3,
            f4: 3500.0,
            b1: f1 * 0.12 + 40.0,
            b2: f2 * 0.10 + 60.0,
            b3: f3 * 0.08 + 80.0,
            voicing: 1.0,
            voice_amp: 1.0,
            noise_amp: 0.0,
            noise_freq: 0.0,
            nasal: 0.0,
            fricative: 0.0,
        }
    }

    fn consonant(voicing: f32, voice_amp: f32, noise_amp: f32, noise_freq: f32) -> Self {
        Self {
            f1: 300.0,
            f2: 1400.0,
            f3: 2400.0,
            f4: 3500.0,
            b1: 80.0,
            b2: 120.0,
            b3: 160.0,
            voicing,
            voice_amp,
            noise_amp,
            noise_freq,
            nasal: 0.0,
            fricative: 0.0,
        }
    }

    fn nasal(f2: f32) -> Self {
        Self {
            f1: 260.0,
            f2,
            f3: 2400.0,
            f4: 3400.0,
            b1: 100.0,
            b2: 160.0,
            b3: 200.0,
            voicing: 1.0,
            voice_amp: 0.55,
            noise_amp: 0.0,
            noise_freq: 0.0,
            nasal: 1.0,
            fricative: 0.0,
        }
    }
}

pub fn target_for(kind: PhonemeKind) -> Target {
    match kind {
        IY => Target::vowel(300.0, 2350.0, 2950.0),
        IH => Target::vowel(400.0, 2000.0, 2600.0),
        EH => Target::vowel(530.0, 1800.0, 2500.0),
        EY => Target::vowel(430.0, 2100.0, 2600.0),
        AE => Target::vowel(660.0, 1720.0, 2400.0),
        AA => Target::vowel(730.0, 1100.0, 2500.0),
        AH => Target::vowel(650.0, 1200.0, 2500.0),
        AO => Target::vowel(550.0, 900.0, 2400.0),
        UH => Target::vowel(450.0, 1050.0, 2300.0),
        UW => Target::vowel(310.0, 950.0, 2300.0),
        UX => Target::vowel(600.0, 1200.0, 2500.0),
        AX => Target::vowel(500.0, 1500.0, 2500.0),
        ER => Target::vowel(490.0, 1350.0, 1700.0),
        UE => Target::vowel(300.0, 1700.0, 2200.0),
        OE => Target::vowel(420.0, 1650.0, 2300.0),
        OEU => Target::vowel(480.0, 1500.0, 2300.0),
        EN => Target::vowel(550.0, 1600.0, 2400.0).with_nasal(),
        AN => Target::vowel(700.0, 1200.0, 2400.0).with_nasal(),
        ON => Target::vowel(550.0, 1000.0, 2300.0).with_nasal(),
        UN => Target::vowel(480.0, 1500.0, 2300.0).with_nasal(),
        EI => Target::vowel(430.0, 2100.0, 2600.0),
        AI => Target::vowel(680.0, 1250.0, 2500.0),
        OI => Target::vowel(540.0, 950.0, 2400.0),
        OW => Target::vowel(470.0, 1000.0, 2400.0),
        AU => Target::vowel(620.0, 1100.0, 2400.0),
        IA => Target::vowel(420.0, 1900.0, 2600.0),
        EA => Target::vowel(500.0, 1800.0, 2500.0),
        UA => Target::vowel(420.0, 1200.0, 2400.0),
        P => Target::consonant(0.0, 0.0, 0.9, 1000.0).with_burst(1000.0),
        B => Target::consonant(1.0, 0.5, 0.7, 1000.0).with_burst(1000.0),
        T => Target::consonant(0.0, 0.0, 0.9, 1800.0).with_burst(1800.0),
        D => Target::consonant(1.0, 0.5, 0.7, 1800.0).with_burst(1800.0),
        K => Target::consonant(0.0, 0.0, 0.9, 2600.0).with_burst(2600.0),
        G => Target::consonant(1.0, 0.5, 0.7, 2600.0).with_burst(2600.0),
        F => Target::consonant(0.0, 0.0, 0.85, 1100.0),
        V => Target::consonant(1.0, 0.5, 0.5, 1100.0),
        TH => Target::consonant(0.0, 0.0, 0.85, 1400.0),
        DH => Target::consonant(1.0, 0.5, 0.5, 1400.0),
        S => Target::consonant(0.0, 0.0, 0.9, 5200.0),
        Z => Target::consonant(1.0, 0.45, 0.5, 5200.0),
        SH => Target::consonant(0.0, 0.0, 0.9, 2600.0),
        ZH => Target::consonant(1.0, 0.45, 0.5, 2600.0),
        CH => Target::consonant(0.0, 0.0, 0.9, 2600.0).with_burst(2600.0),
        JH => Target::consonant(1.0, 0.5, 0.6, 2600.0).with_burst(2600.0),
        H => Target::consonant(0.0, 0.0, 0.6, 700.0),
        M => Target::nasal(1000.0),
        N => Target::nasal(1800.0),
        NG => Target::nasal(2200.0),
        NY => Target::nasal(2400.0),
        L => {
            let mut t = Target::vowel(300.0, 1100.0, 2600.0);
            t.voice_amp = 0.85;
            t
        }
        R => {
            let mut t = Target::vowel(420.0, 1300.0, 1700.0);
            t.voice_amp = 0.85;
            t
        }
        RR => {
            let mut t = Target::vowel(400.0, 1500.0, 2200.0);
            t.voice_amp = 0.85;
            t
        }
        W => {
            let mut t = Target::vowel(300.0, 700.0, 2400.0);
            t.voice_amp = 0.7;
            t
        }
        Y => {
            let mut t = Target::vowel(300.0, 2200.0, 2800.0);
            t.voice_amp = 0.7;
            t
        }
    }
}

impl Target {
    fn with_nasal(mut self) -> Self {
        self.nasal = 1.0;
        self
    }

    fn with_burst(mut self, freq: f32) -> Self {
        self.noise_freq = freq;
        self
    }
}
