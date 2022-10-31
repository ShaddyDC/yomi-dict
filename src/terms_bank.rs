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
    pub text: String,
    pub reading: String,
    pub definition_tags: Option<String>, // TODO Make vector
    pub delinflection: String,           // TODO Make vector
    pub popularity: f32,
    pub definitions: Vec<String>,
    pub sequence: u32,
    pub term_tags: String, // TODO Make vector
}

impl From<TermTuple> for Term {
    fn from(t: TermTuple) -> Self {
        Term {
            text: t.0,
            reading: t.1,
            definition_tags: t.2,
            delinflection: t.3,
            popularity: t.4,
            definitions: t.5,
            sequence: t.6,
            term_tags: t.7,
        }
    }
}
