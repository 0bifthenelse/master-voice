use crate::phoneme::Boundary;

pub struct Sentence {
    pub text: String,
    pub boundary: Boundary,
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
                !(prev_digit && next_digit)
            }
            '!' | '?' | ';' => true,
            ',' | ':' => {
                let prev_digit = prev_char.is_some_and(is_numberish);
                let next_digit = next_char.is_some_and(is_numberish);
                !(prev_digit && next_digit)
            }
            '\n' | '—' => true,
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
            let text = current.trim().to_string();
            if !text.is_empty() {
                let boundary = if c == '?' {
                    Boundary::Question
                } else if matches!(c, '.' | '!') {
                    Boundary::Sentence
                } else {
                    Boundary::Clause
                };
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
    fn abbreviation_dots_keep_text() {
        let sentences = split_sentences("Mr. Smith arrives.");
        assert_eq!(sentences[0].text, "Mr.");
        assert_eq!(sentences[1].text, "Smith arrives.");
    }
}
