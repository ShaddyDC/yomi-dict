use std::cmp::Ordering;

use enumflags2::BitFlags;
use serde::{de::Error, Deserialize, Deserializer};

use crate::deinflect::{Rule, Rules};

#[derive(Deserialize, Debug)]
pub struct TermTuple(
    String,
    String,
    Option<String>,
    #[serde(deserialize_with = "from_string")] Rules,
    f32,
    Vec<String>,
    u32,
    String,
);

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Score(f32);

impl Ord for Score {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}
impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for Score {}

impl std::ops::Neg for Score {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

#[derive(Debug)]
pub struct Term {
    pub expression: String,
    pub reading: String,
    pub definition_tags: Option<String>, // TODO Make vector
    pub rules: Rules,
    pub score: Score,
    pub glossary: Vec<String>,
    pub sequence: u32,
    pub term_tags: String, // TODO Make vector
}

fn from_string<'de, D>(deserializer: D) -> Result<Rules, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    let mut r = BitFlags::<Rule>::empty();
    r.extend(
        s.split(' ')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| Rule::try_from(s).map_err(D::Error::custom))
            .collect::<Result<Vec<Rule>, _>>()?,
    );
    Ok(Rules(r))
}

impl From<TermTuple> for Term {
    fn from(t: TermTuple) -> Self {
        Self {
            reading: if t.1.is_empty() { t.0.clone() } else { t.1 },
            expression: t.0,
            definition_tags: t.2,
            rules: t.3,
            score: Score(t.4),
            glossary: t.5,
            sequence: t.6,
            term_tags: t.7,
        }
    }
}
