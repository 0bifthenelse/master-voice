use crate::lang::Language;
use crate::overrides::Overrides;
use crate::phoneme::{Boundary, Phoneme, PhonemeKind, Stress};

pub mod en;
pub mod fr;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Boundary(Boundary),
}

pub fn tokenize(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    for piece in text.split_whitespace() {
        let all_punct = piece
            .chars()
            .all(|c| matches!(c, '.' | ',' | '!' | '?' | ';' | ':'));
        if all_punct {
            let boundary = if matches!(piece.chars().next(), Some('.') | Some('!') | Some('?')) {
                Boundary::Sentence
            } else {
                Boundary::Clause
            };
            out.push(Token::Boundary(boundary));
        } else {
            out.push(Token::Word(piece.to_string()));
        }
    }
    out
}

fn apply_word_override(word: &str, overrides: &Overrides) -> Option<Vec<(PhonemeKind, u8)>> {
    let lower = word.to_lowercase();
    let entry = overrides.get(&lower)?;
    Some(
        entry
            .iter()
            .map(|k| (*k, 0))
            .collect::<Vec<(PhonemeKind, u8)>>(),
    )
}

pub fn phonemize_tokens(tokens: &[Token], lang: Language, overrides: &Overrides) -> Vec<Phoneme> {
    let mut out: Vec<Phoneme> = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        match token {
            Token::Boundary(boundary) => {
                if let Some(last) = out.last_mut() {
                    last.boundary_after = *boundary;
                }
            }
            Token::Word(word) => {
                if word.is_empty() {
                    continue;
                }
                let next_first = tokens.get(idx + 1).and_then(|t| match t {
                    Token::Word(w) => w.chars().next(),
                    Token::Boundary(_) => None,
                });
                let phones: Vec<(PhonemeKind, u8)> =
                    if let Some(override_phones) = apply_word_override(word, overrides) {
                        override_phones
                    } else {
                        match lang {
                            Language::English => en::phonemize_word(word, next_first),
                            Language::French => fr::phonemize_word(word),
                        }
                    };
                let word_len = phones.len();
                for (i, (kind, stress)) in phones.into_iter().enumerate() {
                    let mut phoneme = Phoneme::new(kind);
                    phoneme.stress = match stress {
                        1 => Stress::Primary,
                        2 => Stress::Secondary,
                        _ => Stress::None,
                    };
                    if i + 1 == word_len {
                        phoneme.boundary_after = Boundary::Word;
                    }
                    out.push(phoneme);
                }
            }
        }
    }
    if let Some(last) = out.last_mut() {
        if last.boundary_after == Boundary::None {
            last.boundary_after = Boundary::Sentence;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phoneme::PhonemeKind::*;

    #[test]
    fn tokenizes_words_and_punct() {
        let tokens = tokenize("Hello , world .");
        assert_eq!(tokens.len(), 4);
        assert!(matches!(tokens[1], Token::Boundary(Boundary::Clause)));
        assert!(matches!(tokens[3], Token::Boundary(Boundary::Sentence)));
    }

    #[test]
    fn word_boundaries_after_words() {
        let overrides = Overrides::default();
        let tokens = tokenize("hello world");
        let phones = phonemize_tokens(&tokens, Language::English, &overrides);
        let word_boundaries = phones
            .iter()
            .filter(|p| p.boundary_after == Boundary::Word)
            .count();
        assert_eq!(word_boundaries, 2);
    }

    #[test]
    fn sentence_boundary_replaces_word() {
        let overrides = Overrides::default();
        let tokens = tokenize("hello .");
        let phones = phonemize_tokens(&tokens, Language::English, &overrides);
        assert_eq!(phones.last().unwrap().boundary_after, Boundary::Sentence);
    }

    #[test]
    fn french_via_dispatch() {
        let overrides = Overrides::default();
        let tokens = tokenize("bonjour");
        let phones = phonemize_tokens(&tokens, Language::French, &overrides);
        assert_eq!(
            phones.iter().map(|p| p.kind).collect::<Vec<_>>(),
            vec![B, ON, Z, UW, RR]
        );
    }
}
