use crate::g2p::en;
use crate::g2p::fr;
use crate::phoneme::PhonemeKind;
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct Overrides {
    entries: HashMap<String, Vec<PhonemeKind>>,
    parse_errors: Vec<(String, String)>,
}

impl Overrides {
    pub fn get(&self, word: &str) -> Option<&Vec<PhonemeKind>> {
        self.entries.get(&word.to_lowercase())
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn first_parse_error(&self) -> Option<(&str, &str)> {
        self.parse_errors
            .first()
            .map(|(word, symbol)| (word.as_str(), symbol.as_str()))
    }

    pub fn parse_symbol(sym: &str) -> Option<PhonemeKind> {
        en::symbol(sym).or_else(|| fr::symbol(sym))
    }

    pub fn insert(&mut self, word: &str, symbols: &[&str]) {
        let mut kinds = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            match Self::parse_symbol(symbol) {
                Some(kind) => kinds.push(kind),
                None => {
                    self.parse_errors
                        .push((word.to_lowercase(), (*symbol).to_string()));
                    return;
                }
            }
        }
        if !kinds.is_empty() {
            self.entries.insert(word.to_lowercase(), kinds);
        }
    }

    pub fn from_toml_table(table: &toml::map::Map<String, toml::Value>) -> Self {
        let mut overrides = Self::default();
        for (word, value) in table {
            let symbols = match value {
                toml::Value::String(s) => s.split_whitespace().collect::<Vec<_>>(),
                toml::Value::Array(items) => {
                    items.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()
                }
                _ => continue,
            };
            overrides.insert(word, &symbols);
        }
        overrides
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_and_gets() {
        let mut overrides = Overrides::default();
        overrides.insert("linux", &["L", "IH", "N", "UX", "K", "S"]);
        assert_eq!(overrides.get("LINUX").unwrap().len(), 6);
    }

    #[test]
    fn records_unknown_symbols_as_errors() {
        let mut overrides = Overrides::default();
        overrides.insert("foo", &["ZZZ", "M", "M"]);
        assert_eq!(overrides.get("foo"), None);
        assert_eq!(overrides.first_parse_error(), Some(("foo", "ZZZ")));
    }
}
