use crate::phoneme::Boundary;

pub struct Sentence {
    pub text: String,
    pub boundary: Boundary,
}

fn is_abbreviation(text: &str) -> bool {
    let token = text
        .split_whitespace()
        .next_back()
        .unwrap_or_default()
        .trim_matches(['"', '\'', '(', '[', '«']);
    token.len() == 1 && token.as_bytes()[0].is_ascii_alphabetic()
        || [
            "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "vs", "etc", "mme", "mlle", "m",
        ]
        .iter()
        .any(|abbreviation| token.eq_ignore_ascii_case(abbreviation))
}

pub fn split_sentences(input: &str) -> Vec<Sentence> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut prev_char: Option<char> = None;

    let is_numberish = |c: char| c.is_ascii_digit();

    while let Some(c) = chars.next() {
        let next_char = chars.peek().copied();
        let boundary_ok = match c {
            '.' => {
                let prev_digit = prev_char.is_some_and(is_numberish);
                let next_digit = next_char.is_some_and(is_numberish);
                !(is_abbreviation(&current) || prev_digit && next_digit)
            }
            '!' | '?' | ';' => true,
            ',' | ':' => {
                let prev_digit = prev_char.is_some_and(is_numberish);
                let next_digit = next_char.is_some_and(is_numberish);
                !(prev_digit && next_digit)
            }
            '\n' | '\u{2014}' => true,
            _ => false,
        };

        if boundary_ok {
            current.push(c);
            while let Some(&next) = chars.peek() {
                if matches!(next, '"' | '\'' | ')' | ']' | '»') {
                    current.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            let boundary = match c {
                '?' if chars.peek() == Some(&'!') => {
                    current.push('!');
                    chars.next();
                    Boundary::Exclaim
                }
                '?' => Boundary::Question,
                '!' => Boundary::Exclaim,
                '.' => Boundary::Sentence,
                _ => Boundary::Clause,
            };
            let text = current.trim().to_string();
            if !text.is_empty() {
                out.push(Sentence { text, boundary });
            }
            while let Some(&next) = chars.peek() {
                if next.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }
            current = String::new();
        } else {
            current.push(c);
        }
        prev_char = Some(c);
    }

    let text = current.trim().to_string();
    if !text.is_empty() {
        out.push(Sentence {
            text,
            boundary: Boundary::Sentence,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_punctuation() {
        let sentences = split_sentences("Hello world. How are you? Fine!");
        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0].text, "Hello world.");
    }

    #[test]
    fn keeps_closing_quotes() {
        let sentences = split_sentences("He said \"hi.\"");
        assert_eq!(sentences.len(), 1);
        assert!(sentences[0].text.ends_with("hi.\""));
    }

    #[test]
    fn splits_on_commas_and_newlines() {
        let sentences = split_sentences("One, two\nthree.");
        assert_eq!(sentences.len(), 3);
    }

    #[test]
    fn keeps_decimal_numbers_intact() {
        let sentences = split_sentences("Version 3.12.4 was released at 10:45 AM.");
        assert_eq!(sentences.len(), 1);
        assert!(sentences[0].text.contains("3.12.4"));
        assert!(sentences[0].text.contains("10:45"));
    }

    #[test]
    fn keeps_french_decimal_comma() {
        let sentences = split_sentences("Il coûte 12,50 euros.");
        assert_eq!(sentences.len(), 1);
        assert!(sentences[0].text.contains("12,50"));
    }

    #[test]
    fn keeps_ips_intact() {
        let sentences = split_sentences("The address is 192.168.1.42.");
        assert_eq!(sentences.len(), 1);
        assert!(sentences[0].text.contains("192.168.1.42"));
    }

    #[test]
    fn abbreviation_dots_do_not_split_names() {
        let sentences = split_sentences("Mr. Smith arrives. Dr. Jones agrees.");
        assert_eq!(sentences.len(), 2);
        assert_eq!(sentences[0].text, "Mr. Smith arrives.");
        assert_eq!(sentences[1].text, "Dr. Jones agrees.");
    }

    #[test]
    fn exclamation_is_exclaim_boundary() {
        let sentences = split_sentences("Stop! Listen.");
        assert_eq!(sentences.len(), 2);
        assert_eq!(sentences[0].boundary, Boundary::Exclaim);
        assert_eq!(sentences[1].boundary, Boundary::Sentence);
    }

    #[test]
    fn question_exclaim_collapses_to_exclaim() {
        let sentences = split_sentences("Really?! Yes.");
        assert_eq!(sentences.len(), 2);
        assert_eq!(sentences[0].text, "Really?!");
        assert_eq!(sentences[0].boundary, Boundary::Exclaim);
        assert_eq!(sentences[1].boundary, Boundary::Sentence);
    }
}
