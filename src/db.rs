#![allow(clippy::future_not_send)]

use std::pin::Pin;

use futures::{future::join_all, Future};
use itertools::Itertools;
use rexie::{Index, KeyRange, ObjectStore, Rexie};
use serde::{Deserialize, Serialize};

use crate::{dict_item::DictItem, terms_bank::Term, Dict, YomiDictError};

pub struct DB {
    rexie: Rexie,
}

#[derive(Deserialize)]
pub struct IdObject {
    id: u32,
}

type StepFuture<'a> = dyn Future<Output = Result<usize, YomiDictError>> + 'a;

pub struct DictInsertionSteps<'a> {
    pub total_count: usize,
    pub steps: Vec<Pin<Box<StepFuture<'a>>>>,
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

    fn create_insertion_future<'a>(
        &'a self,
        store: &'a str,
        dict_id: u8,
        items: Vec<impl Serialize + DictItem + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<usize, YomiDictError>> + 'a>> {
        Box::pin(async move {
            let len = items.len();

            let transaction = self
                .rexie
                .transaction(&[store], rexie::TransactionMode::ReadWrite)
                .map_err(YomiDictError::StorageError)?;

            let store = transaction
                .store(store)
                .map_err(YomiDictError::StorageError)?;

            for mut item in items {
                item.set_dict_id(dict_id);

                store
                    .add(
                        &serde_wasm_bindgen::to_value(&item).map_err(YomiDictError::JsobjError)?,
                        None,
                    )
                    .await
                    .map_err(YomiDictError::StorageError)?;
            }

            transaction
                .commit()
                .await
                .map_err(YomiDictError::StorageError)?;

            Ok(len)
        })
    }

    pub async fn add_dict(&self, dict: Dict) -> Result<(), YomiDictError> {
        let steps = self.add_dict_stepwise(dict).await?;
        let should_total = steps.total_count;

        let total = join_all(steps.steps)
            .await
            .into_iter()
            .sum::<Result<usize, _>>()?;

        debug_assert_eq!(should_total, total);

        Ok(())
    }

    pub async fn add_dict_stepwise(
        &self,
        dict: Dict,
    ) -> Result<DictInsertionSteps<'_>, YomiDictError> {
        const TRANSACTION_SIZE: usize = 1000;

        // TODO Fail transaction on failure
        let transaction = self
            .rexie
            .transaction(&["dictionaries"], rexie::TransactionMode::ReadWrite)
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
            return Ok(DictInsertionSteps {
                total_count: 0,
                steps: vec![],
            }); // TODO duplicate error?
        }

        let dict_id = dictionaries
            .put(
                &serde_wasm_bindgen::to_value(&dict.index).map_err(YomiDictError::JsobjError)?,
                None,
            )
            .await
            .map_err(YomiDictError::StorageError)?;

        transaction
            .commit()
            .await
            .map_err(YomiDictError::StorageError)?;

        let dict_id: u8 =
            serde_wasm_bindgen::from_value(dict_id).map_err(YomiDictError::JsobjError)?;

        let total_count = dict.tags.len() + dict.terms.len() + dict.kanji.len();
        let mut steps = Vec::new();

        steps.extend(
            dict.tags
                .into_iter()
                .chunks(TRANSACTION_SIZE)
                .into_iter()
                .map(|c| (self.create_insertion_future("tags", dict_id, c.collect_vec()))),
        );

        steps.extend(
            dict.terms
                .into_iter()
                .chunks(TRANSACTION_SIZE)
                .into_iter()
                .map(|c| (self.create_insertion_future("terms", dict_id, c.collect_vec()))),
        );

        steps.extend(
            dict.kanji
                .into_iter()
                .chunks(TRANSACTION_SIZE)
                .into_iter()
                .map(|c| (self.create_insertion_future("kanji", dict_id, c.collect_vec()))),
        );

        Ok(DictInsertionSteps { total_count, steps })
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
