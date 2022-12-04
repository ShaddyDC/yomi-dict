pub mod db;
pub mod deinflect;
mod kanji_bank;
mod tag_bank;
mod terms_bank;
pub mod translator;

use std::{
    io::{Read, Seek},
    path::Path,
};

use kanji_bank::Kanji;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use terms_bank::Term;
use thiserror::Error;
use zip::result::ZipError;

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

#[derive(Error, Debug)]
pub enum YomiDictError {
    #[error("An IO error occured: `{0}`")]
    Io(std::io::Error),
    #[error("Archive is invalid: `{0}`")]
    InvalidArchive(&'static str),
    #[error("Archive is not supported: `{0}`")]
    UnsupportedArchive(&'static str),
    #[error("File index.json not found in archive")]
    IndexNotFound,
    #[error("Error parsing Json: `{0}`")]
    JsonError(serde_json::Error),
    #[error("Error parsing JSObject: `{0}`")]
    JsobjError(serde_wasm_bindgen::Error),
    #[error("Error with storage: `{0}`")]
    StorageError(rexie::Error),
}

impl Dict {
    /// # Errors
    ///
    /// Will return `Err` if dictionary couldn't be read.
    pub fn new<R: Read + Seek>(reader: R) -> Result<Self, YomiDictError> {
        let mut archive = zip::ZipArchive::new(reader).map_err(|err| match err {
            ZipError::InvalidArchive(s) => YomiDictError::InvalidArchive(s),
            ZipError::UnsupportedArchive(s) => YomiDictError::UnsupportedArchive(s),
            ZipError::Io(e) => YomiDictError::Io(e),
            ZipError::FileNotFound => YomiDictError::UnsupportedArchive("Unknown error occured"),
        })?;

        let index_json = archive
            .by_name("index.json")
            .map_err(|_| YomiDictError::IndexNotFound)?;
        let index: Index = serde_json::from_reader(index_json).map_err(YomiDictError::JsonError)?;

        let mut terms: Vec<Term> = vec![];
        let mut kanji: Vec<Kanji> = vec![];
        let mut tags: Vec<Tag> = vec![];

        for i in 0..archive.len() {
            let file = archive.by_index(i).map_err(|err| match err {
                ZipError::InvalidArchive(s) => YomiDictError::InvalidArchive(s),
                ZipError::UnsupportedArchive(s) => YomiDictError::UnsupportedArchive(s),
                ZipError::Io(e) => YomiDictError::Io(e),
                ZipError::FileNotFound => {
                    YomiDictError::InvalidArchive("Could not load expected file")
                }
            })?;

            match file.enclosed_name() {
                Some(path) if path == Path::new("index.json") => continue,

                Some(path) if path.to_string_lossy().starts_with("term_bank_") => {
                    let data: Vec<TermTuple> =
                        serde_json::from_reader(file).map_err(YomiDictError::JsonError)?;
                    terms.extend(data.into_iter().map(Term::from));
                }

                Some(path) if path.to_string_lossy().starts_with("kanji_bank_") => {
                    let data: Vec<KanjiTuple> =
                        serde_json::from_reader(file).map_err(YomiDictError::JsonError)?;
                    kanji.extend(data.into_iter().map(Kanji::from));
                }

                Some(path) if path.to_string_lossy().starts_with("tag_bank_") => {
                    let data: Vec<TagTuple> =
                        serde_json::from_reader(file).map_err(YomiDictError::JsonError)?;
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
