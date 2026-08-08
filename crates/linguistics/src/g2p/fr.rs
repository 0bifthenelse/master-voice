use crate::phoneme::PhonemeKind::{self, *};

const DICT_FR: &[(&str, &[PhonemeKind])] = &[
    ("aujourd'hui", &[OW, Z, UW, RR, D, UE, IY]),
    ("monsieur", &[M, AX, S, ZH, OE]),
    ("madame", &[M, AA, D, AA, M]),
    ("mademoiselle", &[M, AA, D, M, W, AA, Z, EH, L]),
    ("messieurs", &[M, EH, S, ZH, OE]),
    ("gens", &[Z, AN]),
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
    ("gentil", &[Z, AN, T, IY]),
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
    ("je", &[Z, AX]),
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
];

fn lookup(word: &str) -> Option<Vec<PhonemeKind>> {
    let lower = word.to_lowercase();
    DICT_FR
        .iter()
        .find(|(w, _)| *w == lower)
        .map(|(_, phones)| phones.to_vec())
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
        'g' => vec![Z, EY],
        'h' => vec![AA, SH],
        'i' => vec![IY],
        'j' => vec![Z, IY],
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

pub fn phonemize_word(word: &str) -> Vec<(PhonemeKind, u8)> {
    let lower = word.to_lowercase();
    if lower.len() == 1 {
        return spell_letter_fr(lower.chars().next().unwrap_or(' '))
            .into_iter()
            .map(|k| (k, 0))
            .collect();
    }
    if let Some(phones) = lookup(&lower) {
        return phones.into_iter().map(|k| (k, 0)).collect();
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
                    "j" => Some(Z),
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
                        out.extend([(Z, 0), (UE, 0), (S, 0), (K, 0)]);
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
                    out.push(Z);
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
                out.push(Z);
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
        assert_eq!(kinds("bonjour"), vec![B, ON, Z, UW, RR]);
        assert_eq!(kinds("aujourd'hui"), vec![OW, Z, UW, RR, D, UE, IY]);
        assert_eq!(kinds("système"), vec![S, IY, S, T, EH, M]);
        assert_eq!(kinds("intelligence"), vec![EN, T, EH, L, IY, Z, AN, S]);
        assert_eq!(kinds("fonctionne"), vec![F, ON, K, S, Y, ON, N]);
        assert_eq!(
            kinds("opérationnel"),
            vec![AO, P, EY, RR, AA, S, Y, ON, N, EH, L]
        );
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
}
