use itertools::Itertools;

use crate::{
    db::DB,
    deinflect::{string_deinflections, Reasons},
    terms_bank::Term,
    YomiDictError,
};

#[derive(Debug)]
pub struct DictEntry {
    pub term: Term,
    pub reasons: Vec<String>,
    pub source_len: usize,
    pub primary_match: bool,
}

#[derive(Debug)]
pub struct DictEntries {
    pub expression: String,
    pub reading: String,
    pub entries: Vec<DictEntry>,
}

pub async fn gather_terms(
    text: &str,
    reasons: &Reasons,
    db: &DB,
) -> Result<Vec<DictEntry>, YomiDictError> {
    let deinflections = string_deinflections(text, reasons);

    let strings = deinflections.iter().map(|d| d.term.as_str());

    let deinflections = deinflections
        .iter()
        .into_grouping_map_by(|d| &d.term)
        .collect::<Vec<_>>();

    let terms = db
        .get_terms(strings)
        .await?
        .into_iter()
        .filter_map(|term| {
            let (deinflections, primary_match) = if deinflections.contains_key(&term.expression) {
                (&deinflections[&term.expression], true)
            } else if deinflections.contains_key(&term.reading) {
                (&deinflections[&term.reading], false)
            } else {
                panic!("One of these should always be given");
            };

            let mut reasons = deinflections
                .iter()
                .filter(|d| d.rules.0.is_empty() || !(d.rules.0 & term.rules.0).is_empty())
                .sorted_unstable_by(|a, b| b.reasons.len().cmp(&a.reasons.len()));

            if let Some(d) = reasons.next() {
                return Some(DictEntry {
                    term,
                    reasons: d.reasons.clone(),
                    source_len: d.source.chars().count(),
                    primary_match,
                });
            }

            None
        })
        .collect();

    Ok(terms)
}

pub async fn get_terms(
    text: &str,
    reasons: &Reasons,
    db: &DB,
) -> Result<Vec<DictEntries>, YomiDictError> {
    let entries = gather_terms(text, reasons, db).await?;

    let terms = entries
        .into_iter()
        .into_group_map_by(|t| (t.term.expression.clone(), t.term.reading.clone()))
        .into_iter()
        .map(|(key, entries)| {
            // Sort definitions in same word
            DictEntries {
                expression: key.0,
                reading: key.1,
                entries: entries
                    .into_iter()
                    .sorted_unstable_by_key(|e| {
                        (
                            e.term.dict_id,
                            -e.term.score,
                            i64::try_from(e.term.glossary.len()).map_or(i64::MIN, |n| -n),
                        )
                    })
                    .collect::<Vec<_>>(),
            }
        })
        .sorted_unstable_by_key(|e| {
            // Sort words
            (
                e.entries[0].term.dict_id,
                i64::try_from(e.entries[0].source_len).map_or(i64::MIN, |n| -n),
                e.entries[0].reasons.len(),
                !e.entries[0].primary_match,
                -e.entries[0].term.score,
                i64::try_from(e.entries[0].term.glossary.len()).map_or(i64::MIN, |n| -n),
            )
        })
        .collect::<Vec<_>>();

    Ok(terms)
}
