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
            let boundary = match piece.chars().next() {
                Some('?') => Boundary::Question,
                Some('!') => Boundary::Exclaim,
                Some('.') => Boundary::Sentence,
                _ => Boundary::Clause,
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

fn emit_phones(out: &mut Vec<Phoneme>, phones: impl IntoIterator<Item = (PhonemeKind, u8, f32)>) {
    let mut phones: Vec<(PhonemeKind, u8, f32)> = phones.into_iter().collect();
    let word_len = phones.len();
    for (i, (kind, stress, shift)) in phones.drain(..).enumerate() {
        let mut phoneme = Phoneme::new(kind);
        phoneme.stress = match stress {
            1 => Stress::Primary,
            2 => Stress::Secondary,
            _ => Stress::None,
        };
        phoneme.pitch_shift = shift;
        if i + 1 == word_len {
            phoneme.boundary_after = Boundary::Word;
        }
        out.push(phoneme);
    }
}

fn flush_fr_clause(out: &mut Vec<Phoneme>, clause: &mut Vec<String>) {
    if clause.is_empty() {
        return;
    }
    let words: Vec<&str> = clause.iter().map(String::as_str).collect();
    emit_phones(out, fr::phonemize_clause(&words));
    clause.clear();
}

pub fn phonemize_tokens(tokens: &[Token], lang: Language, overrides: &Overrides) -> Vec<Phoneme> {
    let mut out: Vec<Phoneme> = Vec::new();
    if lang == Language::French {
        let mut clause: Vec<String> = Vec::new();
        for token in tokens {
            match token {
                Token::Boundary(boundary) => {
                    flush_fr_clause(&mut out, &mut clause);
                    if let Some(last) = out.last_mut() {
                        last.boundary_after = *boundary;
                    }
                }
                Token::Word(word) => {
                    if word.is_empty() {
                        continue;
                    }
                    if let Some(override_phones) = apply_word_override(word, overrides) {
                        flush_fr_clause(&mut out, &mut clause);
                        emit_phones(
                            &mut out,
                            override_phones
                                .into_iter()
                                .map(|(k, s)| (k, s, 0.0))
                                .collect::<Vec<_>>(),
                        );
                    } else {
                        clause.push(word.to_string());
                    }
                }
            }
        }
        flush_fr_clause(&mut out, &mut clause);
        if let Some(last) = out.last_mut() {
            if last.boundary_after == Boundary::None {
                last.boundary_after = Boundary::Sentence;
            }
        }
        return out;
    }
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
                        en::phonemize_word(word, next_first)
                    };
                emit_phones(&mut out, phones.into_iter().map(|(k, s)| (k, s, 0.0)));
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
            vec![B, ON, ZH, UW, RR]
        );
    }

    #[test]
    fn exclaim_token_boundary() {
        let tokens = tokenize("wow !");
        assert!(matches!(tokens[1], Token::Boundary(Boundary::Exclaim)));
    }

    #[test]
    fn exclaim_sets_final_boundary() {
        let overrides = Overrides::default();
        let tokens = tokenize("bravo !");
        let phones = phonemize_tokens(&tokens, Language::French, &overrides);
        assert_eq!(phones.last().unwrap().boundary_after, Boundary::Exclaim);
    }

    #[test]
    fn french_interjection_trailing_bang() {
        let overrides = Overrides::default();
        let tokens = tokenize("zut!");
        let phones = phonemize_tokens(&tokens, Language::French, &overrides);
        assert_eq!(
            phones.iter().map(|p| p.kind).collect::<Vec<_>>(),
            vec![Z, UE, T]
        );
    }
}
