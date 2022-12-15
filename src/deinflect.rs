use std::collections::HashMap;

use enumflags2::{bitflags, BitFlags};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
use wana_kana::{to_hiragana::to_hiragana, to_katakana::to_katakana};

#[bitflags]
#[repr(u8)]
#[derive(Deserialize, Serialize, Copy, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Rule {
    V1 = 0b0000_0001,   // Verb ichidan
    V5 = 0b0000_0010,   // Verb godan
    Vs = 0b0000_0100,   // Verb suru
    Vk = 0b0000_1000,   // Verb kuru
    Vz = 0b0001_0000,   // Verb zuru
    AdjI = 0b0010_0000, // Adjective i
    Iru = 0b0100_0000,  // Intermediate -iru endings for progressive or perfect tense
}

impl TryFrom<&str> for Rule {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "v1" => Ok(Self::V1),
            "v5" => Ok(Self::V5),
            "vs" => Ok(Self::Vs),
            "vk" => Ok(Self::Vk),
            "vz" => Ok(Self::Vz),
            "adj-i" => Ok(Self::AdjI),
            "iru" => Ok(Self::Iru),
            _ => Err(format!("String `{}` is not a valid Rule", value)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rules(pub BitFlags<Rule>);

impl From<Vec<Rule>> for Rules {
    fn from(v: Vec<Rule>) -> Self {
        let mut r = BitFlags::<Rule>::empty();
        r.extend(v);
        Self(r)
    }
}

fn from_vec<'de, D>(deserializer: D) -> Result<Rules, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Vec<Rule> = Deserialize::deserialize(deserializer)?;

    Ok(Rules::from(v))
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ReasonInfo {
    kana_in: String,
    kana_out: String,
    #[serde(deserialize_with = "from_vec")]
    rules_in: Rules,
    #[serde(deserialize_with = "from_vec")]
    rules_out: Rules,
}

#[derive(Deserialize, Debug)]
pub struct Reasons(HashMap<String, Vec<ReasonInfo>>);

#[derive(Clone, Debug)]
pub struct Deinflection {
    pub term: String,
    pub rules: Rules,
    pub source: String,
    pub reasons: Vec<String>,
}

impl Deinflection {
    fn new(term: String, rules: Rules, source: String, reasons: Vec<String>) -> Self {
        Self {
            term,
            rules,
            source,
            reasons,
        }
    }
}

#[must_use]
pub fn inflection_reasons() -> Reasons {
    serde_json::from_str(include_str!("deinflect.json"))
        .expect("Included deinflect.json file should be parsable")
}

#[must_use]
pub fn word_deinflections(source: &str, reasons: &Reasons) -> Vec<Deinflection> {
    let mut results = vec![Deinflection::new(
        source.to_string(),
        Rules(BitFlags::<Rule>::empty()),
        source.to_string(),
        vec![],
    )];

    let mut i = 0;
    while i < results.len() {
        let prev = results[i].clone();
        i += 1;

        for (reason, variants) in &reasons.0 {
            let applicable_variants = variants.iter().filter(|v| {
                (prev.rules.0.is_empty() || prev.rules.0.intersects(v.rules_in.0))
                    && prev.term.ends_with(&v.kana_in)
                    && (prev.term.len() - v.kana_in.len() + v.kana_out.len() > 0)
            });

            results.extend(applicable_variants.map(|v| {
                Deinflection::new(
                    prev.term
                        .strip_suffix(&v.kana_in)
                        .expect("Should be guaranteed by filter")
                        .to_string()
                        + (&v.kana_out),
                    v.rules_out.clone(),
                    source.to_string(),
                    std::iter::once(reason.clone())
                        .chain(prev.reasons.iter().cloned())
                        .collect(),
                )
            }));
        }
    }

    results
}

fn mutate(s: &str) -> Vec<String> {
    // TODO Collapse emphatic sequensec
    vec![s.to_owned(), to_hiragana(s), to_katakana(s)]
}

#[must_use]
pub fn string_deinflections(source: &str, reasons: &Reasons) -> Vec<Deinflection> {
    let substrings: Vec<String> = mutate(source)
        .iter()
        .flat_map(|s| {
            (1..=s.chars().count())
                .rev()
                .map(|i| &s[..s.chars().take(i).map(char::len_utf8).sum()])
        })
        .unique()
        .map(std::borrow::ToOwned::to_owned)
        .collect();

    substrings
        .iter()
        .flat_map(|s| word_deinflections(s, reasons))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deinflections() {
        let reasons = inflection_reasons();

        let d = string_deinflections("聞かれました", &reasons);

        assert!(d.iter().any(|d| d.term.eq("聞かれる")));
        assert!(d.iter().any(|d| d.term.eq("聞く")));
    }

    #[test]
    fn deinflections_romaji() {
        let reasons = inflection_reasons();

        let d = string_deinflections("kikaremashita", &reasons);

        assert!(d.iter().any(|d| d.term.eq("きかれる")));
        assert!(d.iter().any(|d| d.term.eq("きく")));
    }
}
