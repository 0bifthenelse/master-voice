use master_voice_linguistics::lang::Language;
use master_voice_linguistics::overrides::Overrides;
use master_voice_linguistics::phoneme::{Boundary, Phoneme, PhonemeKind, Stress};
use master_voice_linguistics::{phonemize, LingError};

fn word_phones(phonemes: &[Phoneme]) -> Vec<Vec<PhonemeKind>> {
    let mut words = Vec::new();
    let mut current = Vec::new();
    for phoneme in phonemes {
        current.push(phoneme.kind);
        if phoneme.boundary_after != Boundary::None {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

#[test]
fn fixed_sixteen_sentence_corpus_has_expected_boundaries_and_language_spans() {
    use Language::{English, French};
    let cases: &[(&str, Option<Language>, &[Language])] = &[
        ("THE VOICE SYNTHESIS IS ONLINE.", Some(English), &[English]),
        ("THE READY SYSTEM IS HUMAN.", Some(English), &[English]),
        ("I READ THE RECORD YESTERDAY.", Some(English), &[English]),
        ("I CAN'T SCHEDULE THE MEETING.", Some(English), &[English]),
        ("THE COLONEL KNOWS THE KNIGHT.", Some(English), &[English]),
        ("MR. SMITH ARRIVES.", None, &[English]),
        ("CPU GPU TIME.", None, &[English]),
        ("THE SYSTEM EST PRÊT.", None, &[English, French]),
        ("BONJOUR MONSIEUR.", Some(French), &[French]),
        ("LES AMIS.", Some(French), &[French]),
        ("UN ENFANT.", Some(French), &[French]),
        ("L'ADRESSE D'UN HOTEL.", Some(French), &[French]),
        ("UNE PETITE FENETRE.", Some(French), &[French]),
        ("SES FILS ET UN FILS ÉLECTRIQUE.", Some(French), &[French]),
        ("BEAUCOUP DE CHOSES.", Some(French), &[French]),
        (
            "BONJOUR THE SYSTEM EST PRÊT.",
            None,
            &[French, English, French],
        ),
    ];

    for (text, explicit, expected_languages) in cases {
        let utterance = phonemize(text, *explicit, &Overrides::default()).expect("phonemize");
        assert!(!utterance.phonemes.is_empty());
        assert_eq!(
            utterance.phonemes.last().map(|phone| phone.boundary_after),
            Some(Boundary::Sentence)
        );
        assert_eq!(
            utterance
                .language_spans
                .iter()
                .map(|span| span.language)
                .collect::<Vec<_>>(),
            *expected_languages
        );
        assert_eq!(
            utterance.language_spans.first().map(|span| span.word_start),
            Some(0)
        );
        assert!(utterance
            .phonemes
            .iter()
            .any(|phone| phone.stress != Stress::None));
    }
}

#[test]
fn english_context_selects_past_read_and_record_stress() {
    use PhonemeKind::{D, EH, ER, IH, K, R};
    let past = phonemize(
        "I READ THE RECORD YESTERDAY.",
        Some(Language::English),
        &Overrides::default(),
    )
    .expect("past sentence");
    let words = word_phones(&past.phonemes);
    assert_eq!(words[1], vec![R, EH, D]);
    assert_eq!(words[3], vec![R, EH, K, ER, D]);

    let verb = phonemize(
        "I WILL RECORD THE MEETING.",
        Some(Language::English),
        &Overrides::default(),
    )
    .expect("verb sentence");
    let words = word_phones(&verb.phonemes);
    assert_eq!(words[2], vec![R, IH, K, PhonemeKind::AO, R, D]);
    let record_stresses: Vec<_> = verb
        .phonemes
        .iter()
        .filter(|phone| [R, IH, K, PhonemeKind::AO, D].contains(&phone.kind))
        .map(|phone| phone.stress)
        .collect();
    assert!(record_stresses.contains(&Stress::Primary));
}

#[test]
fn canonical_master_sentence_has_expected_words() {
    use PhonemeKind::{AA, AE, AI, AX, B, DH, ER, EY, H, IH, JH, K, L, M, N, R, S, T, Z};
    let utterance = phonemize(
        "I HEREBY ACKNOWLEDGE THAT MY NAME IS MASTER",
        Some(Language::English),
        &Overrides::default(),
    )
    .expect("canonical sentence");
    assert_eq!(
        word_phones(&utterance.phonemes),
        vec![
            vec![AI],
            vec![H, IH, R, B, AI],
            vec![AX, K, N, AA, L, IH, JH],
            vec![DH, AE, T],
            vec![M, AI],
            vec![N, EY, M],
            vec![IH, Z],
            vec![M, AE, S, T, ER],
        ]
    );
}

#[test]
fn french_dictionary_nuclei_liaisons_and_fils_context_are_explicit() {
    use PhonemeKind::{AO, B, F, IY, K, L, N, OW, S, UN, UW, Z};
    let liaisons = phonemize(
        "LES AMIS. UN ENFANT.",
        Some(Language::French),
        &Overrides::default(),
    )
    .expect("liaisons");
    let words = word_phones(&liaisons.phonemes);
    assert_eq!(words[0].last(), Some(&Z));
    assert_eq!(words[2].last(), Some(&N));

    let fils = phonemize(
        "SES FILS ET UN FILS ÉLECTRIQUE.",
        Some(Language::French),
        &Overrides::default(),
    )
    .expect("fils contexts");
    let words = word_phones(&fils.phonemes);
    assert_eq!(words[1], vec![F, IY, S]);
    assert_eq!(words[4], vec![F, IY, L]);

    let beaucoup = phonemize(
        "BEAUCOUP DE CHOSES.",
        Some(Language::French),
        &Overrides::default(),
    )
    .expect("beaucoup");
    assert_eq!(word_phones(&beaucoup.phonemes)[0], vec![B, OW, K, UW]);
    // French phrase accent: only the phrase-final word keeps primary stress.
    assert!(beaucoup
        .phonemes
        .iter()
        .any(|phone| phone.kind == AO && phone.stress == Stress::Primary));
    assert!(!beaucoup
        .phonemes
        .iter()
        .any(|phone| phone.kind == UW && phone.stress != Stress::None));

    let un_hotel = phonemize("D'UN HOTEL.", Some(Language::French), &Overrides::default())
        .expect("mute h liaison");
    assert_eq!(word_phones(&un_hotel.phonemes)[0].last(), Some(&N));
    assert!(un_hotel.phonemes.iter().any(|phone| phone.kind == UN));
}

#[test]
fn unknown_override_symbol_is_a_typed_linguistic_error() {
    let mut overrides = Overrides::default();
    overrides.insert("voice", &["V", "UNKNOWN_PHONE", "S"]);
    let error = phonemize("voice", Some(Language::English), &overrides)
        .err()
        .expect("unknown override must fail");
    assert!(matches!(
        error,
        LingError::UnknownOverridePhoneSymbol { ref word, ref symbol }
            if word == "voice" && symbol == "UNKNOWN_PHONE"
    ));
}
