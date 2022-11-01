use itertools::Itertools;

use crate::{
    deinflect::{string_deinflections, Reasons},
    terms_bank::Term,
    Dict,
};

pub fn find_terms<'d>(text: &str, reasons: &Reasons, dict: &'d Dict) -> Vec<&'d Term> {
    let deinflections = string_deinflections(text, reasons);

    let deinflections = deinflections
        .iter()
        .into_grouping_map_by(|d| &d.term)
        .collect::<Vec<_>>();

    dict.terms
        .iter()
        .filter(|t| {
            deinflections.contains_key(&t.expression)
                && (t.rules.0.is_empty()
                    || deinflections[&t.expression]
                        .iter()
                        .any(|d| !(d.rules.0 & t.rules.0).is_empty()))
        })
        .collect()
}
