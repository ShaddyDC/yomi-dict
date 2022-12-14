use serde::{Deserialize, Serialize};

use crate::dict_item::DictItem;

#[derive(Deserialize, Debug)]
pub struct TagTuple(String, String, f32, String, f32);

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub category: String,
    pub order: f32,
    pub notes: String,
    pub score: f32,
    pub dict_id: u8,
}

impl From<TagTuple> for Tag {
    fn from(t: TagTuple) -> Self {
        Self {
            name: t.0,
            category: t.1,
            order: t.2,
            notes: t.3,
            score: t.4,
            dict_id: 0,
        }
    }
}

impl DictItem for Tag {
    fn set_dict_id(&mut self, dict_id: u8) {
        self.dict_id = dict_id;
    }
}
