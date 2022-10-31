use std::io::Cursor;

use yomi_dict::read;

#[test]
fn test_read_dict() {
    let file = include_bytes!("dict.zip");

    let d = read(Cursor::new(file)).unwrap();

    assert_eq!(d.index.title, "testDict");
    assert!(d.terms.len() != 0 && d.terms[0].expression == "some text");
    assert!(d.terms.len() > 1 && d.terms[1].expression == "some text 2");
    assert!(d.tags.len() != 0 && d.tags[0].name == "name");
    assert!(d.kanji.len() != 0 && d.kanji[0].character == "character");
}
