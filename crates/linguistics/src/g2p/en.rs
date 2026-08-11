use crate::dict::en::lookup;
use crate::phoneme::PhonemeKind::{self, *};

pub struct EnPhonemes {
    pub kinds: Vec<PhonemeKind>,
    pub stress: Vec<u8>,
}

fn spell_letter(c: char) -> Vec<(PhonemeKind, u8)> {
    match c.to_ascii_lowercase() {
        '0' => vec![(Z, 1), (IY, 0), (R, 0), (OW, 0)],
        '1' => vec![(W, 1), (AH, 0), (N, 0)],
        '2' => vec![(T, 1), (UW, 0)],
        '3' => vec![(TH, 1), (R, 0), (IY, 0)],
        '4' => vec![(F, 1), (AO, 0), (R, 0)],
        '5' => vec![(F, 1), (AI, 0), (V, 0)],
        '6' => vec![(S, 1), (IH, 0), (K, 0), (S, 0)],
        '7' => vec![(S, 1), (EH, 0), (V, 0), (AX, 0), (N, 0)],
        '8' => vec![(EY, 1), (T, 0)],
        '9' => vec![(N, 1), (AI, 0), (N, 0)],
        'a' => vec![(EY, 1)],
        'b' => vec![(B, 1), (IY, 0)],
        'c' => vec![(S, 1), (IY, 0)],
        'd' => vec![(D, 1), (IY, 0)],
        'e' => vec![(IY, 1)],
        'f' => vec![(EH, 1), (F, 0)],
        'g' => vec![(JH, 1), (IY, 0)],
        'h' => vec![(EY, 1), (CH, 0)],
        'i' => vec![(AI, 1)],
        'j' => vec![(JH, 1), (EY, 0)],
        'k' => vec![(K, 1), (EY, 0)],
        'l' => vec![(EH, 1), (L, 0)],
        'm' => vec![(EH, 1), (M, 0)],
        'n' => vec![(EH, 1), (N, 0)],
        'o' => vec![(OW, 1)],
        'p' => vec![(P, 1), (IY, 0)],
        'q' => vec![(K, 1), (Y, 0), (UW, 0)],
        'r' => vec![(AA, 1), (R, 0)],
        's' => vec![(EH, 1), (S, 0)],
        't' => vec![(T, 1), (IY, 0)],
        'u' => vec![(Y, 1), (UW, 0)],
        'v' => vec![(V, 1), (IY, 0)],
        'w' => vec![(D, 1), (AH, 0), (B, 0), (AX, 0), (L, 0), (Y, 0), (UW, 0)],
        'x' => vec![(EH, 1), (K, 0), (S, 0)],
        'y' => vec![(W, 1), (AI, 0)],
        'z' => vec![(Z, 1), (IY, 0)],
        _ => vec![(AX, 0)],
    }
}

const CONTRACTION: [(&str, &str); 18] = [
    ("don't", "do nt"),
    ("can't", "can nt"),
    ("won't", "will nt"),
    ("isn't", "is nt"),
    ("aren't", "are nt"),
    ("wasn't", "was nt"),
    ("weren't", "were nt"),
    ("doesn't", "does nt"),
    ("didn't", "did nt"),
    ("couldn't", "could nt"),
    ("wouldn't", "would nt"),
    ("shouldn't", "should nt"),
    ("haven't", "have nt"),
    ("hasn't", "has nt"),
    ("hadn't", "had nt"),
    ("i'm", "i m"),
    ("i'll", "i ll"),
    ("i'd", "i d"),
];

fn map_symbol(sym: &str) -> PhonemeKind {
    match sym {
        "IY" => IY,
        "IH" => IH,
        "EH" => EH,
        "EY" => EY,
        "AE" => AE,
        "AA" => AA,
        "AH" => AH,
        "AO" => AO,
        "UH" => UH,
        "UW" => UW,
        "UX" => UX,
        "AX" => AX,
        "ER" => ER,
        "EI" => EI,
        "AI" | "AY" => AI,
        "OI" | "OY" => OI,
        "OW" => OW,
        "AU" | "AW" => AU,
        "IA" => IA,
        "EA" => EA,
        "UA" => UA,
        "P" => P,
        "B" => B,
        "T" => T,
        "D" => D,
        "K" => K,
        "G" => G,
        "F" => F,
        "V" => V,
        "TH" => TH,
        "DH" => DH,
        "S" => S,
        "Z" => Z,
        "SH" => SH,
        "ZH" => ZH,
        "CH" => CH,
        "JH" => JH,
        "H" | "HH" => H,
        "M" => M,
        "N" => N,
        "NG" => NG,
        "NY" => NY,
        "L" => L,
        "R" => R,
        "RR" => RR,
        "W" => W,
        "Y" => Y,
        _ => AX,
    }
}

pub fn symbol(sym: &str) -> Option<PhonemeKind> {
    let known = [
        "IY", "IH", "EH", "EY", "AE", "AA", "AH", "AO", "UH", "UW", "UX", "AX", "ER", "EI", "AI",
        "OI", "OW", "AU", "IA", "EA", "UA", "P", "B", "T", "D", "K", "G", "F", "V", "TH", "DH",
        "S", "Z", "SH", "ZH", "CH", "JH", "H", "HH", "M", "N", "NG", "NY", "L", "R", "RR", "W",
        "Y", "AY", "AW", "OY",
    ];
    if known.contains(&sym) {
        Some(map_symbol(sym))
    } else {
        None
    }
}

fn parse_dict_entry(phones: &str) -> Vec<(PhonemeKind, u8)> {
    phones
        .split_whitespace()
        .map(|tok| {
            let (sym, stress) = if let Some(idx) = tok.find(|c: char| c.is_ascii_digit()) {
                let stress = tok[idx..].parse::<u8>().unwrap_or(0);
                (&tok[..idx], stress)
            } else {
                (tok, 0)
            };
            (map_symbol(sym), stress)
        })
        .collect()
}

fn is_vowel_letter(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

fn is_vowel_sound(k: PhonemeKind) -> bool {
    matches!(
        k,
        IY | IH
            | EH
            | EY
            | AE
            | AA
            | AH
            | AO
            | UH
            | UW
            | UX
            | AX
            | ER
            | EI
            | AI
            | OI
            | OW
            | AU
            | IA
            | EA
            | UA
    )
}

pub fn phonemize_word(word: &str, next_first: Option<char>) -> Vec<(PhonemeKind, u8)> {
    let lower = word
        .trim_end_matches(['.', ',', '!', '?', ';', ':', '"', ')', ']', '»'])
        .to_lowercase();
    if lower.len() == 1 {
        return spell_letter(lower.chars().next().unwrap_or(' '));
    }
    if let Some(entry) = lookup(&lower) {
        let parsed = parse_dict_entry(entry);
        if lower == "the" && next_first.is_some_and(|c| is_vowel_letter(c.to_ascii_lowercase())) {
            return vec![(DH, 0), (IY, 1)];
        }
        return parsed;
    }
    if let Some((_, expanded)) = CONTRACTION.iter().find(|(c, _)| *c == lower) {
        let mut out = Vec::new();
        for part in expanded.split_whitespace() {
            out.extend(phonemize_word(part, next_first));
        }
        return out;
    }
    if lower.contains('\'') {
        let parts: Vec<&str> = lower.split('\'').collect();
        let mut out = Vec::new();
        let mut skip_next = false;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if skip_next {
                skip_next = false;
                continue;
            }
            out.extend(phonemize_word(part, None));
            if i + 1 < parts.len() {
                match parts[i + 1] {
                    "s" => {
                        let unvoiced = out
                            .last()
                            .map(|(k, _)| matches!(k, &P | &T | &K | &F | &TH | &S | &SH | &CH))
                            .unwrap_or(false);
                        out.push((if unvoiced { S } else { Z }, 0));
                        skip_next = true;
                    }
                    "re" => {
                        out.extend(parse_dict_entry("R ER0"));
                        skip_next = true;
                    }
                    "ve" => {
                        out.push((V, 0));
                        skip_next = true;
                    }
                    "ll" => {
                        out.push((L, 0));
                        skip_next = true;
                    }
                    "m" => {
                        out.push((M, 0));
                        skip_next = true;
                    }
                    "d" => {
                        out.push((D, 0));
                        skip_next = true;
                    }
                    _ => {}
                }
            }
        }
        return out;
    }
    if lower.contains('-') {
        let parts: Vec<&str> = lower.split('-').collect();
        let mut out = Vec::new();
        for part in parts {
            out.extend(phonemize_word(part, None));
        }
        return out;
    }
    rules(&lower)
}

pub fn phonemize_word_context(
    word: &str,
    previous: Option<&str>,
    next_first: Option<char>,
    past_context: bool,
) -> Vec<(PhonemeKind, u8)> {
    let lower = word
        .trim_end_matches(['.', ',', '!', '?', ';', ':', '"', ')', ']', '»'])
        .to_lowercase();
    if lower == "read" && past_context {
        return vec![(R, 0), (EH, 1), (D, 0)];
    }
    if lower == "record" {
        let previous = previous.unwrap_or_default().to_ascii_lowercase();
        if [
            "to", "will", "can", "could", "should", "would", "do", "does", "did", "must",
        ]
        .contains(&previous.as_str())
        {
            return vec![(R, 0), (IH, 0), (K, 0), (AO, 1), (R, 0), (D, 0)];
        }
    }
    phonemize_word(word, next_first)
}

fn rules(word: &str) -> Vec<(PhonemeKind, u8)> {
    let chars: Vec<char> = word.chars().collect();
    let n = chars.len();
    let mut out: Vec<(PhonemeKind, u8)> = Vec::new();
    let mut i = 0usize;

    let at = |i: usize| chars.get(i).copied().unwrap_or(' ');
    let ends_with = |s: &str| {
        let r: String = chars.iter().collect();
        r.ends_with(s)
    };

    if n >= 2 && matches!(at(0), 'k' | 'g' | 'w' | 'p') && matches!(at(1), 'n' | 's') {
        i = 1;
        if at(1) == 's' {
            out.push((S, 0));
        }
    }

    while i < n {
        let c = at(i);
        let next = at(i + 1);
        let next2 = at(i + 2);
        let prev = if i > 0 { at(i - 1) } else { ' ' };

        if is_vowel_letter(c) || (c == 'y' && i > 0) {
            match read_vowel(&chars, i, &out) {
                Some((kind, consumed)) => {
                    out.push((kind, 0));
                    i += consumed;
                }
                None => {
                    i += 1;
                }
            }
            continue;
        }

        match c {
            'b' => {
                if i + 1 < n && at(i + 1) == 'b' {
                    out.push((B, 0));
                    i += 2;
                } else {
                    out.push((B, 0));
                    i += 1;
                }
            }
            'c' => {
                if next == 'h' {
                    let word_str: String = chars.iter().collect();
                    if i > 0 && prev == 't' {
                        out.push((CH, 0));
                    } else if matches!(
                        word_str.as_str(),
                        "school"
                            | "chemistry"
                            | "character"
                            | "ache"
                            | "chorus"
                            | "echo"
                            | "architect"
                            | "technical"
                    ) {
                        out.push((K, 0));
                    } else {
                        out.push((CH, 0));
                    }
                    i += 2;
                } else if next == 'k' {
                    out.push((K, 0));
                    i += 2;
                } else if next == 'e' || next == 'i' || next == 'y' {
                    out.push((S, 0));
                    i += 1;
                } else {
                    out.push((K, 0));
                    i += 1;
                }
            }
            'd' => {
                if next == 'g' && (at(i + 2) == 'e' || at(i + 2) == 'i' || at(i + 2) == 'y') {
                    out.push((JH, 0));
                    i += 2;
                } else if next == 'd' {
                    out.push((D, 0));
                    i += 2;
                } else if next == 'j' {
                    out.push((JH, 0));
                    i += 2;
                } else {
                    out.push((D, 0));
                    i += 1;
                }
            }
            'f' => {
                if next == 'f' {
                    out.push((F, 0));
                    i += 2;
                } else {
                    out.push((F, 0));
                    i += 1;
                }
            }
            'g' => {
                if next == 'n' && i == 0 {
                    out.push((N, 0));
                    i += 2;
                } else if next == 'h' {
                    let after: String = chars[i + 2..].iter().collect();
                    if after.is_empty() {
                        i += 2;
                    } else if after.starts_with('t') || matches!(next2, 't') {
                        out.push((F, 0));
                        i += 2;
                    } else if matches!(next2, 'e' | 'i' | 'y') && !after.starts_with('t') {
                        i += 2;
                    } else {
                        out.push((G, 0));
                        i += 2;
                    }
                } else if (next == 'e' || next == 'i' || next == 'y')
                    && !matches!(
                        word,
                        "get"
                            | "give"
                            | "girl"
                            | "gift"
                            | "gig"
                            | "begin"
                            | "forget"
                            | "target"
                            | "together"
                            | "geese"
                            | "gear"
                    )
                {
                    out.push((JH, 0));
                    i += 1;
                } else if next == 'g' {
                    out.push((G, 0));
                    i += 2;
                } else {
                    out.push((G, 0));
                    i += 1;
                }
            }
            'h' => {
                if next == 'h' {
                    i += 2;
                } else {
                    out.push((H, 0));
                    i += 1;
                }
            }
            'j' => {
                out.push((JH, 0));
                i += 1;
            }
            'k' => {
                if i == 0 && next == 'n' {
                    i += 2;
                } else if next == 'h' || next == 'k' {
                    out.push((K, 0));
                    i += 2;
                } else {
                    out.push((K, 0));
                    i += 1;
                }
            }
            'l' => {
                if next == 'l' {
                    i += 2;
                } else {
                    i += 1;
                }
                out.push((L, 0));
            }
            'm' => {
                if next == 'b' && i + 2 == n {
                    i += 2;
                } else {
                    i += if next == 'm' { 2 } else { 1 };
                }
                out.push((M, 0));
            }
            'n' => {
                if next == 'g' && i + 2 == n {
                    out.push((NG, 0));
                    i += 2;
                } else if next == 'g' {
                    let after: String = chars[i + 2..].iter().collect();
                    if after.starts_with(['e', 'i']) {
                        out.push((N, 0));
                        out.push((JH, 0));
                        i += 2;
                    } else {
                        out.push((NG, 0));
                        i += 2;
                    }
                } else if next == 'k' || next == 'c' {
                    // n before c is /ng/ only before hard c (a, o, u, h);
                    // before soft c (e, i, y) it stays /n/ (sentence, silence).
                    let after_c = chars.get(i + 2).copied().unwrap_or(' ');
                    if next == 'c' && matches!(after_c, 'e' | 'i' | 'y') {
                        out.push((N, 0));
                    } else {
                        out.push((NG, 0));
                    }
                    i += 1;
                } else if next == 'n' {
                    out.push((N, 0));
                    i += 2;
                } else {
                    out.push((N, 0));
                    i += 1;
                }
            }
            'p' => {
                if i == 0 && next == 'n' {
                    i += 2;
                } else if next == 'h' {
                    out.push((F, 0));
                    i += 2;
                } else if next == 'p' {
                    out.push((P, 0));
                    i += 2;
                } else {
                    out.push((P, 0));
                    i += 1;
                }
            }
            'q' => {
                if next == 'u' {
                    out.push((K, 0));
                    out.push((W, 0));
                    i += 2;
                } else {
                    out.push((K, 0));
                    i += 1;
                }
            }
            'r' => {
                if next == 'r' {
                    out.push((R, 0));
                    i += 2;
                } else {
                    out.push((R, 0));
                    i += 1;
                }
            }
            's' => {
                if next == 'h' {
                    out.push((SH, 0));
                    i += 2;
                } else if next == 's' && at(i + 2) == 'i' && at(i + 3) == 'o' {
                    out.push((SH, 0));
                    out.push((AX, 0));
                    out.push((N, 0));
                    i += 5;
                } else if next == 's' {
                    out.push((S, 0));
                    i += 2;
                } else if next == 'i' && (next2 == 'o' || next2 == 'a') {
                    // -sion after a vowel is voiced (vision), after a
                    // consonant voiceless (tension); consume the trailing n.
                    let voiced = is_vowel_sound(out.last().map(|(k, _)| *k).unwrap_or(AX));
                    out.push((if voiced { ZH } else { SH }, 0));
                    out.push((AX, 0));
                    out.push((N, 0));
                    i += 4;
                } else if i > 0
                    && i + 1 < n
                    && (is_vowel_letter(next) || next == 'y')
                    && is_vowel_sound(out.last().map(|(k, _)| *k).unwrap_or(AX))
                {
                    out.push((Z, 0));
                    i += 1;
                } else {
                    out.push((S, 0));
                    i += 1;
                }
            }
            't' => {
                if next == 'h' {
                    let word_str: String = chars.iter().collect();
                    if matches!(
                        word_str.as_str(),
                        "the"
                            | "this"
                            | "that"
                            | "these"
                            | "those"
                            | "they"
                            | "them"
                            | "there"
                            | "their"
                            | "then"
                            | "than"
                            | "with"
                            | "without"
                            | "though"
                            | "through"
                            | "thought"
                            | "three"
                            | "thank"
                            | "thing"
                            | "think"
                            | "both"
                            | "month"
                            | "nothing"
                            | "something"
                            | "anything"
                            | "everything"
                            | "other"
                            | "another"
                            | "mother"
                            | "father"
                            | "brother"
                    ) {
                        out.push((DH, 0));
                    } else {
                        out.push((TH, 0));
                    }
                    i += 2;
                } else if next == 'i' && next2 == 'o' {
                    out.push((SH, 0));
                    out.push((AX, 0));
                    out.push((N, 0));
                    i += 4;
                } else if next == 'i' && (next2 == 'a' || next2 == 'e') {
                    if word.starts_with("question") {
                        out.push((CH, 0));
                        i += 1;
                    } else {
                        out.push((SH, 0));
                        i += 1;
                    }
                } else if next == 'u' && next2 == 'r' && at(i + 3) == 'e' && i + 4 == n {
                    out.push((CH, 0));
                    out.push((ER, 0));
                    i += 4;
                } else if next == 't' {
                    out.push((T, 0));
                    i += 2;
                } else if next == 'c' && next2 == 'h' {
                    out.push((CH, 0));
                    i += 3;
                } else {
                    out.push((T, 0));
                    i += 1;
                }
            }
            'v' => {
                out.push((V, 0));
                i += 1;
            }
            'w' => {
                if next == 'h' {
                    let after: String = chars[i + 2..].iter().collect();
                    if after.starts_with('o') {
                        out.push((H, 0));
                    } else {
                        out.push((W, 0));
                    }
                    i += 2;
                } else if i == 0 && next == 'r' {
                    // word-initial wr: silent w, pronounced r (write, wrong)
                    out.push((R, 0));
                    i += 2;
                } else {
                    out.push((W, 0));
                    i += 1;
                }
            }
            'x' => {
                if i == 0 {
                    out.push((Z, 0));
                } else {
                    out.push((K, 0));
                    out.push((S, 0));
                }
                i += 1;
            }
            'y' => {
                if i == 0 {
                    out.push((Y, 0));
                    i += 1;
                } else {
                    match read_vowel(&chars, i, &out) {
                        Some((kind, consumed)) => {
                            out.push((kind, 0));
                            i += consumed;
                        }
                        None => {
                            i += 1;
                        }
                    }
                }
            }
            'z' => {
                if next == 'z' {
                    out.push((Z, 0));
                    i += 2;
                } else {
                    out.push((Z, 0));
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    if ends_with("ed") && n > 2 {
        let before = at(n - 3);
        let before2 = if n > 3 { at(n - 4) } else { ' ' };
        let is_syllabic = matches!(before, 't' | 'd');
        if is_syllabic {
            if out.last().is_some_and(|(k, _)| is_vowel_sound(*k)) {
                out.push((IH, 0));
                out.push((D, 0));
            } else {
                out.push((AX, 0));
                out.push((D, 0));
            }
        } else if matches!(before, 'p' | 'k' | 'f' | 's' | 'h' | 'x' | 'c') {
            out.push((T, 0));
        } else if before2 != ' ' && (before == 'e') {
            out.push((D, 0));
        }
    }

    assign_stress(&mut out, word);
    out
}

fn read_vowel(chars: &[char], i: usize, out: &[(PhonemeKind, u8)]) -> Option<(PhonemeKind, usize)> {
    let n = chars.len();
    let at = |j: usize| chars.get(j).copied().unwrap_or(' ');

    let mut j = i;
    let mut group = String::new();
    while j < n && (is_vowel_letter(at(j)) || at(j) == 'y') {
        group.push(at(j));
        j += 1;
    }

    let prev = if i > 0 { at(i - 1) } else { ' ' };

    if group == "e" && j == n && i + 1 == n && out.iter().any(|(k, _)| is_vowel_sound(*k)) {
        return None;
    }

    if group.len() >= 2 {
        let g = group.as_str();
        if at(j) == 'r' {
            let kind = match g {
                "ai" | "ei" | "ea" => EA,
                "ee" | "ie" => IA,
                "ou" | "au" => AU,
                "oo" | "oa" => AO,
                "ue" => UA,
                "oi" | "oy" => OI,
                "aw" | "ew" => AO,
                _ => match g.chars().next().unwrap_or('a') {
                    'a' => EA,
                    'e' => IA,
                    'o' => AO,
                    _ => ER,
                },
            };
            return Some((kind, j - i));
        }
        let kind = match g {
            "ai" | "ay" => EI,
            "au" | "aw" => AO,
            "ea" => IY,
            "ee" => IY,
            "ei" => EY,
            "ey" => EY,
            "ie" => IY,
            "oa" => OW,
            "oe" => OW,
            "oo" => UW,
            "ou" => AU,
            "ow" => OW,
            "ue" => UW,
            "ui" => UW,
            "oi" | "oy" => OI,
            "eu" | "ew" => UW,
            "eigh" => EY,
            "igh" => AI,
            _ => {
                let first = g.chars().next().unwrap_or('a');
                let second = g.chars().nth(1).unwrap_or(' ');
                if first == second {
                    let kind = match first {
                        'a' => AE,
                        'e' => EH,
                        'i' => IH,
                        'o' => AA,
                        'u' => UX,
                        _ => AX,
                    };
                    return Some((kind, 2));
                }
                let kind = match first {
                    'a' => AE,
                    'e' => EH,
                    'i' => IH,
                    'o' => AA,
                    'u' => UX,
                    _ => AX,
                };
                return Some((kind, 1));
            }
        };
        return Some((kind, j - i));
    }

    let first = group.chars().next().unwrap_or('a');
    let after_r = at(j) == 'r';

    if first == 'y' {
        if j == n {
            if out.iter().any(|(k, _)| is_vowel_sound(*k)) {
                return Some((IY, 1));
            }
            return Some((AI, 1));
        }
        return Some((IH, 1));
    }

    if after_r {
        let kind = match first {
            'a' => {
                if prev == 'w' {
                    AO
                } else {
                    AA
                }
            }
            'e' | 'i' => ER,
            'o' => {
                if prev == 'w' {
                    ER
                } else {
                    AO
                }
            }
            'u' => ER,
            _ => AX,
        };
        let consume_r = matches!(first, 'e' | 'i' | 'u') || (first == 'o' && prev == 'w');
        return Some((kind, if consume_r { j - i + 1 } else { j - i }));
    }

    let single_cons = |idx: usize| idx + 1 < n && at(idx) == at(idx + 1);
    let magic_e = j + 2 == n
        && at(j + 1) == 'e'
        && !is_vowel_letter(at(j))
        && at(j) != 'r'
        && at(j) != 'w'
        && !single_cons(j);
    let le_suffix = j + 3 == n
        && at(j + 1) == 'l'
        && at(j + 2) == 'e'
        && !is_vowel_letter(at(j))
        && at(j) != 'r'
        && at(j) != 'w'
        && !single_cons(j);

    if magic_e || le_suffix {
        let kind = match first {
            'a' => EI,
            'e' => IY,
            'i' => AI,
            'o' => OW,
            'u' => UW,
            _ => AX,
        };
        return Some((kind, 1));
    }

    if first == 'i' && at(j) == 'g' && at(j + 1) == 'h' {
        // igh reads as /ai/ with a silent gh (night, light, right)
        return Some((AI, 3));
    }

    let kind = match first {
        'a' => AE,
        'e' => EH,
        'i' => IH,
        'o' => AA,
        'u' => UX,
        _ => AX,
    };
    Some((kind, 1))
}

fn assign_stress(out: &mut [(PhonemeKind, u8)], word: &str) {
    let vowel_idx: Vec<usize> = out
        .iter()
        .enumerate()
        .filter(|(_, (k, _))| is_vowel_sound(*k))
        .map(|(idx, _)| idx)
        .collect();
    if vowel_idx.is_empty() {
        return;
    }
    let lower = word.to_lowercase();
    let primary = if lower.ends_with("tion")
        || lower.ends_with("sion")
        || lower.ends_with("ic")
        || lower.ends_with("ical")
        || lower.ends_with("ity")
    {
        if vowel_idx.len() >= 2 {
            vowel_idx.len() - 2
        } else {
            0
        }
    } else if lower.ends_with("ate") && !lower.ends_with("ated") && vowel_idx.len() >= 2 {
        vowel_idx.len() - 1
    } else {
        0
    };
    out[vowel_idx[primary]].1 = 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(word: &str) -> Vec<PhonemeKind> {
        phonemize_word(word, None)
            .into_iter()
            .map(|(k, _)| k)
            .collect()
    }

    fn stressed(word: &str) -> Vec<(PhonemeKind, u8)> {
        phonemize_word(word, None)
    }

    #[test]
    fn corpus_words() {
        assert_eq!(kinds("voice"), vec![V, OI, S]);
        assert_eq!(kinds("master"), vec![M, AE, S, T, ER]);
        assert_eq!(kinds("temperature"), vec![T, EH, M, P, ER, AH, CH, ER]);
        assert_eq!(kinds("degrees"), vec![D, IH, G, R, IY, Z]);
        assert_eq!(kinds("synthesis"), vec![S, IH, N, TH, AX, S, IH, S]);
    }

    #[test]
    fn rules_regular() {
        assert_eq!(kinds("hello"), vec![H, AH, L, OW]);
        assert_eq!(kinds("make"), vec![M, EI, K]);
        assert_eq!(kinds("time"), vec![T, AI, M]);
        assert_eq!(kinds("note"), vec![N, OW, T]);
        assert_eq!(kinds("cat"), vec![K, AE, T]);
        assert_eq!(kinds("ship"), vec![SH, IH, P]);
        assert_eq!(kinds("thin"), vec![TH, IH, N]);
        assert_eq!(kinds("phone"), vec![F, OW, N]);
        assert_eq!(kinds("king"), vec![K, IH, NG]);
        assert_eq!(kinds("she"), vec![SH, IY]);
        assert_eq!(kinds("go"), vec![G, OW]);
    }

    #[test]
    fn stress_marks() {
        let s = stressed("synthesis");
        assert_eq!(s[1], (IH, 1));
        let t = stressed("temperature");
        assert_eq!(t[1], (EH, 1));
    }

    #[test]
    fn contractions() {
        assert_eq!(kinds("don't"), vec![D, OW, N, T]);
        assert_eq!(kinds("it's"), vec![IH, T, S]);
    }

    #[test]
    fn letter_spelling() {
        assert_eq!(kinds("g"), vec![JH, IY]);
        assert_eq!(kinds("q"), vec![K, Y, UW]);
    }

    #[test]
    fn interjections() {
        assert_eq!(kinds("ah"), vec![AA]);
        assert_eq!(kinds("wow"), vec![W, AU]);
        assert_eq!(kinds("oops"), vec![UW, P, S]);
        assert_eq!(kinds("ouch"), vec![AU, CH]);
        assert_eq!(kinds("yay"), vec![Y, EY]);
        assert_eq!(kinds("whoa"), vec![W, OW]);
        assert_eq!(kinds("uh"), vec![AH]);
        assert_eq!(kinds("um"), vec![AH, M]);
        assert_eq!(kinds("uh-huh"), vec![AH, H, AH]);
        assert_eq!(kinds("phew"), vec![F, Y, UW]);
        assert_eq!(kinds("ahem"), vec![AH, H, EH, M]);
        assert_eq!(kinds("oof"), vec![UW, F]);
        assert_eq!(kinds("wow!"), vec![W, AU]);
    }

    #[test]
    fn common_words() {
        assert_eq!(kinds("the"), vec![DH, AH]);
        assert_eq!(kinds("of"), vec![AH, V]);
        assert_eq!(kinds("you"), vec![Y, UW]);
        assert_eq!(kinds("shall"), vec![SH, AE, L]);
        assert_eq!(kinds("might"), vec![M, AI, T]);
        assert_eq!(kinds("must"), vec![M, AH, S, T]);
        assert_eq!(kinds("under"), vec![AH, N, D, ER]);
        assert_eq!(kinds("many"), vec![M, EH, N, IY]);
        assert_eq!(kinds("even"), vec![IY, V, AX, N]);
        assert_eq!(kinds("still"), vec![S, T, IH, L]);
        assert_eq!(kinds("ok"), vec![OW, K, EY]);
        assert_eq!(kinds("please"), vec![P, L, IY, Z]);
        assert_eq!(kinds("thanks"), vec![TH, AE, NG, K, S]);
        assert_eq!(kinds("sorry"), vec![S, AA, R, IY]);
        assert_eq!(kinds("hi"), vec![H, AI]);
        assert_eq!(kinds("great"), vec![G, R, EY, T]);
        assert_eq!(kinds("nice"), vec![N, AI, S]);
        assert_eq!(kinds("wrong"), vec![R, AO, NG]);
        assert_eq!(kinds("up"), vec![AH, P]);
    }
}
