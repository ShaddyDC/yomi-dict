use std::io::Cursor;

use yomi_dict::{deinflect::inflection_reasons, translator::get_terms, *};

#[test]
fn test_find_terms() {
    let file = include_bytes!("dict.zip");

    let dict = read(Cursor::new(file)).unwrap();
    let reasons = inflection_reasons();

    let definitions = get_terms("聞かれましたか", &reasons, &dict);

    assert!(definitions
        .iter()
        .any(|d| d.entries.iter().any(|d| d.term.expression == "聞く")));
}
