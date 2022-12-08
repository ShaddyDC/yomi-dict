#![allow(clippy::future_not_send)]

use futures::future::join_all;
use itertools::Itertools;
use rexie::{Index, KeyRange, ObjectStore, Rexie};
use serde::Deserialize;

use crate::{kanji_bank::Kanji, tag_bank::Tag, terms_bank::Term, Dict, YomiDictError};

pub struct DB {
    rexie: Rexie,
}

#[derive(Deserialize)]
pub struct IdObject {
    id: u32,
}

impl DB {
    pub async fn new(name: &str) -> Result<Self, YomiDictError> {
        let rexie = Rexie::builder(name)
            .version(1)
            .add_object_store(
                ObjectStore::new("dictionaries")
                    .key_path("id")
                    .auto_increment(true)
                    .add_index(Index::new("title", "title")),
            )
            .add_object_store(ObjectStore::new("tags").key_path("id").auto_increment(true))
            .add_object_store(
                ObjectStore::new("terms")
                    .key_path("id")
                    .auto_increment(true)
                    .add_index(Index::new("expression", "expression"))
                    .add_index(Index::new("reading", "reading")),
            )
            .add_object_store(
                ObjectStore::new("kanji")
                    .key_path("id")
                    .auto_increment(true),
            )
            .build()
            .await
            .map_err(YomiDictError::StorageError)?;

        Ok(Self { rexie })
    }

    pub async fn add_dict(&self, dict: Dict) -> Result<(), YomiDictError> {
        // TODO Fail transaction on failure
        let transaction = self
            .rexie
            .transaction(
                &["dictionaries", "tags", "terms", "kanji"],
                rexie::TransactionMode::ReadWrite,
            )
            .map_err(YomiDictError::StorageError)?;

        let dictionaries = transaction
            .store("dictionaries")
            .map_err(YomiDictError::StorageError)?;

        let dict_index = dictionaries
            .index("title")
            .map_err(YomiDictError::StorageError)?;

        if !dict_index
            .get(
                &serde_wasm_bindgen::to_value(&dict.index.title)
                    .map_err(YomiDictError::JsobjError)?,
            )
            .await
            .map_err(YomiDictError::StorageError)?
            .is_undefined()
        {
            return Ok(()); // TODO duplicate error?
        }

        let dict_id = dictionaries
            .put(
                &serde_wasm_bindgen::to_value(&dict.index).map_err(YomiDictError::JsobjError)?,
                None,
            )
            .await
            .map_err(YomiDictError::StorageError)?;

        let dict_id: u8 =
            serde_wasm_bindgen::from_value(dict_id).map_err(YomiDictError::JsobjError)?;

        let tags = transaction
            .store("tags")
            .map_err(YomiDictError::StorageError)?;
        for tag in dict.tags.into_iter().map(|t| Tag { dict_id, ..t }) {
            tags.put(
                &serde_wasm_bindgen::to_value(&tag).map_err(YomiDictError::JsobjError)?,
                None,
            )
            .await
            .map_err(YomiDictError::StorageError)?;
        }

        let terms = transaction
            .store("terms")
            .map_err(YomiDictError::StorageError)?;
        for term in dict.terms.into_iter().map(|t| Term { dict_id, ..t }) {
            terms
                .put(
                    &serde_wasm_bindgen::to_value(&term).map_err(YomiDictError::JsobjError)?,
                    None,
                )
                .await
                .map_err(YomiDictError::StorageError)?;
        }

        let kanjis = transaction
            .store("kanji")
            .map_err(YomiDictError::StorageError)?;
        for kanji in dict.kanji.into_iter().map(|k| Kanji { dict_id, ..k }) {
            kanjis
                .put(
                    &serde_wasm_bindgen::to_value(&kanji).map_err(YomiDictError::JsobjError)?,
                    None,
                )
                .await
                .map_err(YomiDictError::StorageError)?;
        }

        transaction
            .commit()
            .await
            .map_err(YomiDictError::StorageError)?;

        Ok(())
    }

    pub async fn get_terms(
        &self,
        term_list: impl IntoIterator<Item = &str>,
    ) -> Result<Vec<Term>, YomiDictError> {
        let transaction = self
            .rexie
            .transaction(&["terms"], rexie::TransactionMode::ReadOnly)
            .map_err(YomiDictError::StorageError)?;

        let terms = transaction
            .store("terms")
            .map_err(YomiDictError::StorageError)?;

        let indices = vec![
            terms
                .index("expression")
                .map_err(YomiDictError::StorageError)?,
            terms
                .index("reading")
                .map_err(YomiDictError::StorageError)?,
        ];

        let term_list = term_list
            .into_iter()
            .map(|s| {
                serde_wasm_bindgen::to_value(s)
                    .map_err(YomiDictError::JsobjError)
                    .and_then(|s| KeyRange::only(&s).map_err(YomiDictError::StorageError))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let queries = join_all(
            term_list
                .iter()
                .flat_map(|s| indices.iter().map(|i| i.get_all(Some(s), None, None, None))),
        )
        .await;

        let terms = queries
            .into_iter()
            .filter_map(std::result::Result::ok)
            .flatten()
            .map(|(_, obj)| obj)
            .unique_by(|jobj| {
                // TODO error handling
                serde_wasm_bindgen::from_value::<IdObject>(jobj.clone())
                    .map(|obj| obj.id)
                    .unwrap_or(0)
            })
            .map(|jobj| serde_wasm_bindgen::from_value(jobj).map_err(YomiDictError::JsobjError))
            .collect::<Result<Vec<_>, _>>()?;

        transaction
            .done()
            .await
            .map_err(YomiDictError::StorageError)?;

        Ok(terms)
    }
}
