use crate::phoneme::PhonemeKind::{self, *};

const DICT_FR: &[(&str, &[PhonemeKind])] = &[
    ("aujourd'hui", &[OW, ZH, UW, RR, D, UE, IY]),
    ("monsieur", &[M, AX, S, Y, OE]),
    ("beaucoup", &[B, OW, K, UW]),
    ("madame", &[M, AA, D, AA, M]),
    ("mademoiselle", &[M, AA, D, M, W, AA, Z, EH, L]),
    ("messieurs", &[M, EH, S, Y, OE]),
    ("gens", &[ZH, AN]),
    ("fils", &[F, IY, S]),
    ("six", &[S, IY, S]),
    ("dix", &[D, IY, S]),
    ("huit", &[Y, IY, T]),
    ("coûte", &[K, UW, T]),
    ("coût", &[K, UW]),
    ("net", &[N, EH, T]),
    ("ouest", &[W, EH, S, T]),
    ("but", &[B, UE, T]),
    ("direct", &[D, IY, RR, EH, K, T]),
    ("exact", &[EH, G, Z, AA, K, T]),
    ("contact", &[K, ON, T, AA, K, T]),
    ("concept", &[K, ON, S, EH, P, T]),
    ("wagon", &[V, AA, G, ON]),
    ("watt", &[V, AA, T]),
    ("parfum", &[P, AA, RR, F, UN]),
    ("album", &[AA, L, B, AO, M]),
    ("moment", &[M, AO, M, AN]),
    ("comment", &[K, AO, M, AN]),
    ("souvent", &[S, UW, V, AN]),
    ("vent", &[V, AN]),
    ("dent", &[D, AN]),
    ("nord", &[N, AO, RR, D]),
    ("sud", &[S, UE, D]),
    ("temps", &[T, AN]),
    ("mars", &[M, AA, RR]),
    ("ours", &[UW, RR, S]),
    ("sens", &[S, AN, S]),
    ("bus", &[B, UE, S]),
    ("virus", &[V, IY, RR, UE, S]),
    ("gentil", &[ZH, AN, T, IY]),
    ("plomb", &[P, L, ON]),
    ("cap", &[K, AA, P]),
    ("stop", &[S, T, AO, P]),
    ("donc", &[D, ON]),
    ("master", &[M, AA, S, T, EH, RR]),
    ("rust", &[RR, UE, S, T]),
    ("nvidia", &[N, V, IY, D, Y, AA]),
    ("mer", &[M, EH, RR]),
    ("cher", &[S, EH, RR]),
    ("fer", &[F, EH, RR]),
    ("ver", &[V, EH, RR]),
    ("hier", &[Y, EH, RR]),
    ("enfer", &[AN, F, EH, RR]),
    ("hiver", &[IY, V, EH, RR]),
    ("et", &[EY]),
    ("est", &[EH]),
    ("c'est", &[S, EH]),
    ("question", &[K, EH, S, T, Y, ON]),
    ("sont", &[S, ON]),
    ("font", &[F, ON]),
    ("les", &[L, EY]),
    ("des", &[D, EY]),
    ("mes", &[M, EY]),
    ("tes", &[T, EY]),
    ("ses", &[S, EY]),
    ("ces", &[S, EY]),
    ("le", &[L, AX]),
    ("de", &[D, AX]),
    ("je", &[ZH, AX]),
    ("me", &[M, AX]),
    ("te", &[T, AX]),
    ("se", &[S, AX]),
    ("ce", &[S, AX]),
    ("que", &[K, AX]),
    ("ne", &[N, AX]),
    ("fille", &[F, IY, Y]),
    ("bille", &[B, IY, Y]),
    ("grille", &[G, RR, IY, Y]),
    ("ville", &[V, IY, L]),
    ("mille", &[M, IY, L]),
    ("chose", &[SH, OW, Z]),
    ("rose", &[RR, OW, Z]),
    ("pose", &[P, OW, Z]),
    ("examen", &[EH, G, Z, AA, M, EN]),
    ("ennemi", &[EH, N, M, IY]),
    ("femme", &[F, AA, M]),
    ("énergie", &[EY, N, EH, RR, ZH, IY]),
    ("ah", &[AA]),
    ("oh", &[OW]),
    ("eh", &[EH]),
    ("hé", &[EY]),
    ("ha", &[AA]),
    ("hi", &[IY]),
    ("hop", &[AO, P]),
    ("ouf", &[UW, F]),
    ("zut", &[Z, UE, T]),
    ("aïe", &[AA, Y]),
    ("bravo", &[B, RR, AA, V, OW]),
    ("ouah", &[W, AA]),
    ("wouah", &[W, AA]),
    ("euh", &[OE]),
    ("beurk", &[B, OEU, RR, K]),
    ("bof", &[B, AO, F]),
    ("pouah", &[P, W, AA]),
    ("ouais", &[W, EH]),
    ("ouai", &[W, EH]),
    ("oups", &[UW, P, S]),
    ("hein", &[EN]),
    ("ahem", &[AA, EH, M]),
];

pub(crate) fn lookup(word: &str) -> Option<Vec<PhonemeKind>> {
    let lower = word.to_lowercase();
    DICT_FR
        .iter()
        .find(|(w, _)| *w == lower)
        .map(|(_, phones)| phones.to_vec())
}

pub(crate) fn has_dictionary_word(word: &str) -> bool {
    lookup(word).is_some()
}

fn with_final_nucleus(phones: Vec<PhonemeKind>) -> Vec<(PhonemeKind, u8)> {
    let last_vowel = phones.iter().rposition(|kind| is_vowel_sound(*kind));
    phones
        .into_iter()
        .enumerate()
        .map(|(index, kind)| (kind, u8::from(Some(index) == last_vowel)))
        .collect()
}

fn spell_letter_fr(c: char) -> Vec<PhonemeKind> {
    match c.to_ascii_lowercase() {
        '0' => vec![Z, EY, RR, OW],
        '1' => vec![UN],
        '2' => vec![D, OE],
        '3' => vec![T, RR, AA],
        '4' => vec![K, AA, T, RR],
        '5' => vec![S, EN, K],
        '6' => vec![S, IY, S],
        '7' => vec![S, EH, T],
        '8' => vec![Y, IY, T],
        '9' => vec![N, OE, F],
        'a' => vec![AA],
        'b' => vec![B, EY],
        'c' => vec![S, EY],
        'd' => vec![D, EY],
        'e' => vec![AX],
        'f' => vec![EH, F],
        'g' => vec![ZH, EY],
        'h' => vec![AA, SH],
        'i' => vec![IY],
        'j' => vec![ZH, IY],
        'k' => vec![K, AA],
        'l' => vec![EH, L],
        'm' => vec![EH, M],
        'n' => vec![EH, N],
        'o' => vec![OW],
        'p' => vec![P, EY],
        'q' => vec![K, UE],
        'r' => vec![EH, RR],
        's' => vec![EH, S],
        't' => vec![T, EY],
        'u' => vec![UE],
        'v' => vec![V, EY],
        'w' => vec![D, UW, B, L, AX, V, EY],
        'x' => vec![IY, K, S],
        'y' => vec![IY, G, RR, EH, K],
        'z' => vec![Z, EH, D],
        _ => vec![AX],
    }
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
            | UE
            | OE
            | OEU
            | EN
            | AN
            | ON
            | UN
    )
}

pub fn symbol(sym: &str) -> Option<PhonemeKind> {
    use crate::g2p::en;
    let known = ["UE", "OE", "OEU", "EN", "AN", "ON", "UN", "NY", "RR"];
    if known.contains(&sym) {
        return Some(match sym {
            "UE" => UE,
            "OE" => OE,
            "OEU" => OEU,
            "EN" => EN,
            "AN" => AN,
            "ON" => ON,
            "UN" => UN,
            "NY" => NY,
            "RR" => RR,
            _ => unreachable!(),
        });
    }
    en::symbol(sym)
}

fn strip_punct(word: &str) -> &str {
    word.trim_end_matches(['.', ',', '!', '?', ';', ':', '"', ')', ']', '»'])
}

pub fn phonemize_word(word: &str) -> Vec<(PhonemeKind, u8)> {
    let lower = strip_punct(word).to_lowercase();
    if lower.len() == 1 {
        return spell_letter_fr(lower.chars().next().unwrap_or(' '))
            .into_iter()
            .map(|k| (k, 0))
            .collect();
    }
    if let Some(phones) = lookup(&lower) {
        return with_final_nucleus(phones);
    }
    if lower.contains('-') && !lower.starts_with("aujourd") {
        let parts: Vec<&str> = lower.split('-').collect();
        let mut out = Vec::new();
        for part in parts {
            out.extend(phonemize_word(part));
        }
        return out;
    }
    if lower.contains('\'') {
        let parts: Vec<&str> = lower.split('\'').collect();
        let mut out = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if i == 0 && part.len() <= 2 {
                let elision = match *part {
                    "l" => Some(L),
                    "d" => Some(D),
                    "j" => Some(ZH),
                    "c" => Some(S),
                    "n" => Some(N),
                    "s" => Some(S),
                    "m" => Some(M),
                    "t" => Some(T),
                    "qu" => Some(K),
                    "lorsqu" => {
                        out.extend([(L, 0), (AO, 0), (RR, 0), (S, 0), (K, 0)]);
                        continue;
                    }
                    "jusqu" => {
                        out.extend([(ZH, 0), (UE, 0), (S, 0), (K, 0)]);
                        continue;
                    }
                    "puisqu" => {
                        out.extend([(P, 0), (UE, 0), (IY, 0), (S, 0), (K, 0)]);
                        continue;
                    }
                    _ => None,
                };
                if let Some(k) = elision {
                    out.push((k, 0));
                    continue;
                }
            }
            out.extend(phonemize_word(part));
        }
        return out;
    }
    rules(&lower)
}

const NEGATIVE_KEYWORDS: &[&str] = &[
    "pas",
    "jamais",
    "plus",
    "rien",
    "personne",
    "point",
    "guère",
    "que",
    "nullement",
];

/// Weak-form "ne" pitch dip.
const NEG_NE_SHIFT: f32 = -0.06;
/// Pitch fall on the last vowel of the negative keyword.
const NEG_FALL: f32 = -0.15;
/// Per-syllable downward step between "ne" and the keyword.
const NEG_STEP: f32 = 0.05;
/// Maximum accumulated intermediate step.
const NEG_STEP_MAX: u32 = 5;

fn is_ne_word(word: &str) -> bool {
    word == "ne" || word.starts_with("n'")
}

fn is_negative_keyword(word: &str) -> bool {
    NEGATIVE_KEYWORDS.contains(&word)
}

const ASPIRED_H: &[&str] = &[
    "hache", "haine", "hall", "haricot", "hasard", "haut", "héros", "hibou", "hockey", "honte",
];

fn begins_vowel_or_mute_h(word: &str) -> bool {
    let lower = strip_punct(word).to_lowercase();
    let Some(first) = lower.chars().next() else {
        return false;
    };
    matches!(
        first,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'y'
            | 'à'
            | 'â'
            | 'ä'
            | 'é'
            | 'è'
            | 'ê'
            | 'ë'
            | 'î'
            | 'ï'
            | 'ô'
            | 'ö'
            | 'ù'
            | 'û'
            | 'ü'
            | 'œ'
    ) || first == 'h' && !ASPIRED_H.contains(&lower.as_str())
}

fn liaison_kind(word: &str) -> Option<PhonemeKind> {
    match word {
        "les" | "des" | "mes" | "tes" | "ses" | "nos" | "vos" => Some(Z),
        "un" => Some(N),
        _ if word.ends_with("'un") => Some(N),
        _ => None,
    }
}

fn electrical_context(word: &str) -> bool {
    [
        "électr", "electr", "câbl", "cabl", "cuivre", "conduct", "métall", "metall", "fil",
    ]
    .iter()
    .any(|prefix| word.starts_with(prefix))
}

/// Phonemize one clause with discontiguous negation ("ne ... pas") pitch
/// shaping: the weak "ne" dips, intermediate vowels step down per syllable,
/// and the negative keyword's last vowel falls.
pub fn phonemize_clause(words: &[&str]) -> Vec<Vec<(PhonemeKind, u8, f32)>> {
    let mut out = Vec::new();
    let mut in_negation = false;
    let mut step = 0u32;
    for (word_index, raw) in words.iter().enumerate() {
        let word = strip_punct(raw);
        let lower = word.to_lowercase();
        let next_lower = words
            .get(word_index + 1)
            .map(|next| strip_punct(next).to_lowercase());
        let is_ne = is_ne_word(&lower);
        let ends_negation = in_negation && is_negative_keyword(&lower);
        let mut phones = if lower == "fils" && next_lower.as_deref().is_some_and(electrical_context)
        {
            vec![(F, 0), (IY, 1), (L, 0)]
        } else {
            phonemize_word(word)
        };
        if next_lower.as_deref().is_some_and(begins_vowel_or_mute_h) {
            if let Some(kind) = liaison_kind(&lower) {
                phones.push((kind, 0));
            }
        }
        let last_vowel_idx = phones.iter().rposition(|(k, _)| is_vowel_sound(*k));
        let mut current = Vec::with_capacity(phones.len());
        for (i, (kind, stress)) in phones.into_iter().enumerate() {
            let mut shift = 0.0;
            if is_vowel_sound(kind) {
                if ends_negation && Some(i) == last_vowel_idx {
                    shift = NEG_FALL;
                } else if in_negation {
                    step += 1;
                    shift = -NEG_STEP * (step.min(NEG_STEP_MAX) as f32);
                } else if is_ne {
                    shift = NEG_NE_SHIFT;
                }
            }
            current.push((kind, stress, shift));
        }
        if !current.is_empty() {
            out.push(current);
        }
        if is_ne {
            in_negation = true;
            step = 0;
        }
        if ends_negation {
            in_negation = false;
        }
    }
    out
}

fn rules(word: &str) -> Vec<(PhonemeKind, u8)> {
    let chars: Vec<char> = word.chars().collect();
    let n = chars.len();
    let mut out: Vec<PhonemeKind> = Vec::new();
    let mut i = 0usize;

    let at = |j: usize| chars.get(j).copied().unwrap_or(' ');
    let at2 = |j: usize, off: usize| chars.get(j + off).copied().unwrap_or(' ');
    let is_vowel_letter = |c: char| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');

    while i < n {
        let c = at(i);
        let next = at(i + 1);
        let next2 = at(i + 2);
        let next3 = at(i + 3);
        let next4 = at(i + 4);
        let prev = if i > 0 { at(i - 1) } else { ' ' };
        let is_final = i + 1 == n;

        match c {
            'a' => {
                if next == 'i' {
                    if next2 == 'n' || next2 == 'm' {
                        let after = at2(i, 3);
                        if is_vowel_letter(after) || after == 'h' {
                            out.push(EH);
                            out.push(N);
                            i += 3;
                        } else {
                            out.push(EN);
                            i += 3;
                        }
                    } else if next2 == 'l' {
                        if next3 == 'l' {
                            out.push(AA);
                            out.push(Y);
                            i += 4;
                        } else {
                            out.push(AA);
                            out.push(Y);
                            i += 3;
                        }
                    } else {
                        out.push(EH);
                        i += 2;
                    }
                } else if next == 'u' {
                    out.push(OW);
                    i += 2;
                } else if next == 'n' || next == 'm' {
                    let after = at2(i, 2);
                    if after == 'n' || after == 'm' {
                        out.push(AA);
                        out.push(if after == 'm' { M } else { N });
                        i += 3;
                    } else if is_vowel_letter(after) || after == 'h' {
                        out.push(AA);
                        out.push(if next == 'm' { M } else { N });
                        i += 2;
                    } else {
                        out.push(AN);
                        i += 2;
                    }
                } else if next == 'y' {
                    out.push(EH);
                    out.push(Y);
                    i += 2;
                } else {
                    out.push(AA);
                    i += 1;
                }
            }
            'à' | 'â' => {
                out.push(AA);
                i += 1;
            }
            'e' => {
                if next == 'a' {
                    if next2 == 'u' {
                        out.push(OW);
                        i += 3;
                    } else {
                        out.push(OW);
                        i += 2;
                    }
                } else if next == 'u' {
                    if next2 == 'i' {
                        if next3 == 'l' {
                            if next4 == 'e' {
                                out.push(OEU);
                                out.push(Y);
                                i += 5;
                            } else {
                                out.push(OEU);
                                out.push(Y);
                                i += 4;
                            }
                        } else {
                            out.push(OE);
                            i += 3;
                        }
                    } else if next2 == 'r' {
                        out.push(OEU);
                        out.push(RR);
                        i += 3;
                    } else {
                        out.push(OE);
                        i += 2;
                    }
                } else if next == 'i' {
                    if next2 == 'n' || next2 == 'm' {
                        let after = at2(i, 3);
                        if is_vowel_letter(after) || after == 'h' {
                            out.push(EH);
                            out.push(N);
                            i += 3;
                        } else {
                            out.push(EN);
                            i += 3;
                        }
                    } else if next2 == 'l' && next3 == 'l' {
                        out.push(EH);
                        out.push(Y);
                        i += 4;
                    } else {
                        out.push(EH);
                        i += 2;
                    }
                } else if next == 'n' || next == 'm' {
                    let after = at2(i, 2);
                    if after == 'n' || after == 'm' {
                        out.push(EH);
                        out.push(if after == 'm' { M } else { N });
                        i += 3;
                    } else if is_vowel_letter(after) || after == 'h' {
                        out.push(AX);
                        out.push(if next == 'm' { M } else { N });
                        i += 2;
                    } else {
                        out.push(AN);
                        i += 2;
                    }
                } else if next == 'r' && (is_final || i + 2 == n) {
                    if n >= 5 {
                        out.push(EY);
                    } else {
                        out.push(EH);
                        out.push(RR);
                    }
                    i += 2;
                } else if next == 'z' && is_final {
                    out.push(EY);
                    i += 2;
                } else if next == 's' && i + 2 == n {
                    i += 2;
                } else if next == 'x' && i + 2 == n {
                    out.push(EY);
                    i += 2;
                } else if is_final {
                    i += 1;
                } else if next == next2 {
                    out.push(EH);
                    i += 1;
                } else if !is_vowel_letter(next) && is_vowel_letter(next2) {
                    out.push(AX);
                    i += 1;
                } else {
                    out.push(EH);
                    i += 1;
                }
            }
            'é' => {
                out.push(EY);
                i += 1;
            }
            'è' | 'ê' | 'ë' => {
                out.push(EH);
                i += 1;
            }
            'i' => {
                if next == 'e' && next2 == 'n' {
                    let after = at2(i, 3);
                    if is_vowel_letter(after) || after == 'h' {
                        out.push(IY);
                        out.push(N);
                        i += 3;
                    } else {
                        out.push(Y);
                        out.push(EN);
                        i += 3;
                    }
                } else if next == 'e' && next2 == 'l' && next3 == 'l' {
                    out.push(Y);
                    out.push(EH);
                    out.push(L);
                    i += 4;
                } else if next == 'e' && next2 == 'l' && i + 3 == n {
                    out.push(Y);
                    out.push(EH);
                    out.push(L);
                    i += 3;
                } else if next == 'e' && next2 == 'r' && i + 3 == n {
                    out.push(Y);
                    out.push(EY);
                    i += 3;
                } else if next == 'e' && matches!(next2, 't' | 'd' | 's' | 'z') && i + 3 == n {
                    out.push(Y);
                    out.push(EY);
                    i += 2;
                } else if next == 'e' && i + 2 == n {
                    out.push(IY);
                    i += 2;
                } else if next == 'n' || next == 'm' {
                    let after = at2(i, 2);
                    if after == 'n' || after == 'm' {
                        out.push(IY);
                        out.push(if after == 'm' { M } else { N });
                        i += 3;
                    } else if is_vowel_letter(after) || after == 'h' {
                        out.push(IY);
                        out.push(if next == 'm' { M } else { N });
                        i += 2;
                    } else {
                        out.push(EN);
                        i += 2;
                    }
                } else if next == 'l' && next2 == 'l' {
                    if is_vowel_letter(prev) {
                        out.push(Y);
                        i += 3;
                    } else {
                        out.push(IY);
                        out.push(L);
                        i += 3;
                    }
                } else {
                    out.push(IY);
                    i += 1;
                }
            }
            'î' | 'ï' => {
                out.push(IY);
                i += 1;
            }
            'o' => {
                if next == 'i' {
                    if next2 == 'n' {
                        let after = at2(i, 3);
                        if is_vowel_letter(after) || after == 'h' {
                            out.push(W);
                            out.push(AA);
                            out.push(N);
                            i += 3;
                        } else {
                            out.push(W);
                            out.push(EN);
                            i += 3;
                        }
                    } else {
                        out.push(W);
                        out.push(AA);
                        i += 2;
                    }
                } else if next == 'u' {
                    if next2 == 'i' && next3 == 'l' {
                        out.push(UW);
                        out.push(Y);
                        i += 4;
                    } else {
                        out.push(UW);
                        i += 2;
                    }
                } else if next == 'ù' {
                    out.push(UW);
                    i += 2;
                } else if next == 'n' || next == 'm' {
                    let after = at2(i, 2);
                    if after == 'n' || after == 'm' {
                        out.push(AO);
                        out.push(if after == 'm' { M } else { N });
                        i += 3;
                    } else if is_vowel_letter(after) || after == 'h' {
                        out.push(AO);
                        out.push(if next == 'm' { M } else { N });
                        i += 2;
                    } else {
                        out.push(ON);
                        i += 2;
                    }
                } else if next == 's' && i + 2 == n {
                    out.push(OW);
                    i += 2;
                } else if is_final || (i + 2 == n && !is_vowel_letter(next)) {
                    out.push(OW);
                    i += 1;
                } else {
                    out.push(AO);
                    i += 1;
                }
            }
            'ô' => {
                out.push(OW);
                i += 1;
            }
            'u' => {
                if next == 'n' || next == 'm' {
                    let after = at2(i, 2);
                    if after == 'n' || after == 'm' {
                        out.push(UE);
                        out.push(N);
                        i += 3;
                    } else if is_vowel_letter(after) || after == 'h' {
                        out.push(UE);
                        out.push(N);
                        i += 2;
                    } else {
                        out.push(UN);
                        i += 2;
                    }
                } else if next == 'i' {
                    if next2 == 'n' {
                        let after = at2(i, 3);
                        if is_vowel_letter(after) || after == 'h' {
                            out.push(Y);
                            out.push(IY);
                            out.push(N);
                            i += 3;
                        } else {
                            out.push(Y);
                            out.push(EN);
                            i += 3;
                        }
                    } else {
                        out.push(Y);
                        out.push(IY);
                        i += 2;
                    }
                } else {
                    out.push(UE);
                    i += 1;
                }
            }
            'ù' | 'û' | 'ü' => {
                out.push(UE);
                i += 1;
            }
            'y' => {
                if is_vowel_letter(prev) && is_vowel_letter(next) {
                    out.push(IY);
                    out.push(Y);
                } else {
                    out.push(IY);
                }
                i += 1;
            }
            'b' => {
                out.push(B);
                i += 1;
            }
            'c' => {
                if next == 'h' {
                    out.push(SH);
                    i += 2;
                } else if next == 'e'
                    || next == 'i'
                    || next == 'y'
                    || next == 'é'
                    || next == 'è'
                    || next == 'ê'
                {
                    out.push(S);
                    i += 1;
                } else if next == 'c' && (next2 == 'e' || next2 == 'i') {
                    out.push(K);
                    out.push(S);
                    i += 2;
                } else if next == 'c' {
                    out.push(K);
                    i += 2;
                } else if is_final && !matches!(prev, 'n' | 'm') {
                    out.push(K);
                    i += 1;
                } else if is_final {
                    i += 1;
                } else {
                    out.push(K);
                    i += 1;
                }
            }
            'ç' => {
                out.push(S);
                i += 1;
            }
            'd' => {
                if is_final {
                    i += 1;
                } else {
                    out.push(D);
                    i += 1;
                }
            }
            'f' => {
                out.push(F);
                i += 1;
            }
            'g' => {
                if next == 'n' {
                    out.push(NY);
                    i += 2;
                } else if next == 'u' && (next2 == 'e' || next2 == 'i' || next2 == 'y') {
                    out.push(G);
                    i += 2;
                } else if next == 'e' || next == 'i' || next == 'y' || next == 'é' || next == 'è'
                {
                    out.push(ZH);
                    i += 1;
                } else if next == 't' && i + 2 == n {
                    i += 2;
                } else if is_final {
                    i += 1;
                } else {
                    out.push(G);
                    i += 1;
                }
            }
            'h' => {
                i += 1;
            }
            'j' => {
                out.push(ZH);
                i += 1;
            }
            'k' => {
                out.push(K);
                i += 1;
            }
            'l' => {
                if next == 'l' {
                    out.push(L);
                    i += 2;
                } else {
                    out.push(L);
                    i += 1;
                }
            }
            'm' => {
                if next == 'm' {
                    out.push(M);
                    i += 2;
                } else {
                    out.push(M);
                    i += 1;
                }
            }
            'n' => {
                if next == 'n' {
                    out.push(N);
                    i += 2;
                } else {
                    out.push(N);
                    i += 1;
                }
            }
            'p' => {
                if next == 'h' {
                    out.push(F);
                    i += 2;
                } else if next == 'p' {
                    out.push(P);
                    i += 2;
                } else if is_final
                    && !matches!(word, "cap" | "stop" | "slip" | "hip" | "clip" | "zip")
                {
                    i += 1;
                } else {
                    out.push(P);
                    i += 1;
                }
            }
            'q' => {
                if next == 'u' {
                    out.push(K);
                    i += 2;
                } else {
                    out.push(K);
                    i += 1;
                }
            }
            'r' => {
                if next == 'r' {
                    out.push(RR);
                    i += 2;
                } else {
                    out.push(RR);
                    i += 1;
                }
            }
            's' => {
                if next == 'c' && next2 == 'h' {
                    out.push(SH);
                    i += 3;
                } else if (next == 'c' && (next2 == 'e' || next2 == 'i')) || next == 's' {
                    out.push(S);
                    i += 2;
                } else if prev != ' '
                    && i > 0
                    && (is_vowel_letter(prev)
                        || matches!(prev, 'é' | 'è' | 'ê' | 'à' | 'â' | 'î' | 'ô' | 'ù' | 'û'))
                    && !is_final
                    && (is_vowel_letter(next) || matches!(next, 'é' | 'è' | 'ê'))
                {
                    out.push(Z);
                    i += 1;
                } else if is_final
                    && !matches!(
                        word,
                        "fils"
                            | "ours"
                            | "bus"
                            | "virus"
                            | "sens"
                            | "autobus"
                            | "tennis"
                            | "hélas"
                            | "albatros"
                    )
                {
                    i += 1;
                } else {
                    out.push(S);
                    i += 1;
                }
            }
            't' => {
                if next == 'i' && next2 == 'o' {
                    if prev == 's' {
                        out.push(T);
                        out.push(Y);
                        out.push(ON);
                    } else {
                        out.push(S);
                        out.push(Y);
                        out.push(ON);
                    }
                    i += 4;
                } else if next == 'i' && next2 == 'e' {
                    if matches!(word, "amitié" | "moitié" | "pitié") {
                        out.push(T);
                        out.push(Y);
                        out.push(EY);
                        i += 3;
                    } else {
                        out.push(S);
                        out.push(Y);
                        i += 2;
                    }
                } else if next == 't' {
                    out.push(T);
                    i += 2;
                } else if is_final
                    && (prev == 'n'
                        || !matches!(
                            word,
                            "huit"
                                | "net"
                                | "ouest"
                                | "but"
                                | "direct"
                                | "exact"
                                | "contact"
                                | "concept"
                        ))
                {
                    i += 1;
                } else {
                    out.push(T);
                    i += 1;
                }
            }
            'v' => {
                out.push(V);
                i += 1;
            }
            'w' => {
                out.push(W);
                i += 1;
            }
            'x' => {
                if is_final {
                    if matches!(word, "six" | "dix") {
                        out.push(S);
                    }
                    i += 1;
                } else if prev == ' ' || is_vowel_letter(prev) {
                    if is_vowel_letter(next) {
                        out.push(G);
                        out.push(Z);
                    } else {
                        out.push(K);
                        out.push(S);
                    }
                    i += 1;
                } else {
                    out.push(K);
                    out.push(S);
                    i += 1;
                }
            }
            'z' => {
                if is_final {
                    i += 1;
                } else {
                    out.push(Z);
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    let mut result: Vec<(PhonemeKind, u8)> = Vec::with_capacity(out.len());
    let last_vowel = out.iter().rposition(|k| is_vowel_sound(*k));
    for (idx, k) in out.into_iter().enumerate() {
        let stress = if Some(idx) == last_vowel { 1 } else { 0 };
        result.push((k, stress));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(word: &str) -> Vec<PhonemeKind> {
        phonemize_word(word).into_iter().map(|(k, _)| k).collect()
    }

    #[test]
    fn corpus_words() {
        assert_eq!(kinds("bonjour"), vec![B, ON, ZH, UW, RR]);
        assert_eq!(kinds("aujourd'hui"), vec![OW, ZH, UW, RR, D, UE, IY]);
        assert_eq!(kinds("système"), vec![S, IY, S, T, EH, M]);
        assert_eq!(kinds("intelligence"), vec![EN, T, EH, L, IY, ZH, AN, S]);
        assert_eq!(kinds("fonctionne"), vec![F, ON, K, S, Y, ON, N]);
        assert_eq!(
            kinds("opérationnel"),
            vec![AO, P, EY, RR, AA, S, Y, ON, N, EH, L]
        );
    }

    #[test]
    fn j_elision_is_zh() {
        // "j'ai" = /ʒɛ/: the elided j is ZH (the 'ai' vowel is the
        // codebase's regular /ɛ/ mapping, not /e/).
        assert_eq!(kinds("j'ai"), vec![ZH, EH]);
    }

    #[test]
    fn regular_words() {
        assert_eq!(kinds("chat"), vec![SH, AA]);
        assert_eq!(kinds("maison"), vec![M, EH, Z, ON]);
        assert_eq!(kinds("parler"), vec![P, AA, RR, L, EY]);
        assert_eq!(kinds("les"), vec![L, EY]);
        assert_eq!(kinds("et"), vec![EY]);
        assert_eq!(kinds("est"), vec![EH]);
        assert_eq!(kinds("quatre"), vec![K, AA, T, RR]);
        assert_eq!(kinds("bien"), vec![B, Y, EN]);
        assert_eq!(kinds("enfant"), vec![AN, F, AN]);
        assert_eq!(kinds("deux"), vec![D, OE]);
        assert_eq!(kinds("heure"), vec![OEU, RR]);
        assert_eq!(kinds("nation"), vec![N, AA, S, Y, ON]);
        assert_eq!(kinds("ville"), vec![V, IY, L]);
        assert_eq!(kinds("fille"), vec![F, IY, Y]);
        assert_eq!(kinds("pain"), vec![P, EN]);
        assert_eq!(kinds("année"), vec![AA, N, EY]);
        assert_eq!(kinds("abord"), vec![AA, B, AO, RR]);
        assert_eq!(kinds("où"), vec![UW]);
    }

    #[test]
    fn elision() {
        assert_eq!(kinds("l'adresse"), vec![L, AA, D, RR, EH, S]);
    }

    #[test]
    fn stress_on_last_vowel() {
        let word = phonemize_word("bonjour");
        let stressed = word.iter().filter(|(_, s)| *s == 1).count();
        assert_eq!(stressed, 1);
    }

    #[test]
    fn interjections() {
        assert_eq!(kinds("ah"), vec![AA]);
        assert_eq!(kinds("oh"), vec![OW]);
        assert_eq!(kinds("hé"), vec![EY]);
        assert_eq!(kinds("zut"), vec![Z, UE, T]);
        assert_eq!(kinds("bravo"), vec![B, RR, AA, V, OW]);
        assert_eq!(kinds("euh"), vec![OE]);
        assert_eq!(kinds("beurk"), vec![B, OEU, RR, K]);
        assert_eq!(kinds("oups"), vec![UW, P, S]);
        assert_eq!(kinds("hein"), vec![EN]);
        assert_eq!(kinds("aïe"), vec![AA, Y]);
        assert_eq!(kinds("ouais"), vec![W, EH]);
        assert_eq!(kinds("zut!"), vec![Z, UE, T]);
    }

    fn flat_clause(words: &[&str]) -> Vec<(PhonemeKind, u8, f32)> {
        phonemize_clause(words).into_iter().flatten().collect()
    }

    #[test]
    fn negation_pitch_shapes() {
        let clause = flat_clause(&["je", "ne", "mange", "pas"]);
        let vowels: Vec<(PhonemeKind, f32)> = clause
            .iter()
            .filter(|(k, _, _)| is_vowel_sound(*k))
            .map(|(k, _, s)| (*k, *s))
            .collect();
        assert_eq!(
            vowels,
            vec![(AX, 0.0), (AX, -0.06), (AN, -0.05), (AA, -0.15)]
        );
    }

    #[test]
    fn negation_resets_after_keyword() {
        let clause = flat_clause(&["ne", "pas", "bonjour"]);
        let vowels: Vec<f32> = clause
            .iter()
            .filter(|(k, _, _)| is_vowel_sound(*k))
            .map(|(_, _, s)| *s)
            .collect();
        assert_eq!(vowels, vec![-0.06, -0.15, 0.0, 0.0]);
    }

    #[test]
    fn clause_groups_one_entry_per_word() {
        let clause = phonemize_clause(&["je", "ne", "mange", "pas"]);
        assert_eq!(clause.len(), 4);
        assert!(clause.iter().all(|word| !word.is_empty()));
    }

    #[test]
    fn negation_multiword_keyword_falls_on_last_vowel() {
        let clause = flat_clause(&["ne", "personne"]);
        assert_eq!(clause[6], (AO, 1, -0.15));
    }
}
