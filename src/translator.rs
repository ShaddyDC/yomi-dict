use itertools::Itertools;

use crate::{
    deinflect::{string_deinflections, Reasons},
    terms_bank::Term,
    Dict,
};

#[derive(Debug)]
pub struct DictEntry<'a> {
    pub term: &'a Term,
    pub reasons: Vec<String>,
    pub source_len: usize,
    pub primary_match: bool,
}

#[derive(Debug)]
pub struct DictEntries<'a> {
    pub expression: &'a str,
    pub reading: &'a str,
    pub entries: Vec<DictEntry<'a>>,
}

pub fn gather_terms<'d>(text: &str, reasons: &Reasons, dict: &'d Dict) -> Vec<DictEntry<'d>> {
    let deinflections = string_deinflections(text, reasons);

    let deinflections = deinflections
        .iter()
        .into_grouping_map_by(|d| &d.term)
        .collect::<Vec<_>>();

    dict.terms
        .iter()
        .filter_map(|term| {
            for value in [&term.expression, &term.reading] {
                if deinflections.contains_key(value) {
                    let mut reasons = deinflections[value]
                        .iter()
                        .filter(|d| d.rules.0.is_empty() || !(d.rules.0 & term.rules.0).is_empty())
                        .sorted_unstable_by_key(|d| {
                            i64::try_from(d.reasons.len()).map_or(i64::MIN, |n| -n)
                        });
                    if let Some(d) = reasons.next() {
                        return Some(DictEntry {
                            term,
                            reasons: d.reasons.clone(),
                            source_len: d.source.chars().count(),
                            primary_match: value == &term.expression,
                        });
                    }
                }
            }
            None
        })
        .collect()
}

pub fn get_terms<'d>(text: &str, reasons: &Reasons, dict: &'d Dict) -> Vec<DictEntries<'d>> {
    let entries = gather_terms(text, reasons, dict);

    entries
        .into_iter()
        .into_grouping_map_by(|t| (&t.term.expression, &t.term.reading))
        .collect::<Vec<_>>()
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
                i64::try_from(e.entries[0].source_len).map_or(i64::MIN, |n| -n),
                e.entries[0].reasons.len(),
                !e.entries[0].primary_match,
                -e.entries[0].term.score,
                i64::try_from(e.entries[0].term.glossary.len()).map_or(i64::MIN, |n| -n),
            )
        })
        .collect::<Vec<_>>()
}
