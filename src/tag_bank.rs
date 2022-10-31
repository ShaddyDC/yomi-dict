use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TagTuple(String, String, f32, String, f32);

#[derive(Debug)]
pub struct Tag {
    pub name: String,
    pub category: String,
    pub order: f32,
    pub notes: String,
    pub score: f32,
}

impl From<TagTuple> for Tag {
    fn from(t: TagTuple) -> Self {
        Tag {
            name: t.0,
            category: t.1,
            order: t.2,
            notes: t.3,
            score: t.4,
        }
    }
}