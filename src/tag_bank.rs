use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TagTuple(String, String, f32, String, f32);

#[derive(Debug)]
pub struct Tag {
    pub name: String,
    pub category: String,
    pub sorting_key: f32,
    pub notes: String,
    pub popularity: f32,
}

impl From<TagTuple> for Tag {
    fn from(t: TagTuple) -> Self {
        Tag {
            name: t.0,
            category: t.1,
            sorting_key: t.2,
            notes: t.3,
            popularity: t.4,
        }
    }
}
