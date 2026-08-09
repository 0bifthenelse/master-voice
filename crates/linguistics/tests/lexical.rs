use master_voice_linguistics::g2p;
use master_voice_linguistics::lang::Language;
use master_voice_linguistics::normalize::{normalize_sentence, NormalizeOptions};
use master_voice_linguistics::overrides::Overrides;
use master_voice_linguistics::phoneme::{Boundary, PhonemeKind};
use master_voice_linguistics::phonemize;

const EN_ORDINARY: [&str; 10] = [
    "HELLO", "THIS", "SYSTEM", "IS", "READY", "MASTER", "VOICE", "ONLINE", "AND", "THE",
];

const FR_ORDINARY: [&str; 10] = [
    "BONJOUR", "LE", "SYSTÈME", "EST", "PRÊT", "VOIX", "JE", "SUIS", "LA", "ET",
];

const EN_SENTENCES: [&str; 3] = [
    "Hello, this is the MASTER voice. The system is online and ready.",
    "HELLO, THIS SYSTEM IS READY.",
    "The CPU and GPU are online.",
];

const FR_SENTENCES: [&str; 4] = [
    "Bonjour, je suis la voix MASTER. Le système est en ligne et prêt.",
    "BONJOUR, LE SYSTÈME EST PRÊT.",
    "Le CPU et le GPU sont disponibles.",
    "Aujourd'hui, nous sommes à Auch, dans le Gers.",
];

fn kinds(text: &str, lang: Language) -> Vec<PhonemeKind> {
    phonemize(text, Some(lang), &Overrides::default())
        .unwrap()
        .phonemes
        .into_iter()
        .map(|p| p.kind)
        .collect()
}

fn normalized_tokens(text: &str, lang: Language) -> Vec<String> {
    normalize_sentence(text, lang, &NormalizeOptions::default())
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn spelled_letters(word: &str) -> String {
    word.chars()
        .map(|c| c.to_string())
        .collect::<Vec<String>>()
        .join(" ")
}

#[test]
fn uppercase_ordinary_words_phonemize_as_words() {
    for word in EN_ORDINARY {
        let upper = kinds(word, Language::English);
        let lower = kinds(&word.to_lowercase(), Language::English);
        assert_eq!(
            upper, lower,
            "english uppercase {word} diverged from lowercase"
        );
    }
    for word in FR_ORDINARY {
        let upper = kinds(word, Language::French);
        let lower = kinds(&word.to_lowercase(), Language::French);
        assert_eq!(
            upper, lower,
            "french uppercase {word} diverged from lowercase"
        );
    }
}

#[test]
fn uppercase_ordinary_words_are_not_split_into_characters() {
    for word in EN_ORDINARY {
        let tokens = normalized_tokens(word, Language::English);
        assert_eq!(
            tokens.len(),
            1,
            "english {word} normalized to {tokens:?} instead of one token"
        );
    }
    for word in FR_ORDINARY {
        let tokens = normalized_tokens(word, Language::French);
        assert_eq!(
            tokens.len(),
            1,
            "french {word} normalized to {tokens:?} instead of one token"
        );
    }
}

#[test]
fn uppercase_sentences_match_their_lowercase_form() {
    let upper = kinds("HELLO, THIS SYSTEM IS READY.", Language::English);
    let lower = kinds("hello, this system is ready.", Language::English);
    assert_eq!(upper, lower, "english all-caps sentence diverged");

    let upper = kinds("BONJOUR, LE SYSTÈME EST PRÊT.", Language::French);
    let lower = kinds("bonjour, le système est prêt.", Language::French);
    assert_eq!(upper, lower, "french all-caps sentence diverged");
}

#[test]
fn genuine_initialisms_still_spell() {
    for (word, lang) in [
        ("CPU", Language::English),
        ("GPU", Language::English),
        ("IP", Language::English),
        ("CPU", Language::French),
        ("GPU", Language::French),
    ] {
        let acronym = kinds(word, lang);
        let spelled = kinds(&spelled_letters(word), lang);
        assert_eq!(
            acronym, spelled,
            "{word} in {lang:?} stopped spelling as an initialism"
        );
    }
}

#[test]
fn unknown_words_use_g2p_fallback_not_letter_spelling() {
    for (word, lang) in [
        ("zorblatt", Language::English),
        ("ZORBLATT", Language::English),
        ("gravumine", Language::French),
        ("GRAVUMINE", Language::French),
    ] {
        let word_form = kinds(word, lang);
        let spelled = kinds(&spelled_letters(&word.to_uppercase()), lang);
        assert_ne!(
            word_form, spelled,
            "{word} in {lang:?} degenerated into letter spelling"
        );
    }
}

#[test]
fn required_sentences_normalize_to_expected_tokens() {
    let expected_en = [
        "hello , this is the master voice . the system is online and ready .",
        "hello , this system is ready .",
        "the c p u and g p u are online .",
    ];
    for (sentence, expected) in EN_SENTENCES.iter().zip(expected_en) {
        let actual = normalized_tokens(sentence, Language::English)
            .join(" ")
            .to_lowercase();
        assert_eq!(actual, expected, "english sentence {sentence:?} diverged");
    }

    let expected_fr = [
        "bonjour , je suis la voix master . le système est en ligne et prêt .",
        "bonjour , le système est prêt .",
        "le c p u et le g p u sont disponibles .",
        "aujourd'hui , nous sommes à auch , dans le gers .",
    ];
    for (sentence, expected) in FR_SENTENCES.iter().zip(expected_fr) {
        let actual = normalized_tokens(sentence, Language::French)
            .join(" ")
            .to_lowercase();
        assert_eq!(actual, expected, "french sentence {sentence:?} diverged");
    }
}

#[test]
fn tokenizer_preserves_word_count_for_plain_sentences() {
    let tokens = g2p::tokenize(&normalize_sentence(
        "HELLO, THIS SYSTEM IS READY.",
        Language::English,
        &NormalizeOptions::default(),
    ));
    let words = tokens
        .iter()
        .filter(|t| matches!(t, g2p::Token::Word(_)))
        .count();
    assert_eq!(
        words, 5,
        "expected five lexical words, tokenizer produced {words} from {tokens:?}"
    );
}

fn boundary_marks(text: &str, lang: Language) -> usize {
    phonemize(text, Some(lang), &Overrides::default())
        .unwrap()
        .phonemes
        .iter()
        .filter(|p| p.boundary_after != Boundary::None)
        .count()
}

#[test]
fn french_multiword_clauses_keep_word_boundaries() {
    for (text, words) in [
        ("je suis la voix", 4),
        ("le système est en ligne et prêt", 7),
        ("nous sommes à Auch dans le Gers", 7),
    ] {
        let marks = boundary_marks(text, Language::French);
        assert_eq!(
            marks, words,
            "french clause {text:?} kept {marks} boundaries for {words} words"
        );
    }
}

#[test]
fn french_boundary_density_matches_english() {
    let french = boundary_marks("je suis la voix", Language::French);
    let english = boundary_marks("this is my voice", Language::English);
    assert_eq!(
        french, english,
        "french flattened word boundaries relative to english"
    );
}
