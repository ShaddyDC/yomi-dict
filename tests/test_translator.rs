use std::io::Cursor;

use rexie::Rexie;
use wasm_bindgen_test::wasm_bindgen_test;
use yomi_dict::{db::DB, deinflect::inflection_reasons, translator::get_terms, *};

async fn cleanup_db(name: &str) {
    Rexie::delete(name).await.unwrap();
}

#[wasm_bindgen_test]
async fn test_find_terms() {
    cleanup_db("test_find_terms").await;

    let file = include_bytes!("dict.zip");

    let dict = read(Cursor::new(file)).unwrap();
    let reasons = inflection_reasons();

    let db = DB::new("test_find_terms").await.unwrap();

    db.add_dict(dict).await.unwrap();

    let definitions = get_terms("聞かれましたか", &reasons, &db).await.unwrap();

    assert!(definitions
        .iter()
        .any(|d| d.entries.iter().any(|d| d.term.expression == "聞く")));
}
