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

#[derive(Debug)]
pub struct Term {
    pub expression: String,
    pub reading: String,
    pub definition_tags: Option<String>, // TODO Make vector
    pub rules: Rules,
    pub score: f32,
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
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| Rule::try_from(s).map_err(D::Error::custom))
            .collect::<Result<Vec<Rule>, _>>()?,
    );
    Ok(Rules(r))
}

impl From<TermTuple> for Term {
    fn from(t: TermTuple) -> Self {
        Term {
            reading: if t.1.is_empty() { t.0.clone() } else { t.1 },
            expression: t.0,
            definition_tags: t.2,
            rules: t.3,
            score: t.4,
            glossary: t.5,
            sequence: t.6,
            term_tags: t.7,
        }
    }
}
