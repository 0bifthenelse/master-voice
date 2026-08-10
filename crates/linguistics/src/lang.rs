#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    French,
    English,
}

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Language::French => "fr-FR",
            Language::English => "en-US",
        }
    }

    pub fn from_code(input: &str) -> Option<Language> {
        match input.to_ascii_lowercase().as_str() {
            "fr" | "fr-fr" | "fra" | "fre" | "french" => Some(Language::French),
            "en" | "en-us" | "en-gb" | "eng" | "english" => Some(Language::English),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Language::French => "French",
            Language::English => "English",
        }
    }
}

pub const FRENCH_MARKERS: [&str; 73] = [
    "le",
    "la",
    "les",
    "un",
    "une",
    "des",
    "du",
    "de",
    "et",
    "est",
    "je",
    "vous",
    "nous",
    "il",
    "elle",
    "ils",
    "elles",
    "ce",
    "cette",
    "ces",
    "mon",
    "ma",
    "mes",
    "ton",
    "ta",
    "tes",
    "son",
    "sa",
    "ses",
    "notre",
    "votre",
    "leur",
    "leurs",
    "qui",
    "que",
    "quoi",
    "dans",
    "pour",
    "avec",
    "sans",
    "sur",
    "sous",
    "par",
    "pas",
    "plus",
    "mais",
    "ou",
    "où",
    "donc",
    "car",
    "ni",
    "si",
    "très",
    "bien",
    "tout",
    "tous",
    "aussi",
    "encore",
    "déjà",
    "après",
    "avant",
    "entre",
    "chez",
    "vers",
    "selon",
    "système",
    "aujourd'hui",
    "français",
    "française",
    "bonjour",
    "merci",
    "fonctionne",
    "opérationnel",
];

pub const ENGLISH_MARKERS: [&str; 68] = [
    "the",
    "and",
    "of",
    "to",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "you",
    "your",
    "we",
    "they",
    "this",
    "that",
    "these",
    "those",
    "with",
    "from",
    "for",
    "not",
    "have",
    "has",
    "had",
    "will",
    "would",
    "can",
    "could",
    "should",
    "about",
    "there",
    "which",
    "what",
    "when",
    "where",
    "who",
    "how",
    "because",
    "but",
    "so",
    "if",
    "then",
    "than",
    "very",
    "also",
    "just",
    "like",
    "more",
    "most",
    "some",
    "any",
    "only",
    "into",
    "through",
    "during",
    "before",
    "after",
    "while",
    "system",
    "online",
    "please",
    "hello",
    "thanks",
    "works",
    "voice",
    "synthesis",
    "temperature",
];

fn has_french_diacritic(word: &str) -> bool {
    word.chars().any(|character| {
        matches!(
            character,
            'é' | 'è' | 'ê' | 'ë' | 'à' | 'â' | 'ç' | 'ù' | 'û' | 'î' | 'ï' | 'ô' | 'œ'
        )
    })
}

pub fn route_words(words: &[&str]) -> Vec<Language> {
    let cleaned: Vec<String> = words
        .iter()
        .map(|word| {
            word.trim_matches(|character: char| {
                !character.is_alphanumeric() && character != '\'' && character != '-'
            })
            .to_lowercase()
        })
        .collect();
    let confident: Vec<Option<Language>> = cleaned
        .iter()
        .map(|word| {
            let french_marker = FRENCH_MARKERS.contains(&word.as_str());
            let english_marker = ENGLISH_MARKERS.contains(&word.as_str());
            let english_special = ["mr", "mrs", "ms", "dr", "smith"].contains(&word.as_str());
            let french_dictionary = crate::g2p::fr::has_dictionary_word(word);
            let english_dictionary = crate::dict::en::lookup(word).is_some();
            if has_french_diacritic(word)
                || french_marker && !english_marker
                || french_dictionary && !english_dictionary
            {
                Some(Language::French)
            } else if english_special
                || english_marker && !french_marker
                || english_dictionary && !french_dictionary
            {
                Some(Language::English)
            } else {
                None
            }
        })
        .collect();
    let fallback = detect(&cleaned.join(" ")).unwrap_or(Language::French);
    confident
        .iter()
        .enumerate()
        .map(|(index, language)| {
            if let Some(language) = language {
                return *language;
            }
            let left =
                confident[..index]
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(candidate, language)| {
                        language.map(|language| (index - candidate, language))
                    });
            let right = confident[index + 1..]
                .iter()
                .enumerate()
                .find_map(|(offset, language)| language.map(|language| (offset + 1, language)));
            match (left, right) {
                (Some((left_distance, left_language)), Some((right_distance, right_language))) => {
                    if left_distance <= right_distance {
                        left_language
                    } else {
                        right_language
                    }
                }
                (Some((_, language)), None) | (None, Some((_, language))) => language,
                (None, None) => fallback,
            }
        })
        .collect()
}

pub fn detect(text: &str) -> Option<Language> {
    let lower = text.to_lowercase();
    let words: Vec<String> = lower
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-')
                .to_string()
        })
        .collect();
    let mut fr_score = 0usize;
    let mut en_score = 0usize;

    for word in &words {
        if FRENCH_MARKERS.contains(&word.as_str()) {
            fr_score += 1;
        }
        if ENGLISH_MARKERS.contains(&word.as_str()) {
            en_score += 1;
        }
    }

    let diacritics: usize = lower
        .chars()
        .filter(|c| {
            matches!(
                c,
                'é' | 'è' | 'ê' | 'ë' | 'à' | 'â' | 'ç' | 'ù' | 'û' | 'î' | 'ï' | 'ô' | 'œ'
            )
        })
        .count();
    fr_score += diacritics * 2;

    if fr_score > en_score {
        Some(Language::French)
    } else if en_score > fr_score {
        Some(Language::English)
    } else if diacritics >= 2 {
        Some(Language::French)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_french() {
        assert_eq!(
            detect("Bonjour, le système vocal MASTER est opérationnel."),
            Some(Language::French)
        );
        assert_eq!(
            detect("Aujourd'hui, nous sommes à Auch."),
            Some(Language::French)
        );
        assert_eq!(
            detect("Il coûte 12,50 euros et sera disponible à 14 h 35."),
            Some(Language::French)
        );
    }

    #[test]
    fn detects_english() {
        assert_eq!(
            detect("MASTER voice synthesis is online."),
            Some(Language::English)
        );
        assert_eq!(
            detect("The temperature is twenty-one point five degrees."),
            Some(Language::English)
        );
        assert_eq!(
            detect("Version 3.12.4 was released at 10:45 AM."),
            Some(Language::English)
        );
    }

    #[test]
    fn no_false_positive_on_est_si() {
        assert_eq!(
            detect("The best system is online."),
            Some(Language::English)
        );
    }
}
