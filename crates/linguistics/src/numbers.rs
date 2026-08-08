use crate::lang::Language;

const ONES_EN: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];

const TENS_EN: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

const ORDINAL_ONES_EN: [&str; 20] = [
    "zeroth",
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eighth",
    "ninth",
    "tenth",
    "eleventh",
    "twelfth",
    "thirteenth",
    "fourteenth",
    "fifteenth",
    "sixteenth",
    "seventeenth",
    "eighteenth",
    "nineteenth",
];

const ORDINAL_TENS_EN: [&str; 10] = [
    "",
    "",
    "twentieth",
    "thirtieth",
    "fortieth",
    "fiftieth",
    "sixtieth",
    "seventieth",
    "eightieth",
    "ninetieth",
];

const ONES_FR: [&str; 20] = [
    "zéro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix", "onze",
    "douze", "treize", "quatorze", "quinze", "seize", "dix-sept", "dix-huit", "dix-neuf",
];

const TENS_FR: [&str; 10] = [
    "",
    "",
    "vingt",
    "trente",
    "quarante",
    "cinquante",
    "soixante",
    "soixante-dix",
    "quatre-vingts",
    "quatre-vingt-dix",
];

const ORDINAL_FR: [&str; 20] = [
    "zéroième",
    "premier",
    "deuxième",
    "troisième",
    "quatrième",
    "cinquième",
    "sixième",
    "septième",
    "huitième",
    "neuvième",
    "dixième",
    "onzième",
    "douzième",
    "treizième",
    "quatorzième",
    "quinzième",
    "seizième",
    "dix-septième",
    "dix-huitième",
    "dix-neuvième",
];

fn cardinal_en(mut n: u64) -> String {
    if n == 0 {
        return "zero".to_string();
    }
    let mut parts = Vec::new();
    let scales = [
        (1_000_000_000_000u64, "trillion"),
        (1_000_000_000, "billion"),
        (1_000_000, "million"),
        (1_000, "thousand"),
    ];
    for (scale, name) in scales {
        if n >= scale {
            let count = n / scale;
            parts.push(format!("{} {}", cardinal_en(count), name));
            n %= scale;
        }
    }
    if n >= 100 {
        parts.push(format!("{} hundred", cardinal_en(n / 100)));
        n %= 100;
    }
    if n > 0 {
        if n < 20 {
            parts.push(ONES_EN[n as usize].to_string());
        } else {
            let tens = TENS_EN[(n / 10) as usize];
            let ones = n % 10;
            if ones == 0 {
                parts.push(tens.to_string());
            } else {
                parts.push(format!("{}-{}", tens, ONES_EN[ones as usize]));
            }
        }
    }
    parts.join(" ")
}

fn ordinal_en(n: u64) -> String {
    if n < 20 {
        return ORDINAL_ONES_EN[n as usize].to_string();
    }
    if n < 100 {
        let tens = n / 10;
        let ones = n % 10;
        if ones == 0 {
            return ORDINAL_TENS_EN[tens as usize].to_string();
        }
        return format!(
            "{}-{}",
            TENS_EN[tens as usize], ORDINAL_ONES_EN[ones as usize]
        );
    }
    let base = cardinal_en(n);
    let mut words = base.split(' ').map(String::from).collect::<Vec<_>>();
    let last = words.pop().unwrap_or_default();
    let ordinal = if let Some(idx) = last.find('-') {
        let (head, tail) = last.split_at(idx + 1);
        let tail_num: u64 = tail.parse().unwrap_or(0);
        format!(
            "{}{}",
            head,
            if tail_num < 20 {
                ORDINAL_ONES_EN[tail_num as usize]
            } else {
                tail
            }
        )
    } else if matches!(
        last.as_str(),
        "hundred" | "thousand" | "million" | "billion" | "trillion"
    ) {
        format!("{}th", last)
    } else if let Some(pos) = ONES_EN.iter().position(|w| *w == last) {
        ORDINAL_ONES_EN[pos].to_string()
    } else if let Some(pos) = TENS_EN.iter().position(|w| *w == last) {
        ORDINAL_TENS_EN[pos].to_string()
    } else {
        last
    };
    words.push(ordinal);
    words.join(" ")
}

fn cardinal_fr(mut n: u64) -> String {
    if n == 0 {
        return "zéro".to_string();
    }
    let mut parts = Vec::new();
    let scales = [
        (1_000_000_000_000u64, "billion"),
        (1_000_000_000, "milliard"),
        (1_000_000, "million"),
        (1_000, "mille"),
    ];
    for (scale, name) in scales {
        if n >= scale {
            let count = n / scale;
            let count_word = if scale == 1_000 && count == 1 {
                "".to_string()
            } else {
                cardinal_fr(count)
            };
            let plural = if count > 1 && scale == 1_000_000 {
                "s"
            } else {
                ""
            };
            parts.push(format!(
                "{}{}{}",
                if count_word.is_empty() {
                    String::new()
                } else {
                    count_word + " "
                },
                name,
                plural
            ));
            n %= scale;
        }
    }
    if n >= 100 {
        let hundreds = n / 100;
        let remainder = n % 100;
        match hundreds {
            1 => parts.push("cent".to_string()),
            h => {
                let plural = if remainder == 0 { "s" } else { "" };
                parts.push(format!("{} cent{}", cardinal_fr(h), plural));
            }
        }
        n = remainder;
    }
    if n > 0 {
        let word = match n {
            71 => "soixante et onze".to_string(),
            81 => "quatre-vingt-un".to_string(),
            91 => "quatre-vingt-onze".to_string(),
            72..=79 => format!("soixante-{}", ONES_FR[(n - 60) as usize]),
            92..=99 => format!("quatre-vingt-{}", ONES_FR[(n - 80) as usize]),
            _ if n < 20 => ONES_FR[n as usize].to_string(),
            _ => {
                let tens = n / 10;
                let ones = n % 10;
                if ones == 0 {
                    if tens == 8 {
                        "quatre-vingts".to_string()
                    } else {
                        TENS_FR[tens as usize].to_string()
                    }
                } else if ones == 1 && !matches!(tens, 7 | 9) {
                    format!("{} et un", TENS_FR[tens as usize])
                } else {
                    format!("{}-{}", TENS_FR[tens as usize], ONES_FR[ones as usize])
                }
            }
        };
        parts.push(word);
    }
    parts.join(" ")
}

fn ordinal_fr(n: u64) -> String {
    if n == 1 {
        return "premier".to_string();
    }
    if n < 20 {
        return ORDINAL_FR[n as usize].to_string();
    }
    let base = cardinal_fr(n);
    let mut words = base.rsplit(' ').collect::<Vec<_>>();
    let last = words[0];
    let ordinal = if let Some(idx) = last.rfind('-') {
        let tail = &last[idx + 1..];
        let tail_num: u64 = tail.parse().unwrap_or(0);
        if tail_num == 1 {
            format!("{}-unième", &last[..idx])
        } else if tail_num < 20 {
            format!(
                "{}-{}",
                &last[..idx],
                ORDINAL_FR[tail_num as usize]
                    .trim_start_matches("premier")
                    .replace("premier", "unième")
            )
        } else {
            last.to_string()
        }
    } else if last.parse::<u64>().is_ok() {
        format!("{}ième", last)
    } else if last == "cent" {
        "centième".to_string()
    } else if last == "mille" {
        "millième".to_string()
    } else {
        last.to_string()
    };
    words[0] = &ordinal;
    words.join(" ")
}

fn digits_spelled_en(digits: &str) -> String {
    digits
        .chars()
        .map(|c| match c {
            '0' => "zero",
            '1' => "one",
            '2' => "two",
            '3' => "three",
            '4' => "four",
            '5' => "five",
            '6' => "six",
            '7' => "seven",
            '8' => "eight",
            '9' => "nine",
            _ => "",
        })
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn digits_spelled_fr(digits: &str) -> String {
    digits
        .chars()
        .map(|c| match c {
            '0' => "zéro",
            '1' => "un",
            '2' => "deux",
            '3' => "trois",
            '4' => "quatre",
            '5' => "cinq",
            '6' => "six",
            '7' => "sept",
            '8' => "huit",
            '9' => "neuf",
            _ => "",
        })
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn year_en(year: u64) -> String {
    if year < 1000 {
        return cardinal_en(year);
    }
    match year {
        1000 => "one thousand".to_string(),
        2000 => "two thousand".to_string(),
        1001..=1099 => format!("one thousand {}", cardinal_en(year % 1000)),
        1100..=1999 => {
            let century = year / 100;
            let rest = year % 100;
            match rest {
                0 => format!("{} hundred", cardinal_en(century)),
                1..=9 => format!("{} oh {}", cardinal_en(century), cardinal_en(rest)),
                _ => format!("{} {}", cardinal_en(century), cardinal_en(rest)),
            }
        }
        2001..=2009 => format!("two thousand {}", cardinal_en(year % 1000)),
        _ => {
            let first = year / 100;
            let rest = year % 100;
            if rest == 0 {
                format!("{} hundred", cardinal_en(first))
            } else if rest < 10 {
                format!("{} oh {}", cardinal_en(first), cardinal_en(rest))
            } else {
                format!("{} {}", cardinal_en(first), cardinal_en(rest))
            }
        }
    }
}

fn year_fr(year: u64) -> String {
    if year < 1000 {
        return cardinal_fr(year);
    }
    let thousands = year / 1000;
    let rest = year % 1000;
    match thousands {
        1 => format!(
            "mille {}",
            if rest == 0 {
                "".to_string()
            } else {
                cardinal_fr(rest)
            }
        )
        .trim()
        .to_string(),
        t => {
            let head = cardinal_fr(t);
            if rest == 0 {
                format!("{} mille", head)
            } else {
                format!("{} mille {}", head, cardinal_fr(rest))
            }
        }
    }
}

pub fn cardinal(n: u64, lang: Language) -> String {
    match lang {
        Language::English => cardinal_en(n),
        Language::French => cardinal_fr(n),
    }
}

pub fn ordinal(n: u64, lang: Language) -> String {
    match lang {
        Language::English => ordinal_en(n),
        Language::French => ordinal_fr(n),
    }
}

pub fn digits_spelled(digits: &str, lang: Language) -> String {
    match lang {
        Language::English => digits_spelled_en(digits),
        Language::French => digits_spelled_fr(digits),
    }
}

pub fn year(n: u64, lang: Language) -> String {
    match lang {
        Language::English => year_en(n),
        Language::French => year_fr(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_cardinals() {
        assert_eq!(cardinal_en(0), "zero");
        assert_eq!(cardinal_en(12), "twelve");
        assert_eq!(cardinal_en(21), "twenty-one");
        assert_eq!(cardinal_en(42), "forty-two");
        assert_eq!(cardinal_en(100), "one hundred");
        assert_eq!(cardinal_en(101), "one hundred one");
        assert_eq!(cardinal_en(1000), "one thousand");
        assert_eq!(
            cardinal_en(1_234_567),
            "one million two hundred thirty-four thousand five hundred sixty-seven"
        );
    }

    #[test]
    fn english_ordinals() {
        assert_eq!(ordinal_en(1), "first");
        assert_eq!(ordinal_en(2), "second");
        assert_eq!(ordinal_en(3), "third");
        assert_eq!(ordinal_en(4), "fourth");
        assert_eq!(ordinal_en(21), "twenty-first");
        assert_eq!(ordinal_en(100), "one hundredth");
        assert_eq!(ordinal_en(101), "one hundred first");
    }

    #[test]
    fn french_cardinals() {
        assert_eq!(cardinal_fr(12), "douze");
        assert_eq!(cardinal_fr(21), "vingt et un");
        assert_eq!(cardinal_fr(70), "soixante-dix");
        assert_eq!(cardinal_fr(71), "soixante et onze");
        assert_eq!(cardinal_fr(80), "quatre-vingts");
        assert_eq!(cardinal_fr(81), "quatre-vingt-un");
        assert_eq!(cardinal_fr(90), "quatre-vingt-dix");
        assert_eq!(cardinal_fr(91), "quatre-vingt-onze");
        assert_eq!(cardinal_fr(200), "deux cents");
        assert_eq!(cardinal_fr(201), "deux cent un");
        assert_eq!(cardinal_fr(1000), "mille");
        assert_eq!(cardinal_fr(2026), "deux mille vingt-six");
        assert_eq!(cardinal_fr(1945), "mille neuf cent quarante-cinq");
    }

    #[test]
    fn french_ordinals() {
        assert_eq!(ordinal_fr(1), "premier");
        assert_eq!(ordinal_fr(2), "deuxième");
        assert_eq!(ordinal_fr(5), "cinquième");
    }

    #[test]
    fn english_years() {
        assert_eq!(year_en(2026), "twenty twenty-six");
        assert_eq!(year_en(1999), "nineteen ninety-nine");
        assert_eq!(year_en(2000), "two thousand");
        assert_eq!(year_en(2005), "two thousand five");
        assert_eq!(year_en(1905), "nineteen oh five");
    }
}
