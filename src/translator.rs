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
    pub source_match: String,
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
            if deinflections.contains_key(&term.expression)
                && deinflections[&term.expression]
                    .iter()
                    .any(|d| d.rules.0.is_empty() || !(d.rules.0 & term.rules.0).is_empty())
            {
                return Some(DictEntry {
                    term,
                    reasons: deinflections[&term.expression][0].reasons.clone(),
                    source_match: deinflections[&term.expression][0].source.clone(), // TODO: Make sure longest is included and rules match
                    primary_match: true,
                });
            } else if deinflections.contains_key(&term.reading)
                && deinflections[&term.reading]
                    .iter()
                    .any(|d| d.rules.0.is_empty() || !(d.rules.0 & term.rules.0).is_empty())
            {
                return Some(DictEntry {
                    term,
                    reasons: deinflections[&term.reading][0].reasons.clone(),
                    source_match: deinflections[&term.reading][0].source.clone(),
                    primary_match: false,
                });
            }
            None
        })
        .collect()
}

pub fn get_terms<'d>(text: &str, reasons: &Reasons, dict: &'d Dict) -> Vec<DictEntries<'d>> {
    let entries = gather_terms(text, reasons, dict);

    // TODO: Lengths doesn't refer to "string length" but byte length

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
                    .sorted_unstable_by(|a, b| {
                        b.term
                            .score
                            .partial_cmp(&a.term.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then(b.term.glossary.len().cmp(&a.term.glossary.len()))
                    })
                    .collect::<Vec<_>>(),
            }
        })
        .sorted_unstable_by(|a, b| {
            // Sort words

            match (
                b.entries[0].source_match.len(),
                a.entries[0].reasons.len(),
                b.entries[0].primary_match,
            )
                .cmp(&(
                    a.entries[0].source_match.len(),
                    b.entries[0].reasons.len(),
                    a.entries[0].primary_match,
                )) {
                std::cmp::Ordering::Equal => (),
                res => return res,
            }

            match b.entries[0]
                .term
                .score
                .partial_cmp(&a.entries[0].term.score)
            {
                Some(std::cmp::Ordering::Equal) => (),
                None => (),
                Some(res) => return res,
            }

            b.entries[0]
                .term
                .glossary
                .len()
                .cmp(&a.entries[0].term.glossary.len())
        })
        .collect::<Vec<_>>()
}
