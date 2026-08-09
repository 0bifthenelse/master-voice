use crate::lang::Language;
use crate::numbers;
use crate::unicode::{clean_text, collapse_whitespace, is_word_char};

pub struct NormalizeOptions {
    pub read_urls: bool,
    pub read_paths: bool,
}

impl Default for NormalizeOptions {
    fn default() -> Self {
        Self {
            read_urls: true,
            read_paths: true,
        }
    }
}

const EN_MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

const ABBREV_EN: [(&str, &str); 16] = [
    ("mr.", "mister"),
    ("mrs.", "missus"),
    ("ms.", "miss"),
    ("dr.", "doctor"),
    ("st.", "saint"),
    ("vs.", "versus"),
    ("etc.", "etcetera"),
    ("e.g.", "for example"),
    ("i.e.", "that is"),
    ("approx.", "approximately"),
    ("fig.", "figure"),
    ("no.", "number"),
    ("inc.", "incorporated"),
    ("ltd.", "limited"),
    ("dept.", "department"),
    ("est.", "estimated"),
];

const ABBREV_FR: [(&str, &str); 10] = [
    ("m.", "monsieur"),
    ("mm.", "messieurs"),
    ("mme", "madame"),
    ("mlle", "mademoiselle"),
    ("dr.", "docteur"),
    ("st.", "saint"),
    ("etc.", "et cetera"),
    ("ex.", "exemple"),
    ("av.", "avenue"),
    ("n°", "numéro"),
];

const PHRASE_DICT: [(&str, &str); 1] = [("postgresql", "postgres Q L")];

fn strip_markdown(input: &str) -> String {
    let mut text = input.to_string();
    text = text.replace("```", "\u{0}\u{1}\u{0}");
    let mut out = String::new();
    let mut in_fence = false;
    let mut in_inline_code = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{0}' => {
                if chars.peek() == Some(&'\u{1}') {
                    chars.next();
                    if chars.peek() == Some(&'\u{0}') {
                        chars.next();
                        in_fence = !in_fence;
                    }
                }
            }
            '`' if !in_fence => {
                in_inline_code = !in_inline_code;
            }
            _ if in_fence => continue,
            _ if !in_inline_code && c == '#' => {
                let mut next = chars.peek();
                let mut count = 1;
                while next == Some(&'#') {
                    chars.next();
                    count += 1;
                    next = chars.peek();
                }
                if count <= 6 && next == Some(&' ') {
                    continue;
                }
                out.push(c);
            }
            _ if !in_inline_code && c == '>' => {
                if out.ends_with('\n') || out.is_empty() {
                    continue;
                }
                out.push(c);
            }
            _ if !in_inline_code && (c == '*' || c == '_') => {
                let mut next = chars.peek();
                let mut count = 1;
                while next == Some(&c) {
                    chars.next();
                    count += 1;
                    next = chars.peek();
                }
                if count <= 3 {
                    continue;
                }
                for _ in 0..count {
                    out.push(c);
                }
            }
            _ if !in_inline_code && c == '~' => continue,
            _ if !in_inline_code && c == '[' => {
                let rest: String = chars.clone().take(200).collect();
                if let Some(close) = rest.find(']') {
                    let after = rest[close + 1..].chars().next();
                    if after == Some('(') {
                        let label = &rest[..close];
                        if let Some(paren) = rest[close + 1..].find(')') {
                            let url = &rest[close + 1..close + 1 + paren];
                            if !url.starts_with("http") {
                                out.push_str(label);
                            }
                            for _ in 0..=close + 1 + paren {
                                chars.next();
                            }
                            continue;
                        }
                    }
                }
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn expand_url(url: &str, lang: Language) -> String {
    let cleaned = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    let cleaned = cleaned.trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '>']);
    let parts: Vec<&str> = cleaned.split(['/', '?', '#']).collect();
    let domain = parts[0];
    let mut out = Vec::new();
    for (i, segment) in domain.split('.').enumerate() {
        if i > 0 {
            out.push(
                match lang {
                    Language::English => "dot",
                    Language::French => "point",
                }
                .to_string(),
            );
        }
        if !segment.is_empty() {
            out.push(segment.to_string());
        }
    }
    if out.is_empty() {
        return match lang {
            Language::English => "link".to_string(),
            Language::French => "lien".to_string(),
        };
    }
    out.join(" ")
}

fn expand_path(path: &str, lang: Language) -> String {
    let slash = match lang {
        Language::English => "slash",
        Language::French => "slash",
    };
    let dot = match lang {
        Language::English => "dot",
        Language::French => "point",
    };
    let mut out = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() {
            if out.last().is_none_or(|w| w != slash) {
                out.push(slash.to_string());
            }
            continue;
        }
        if let Some(ext) = segment.rsplit_once('.') {
            if ext.1.len() <= 5
                && !ext.1.is_empty()
                && ext.1.chars().all(|c| c.is_ascii_alphabetic())
            {
                out.push(ext.0.to_string());
                out.push(dot.to_string());
                out.push(ext.1.to_string());
                continue;
            }
        }
        out.push(segment.to_string());
    }
    if out.is_empty() {
        return slash.to_string();
    }
    out.join(" ")
}

fn is_ip(groups: &[&str]) -> bool {
    groups.len() == 4
        && groups
            .iter()
            .all(|g| !g.is_empty() && g.len() <= 3 && g.chars().all(|c| c.is_ascii_digit()))
}

fn classify_number_run(run: &str, lang: Language) -> String {
    let digits_only = run.replace([' ', ','], "").replace('.', "");
    if digits_only.chars().all(|c| c.is_ascii_digit()) {
        let n: u64 = digits_only.parse().unwrap_or(0);
        return numbers::cardinal(n, lang);
    }
    run.to_string()
}

const INITIALISMS: [&str; 18] = [
    "AI", "AM", "API", "CPU", "GPU", "GUI", "IDE", "IO", "IP", "OS", "PM", "RAM", "ROM", "UI",
    "URL", "USB", "UTF", "UX",
];

fn is_initialism(word: &str) -> bool {
    let letters: String = word.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if letters.is_empty() {
        return true;
    }
    if INITIALISMS.contains(&letters.as_str()) {
        return true;
    }
    !letters
        .chars()
        .any(|c| matches!(c, 'A' | 'E' | 'I' | 'O' | 'U' | 'Y'))
}

fn push_initialism_or_word(out: &mut Vec<String>, word: &str) {
    if is_initialism(word) {
        for letter in word.chars().filter(|x| *x != '-') {
            out.push(letter.to_string());
        }
    } else {
        out.push(word.to_lowercase());
    }
}

pub fn normalize_sentence(input: &str, lang: Language, opts: &NormalizeOptions) -> String {
    let cleaned = clean_text(input);
    let stripped = strip_markdown(&cleaned);
    let stripped = collapse_whitespace(&stripped);
    let mut out: Vec<String> = Vec::new();
    let mut chars = stripped.chars().peekable();

    while let Some(c) = chars.peek().copied() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c.is_ascii_digit() {
            let mut run = String::new();
            let mut has_comma = false;
            let mut has_dot = false;
            let mut h_seen = false;
            let mut has_colon = false;
            loop {
                match chars.peek().copied() {
                    Some(d) if d.is_ascii_digit() => {
                        run.push(d);
                        chars.next();
                    }
                    Some(d) if matches!(d, ',' | '.') => {
                        let mut look = chars.clone();
                        look.next();
                        if look.peek().copied().is_some_and(|a| a.is_ascii_digit()) {
                            if d == ',' {
                                has_comma = true;
                            } else {
                                has_dot = true;
                            }
                            run.push(d);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    Some(d) if matches!(d, 'h' | 'H') && !has_comma && !has_dot && !h_seen => {
                        run.push('h');
                        h_seen = true;
                        chars.next();
                    }
                    Some(':') if !has_comma && !has_dot && !h_seen && !has_colon => {
                        run.push(':');
                        has_colon = true;
                        chars.next();
                    }
                    Some(' ') | Some('\t') => {
                        let mut look = chars.clone();
                        look.next();
                        match look.peek().copied() {
                            Some(d)
                                if matches!(d, 'h' | 'H') && !has_comma && !has_dot && !h_seen =>
                            {
                                run.push('h');
                                h_seen = true;
                                chars.next();
                                chars.next();
                            }
                            Some(d)
                                if d.is_ascii_digit() && (h_seen || (!has_comma && !has_dot)) =>
                            {
                                let mut digits_ahead = look.clone();
                                let mut count = 0usize;
                                while digits_ahead
                                    .peek()
                                    .copied()
                                    .is_some_and(|x| x.is_ascii_digit())
                                {
                                    count += 1;
                                    digits_ahead.next();
                                }
                                if h_seen || count <= 3 {
                                    run.push(' ');
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            _ => break,
                        }
                    }
                    _ => break,
                }
            }
            let raw = run.clone();
            if h_seen {
                let digits: Vec<&str> = raw.split('h').collect();
                let hours: u64 = digits[0].parse().unwrap_or(0);
                let minutes = if digits.len() > 1 {
                    digits[1].trim().parse::<u64>().ok()
                } else {
                    None
                };
                let hour_word = match lang {
                    Language::English => {
                        let hour_word = match hours {
                            0 => "midnight".to_string(),
                            12 => "noon".to_string(),
                            _ if hours > 12 => numbers::cardinal(hours - 12, lang),
                            _ => numbers::cardinal(hours, lang),
                        };
                        match minutes {
                            None => hour_word,
                            Some(0) => format!("{} o'clock", hour_word),
                            Some(m) => format!("{} {}", hour_word, numbers::cardinal(m, lang)),
                        }
                    }
                    Language::French => {
                        let hour_word = match hours {
                            0 => "minuit".to_string(),
                            12 => "midi".to_string(),
                            _ => numbers::cardinal(hours, lang),
                        };
                        match minutes {
                            None => hour_word,
                            Some(0) => format!("{} heures", hour_word),
                            Some(m) => {
                                format!("{} heures {}", hour_word, numbers::cardinal(m, lang))
                            }
                        }
                    }
                };
                out.push(hour_word);
                continue;
            }
            if raw.contains(':') {
                let digits: Vec<&str> = raw.split(':').collect();
                let hours: u64 = digits[0].parse().unwrap_or(0);
                let minutes: u64 = digits.get(1).and_then(|d| d.parse().ok()).unwrap_or(0);
                let seconds = digits.get(2).and_then(|d| d.parse::<u64>().ok());
                let hour_word = match lang {
                    Language::English => match hours {
                        0 => "midnight".to_string(),
                        12 => "noon".to_string(),
                        _ if hours > 12 => numbers::cardinal(hours - 12, lang),
                        _ => numbers::cardinal(hours, lang),
                    },
                    Language::French => match hours {
                        0 => "minuit".to_string(),
                        12 => "midi".to_string(),
                        _ => numbers::cardinal(hours, lang),
                    },
                };
                if let Some(secs) = seconds {
                    out.push(format!(
                        "{} {} {}",
                        hour_word,
                        numbers::cardinal(minutes, lang),
                        numbers::cardinal(secs, lang)
                    ));
                } else if minutes == 0 {
                    out.push(format!(
                        "{} {}",
                        hour_word,
                        match lang {
                            Language::English => "o'clock",
                            Language::French => "heures",
                        }
                    ));
                } else if minutes < 10 && lang == Language::English {
                    out.push(format!(
                        "{} oh {}",
                        hour_word,
                        numbers::cardinal(minutes, lang)
                    ));
                } else {
                    out.push(format!(
                        "{} {}",
                        hour_word,
                        numbers::cardinal(minutes, lang)
                    ));
                }
                continue;
            }
            let groups: Vec<&str> = raw.split('.').collect();
            if is_ip(&groups) {
                let mut parts = Vec::new();
                for (i, g) in groups.iter().enumerate() {
                    if i > 0 {
                        parts.push(
                            match lang {
                                Language::English => "point",
                                Language::French => "point",
                            }
                            .to_string(),
                        );
                    }
                    parts.push(numbers::cardinal(g.parse().unwrap_or(0), lang));
                }
                out.push(parts.join(" "));
                continue;
            }
            if groups.len() >= 3 && groups.iter().all(|g| !g.is_empty()) {
                let mut parts = Vec::new();
                for (i, g) in groups.iter().enumerate() {
                    if i > 0 {
                        parts.push("point".to_string());
                    }
                    parts.push(numbers::cardinal(g.parse().unwrap_or(0), lang));
                }
                out.push(parts.join(" "));
                continue;
            }
            if has_comma && !has_dot {
                let groups: Vec<&str> = raw.split(',').collect();
                let thousands = groups.len() > 1 && groups[1..].iter().all(|g| g.len() == 3);
                if thousands {
                    let digits = raw.replace(',', "");
                    out.push(numbers::cardinal(digits.parse().unwrap_or(0), lang));
                    continue;
                }
                let integer_part = groups[0];
                let frac_part = groups.get(1).copied().unwrap_or("");
                let integer_word = numbers::cardinal(
                    integer_part.replace([' ', ','], "").parse().unwrap_or(0),
                    lang,
                );
                let frac_word = numbers::digits_spelled(frac_part, lang);
                out.push(format!(
                    "{} {} {}",
                    integer_word,
                    match lang {
                        Language::English => "point",
                        Language::French => "virgule",
                    },
                    frac_word
                ));
                continue;
            }
            if has_dot {
                let integer_part = raw.split('.').next().unwrap_or("");
                let frac_part = raw.split('.').nth(1).unwrap_or("");
                let integer_word = numbers::cardinal(
                    integer_part.replace([' ', ','], "").parse().unwrap_or(0),
                    lang,
                );
                let frac_word = numbers::digits_spelled(frac_part, lang);
                out.push(format!("{} point {}", integer_word, frac_word));
                continue;
            }
            let plain_digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
            if !plain_digits.is_empty() && plain_digits.len() == raw.len() {
                let n: u64 = plain_digits.parse().unwrap_or(0);
                let rest: String = chars.clone().collect();
                let mut consumed = 0usize;
                let mut word: Option<String> = None;
                if lang == Language::English {
                    for suffix in ["st", "nd", "rd", "th"] {
                        if rest.starts_with(suffix)
                            && rest.len() > 2
                            && !rest[2..]
                                .chars()
                                .next()
                                .is_some_and(|c| c.is_alphanumeric())
                        {
                            word = Some(numbers::ordinal(n, lang));
                            consumed = 2;
                            break;
                        }
                    }
                    if word.is_none() && plain_digits.len() == 4 && (1000..=2099).contains(&n) {
                        word = Some(numbers::year(n, lang));
                    }
                    if word.is_none() && n <= 31 {
                        if let Some(prev) = out.last() {
                            if EN_MONTHS.contains(&prev.to_lowercase().as_str()) {
                                word = Some(numbers::ordinal(n, lang));
                            }
                        }
                        if word.is_none() && rest.to_lowercase().starts_with(" march")
                            || word.is_none() && rest.to_lowercase().starts_with(" may")
                            || word.is_none() && rest.to_lowercase().starts_with(" june")
                        {
                            word = Some(numbers::ordinal(n, lang));
                        }
                    }
                } else {
                    for (suffix, len) in [
                        ("er", 2),
                        ("re", 2),
                        ("me", 2),
                        ("ème", 3),
                        ("nd", 2),
                        ("e", 1),
                    ] {
                        if rest.starts_with(suffix)
                            && rest.len() > len
                            && !rest[len..]
                                .chars()
                                .next()
                                .is_some_and(|c| c.is_alphanumeric())
                        {
                            if n != 1 || matches!(suffix, "er" | "re") {
                                word = Some(numbers::ordinal(n, lang));
                            }
                            consumed = len;
                            break;
                        }
                    }
                }
                if let Some(w) = word {
                    out.push(w);
                    for _ in 0..consumed {
                        chars.next();
                    }
                    continue;
                }
            }
            out.push(classify_number_run(&raw, lang));
            continue;
        }
        if c == '/' || c == '\\' {
            if opts.read_paths {
                let mut path = String::new();
                while let Some(&d) = chars.peek() {
                    if is_word_char(d) || matches!(d, '/' | '\\') {
                        path.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if path.len() > 1 {
                    out.push(expand_path(&path.replace('\\', "/"), lang));
                    continue;
                }
                chars.next();
                out.push(
                    match lang {
                        Language::English => "slash",
                        Language::French => "slash",
                    }
                    .to_string(),
                );
                continue;
            }
            chars.next();
            out.push(
                match lang {
                    Language::English => "slash",
                    Language::French => "slash",
                }
                .to_string(),
            );
            continue;
        }
        if c.is_alphabetic() || c == '\'' || c == '_' {
            if opts.read_urls {
                let rest: String = chars.clone().collect();
                let prefix = ["http://", "https://", "www."]
                    .iter()
                    .find(|p| rest.starts_with(**p))
                    .copied();
                if let Some(prefix) = prefix {
                    let after = &rest[prefix.len()..];
                    let end = after
                        .find(|ch: char| ch.is_whitespace())
                        .unwrap_or(after.len());
                    let url = format!("{}{}", prefix, &after[..end]);
                    for _ in 0..url.chars().count() {
                        chars.next();
                    }
                    out.push(expand_url(&url, lang));
                    continue;
                }
            }
            let mut word = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_alphanumeric() || matches!(d, '\'' | '_' | '-') {
                    word.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            let lower = word.to_lowercase();
            if let Some(expansion) = PHRASE_DICT
                .iter()
                .find(|(w, _)| *w == lower)
                .map(|(_, e)| *e)
            {
                out.push(expansion.to_string());
                continue;
            }
            let version_digits = word
                .strip_prefix(['v', 'V'])
                .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()));
            if let Some(rest) = version_digits {
                let mut version = format!(
                    "version {}",
                    numbers::cardinal(rest.parse().unwrap_or(0), lang)
                );
                loop {
                    let mut look = chars.clone();
                    if look.peek() == Some(&'.') {
                        look.next();
                        if look.peek().is_some_and(|a| a.is_ascii_digit()) {
                            let mut seg = String::new();
                            while let Some(&d) = look.peek() {
                                if d.is_ascii_digit() {
                                    seg.push(d);
                                    look.next();
                                } else {
                                    break;
                                }
                            }
                            version.push_str(" point ");
                            version.push_str(&numbers::cardinal(seg.parse().unwrap_or(0), lang));
                            chars = look;
                            continue;
                        }
                    }
                    break;
                }
                out.push(version);
                continue;
            }
            if word
                .chars()
                .all(|x| x.is_ascii_uppercase() || x == '-' || x.is_ascii_digit())
                && word.len() >= 2
            {
                push_initialism_or_word(&mut out, &word);
                continue;
            }
            if word.chars().any(|x| x.is_ascii_digit()) {
                let mut runs: Vec<String> = Vec::new();
                let mut current = String::new();
                let mut current_is_digit = false;
                for ch in word.chars() {
                    let is_digit = ch.is_ascii_digit();
                    if current.is_empty() {
                        current.push(ch);
                        current_is_digit = is_digit;
                    } else if is_digit == current_is_digit {
                        current.push(ch);
                    } else {
                        runs.push(std::mem::take(&mut current));
                        current.push(ch);
                        current_is_digit = is_digit;
                    }
                }
                if !current.is_empty() {
                    runs.push(current);
                }
                for (i, run) in runs.iter().enumerate() {
                    if i > 0 {
                        out.push(" ".to_string());
                    }
                    if run.chars().all(|x| x.is_ascii_digit()) {
                        out.push(numbers::cardinal(run.parse().unwrap_or(0), lang));
                    } else {
                        out.push(run.clone());
                    }
                }
                continue;
            }
            let version_digits = word
                .strip_prefix(['v', 'V'])
                .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()));
            if let Some(rest) = version_digits {
                let mut version = format!(
                    "version {}",
                    numbers::cardinal(rest.parse().unwrap_or(0), lang)
                );
                loop {
                    let mut look = chars.clone();
                    if look.peek() == Some(&'.') {
                        look.next();
                        if look.peek().is_some_and(|a| a.is_ascii_digit()) {
                            let mut seg = String::new();
                            while let Some(&d) = look.peek() {
                                if d.is_ascii_digit() {
                                    seg.push(d);
                                    look.next();
                                } else {
                                    break;
                                }
                            }
                            version.push_str(" point ");
                            version.push_str(&numbers::cardinal(seg.parse().unwrap_or(0), lang));
                            chars = look;
                            continue;
                        }
                    }
                    break;
                }
                out.push(version);
                continue;
            }
            let abbrev = if lang == Language::French {
                ABBREV_FR.iter().find(|(a, _)| {
                    lower.starts_with(a)
                        && (lower.len() == a.len()
                            || lower.starts_with(&format!("{}.", a))
                            || a.ends_with('.'))
                })
            } else {
                ABBREV_EN.iter().find(|(a, _)| {
                    lower.starts_with(a)
                        || lower.starts_with(&format!("{}.", a.trim_end_matches('.')))
                })
            };
            if let Some((a, expansion)) = abbrev {
                if lower.starts_with(a) && word.len() >= a.len() {
                    out.push(expansion.to_string());
                    let consumed = a.trim_end_matches('.').len();
                    for _ in 0..consumed {
                        chars.next();
                    }
                    if chars.peek() == Some(&'.') {
                        chars.next();
                    }
                    continue;
                }
            }
            if c == '_' && word.len() > 1 && word.chars().all(|x| x == '_') {
                out.push(
                    match lang {
                        Language::English => "underscore",
                        Language::French => "tiret bas",
                    }
                    .to_string(),
                );
                continue;
            }
            if word.contains('_') && !word.contains('\'') {
                let parts: Vec<&str> = word.split('_').collect();
                let mut joined = String::new();
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        joined.push(' ');
                    }
                    joined.push_str(part);
                }
                out.push(joined);
                continue;
            }
            let mut split = Vec::new();
            let mut current = String::new();
            let mut prev_lower = false;
            for ch in word.chars() {
                if ch.is_ascii_uppercase() && prev_lower && !current.is_empty() {
                    split.push(std::mem::take(&mut current));
                }
                current.push(ch);
                prev_lower = ch.is_lowercase();
            }
            if !current.is_empty() {
                split.push(current);
            }
            if split.len() > 1 {
                for part in split {
                    if part.len() >= 2 && part.chars().all(|x| x.is_ascii_uppercase()) {
                        push_initialism_or_word(&mut out, &part);
                    } else {
                        out.push(part);
                    }
                }
                continue;
            }
            out.push(word);
            continue;
        }
        match c {
            '&' => {
                out.push(
                    match lang {
                        Language::English => "and",
                        Language::French => "et",
                    }
                    .to_string(),
                );
            }
            '%' => {
                out.push(
                    match lang {
                        Language::English => "percent",
                        Language::French => "pour cent",
                    }
                    .to_string(),
                );
            }
            '$' => out.push("dollars".to_string()),
            '€' => out.push("euros".to_string()),
            '£' => out.push("pounds".to_string()),
            '@' => {
                out.push(
                    match lang {
                        Language::English => "at",
                        Language::French => "arobase",
                    }
                    .to_string(),
                );
            }
            '#' => out.push("number".to_string()),
            '+' => out.push("plus".to_string()),
            '=' => out.push("equals".to_string()),
            '*' => out.push("star".to_string()),
            '°' => {
                out.push(
                    match lang {
                        Language::English => "degrees",
                        Language::French => "degrés",
                    }
                    .to_string(),
                );
            }
            '|' => out.push("pipe".to_string()),
            '^' => out.push("caret".to_string()),
            '~' => out.push("tilde".to_string()),
            '.' | ',' | '!' | '?' | ';' | ':' => {
                out.push(c.to_string());
            }
            '"' | '(' | ')' | '[' | ']' | '{' | '}' | '«' | '»' => {}
            _ => {}
        }
        chars.next();
    }

    let mut joined = out.join(" ");
    joined = joined
        .split_whitespace()
        .map(|w| w.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    joined = collapse_whitespace(&joined);

    joined = expand_en_am_pm(&joined, lang);

    joined
}

fn expand_en_am_pm(input: &str, lang: Language) -> String {
    let mut out = input.to_string();
    out = out.replace(
        " A M",
        match lang {
            Language::English => " A M",
            Language::French => " du matin",
        },
    );
    out = out.replace(
        " P M",
        match lang {
            Language::English => " P M",
            Language::French => " de l après-midi",
        },
    );
    out
}

pub fn expand_ordinal_suffix(input: &str, lang: Language) -> Option<String> {
    let lower = input.to_lowercase();
    if lang == Language::English {
        let digits = lower.trim_end_matches(['s', 't', 'n', 'd', 'r', 'h']);
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let suffix = &lower[digits.len()..];
        let n: u64 = digits.parse().ok()?;
        if n == 0 {
            return None;
        }
        if !matches!(suffix, "st" | "nd" | "rd" | "th") {
            return None;
        }
        let expected = if (n % 100) / 10 == 1 {
            "th"
        } else {
            match n % 10 {
                1 => "st",
                2 => "nd",
                3 => "rd",
                _ => "th",
            }
        };
        if suffix != expected {
            return None;
        }
        return Some(numbers::ordinal(n, lang));
    }
    for (suffix, len) in [
        ("er", 2),
        ("re", 2),
        ("ème", 3),
        ("me", 2),
        ("nd", 2),
        ("de", 2),
        ("e", 1),
    ] {
        if lower.ends_with(suffix) && lower.chars().count() > len {
            let digits: String = lower.chars().take(lower.chars().count() - len).collect();
            if digits.chars().all(|c| c.is_ascii_digit()) {
                let n: u64 = digits.parse().ok()?;
                if n == 0 {
                    return None;
                }
                if n == 1 && matches!(suffix, "er" | "re") {
                    return Some("premier".to_string());
                }
                return Some(numbers::ordinal(n, lang));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm_en(text: &str) -> String {
        normalize_sentence(text, Language::English, &NormalizeOptions::default())
    }

    fn norm_fr(text: &str) -> String {
        normalize_sentence(text, Language::French, &NormalizeOptions::default())
    }

    #[test]
    fn english_numbers() {
        assert_eq!(norm_en("42 percent"), "forty-two percent");
        assert_eq!(
            norm_en("The GPU is running at 42 percent."),
            "The G P U is running at forty-two percent ."
        );
        assert_eq!(
            norm_en("Version 3.12.4 was released at 10:45 AM."),
            "Version three point twelve point four was released at ten forty-five A M ."
        );
        assert_eq!(norm_en("1,000"), "one thousand");
    }

    #[test]
    fn english_ip() {
        assert_eq!(
            norm_en("The address is 192.168.1.42."),
            "The address is one hundred ninety-two point one hundred sixty-eight point one point forty-two ."
        );
    }

    #[test]
    fn french_numbers() {
        assert_eq!(
            norm_fr("Il coûte 12,50 euros"),
            "Il coûte douze virgule cinq zéro euros"
        );
        assert_eq!(norm_fr("à 14 h 35"), "à quatorze heures trente-cinq");
        assert_eq!(
            norm_fr("L'adresse IP est 192.168.1.42."),
            "L'adresse I P est cent quatre-vingt-douze point cent soixante-huit point un point quarante-deux ."
        );
    }

    #[test]
    fn english_ordinal_suffix() {
        assert_eq!(
            expand_ordinal_suffix("21st", Language::English),
            Some("twenty-first".to_string())
        );
        assert_eq!(
            expand_ordinal_suffix("3rd", Language::English),
            Some("third".to_string())
        );
        assert_eq!(
            expand_ordinal_suffix("12th", Language::English),
            Some("twelfth".to_string())
        );
        assert_eq!(
            expand_ordinal_suffix("5ème", Language::French),
            Some("cinquième".to_string())
        );
    }

    #[test]
    fn strips_markdown() {
        assert_eq!(
            norm_en("Hello **bold** and `code` here."),
            "Hello bold and code here ."
        );
        assert_eq!(norm_en("# Heading\nBody text"), "Heading Body text");
    }

    #[test]
    fn expands_urls() {
        assert_eq!(
            norm_en("see https://example.com now"),
            "see example dot com now"
        );
    }

    #[test]
    fn camel_case_and_acronyms() {
        assert_eq!(
            norm_en("WebGPU and GPU and NVIDIA"),
            "Web G P U and G P U and nvidia"
        );
        assert_eq!(
            norm_fr("GPU, CPU, Rust, NVIDIA et WebGPU"),
            "G P U , C P U , Rust , nvidia et Web G P U"
        );
    }

    #[test]
    fn paths_and_versions() {
        assert_eq!(
            norm_en("The file is in /home/user/repo/main.rs"),
            "The file is in slash home user repo main dot rs"
        );
        assert_eq!(norm_en("v1.2.3"), "version one point two point three");
    }

    #[test]
    fn english_dates() {
        assert_eq!(norm_en("March 5, 2026"), "March fifth , twenty twenty-six");
        assert_eq!(norm_en("May 1 2026"), "May first twenty twenty-six");
    }
}
