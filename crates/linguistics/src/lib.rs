pub mod dict;
pub mod g2p;
pub mod lang;
pub mod normalize;
pub mod numbers;
pub mod overrides;
pub mod phoneme;
pub mod sentence;
pub mod unicode;

use lang::Language;
use phoneme::Phoneme;

pub struct Utterance {
    pub language: Language,
    pub phonemes: Vec<Phoneme>,
    pub source_text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LingError {
    #[error("no speakable text")]
    EmptyInput,
}

pub fn phonemize(
    text: &str,
    language: Option<Language>,
    overrides: &overrides::Overrides,
) -> Result<Utterance, LingError> {
    let opts = normalize::NormalizeOptions::default();
    let sentences = sentence::split_sentences(text);
    let mut all: Vec<Phoneme> = Vec::new();
    let mut lang_used = language.unwrap_or(Language::English);
    for s in &sentences {
        let lang = language.unwrap_or_else(|| lang::detect(&s.text).unwrap_or(Language::English));
        lang_used = lang;
        let normalized = normalize::normalize_sentence(&s.text, lang, &opts);
        let tokens = g2p::tokenize(&normalized);
        let mut phones = g2p::phonemize_tokens(&tokens, lang, overrides);
        if let Some(last) = phones.last_mut() {
            last.boundary_after = s.boundary;
        }
        all.extend(phones);
    }
    if all.is_empty() {
        return Err(LingError::EmptyInput);
    }
    Ok(Utterance {
        language: lang_used,
        phonemes: all,
        source_text: text.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use phoneme::PhonemeKind;
    use phoneme::PhonemeKind::*;

    fn phonemes(text: &str, lang: Option<Language>) -> Vec<PhonemeKind> {
        phonemize(text, lang, &overrides::Overrides::default())
            .unwrap()
            .phonemes
            .into_iter()
            .map(|p| p.kind)
            .collect()
    }

    #[test]
    fn french_corpus_1() {
        assert_eq!(
            phonemes(
                "Bonjour, je suis le système de synthèse vocale MASTER.",
                Some(Language::French)
            ),
            vec![
                B, ON, Z, UW, RR, Z, AX, S, Y, IY, L, AX, S, IY, S, T, EH, M, D, AX, S, IY, N, T,
                EH, Z, V, AO, K, AA, L, M, AA, S, T, EH, RR
            ]
        );
    }

    #[test]
    fn french_corpus_2() {
        assert_eq!(
            phonemes(
                "Aujourd'hui, nous sommes à Auch, dans le Gers.",
                Some(Language::French)
            ),
            vec![OW, Z, UW, RR, D, UE, IY, N, UW, S, AO, M, AA, OW, SH, D, AN, L, AX, Z, EH, RR]
        );
    }

    #[test]
    fn french_corpus_3() {
        assert_eq!(
            phonemes(
                "L'intelligence artificielle fonctionne correctement.",
                Some(Language::French)
            ),
            vec![
                L, EN, T, EH, L, IY, Z, AN, S, AA, RR, T, IY, F, IY, S, Y, EH, L, F, ON, K, S, Y,
                ON, N, K, AO, RR, EH, K, T, AX, M, AN
            ]
        );
    }

    #[test]
    fn french_corpus_money_time() {
        assert_eq!(
            phonemes(
                "Il coûte 12,50 euros et sera disponible à 14 h 35.",
                Some(Language::French)
            ),
            vec![
                IY, L, K, UW, T, D, UW, Z, V, IY, RR, G, UE, L, S, EN, K, Z, EY, RR, OW, OEU, RR,
                OW, EY, S, AX, RR, AA, D, IY, S, P, AO, N, IY, B, L, AA, K, AA, T, AO, RR, Z, OEU,
                RR, T, RR, AN, T, S, EN, K
            ]
        );
    }

    #[test]
    fn french_corpus_ip() {
        assert_eq!(
            phonemes("L'adresse IP est 192.168.1.42.", Some(Language::French)),
            vec![
                L, AA, D, RR, EH, S, IY, P, EY, EH, S, AN, K, AA, T, RR, V, EN, D, UW, Z, P, W, EN,
                S, AN, S, W, AA, G, Z, AN, T, Y, IY, T, P, W, EN, UN, P, W, EN, K, AA, RR, AN, T,
                D, OE
            ]
        );
    }

    #[test]
    fn french_corpus_tech() {
        assert_eq!(
            phonemes("GPU, CPU, Rust, NVIDIA et WebGPU.", Some(Language::French)),
            vec![
                Z, EY, P, EY, UE, S, EY, P, EY, UE, RR, UE, S, T, N, V, IY, D, Y, AA, EY, W, EH, B,
                Z, EY, P, EY, UE
            ]
        );
    }

    #[test]
    fn english_corpus_1() {
        assert_eq!(
            phonemes("MASTER voice synthesis is online.", Some(Language::English)),
            vec![M, AE, S, T, ER, V, OI, S, S, IH, N, TH, AH, S, IH, S, IH, Z, AA, N, L, AI, N]
        );
    }

    #[test]
    fn english_corpus_temp() {
        assert_eq!(
            phonemes(
                "The temperature is twenty-one point five degrees.",
                Some(Language::English)
            ),
            vec![
                DH, AH, T, EH, M, P, ER, AH, CH, ER, IH, Z, T, W, EH, N, T, IY, W, AH, N, P, OI, N,
                T, F, AI, V, D, IH, G, R, IY, Z
            ]
        );
    }

    #[test]
    fn english_corpus_tech() {
        assert_eq!(
            phonemes(
                "Rust, WebGPU, NVIDIA, PostgreSQL, and UTF-8.",
                Some(Language::English)
            ),
            vec![
                R, AH, S, T, W, EH, B, JH, IY, P, IY, Y, UW, EH, N, V, IH, D, IY, AX, P, OW, S, T,
                G, R, EH, S, K, Y, UW, EH, L, AE, N, D, Y, UW, T, IY, EH, F, EY, T
            ]
        );
    }

    #[test]
    fn english_corpus_version_time() {
        assert_eq!(
            phonemes(
                "Version 3.12.4 was released at 10:45 AM.",
                Some(Language::English)
            ),
            vec![
                V, ER, ZH, AH, N, TH, R, IY, P, OI, N, T, T, W, EH, L, V, P, OI, N, T, F, AO, R, W,
                AA, Z, R, IH, L, IY, S, T, AE, T, T, EH, N, F, AO, R, T, IY, F, AI, V, EY, EH, M
            ]
        );
    }

    #[test]
    fn english_corpus_path() {
        assert_eq!(
            phonemes(
                "The file is located in slash home slash user slash repository.",
                Some(Language::English)
            ),
            vec![
                DH, AH, F, AI, L, IH, Z, L, OW, K, EY, T, IH, D, IH, N, S, L, AE, SH, H, OW, M, S,
                L, AE, SH, Y, UW, Z, ER, S, L, AE, SH, R, IH, P, AA, Z, AH, T, AO, R, IY
            ]
        );
    }

    #[test]
    fn auto_detects_french() {
        let utterance = phonemize(
            "Bonjour, le système vocal MASTER est opérationnel.",
            None,
            &overrides::Overrides::default(),
        )
        .unwrap();
        assert_eq!(utterance.language, Language::French);
    }

    #[test]
    fn empty_input_errors() {
        assert!(phonemize("   ", None, &overrides::Overrides::default()).is_err());
        assert!(phonemize("", None, &overrides::Overrides::default()).is_err());
    }
}
