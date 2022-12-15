use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::dict_item::DictItem;

#[derive(Deserialize, Debug)]
pub struct KanjiTuple(
    String,
    String,
    String,
    String,
    Vec<String>,
    HashMap<String, String>,
);

#[derive(Debug, Serialize, Deserialize)]
pub struct Kanji {
    pub character: String,
    pub onyomi: String,
    pub kunyomi: String,
    pub tags: String,
    pub meanings: Vec<String>,
    pub stats: HashMap<String, String>,
    pub dict_id: u8,
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
            dict_id: 0,
        }
    }
}

impl DictItem for Kanji {
    fn set_dict_id(&mut self, dict_id: u8) {
        self.dict_id = dict_id;
    }
}
