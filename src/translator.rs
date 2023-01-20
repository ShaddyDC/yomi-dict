use std::collections::HashMap;

use itertools::Itertools;

use crate::{
    db::DBImpl,
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

/// Returns a list of terms that could be derived by deinflecting the input text or its substrings.
/// Returned is a list of all matching dictionary entries with the rules for the match
pub async fn get_raw_terms(
    text: &str,
    reasons: &Reasons,
    db: &impl DBImpl,
) -> Result<Vec<DictEntry>, YomiDictError> {
    let text_deinflections = string_deinflections(text, reasons);

    let lookup_strings = text_deinflections.iter().map(|d| d.term.as_str());

    let term_derivations = text_deinflections
        .iter()
        .into_group_map_by(|d| &d.term)
        .into_iter()
        .map(|(s, v)| {
            (
                s,
                v.into_iter()
                    .sorted_unstable_by_key(|d| std::cmp::Reverse(d.reasons.len()))
                    .collect_vec(),
            )
        })
        .collect::<HashMap<_, _>>();

    let terms = db
        .get_raw_matches(lookup_strings)
        .await?
        .into_iter()
        .filter_map(|term| {
            let derivations = if term_derivations.contains_key(&term.expression) {
                Some((&term_derivations[&term.expression], true))
            } else if term_derivations.contains_key(&term.reading) {
                Some((&term_derivations[&term.reading], false))
            } else {
                None // Terms should be retrieved from db by exact match of either expression or reading
            };

            let derivation = derivations.and_then(|(derivations, primary_match)| {
                derivations
                    .iter()
                    .filter(|d| d.rules.0.is_empty() || d.rules.0.intersects(term.rules.0))
                    .next()
                    .map(|d| (d, primary_match))
            });

            derivation.map(|(d, primary_match)| DictEntry {
                term,
                reasons: d.reasons.clone(),
                source_len: d.source.chars().count(),
                primary_match,
            })
        })
        .collect();

    Ok(terms)
}

/// Returns a list of terms that could be derived by deinflecting the input text or its substrings.
/// The list is processed to be grouped. Groups share an identical expression and reading.
pub async fn get_grouped_terms(
    text: &str,
    reasons: &Reasons,
    db: &impl DBImpl,
) -> Result<Vec<DictEntries>, YomiDictError> {
    let entries = get_raw_terms(text, reasons, db).await?;

    let terms = entries
        .into_iter()
        .into_group_map_by(|t| (t.term.expression.clone(), t.term.reading.clone()))
        .into_iter()
        .map(|((expression, reading), entries)| {
            // Sort definitions in same word
            DictEntries {
                expression,
                reading,
                entries: entries
                    .into_iter()
                    .sorted_unstable_by_key(|e| {
                        (
                            e.term.dict_id,
                            -e.term.score,
                            std::cmp::Reverse(e.term.glossary.len()),
                        )
                    })
                    .collect::<Vec<_>>(),
            }
        })
        .sorted_unstable_by_key(|e| {
            // Sort words
            (
                e.entries[0].term.dict_id,
                std::cmp::Reverse(e.entries[0].source_len),
                e.entries[0].reasons.len(),
                !e.entries[0].primary_match,
                -e.entries[0].term.score,
                std::cmp::Reverse(e.entries[0].term.glossary.len()),
            )
        })
        .collect_vec();

    Ok(terms)
}
