mod kanji_bank;
mod terms_bank;

use std::{
    io::{Read, Seek},
    path::Path,
};

use kanji_bank::Kanji;
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use terms_bank::Term;
use thiserror::Error;
use zip::result::ZipError;

use crate::{kanji_bank::KanjiTuple, terms_bank::TermTuple};

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
enum Version {
    // V1 = 1, // We do not support version 1
    V2 = 2,
    V3 = 3,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
enum FrequencyMode {
    OccurenceBased,
    RankBased,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Index {
    title: String,
    revision: String,
    sequenced: Option<bool>,
    #[serde(alias = "version")]
    format: Version,
    author: Option<String>,
    url: Option<String>,
    description: Option<String>,
    attribution: Option<String>,
    frequency_mode: Option<FrequencyMode>,
}

pub struct Dict {
    index: Index,
    terms: Vec<Term>,
    kanji: Vec<Kanji>,
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
}

pub fn parse<R: Read + Seek>(reader: R) -> Result<Dict, YomiDictError> {
    // let d: TermTuple = serde_json::from_str(
    //     r#"["ヽ","",null,"",2,["ヽ\n〘unc〙\nrepetition mark in katakana.\n→一の字点"],1,""]"#,
    // )
    // .unwrap();

    let mut archive = zip::ZipArchive::new(reader).or_else(|err| match err {
        ZipError::InvalidArchive(s) => Err(YomiDictError::InvalidArchive(s)),
        ZipError::UnsupportedArchive(s) => Err(YomiDictError::UnsupportedArchive(s)),
        ZipError::Io(e) => Err(YomiDictError::Io(e)),
        _ => Err(YomiDictError::UnsupportedArchive("Unknown error occured")),
    })?;

    let index_json = archive
        .by_name("index.json")
        .or_else(|_| Err(YomiDictError::IndexNotFound))?;
    let index: Index =
        serde_json::from_reader(index_json).or_else(|err| Err(YomiDictError::JsonError(err)))?;

    let mut terms: Vec<Term> = vec![];
    let mut kanji: Vec<Kanji> = vec![];

    for i in 0..archive.len() {
        let file = archive.by_index(i).or_else(|err| match err {
            ZipError::InvalidArchive(s) => Err(YomiDictError::InvalidArchive(s)),
            ZipError::UnsupportedArchive(s) => Err(YomiDictError::UnsupportedArchive(s)),
            ZipError::Io(e) => Err(YomiDictError::Io(e)),
            ZipError::FileNotFound => Err(YomiDictError::InvalidArchive(
                "Could not load expected file",
            )),
        })?;

        println!(
            "file: {:?}",
            file.enclosed_name().unwrap_or(Path::new("invalid name"))
        );

        match file.enclosed_name() {
            Some(path) if path == Path::new("index.json") => continue,

            Some(path) if path.starts_with("term_bank_") => {
                let data: Vec<TermTuple> = serde_json::from_reader(file)
                    .or_else(|err| Err(YomiDictError::JsonError(err)))?;
                terms.extend(data.into_iter().map(|t| Term::from(t)));
            }

            Some(path) if path.starts_with("kanji_bank_") => {
                let data: Vec<KanjiTuple> = serde_json::from_reader(file)
                    .or_else(|err| Err(YomiDictError::JsonError(err)))?;
                kanji.extend(data.into_iter().map(|k| Kanji::from(k)));
            }
            _ => continue,
        };
    }

    Ok(Dict {
        index,
        terms,
        kanji,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn it_works() {
        let fname = std::path::Path::new("dict.zip");
        let file = fs::File::open(&fname).unwrap();

        parse(file).unwrap();
    }
}
