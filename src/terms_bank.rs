use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TermTuple(
    String,
    String,
    Option<String>,
    String,
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
    pub rules: String,                   // TODO Make vector
    pub score: f32,
    pub glossary: Vec<String>,
    pub sequence: u32,
    pub term_tags: String, // TODO Make vector
}

impl From<TermTuple> for Term {
    fn from(t: TermTuple) -> Self {
        Term {
            expression: t.0,
            reading: t.1,
            definition_tags: t.2,
            rules: t.3,
            score: t.4,
            glossary: t.5,
            sequence: t.6,
            term_tags: t.7,
        }
    }
}
