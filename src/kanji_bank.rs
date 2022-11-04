use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct KanjiTuple(
    String,
    String,
    String,
    String,
    Vec<String>,
    HashMap<String, String>,
);

#[derive(Debug)]
pub struct Kanji {
    pub character: String,
    pub onyomi: String,
    pub kunyomi: String,
    pub tags: String,
    pub meanings: Vec<String>,
    pub stats: HashMap<String, String>,
}

impl From<KanjiTuple> for Kanji {
    fn from(t: KanjiTuple) -> Self {
        Self {
            character: t.0,
            onyomi: t.1,
            kunyomi: t.2,
            tags: t.3,
            meanings: t.4,
            stats: t.5,
        }
    }
}
