use unicode_normalization::UnicodeNormalization;

pub fn clean_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let normalized = input.nfc().collect::<String>();
    for c in normalized.chars() {
        match c {
            '\u{feff}' | '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}' => continue,
            '\n' | '\t' => out.push(c),
            c if c.is_control() => continue,
            _ => out.push(c),
        }
    }
    out.chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' | '\u{201b}' | '\u{2032}' => '\'',
            '\u{201c}' | '\u{201d}' | '\u{201f}' | '\u{2033}' => '"',
            '\u{2013}' => '-',
            '\u{2014}' | '\u{2015}' => ',',
            '\u{2026}' => '.',
            '\u{00a0}' => ' ',
            _ => c,
        })
        .collect()
}

pub fn collapse_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pending_newline = false;
    let mut last_was_space = false;
    for c in input.chars() {
        match c {
            '\n' => {
                pending_newline = true;
                last_was_space = true;
            }
            ' ' | '\t' => {
                if !last_was_space {
                    out.push(' ');
                }
                last_was_space = true;
            }
            _ => {
                if pending_newline {
                    out.push(' ');
                    pending_newline = false;
                }
                out.push(c);
                last_was_space = false;
            }
        }
    }
    out
}

pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '-' || c == '_' || c == '.'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_controls_and_bom() {
        assert_eq!(clean_text("\u{feff}Hello\u{0} world\u{7f}"), "Hello world");
    }

    #[test]
    fn normalizes_quotes_and_dashes() {
        assert_eq!(clean_text("\u{2018}hi\u{2019} \u{2014} ok"), "'hi' , ok");
    }

    #[test]
    fn collapses_spaces_and_newlines() {
        assert_eq!(collapse_whitespace("a  b\n\n c"), "a b c");
    }
}
