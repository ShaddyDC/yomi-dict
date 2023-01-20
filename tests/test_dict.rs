use std::io::Cursor;

use yomi_dict::Dict;

#[test]
fn test_read_dict() {
    let file = include_bytes!("dict.zip");

    let d = Dict::new(Cursor::new(file)).unwrap();

    assert_eq!(d.index.title, "testDict");
    assert!(!d.terms.is_empty() && d.terms[0].expression == "some text");
    assert!(d.terms.len() > 1 && d.terms[1].expression == "some text 2");
    assert!(!d.tags.is_empty() && d.tags[0].name == "name");
    assert!(!d.kanji.is_empty() && d.kanji[0].character == "character");
}
