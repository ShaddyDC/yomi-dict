use std::collections::HashMap;

use enumflags2::{bitflags, BitFlags};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
use wana_kana::ConvertJapanese;

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
            _ => Err(format!("String `{value}` is not a valid Rule")),
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

pub trait Deinflectable {
    #[must_use]
    fn word_deinflections(&self, reasons: &Reasons) -> Vec<Deinflection>;
    #[must_use]
    fn string_deinflections(&self, reasons: &Reasons) -> Vec<Deinflection>;
}

impl Deinflectable for &str {
    /// Get all possible deinflections of the given word (the entire string) given the list of rules.
    fn word_deinflections(&self, reasons: &Reasons) -> Vec<Deinflection> {
        let mut results = vec![Deinflection::new(
            (*self).to_string(),
            Rules(BitFlags::<Rule>::empty()),
            (*self).to_string(),
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
                        (*self).to_string(),
                        std::iter::once(reason.clone())
                            .chain(prev.reasons.iter().cloned())
                            .collect(),
                    )
                }));
            }
        }

        results
    }

    /// Get all possible deinflections of the given string and its prefixes given the list of rules.
    fn string_deinflections(&self, reasons: &Reasons) -> Vec<Deinflection> {
        let substrings: Vec<String> = mutate(self)
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
            .flat_map(|s| s.as_str().word_deinflections(reasons))
            .collect()
    }
}

fn mutate(s: &str) -> Vec<String> {
    // TODO Collapse emphatic sequensec
    vec![s.to_owned(), s.to_hiragana(), s.to_katakana()]
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deinflections() {
        let reasons = inflection_reasons();

        let d = "聞かれました".string_deinflections(&reasons);

        assert!(d.iter().any(|d| d.term.eq("聞かれる")));
        assert!(d.iter().any(|d| d.term.eq("聞く")));
    }

    #[test]
    fn deinflections_romaji() {
        let reasons = inflection_reasons();

        let d = "kikaremashita".string_deinflections(&reasons);

        assert!(d.iter().any(|d| d.term.eq("きかれる")));
        assert!(d.iter().any(|d| d.term.eq("きく")));
    }

    struct DeinflectValidTest {
        term: &'static str,
        source: &'static str,
        rule: &'static str,
        reasons: Vec<&'static str>,
    }

    struct DeinflectInvalidTest {
        term: &'static str,
        source: &'static str,
        rule: &'static str,
    }

    // Test cases taken from yomichan

    #[test]
    fn valid_cases() {
        let cases = vec![
            // Adjective
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しい",
                rule: "adj-i",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しそう",
                rule: "adj-i",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しすぎる",
                rule: "adj-i",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しかったら",
                rule: "adj-i",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しかったり",
                rule: "adj-i",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しくて",
                rule: "adj-i",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しく",
                rule: "adj-i",
                reasons: vec!["adv"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しくない",
                rule: "adj-i",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しさ",
                rule: "adj-i",
                reasons: vec!["noun"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しかった",
                rule: "adj-i",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しくありません",
                rule: "adj-i",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しくありませんでした",
                rule: "adj-i",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しき",
                rule: "adj-i",
                reasons: vec!["-ki"],
            },
            DeinflectValidTest {
                term: "愛しい",
                source: "愛しげ",
                rule: "adj-i",
                reasons: vec!["-ge"],
            },
            // Common verbs
            DeinflectValidTest {
                term: "食べる",
                source: "食べる",
                rule: "v1",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べます",
                rule: "v1",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べた",
                rule: "v1",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べました",
                rule: "v1",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べて",
                rule: "v1",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べられる",
                rule: "v1",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べられる",
                rule: "v1",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べさせる",
                rule: "v1",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べさせられる",
                rule: "v1",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べろ",
                rule: "v1",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べない",
                rule: "v1",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べません",
                rule: "v1",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べなかった",
                rule: "v1",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べませんでした",
                rule: "v1",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べなくて",
                rule: "v1",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べられない",
                rule: "v1",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べられない",
                rule: "v1",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べさせない",
                rule: "v1",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べさせられない",
                rule: "v1",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べるな",
                rule: "v1",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べれば",
                rule: "v1",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べちゃう",
                rule: "v1",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べちまう",
                rule: "v1",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べなさい",
                rule: "v1",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べそう",
                rule: "v1",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べすぎる",
                rule: "v1",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べたい",
                rule: "v1",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べたら",
                rule: "v1",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べたり",
                rule: "v1",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べず",
                rule: "v1",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べぬ",
                rule: "v1",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べ",
                rule: "v1",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べましょう",
                rule: "v1",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べよう",
                rule: "v1",
                reasons: vec!["volitional"],
            },
            // vec!["causative passive"]
            DeinflectValidTest {
                term: "食べる",
                source: "食べとく",
                rule: "v1",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べている",
                rule: "v1",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べておる",
                rule: "v1",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べてる",
                rule: "v1",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べとる",
                rule: "v1",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べてしまう",
                rule: "v1",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買う",
                rule: "v5",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買います",
                rule: "v5",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買った",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買いました",
                rule: "v5",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買って",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買える",
                rule: "v5",
                reasons: vec!["potential"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買われる",
                rule: "v5",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買わせる",
                rule: "v5",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買わせられる",
                rule: "v5",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買え",
                rule: "v5",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買わない",
                rule: "v5",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買いません",
                rule: "v5",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買わなかった",
                rule: "v5",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買いませんでした",
                rule: "v5",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買わなくて",
                rule: "v5",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買えない",
                rule: "v5",
                reasons: vec!["potential", "negative"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買われない",
                rule: "v5",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買わせない",
                rule: "v5",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買わせられない",
                rule: "v5",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買うな",
                rule: "v5",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買えば",
                rule: "v5",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買っちゃう",
                rule: "v5",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買っちまう",
                rule: "v5",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買いなさい",
                rule: "v5",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買いそう",
                rule: "v5",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買いすぎる",
                rule: "v5",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買いたい",
                rule: "v5",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買ったら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買ったり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買わず",
                rule: "v5",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買わぬ",
                rule: "v5",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買い",
                rule: "v5",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買いましょう",
                rule: "v5",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買おう",
                rule: "v5",
                reasons: vec!["volitional"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買わされる",
                rule: "v5",
                reasons: vec!["causative passive"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買っとく",
                rule: "v5",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買っている",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買っておる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買ってる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買っとる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "買う",
                source: "買ってしまう",
                rule: "v5",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行く",
                rule: "v5",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行きます",
                rule: "v5",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行った",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行きました",
                rule: "v5",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行って",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行ける",
                rule: "v5",
                reasons: vec!["potential"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かれる",
                rule: "v5",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かせる",
                rule: "v5",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かせられる",
                rule: "v5",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行け",
                rule: "v5",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かない",
                rule: "v5",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行きません",
                rule: "v5",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かなかった",
                rule: "v5",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行きませんでした",
                rule: "v5",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かなくて",
                rule: "v5",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行けない",
                rule: "v5",
                reasons: vec!["potential", "negative"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かれない",
                rule: "v5",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かせない",
                rule: "v5",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かせられない",
                rule: "v5",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行くな",
                rule: "v5",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行けば",
                rule: "v5",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行っちゃう",
                rule: "v5",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行っちまう",
                rule: "v5",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行きなさい",
                rule: "v5",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行きそう",
                rule: "v5",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行きすぎる",
                rule: "v5",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行きたい",
                rule: "v5",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行いたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行いたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かず",
                rule: "v5",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かぬ",
                rule: "v5",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行き",
                rule: "v5",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行きましょう",
                rule: "v5",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行こう",
                rule: "v5",
                reasons: vec!["volitional"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行かされる",
                rule: "v5",
                reasons: vec!["causative passive"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行いとく",
                rule: "v5",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行っている",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行っておる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行ってる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行っとる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "行く",
                source: "行ってしまう",
                rule: "v5",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぐ",
                rule: "v5",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぎます",
                rule: "v5",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いだ",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぎました",
                rule: "v5",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いで",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳げる",
                rule: "v5",
                reasons: vec!["potential"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がれる",
                rule: "v5",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がせる",
                rule: "v5",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がせられる",
                rule: "v5",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳げ",
                rule: "v5",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がない",
                rule: "v5",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぎません",
                rule: "v5",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がなかった",
                rule: "v5",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぎませんでした",
                rule: "v5",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がなくて",
                rule: "v5",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳げない",
                rule: "v5",
                reasons: vec!["potential", "negative"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がれない",
                rule: "v5",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がせない",
                rule: "v5",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がせられない",
                rule: "v5",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぐな",
                rule: "v5",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳げば",
                rule: "v5",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いじゃう",
                rule: "v5",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いじまう",
                rule: "v5",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぎなさい",
                rule: "v5",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぎそう",
                rule: "v5",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぎすぎる",
                rule: "v5",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぎたい",
                rule: "v5",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いだら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いだり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がず",
                rule: "v5",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がぬ",
                rule: "v5",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぎ",
                rule: "v5",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ぎましょう",
                rule: "v5",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳ごう",
                rule: "v5",
                reasons: vec!["volitional"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳がされる",
                rule: "v5",
                reasons: vec!["causative passive"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いどく",
                rule: "v5",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いでいる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いでおる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いでる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "泳ぐ",
                source: "泳いでしまう",
                rule: "v5",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話す",
                rule: "v5",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話します",
                rule: "v5",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話した",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しました",
                rule: "v5",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話して",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話せる",
                rule: "v5",
                reasons: vec!["potential"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話される",
                rule: "v5",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話させる",
                rule: "v5",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話させられる",
                rule: "v5",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話せ",
                rule: "v5",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話さない",
                rule: "v5",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しません",
                rule: "v5",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話さなかった",
                rule: "v5",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しませんでした",
                rule: "v5",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話さなくて",
                rule: "v5",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話せない",
                rule: "v5",
                reasons: vec!["potential", "negative"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話されない",
                rule: "v5",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話させない",
                rule: "v5",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話させられない",
                rule: "v5",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話すな",
                rule: "v5",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話せば",
                rule: "v5",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しちゃう",
                rule: "v5",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しちまう",
                rule: "v5",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しなさい",
                rule: "v5",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しそう",
                rule: "v5",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しすぎる",
                rule: "v5",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話したい",
                rule: "v5",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話したら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話したり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話さず",
                rule: "v5",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話さぬ",
                rule: "v5",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話し",
                rule: "v5",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しましょう",
                rule: "v5",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話そう",
                rule: "v5",
                reasons: vec!["volitional"],
            },
            // vec!["causative passive"]
            DeinflectValidTest {
                term: "話す",
                source: "話しとく",
                rule: "v5",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話している",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しておる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話してる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話しとる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "話す",
                source: "話してしまう",
                rule: "v5",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待つ",
                rule: "v5",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ちます",
                rule: "v5",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待った",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ちました",
                rule: "v5",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待って",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待てる",
                rule: "v5",
                reasons: vec!["potential"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たれる",
                rule: "v5",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たせる",
                rule: "v5",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たせられる",
                rule: "v5",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待て",
                rule: "v5",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たない",
                rule: "v5",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ちません",
                rule: "v5",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たなかった",
                rule: "v5",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ちませんでした",
                rule: "v5",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たなくて",
                rule: "v5",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待てない",
                rule: "v5",
                reasons: vec!["potential", "negative"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たれない",
                rule: "v5",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たせない",
                rule: "v5",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たせられない",
                rule: "v5",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待つな",
                rule: "v5",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待てば",
                rule: "v5",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待っちゃう",
                rule: "v5",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待っちまう",
                rule: "v5",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ちなさい",
                rule: "v5",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ちそう",
                rule: "v5",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ちすぎる",
                rule: "v5",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ちたい",
                rule: "v5",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ったら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ったり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たず",
                rule: "v5",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たぬ",
                rule: "v5",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ち",
                rule: "v5",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ちましょう",
                rule: "v5",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待とう",
                rule: "v5",
                reasons: vec!["volitional"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待たされる",
                rule: "v5",
                reasons: vec!["causative passive"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待っとく",
                rule: "v5",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待っている",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待っておる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ってる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待っとる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "待つ",
                source: "待ってしまう",
                rule: "v5",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死ぬ",
                rule: "v5",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死にます",
                rule: "v5",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んだ",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死にました",
                rule: "v5",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んで",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死ねる",
                rule: "v5",
                reasons: vec!["potential"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死なれる",
                rule: "v5",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死なせる",
                rule: "v5",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死なせられる",
                rule: "v5",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死ね",
                rule: "v5",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死なない",
                rule: "v5",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死にません",
                rule: "v5",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死ななかった",
                rule: "v5",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死にませんでした",
                rule: "v5",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死ななくて",
                rule: "v5",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死ねない",
                rule: "v5",
                reasons: vec!["potential", "negative"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死なれない",
                rule: "v5",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死なせない",
                rule: "v5",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死なせられない",
                rule: "v5",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死ぬな",
                rule: "v5",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死ねば",
                rule: "v5",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んじゃう",
                rule: "v5",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んじまう",
                rule: "v5",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死になさい",
                rule: "v5",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死にそう",
                rule: "v5",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死にすぎる",
                rule: "v5",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死にたい",
                rule: "v5",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んだら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んだり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死なず",
                rule: "v5",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死なぬ",
                rule: "v5",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死に",
                rule: "v5",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死にましょう",
                rule: "v5",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死のう",
                rule: "v5",
                reasons: vec!["volitional"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死なされる",
                rule: "v5",
                reasons: vec!["causative passive"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んどく",
                rule: "v5",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んでいる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んでおる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んでる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "死ぬ",
                source: "死んでしまう",
                rule: "v5",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ぶ",
                rule: "v5",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊びます",
                rule: "v5",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んだ",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊びました",
                rule: "v5",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んで",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊べる",
                rule: "v5",
                reasons: vec!["potential"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばれる",
                rule: "v5",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばせる",
                rule: "v5",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばせられる",
                rule: "v5",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊べ",
                rule: "v5",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばない",
                rule: "v5",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊びません",
                rule: "v5",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばなかった",
                rule: "v5",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊びませんでした",
                rule: "v5",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばなくて",
                rule: "v5",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊べない",
                rule: "v5",
                reasons: vec!["potential", "negative"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばれない",
                rule: "v5",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばせない",
                rule: "v5",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばせられない",
                rule: "v5",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ぶな",
                rule: "v5",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊べば",
                rule: "v5",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んじゃう",
                rule: "v5",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んじまう",
                rule: "v5",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊びなさい",
                rule: "v5",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊びそう",
                rule: "v5",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊びすぎる",
                rule: "v5",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊びたい",
                rule: "v5",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んだら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んだり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばず",
                rule: "v5",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばぬ",
                rule: "v5",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊び",
                rule: "v5",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊びましょう",
                rule: "v5",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ぼう",
                rule: "v5",
                reasons: vec!["volitional"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊ばされる",
                rule: "v5",
                reasons: vec!["causative passive"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んどく",
                rule: "v5",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んでいる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んでおる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んでる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "遊ぶ",
                source: "遊んでしまう",
                rule: "v5",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲む",
                rule: "v5",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲みます",
                rule: "v5",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んだ",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲みました",
                rule: "v5",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んで",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲める",
                rule: "v5",
                reasons: vec!["potential"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲まれる",
                rule: "v5",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲ませる",
                rule: "v5",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲ませられる",
                rule: "v5",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲め",
                rule: "v5",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲まない",
                rule: "v5",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲みません",
                rule: "v5",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲まなかった",
                rule: "v5",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲みませんでした",
                rule: "v5",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲まなくて",
                rule: "v5",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲めない",
                rule: "v5",
                reasons: vec!["potential", "negative"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲まれない",
                rule: "v5",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲ませない",
                rule: "v5",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲ませられない",
                rule: "v5",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲むな",
                rule: "v5",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲めば",
                rule: "v5",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んじゃう",
                rule: "v5",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んじまう",
                rule: "v5",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲みなさい",
                rule: "v5",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲みそう",
                rule: "v5",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲みすぎる",
                rule: "v5",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲みたい",
                rule: "v5",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んだら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んだり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲まず",
                rule: "v5",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲まぬ",
                rule: "v5",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲み",
                rule: "v5",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲みましょう",
                rule: "v5",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲もう",
                rule: "v5",
                reasons: vec!["volitional"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲まされる",
                rule: "v5",
                reasons: vec!["causative passive"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んどく",
                rule: "v5",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んでいる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んでおる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んでる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "飲む",
                source: "飲んでしまう",
                rule: "v5",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作る",
                rule: "v5",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作ります",
                rule: "v5",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作った",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作りました",
                rule: "v5",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作って",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作れる",
                rule: "v5",
                reasons: vec!["potential"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作られる",
                rule: "v5",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作らせる",
                rule: "v5",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作らせられる",
                rule: "v5",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作れ",
                rule: "v5",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作らない",
                rule: "v5",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作りません",
                rule: "v5",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作らなかった",
                rule: "v5",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作りませんでした",
                rule: "v5",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作らなくて",
                rule: "v5",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作れない",
                rule: "v5",
                reasons: vec!["potential", "negative"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作られない",
                rule: "v5",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作らせない",
                rule: "v5",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作らせられない",
                rule: "v5",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作るな",
                rule: "v5",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作れば",
                rule: "v5",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作っちゃう",
                rule: "v5",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作っちまう",
                rule: "v5",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作りなさい",
                rule: "v5",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作りそう",
                rule: "v5",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作りすぎる",
                rule: "v5",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作りたい",
                rule: "v5",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作ったら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作ったり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作らず",
                rule: "v5",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作らぬ",
                rule: "v5",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作り",
                rule: "v5",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作りましょう",
                rule: "v5",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作ろう",
                rule: "v5",
                reasons: vec!["volitional"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作らされる",
                rule: "v5",
                reasons: vec!["causative passive"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作っとく",
                rule: "v5",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作っている",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作っておる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作ってる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作っとる",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "作る",
                source: "作ってしまう",
                rule: "v5",
                reasons: vec!["-te", "-shimau"],
            },
            // Irregular verbs
            DeinflectValidTest {
                term: "為る",
                source: "為る",
                rule: "vs",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ます",
                rule: "vs",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為た",
                rule: "vs",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ました",
                rule: "vs",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為て",
                rule: "vs",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為られる",
                rule: "vs",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為れる",
                rule: "vs",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為せる",
                rule: "vs",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為させる",
                rule: "vs",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為せられる",
                rule: "vs",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為させられる",
                rule: "vs",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ろ",
                rule: "vs",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ない",
                rule: "vs",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ません",
                rule: "vs",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為なかった",
                rule: "vs",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ませんでした",
                rule: "vs",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為なくて",
                rule: "vs",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為られない",
                rule: "vs",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為れない",
                rule: "vs",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為せない",
                rule: "vs",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為させない",
                rule: "vs",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為せられない",
                rule: "vs",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為させられない",
                rule: "vs",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為るな",
                rule: "vs",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為れば",
                rule: "vs",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ちゃう",
                rule: "vs",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ちまう",
                rule: "vs",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為なさい",
                rule: "vs",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為そう",
                rule: "vs",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為すぎる",
                rule: "vs",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為たい",
                rule: "vs",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為たら",
                rule: "vs",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為たり",
                rule: "vs",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ず",
                rule: "vs",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ぬ",
                rule: "vs",
                reasons: vec!["-nu"],
            },
            // vec!["masu stem"]
            DeinflectValidTest {
                term: "為る",
                source: "為ましょう",
                rule: "vs",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為よう",
                rule: "vs",
                reasons: vec!["volitional"],
            },
            // vec!["causative passive"]
            DeinflectValidTest {
                term: "為る",
                source: "為とく",
                rule: "vs",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ている",
                rule: "vs",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為ておる",
                rule: "vs",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為てる",
                rule: "vs",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為とる",
                rule: "vs",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "為る",
                source: "為てしまう",
                rule: "vs",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "する",
                source: "する",
                rule: "vs",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "する",
                source: "します",
                rule: "vs",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "する",
                source: "した",
                rule: "vs",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しました",
                rule: "vs",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "する",
                source: "して",
                rule: "vs",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "する",
                source: "せられる",
                rule: "vs",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "する",
                source: "される",
                rule: "vs",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "する",
                source: "させる",
                rule: "vs",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "せさせる",
                rule: "vs",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "させられる",
                rule: "vs",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "する",
                source: "せさせられる",
                rule: "vs",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しろ",
                rule: "vs",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しない",
                rule: "vs",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しません",
                rule: "vs",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しなかった",
                rule: "vs",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しませんでした",
                rule: "vs",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しなくて",
                rule: "vs",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "する",
                source: "せられない",
                rule: "vs",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "されない",
                rule: "vs",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "させない",
                rule: "vs",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "せさせない",
                rule: "vs",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "させられない",
                rule: "vs",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "せさせられない",
                rule: "vs",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "するな",
                rule: "vs",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "する",
                source: "すれば",
                rule: "vs",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しちゃう",
                rule: "vs",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しちまう",
                rule: "vs",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しなさい",
                rule: "vs",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しそう",
                rule: "vs",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しすぎる",
                rule: "vs",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "する",
                source: "したい",
                rule: "vs",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "する",
                source: "したら",
                rule: "vs",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "する",
                source: "したり",
                rule: "vs",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "する",
                source: "せず",
                rule: "vs",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "する",
                source: "せぬ",
                rule: "vs",
                reasons: vec!["-nu"],
            },
            // vec!["masu stem"]
            DeinflectValidTest {
                term: "する",
                source: "しましょう",
                rule: "vs",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しよう",
                rule: "vs",
                reasons: vec!["volitional"],
            },
            // vec!["causative passive"]
            DeinflectValidTest {
                term: "する",
                source: "しとく",
                rule: "vs",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "する",
                source: "している",
                rule: "vs",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しておる",
                rule: "vs",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "する",
                source: "してる",
                rule: "vs",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "する",
                source: "しとる",
                rule: "vs",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "する",
                source: "してしまう",
                rule: "vs",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来る",
                rule: "vk",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ます",
                rule: "vk",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来た",
                rule: "vk",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ました",
                rule: "vk",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来て",
                rule: "vk",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来られる",
                rule: "vk",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来られる",
                rule: "vk",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来させる",
                rule: "vk",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来させられる",
                rule: "vk",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来い",
                rule: "vk",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ない",
                rule: "vk",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ません",
                rule: "vk",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来なかった",
                rule: "vk",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ませんでした",
                rule: "vk",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来なくて",
                rule: "vk",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来られない",
                rule: "vk",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来られない",
                rule: "vk",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来させない",
                rule: "vk",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来させられない",
                rule: "vk",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来るな",
                rule: "vk",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来れば",
                rule: "vk",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ちゃう",
                rule: "vk",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ちまう",
                rule: "vk",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来なさい",
                rule: "vk",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来そう",
                rule: "vk",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来すぎる",
                rule: "vk",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来たい",
                rule: "vk",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来たら",
                rule: "vk",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来たり",
                rule: "vk",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ず",
                rule: "vk",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ぬ",
                rule: "vk",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来",
                rule: "vk",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ましょう",
                rule: "vk",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来よう",
                rule: "vk",
                reasons: vec!["volitional"],
            },
            // vec!["causative passive"]
            DeinflectValidTest {
                term: "来る",
                source: "来とく",
                rule: "vk",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ている",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来ておる",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来てる",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来とる",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "来る",
                source: "来てしまう",
                rule: "vk",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來る",
                rule: "vk",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ます",
                rule: "vk",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來た",
                rule: "vk",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ました",
                rule: "vk",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來て",
                rule: "vk",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來られる",
                rule: "vk",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來られる",
                rule: "vk",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來させる",
                rule: "vk",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來させられる",
                rule: "vk",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來い",
                rule: "vk",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ない",
                rule: "vk",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ません",
                rule: "vk",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來なかった",
                rule: "vk",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ませんでした",
                rule: "vk",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來なくて",
                rule: "vk",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來られない",
                rule: "vk",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來られない",
                rule: "vk",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來させない",
                rule: "vk",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來させられない",
                rule: "vk",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來るな",
                rule: "vk",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來れば",
                rule: "vk",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ちゃう",
                rule: "vk",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ちまう",
                rule: "vk",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來なさい",
                rule: "vk",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來そう",
                rule: "vk",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來すぎる",
                rule: "vk",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來たい",
                rule: "vk",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來たら",
                rule: "vk",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來たり",
                rule: "vk",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ず",
                rule: "vk",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ぬ",
                rule: "vk",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來",
                rule: "vk",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ましょう",
                rule: "vk",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來よう",
                rule: "vk",
                reasons: vec!["volitional"],
            },
            // vec!["causative passive"]
            DeinflectValidTest {
                term: "來る",
                source: "來とく",
                rule: "vk",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ている",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來ておる",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來てる",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來とる",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "來る",
                source: "來てしまう",
                rule: "vk",
                reasons: vec!["-te", "-shimau"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "くる",
                rule: "vk",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きます",
                rule: "vk",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きた",
                rule: "vk",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きました",
                rule: "vk",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きて",
                rule: "vk",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こられる",
                rule: "vk",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こられる",
                rule: "vk",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こさせる",
                rule: "vk",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こさせられる",
                rule: "vk",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こい",
                rule: "vk",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こない",
                rule: "vk",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きません",
                rule: "vk",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こなかった",
                rule: "vk",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きませんでした",
                rule: "vk",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こなくて",
                rule: "vk",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こられない",
                rule: "vk",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こられない",
                rule: "vk",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こさせない",
                rule: "vk",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こさせられない",
                rule: "vk",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "くるな",
                rule: "vk",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "くれば",
                rule: "vk",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きちゃう",
                rule: "vk",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きちまう",
                rule: "vk",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きなさい",
                rule: "vk",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きそう",
                rule: "vk",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きすぎる",
                rule: "vk",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きたい",
                rule: "vk",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きたら",
                rule: "vk",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きたり",
                rule: "vk",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こず",
                rule: "vk",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こぬ",
                rule: "vk",
                reasons: vec!["-nu"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "き",
                rule: "vk",
                reasons: vec!["masu stem"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きましょう",
                rule: "vk",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "こよう",
                rule: "vk",
                reasons: vec!["volitional"],
            },
            // vec!["causative passive"]
            DeinflectValidTest {
                term: "くる",
                source: "きとく",
                rule: "vk",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きている",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きておる",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きてる",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きとる",
                rule: "vk",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "くる",
                source: "きてしまう",
                rule: "vk",
                reasons: vec!["-te", "-shimau"],
            },
            // Zuru verbs
            DeinflectValidTest {
                term: "論ずる",
                source: "論ずる",
                rule: "vz",
                reasons: vec![],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じます",
                rule: "vz",
                reasons: vec!["polite"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じた",
                rule: "vz",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じました",
                rule: "vz",
                reasons: vec!["polite past"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じて",
                rule: "vz",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ぜられる",
                rule: "vz",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ざれる",
                rule: "vz",
                reasons: vec!["potential or passive"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じされる",
                rule: "vz",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ぜされる",
                rule: "vz",
                reasons: vec!["passive"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じさせる",
                rule: "vz",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ぜさせる",
                rule: "vz",
                reasons: vec!["causative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じさせられる",
                rule: "vz",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ぜさせられる",
                rule: "vz",
                reasons: vec!["causative", "potential or passive"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じろ",
                rule: "vz",
                reasons: vec!["imperative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じない",
                rule: "vz",
                reasons: vec!["negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じません",
                rule: "vz",
                reasons: vec!["polite negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じなかった",
                rule: "vz",
                reasons: vec!["negative", "past"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じませんでした",
                rule: "vz",
                reasons: vec!["polite past negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じなくて",
                rule: "vz",
                reasons: vec!["negative", "-te"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ぜられない",
                rule: "vz",
                reasons: vec!["potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じされない",
                rule: "vz",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ぜされない",
                rule: "vz",
                reasons: vec!["passive", "negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じさせない",
                rule: "vz",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ぜさせない",
                rule: "vz",
                reasons: vec!["causative", "negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じさせられない",
                rule: "vz",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ぜさせられない",
                rule: "vz",
                reasons: vec!["causative", "potential or passive", "negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ずるな",
                rule: "vz",
                reasons: vec!["imperative negative"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ずれば",
                rule: "vz",
                reasons: vec!["-ba"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じちゃう",
                rule: "vz",
                reasons: vec!["-chau"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じちまう",
                rule: "vz",
                reasons: vec!["-chimau"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じなさい",
                rule: "vz",
                reasons: vec!["-nasai"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じそう",
                rule: "vz",
                reasons: vec!["-sou"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じすぎる",
                rule: "vz",
                reasons: vec!["-sugiru"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じたい",
                rule: "vz",
                reasons: vec!["-tai"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じたら",
                rule: "vz",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じたり",
                rule: "vz",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ぜず",
                rule: "vz",
                reasons: vec!["-zu"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論ぜぬ",
                rule: "vz",
                reasons: vec!["-nu"],
            },
            // vec!["masu stem"]
            DeinflectValidTest {
                term: "論ずる",
                source: "論じましょう",
                rule: "vz",
                reasons: vec!["polite volitional"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じよう",
                rule: "vz",
                reasons: vec!["volitional"],
            },
            // vec!["causative passive"]
            DeinflectValidTest {
                term: "論ずる",
                source: "論じとく",
                rule: "vz",
                reasons: vec!["-toku"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じている",
                rule: "vz",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じておる",
                rule: "vz",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じてる",
                rule: "vz",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じとる",
                rule: "vz",
                reasons: vec!["-te", "progressive or perfect"],
            },
            DeinflectValidTest {
                term: "論ずる",
                source: "論じてしまう",
                rule: "vz",
                reasons: vec!["-te", "-shimau"],
            },
            // Uncommon irregular verbs
            DeinflectValidTest {
                term: "のたまう",
                source: "のたもうて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "のたまう",
                source: "のたもうた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "のたまう",
                source: "のたもうたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "のたまう",
                source: "のたもうたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "おう",
                source: "おうて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "こう",
                source: "こうて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "そう",
                source: "そうて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "とう",
                source: "とうて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "請う",
                source: "請うて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "乞う",
                source: "乞うて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "恋う",
                source: "恋うて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "問う",
                source: "問うて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "負う",
                source: "負うて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "沿う",
                source: "沿うて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "添う",
                source: "添うて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "副う",
                source: "副うて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "厭う",
                source: "厭うて",
                rule: "v5",
                reasons: vec!["-te"],
            },
            DeinflectValidTest {
                term: "おう",
                source: "おうた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "こう",
                source: "こうた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "そう",
                source: "そうた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "とう",
                source: "とうた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "請う",
                source: "請うた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "乞う",
                source: "乞うた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "恋う",
                source: "恋うた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "問う",
                source: "問うた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "負う",
                source: "負うた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "沿う",
                source: "沿うた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "添う",
                source: "添うた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "副う",
                source: "副うた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "厭う",
                source: "厭うた",
                rule: "v5",
                reasons: vec!["past"],
            },
            DeinflectValidTest {
                term: "おう",
                source: "おうたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "こう",
                source: "こうたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "そう",
                source: "そうたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "とう",
                source: "とうたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "請う",
                source: "請うたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "乞う",
                source: "乞うたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "恋う",
                source: "恋うたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "問う",
                source: "問うたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "負う",
                source: "負うたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "沿う",
                source: "沿うたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "添う",
                source: "添うたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "副う",
                source: "副うたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "厭う",
                source: "厭うたら",
                rule: "v5",
                reasons: vec!["-tara"],
            },
            DeinflectValidTest {
                term: "おう",
                source: "おうたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "こう",
                source: "こうたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "そう",
                source: "そうたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "とう",
                source: "とうたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "請う",
                source: "請うたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "乞う",
                source: "乞うたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "恋う",
                source: "恋うたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "問う",
                source: "問うたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "負う",
                source: "負うたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "沿う",
                source: "沿うたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "添う",
                source: "添うたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "副う",
                source: "副うたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            DeinflectValidTest {
                term: "厭う",
                source: "厭うたり",
                rule: "v5",
                reasons: vec!["-tari"],
            },
            // Combinations
            DeinflectValidTest {
                term: "抱き抱える",
                source: "抱き抱えていなければ",
                rule: "v1",
                reasons: vec!["-te", "progressive or perfect", "negative", "-ba"],
            },
            DeinflectValidTest {
                term: "抱きかかえる",
                source: "抱きかかえていなければ",
                rule: "v1",
                reasons: vec!["-te", "progressive or perfect", "negative", "-ba"],
            },
            DeinflectValidTest {
                term: "打ち込む",
                source: "打ち込んでいませんでした",
                rule: "v5",
                reasons: vec!["-te", "progressive or perfect", "polite past negative"],
            },
            DeinflectValidTest {
                term: "食べる",
                source: "食べさせられたくなかった",
                rule: "v1",
                reasons: vec![
                    "causative",
                    "potential or passive",
                    "-tai",
                    "negative",
                    "past",
                ],
            },
            // separate group

            // -e
            DeinflectValidTest {
                term: "すごい",
                source: "すげえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "やばい",
                source: "やべえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "うるさい",
                source: "うるせえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "ひどい",
                source: "ひでえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "ない",
                source: "ねえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "できる",
                source: "できねえ",
                rule: "v1",
                reasons: vec!["negative", "-e"],
            },
            DeinflectValidTest {
                term: "しんじる",
                source: "しんじねえ",
                rule: "v1",
                reasons: vec!["negative", "-e"],
            },
            DeinflectValidTest {
                term: "さむい",
                source: "さめえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "さむい",
                source: "さみい",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "あつい",
                source: "あちぇえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "あつい",
                source: "あちい",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "やすい",
                source: "やせえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "たかい",
                source: "たけえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "かわいい",
                source: "かわええ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "つよい",
                source: "ついぇえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "こわい",
                source: "こうぇえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "みじかい",
                source: "みじけえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "ながい",
                source: "なげえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "くさい",
                source: "くせえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "うまい",
                source: "うめえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "でかい",
                source: "でけえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "まずい",
                source: "まっぜえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "ちっちゃい",
                source: "ちっちぇえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "あかい",
                source: "あけえ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "こわい",
                source: "こええ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
            DeinflectValidTest {
                term: "つよい",
                source: "つええ",
                rule: "adj-i",
                reasons: vec!["-e"],
            },
        ];

        let reasons = inflection_reasons();

        for case in cases {
            let rules = Rules::from(vec![Rule::try_from(case.rule).unwrap()]);
            let deinflections = case.source.string_deinflections(&reasons);

            println!("Deinflections: {deinflections:#?}");

            let mut matches = deinflections
                .into_iter()
                .filter(|d| d.term == case.term)
                .filter(|d| d.rules.0.is_empty() || d.rules.0.contains(rules.0))
                .filter(|d| d.reasons == case.reasons);

            let term = case.term;
            let source = case.source;
            let rule = case.rule;
            let reasons = &case.reasons;

            assert!(
                matches.next().is_some(),
                "{source} does not have term candidate {term} with {rule} and {reasons:?}"
            );
        }
    }

    #[test]
    fn invalid_cases() {
        let cases = vec![
            DeinflectInvalidTest {
                term: "する",
                source: "すます",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すた",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すました",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すて",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すれる",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すせる",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すせられる",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すろ",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すない",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すません",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すなかった",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すませんでした",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すなくて",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すれない",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すせない",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "する",
                source: "すせられない",
                rule: "vs",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くます",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くた",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くました",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くて",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くられる",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くられる",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くさせる",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くさせられる",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くい",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くない",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くません",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くなかった",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くませんでした",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くなくて",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くられない",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くられない",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くさせない",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "くる",
                source: "くさせられない",
                rule: "vk",
            },
            DeinflectInvalidTest {
                term: "かわいい",
                source: "かわいげ",
                rule: "adj-i",
                // reasons: vec!["-ge"],
            },
            DeinflectInvalidTest {
                term: "可愛い",
                source: "かわいげ",
                rule: "adj-i",
                // reasons: vec!["-ge"],
            },
        ];

        let reasons = inflection_reasons();

        for case in cases {
            let rules = Rules::from(vec![Rule::try_from(case.rule).unwrap()]);
            let deinflections = case.source.string_deinflections(&reasons);

            let mut matches = deinflections
                .into_iter()
                .filter(|d| d.term == case.term)
                .filter(|d| d.rules.0.is_empty() || d.rules.0.contains(rules.0));

            let term = case.term;
            let source = case.source;
            let rule = case.rule;

            assert!(
                matches.next().is_none(),
                "{source} has term candidate {term} with {rule}"
            );
        }
    }
}
