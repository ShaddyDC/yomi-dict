pub mod db;
pub mod deinflect;
mod dict_item;
mod error;
mod kanji_bank;
mod tag_bank;
mod terms_bank;
pub mod translator;

use std::{
    io::{Read, Seek},
    path::Path,
};

pub use error::YomiDictError;
use kanji_bank::Kanji;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use terms_bank::Term;

use crate::{
    kanji_bank::KanjiTuple,
    tag_bank::{Tag, TagTuple},
    terms_bank::TermTuple,
};

#[derive(Deserialize_repr, Serialize_repr, Debug)]
#[repr(u8)]
pub enum Version {
    // V1 = 1, // We do not support version 1
    V2 = 2,
    V3 = 3,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum FrequencyMode {
    OccurenceBased,
    RankBased,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    pub title: String,
    pub revision: String,
    pub sequenced: Option<bool>,
    #[serde(alias = "version")]
    pub format: Version,
    pub author: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub attribution: Option<String>,
    pub frequency_mode: Option<FrequencyMode>,
}

#[derive(Serialize, Deserialize)]
pub struct Dict {
    pub index: Index,
    pub terms: Vec<Term>,
    pub kanji: Vec<Kanji>,
    pub tags: Vec<Tag>,
}

impl Dict {
    /// # Errors
    ///
    /// Will return `Err` if dictionary couldn't be read.
    pub fn new<R: Read + Seek>(reader: R) -> Result<Self, YomiDictError> {
        let mut archive = zip::ZipArchive::new(reader)?;

        let index_json = archive.by_name("index.json")?;
        let index: Index = serde_json::from_reader(index_json)?;

        let mut terms: Vec<Term> = vec![];
        let mut kanji: Vec<Kanji> = vec![];
        let mut tags: Vec<Tag> = vec![];

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;

            match file.enclosed_name() {
                Some(path) if path == Path::new("index.json") => continue,

                Some(path) if path.to_string_lossy().starts_with("term_bank_") => {
                    let data: Vec<TermTuple> = serde_json::from_reader(file)?;
                    terms.extend(data.into_iter().map(Term::from));
                }

                Some(path) if path.to_string_lossy().starts_with("kanji_bank_") => {
                    let data: Vec<KanjiTuple> = serde_json::from_reader(file)?;
                    kanji.extend(data.into_iter().map(Kanji::from));
                }

                Some(path) if path.to_string_lossy().starts_with("tag_bank_") => {
                    let data: Vec<TagTuple> = serde_json::from_reader(file)?;
                    tags.extend(data.into_iter().map(Tag::from));
                }
                _ => continue,
            };
        }

        Ok(Self {
            index,
            terms,
            kanji,
            tags,
        })
    }
}
