use std::io::Cursor;

use rexie::Rexie;
use wasm_bindgen_test::wasm_bindgen_test;
use yomi_dict::{deinflect::inflection_reasons, Dict, IndexedDB, DB};

async fn cleanup_db(name: &str) {
    Rexie::delete(name).await.unwrap();
}

#[wasm_bindgen_test]
async fn test_find_terms() {
    cleanup_db("test_find_terms").await;

    let file = include_bytes!("dict.zip");

    let dict = Dict::new(Cursor::new(file)).unwrap();
    let reasons = inflection_reasons();

    let db = IndexedDB::new("test_find_terms").await.unwrap();

    db.add_dict(dict).await.unwrap();

    let definitions = db.find_terms("聞かれましたか", &reasons).await.unwrap();

    assert!(definitions
        .iter()
        .any(|d| d.entries.iter().any(|d| d.term.expression == "聞く")));
}

#[wasm_bindgen_test]
async fn test_no_duplicates() {
    cleanup_db("test_no_duplicates").await;

    let file = include_bytes!("dict.zip");

    let dict = Dict::new(Cursor::new(file)).unwrap();
    let reasons = inflection_reasons();

    let db = IndexedDB::new("test_no_duplicates").await.unwrap();

    db.add_dict(dict).await.unwrap();

    let definitions = db.find_terms("no_reading", &reasons).await.unwrap();

    // Don't duplicate these
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions.first().unwrap().entries.len(), 1);
}

#[wasm_bindgen_test]
async fn test_multi_match() {
    cleanup_db("test_multi_match").await;

    let file = include_bytes!("dict.zip");

    let dict = Dict::new(Cursor::new(file)).unwrap();
    let reasons = inflection_reasons();

    let db = IndexedDB::new("test_multi_match").await.unwrap();

    db.add_dict(dict).await.unwrap();

    let definitions = db.find_terms("すばやい", &reasons).await.unwrap();

    assert!(definitions
        .iter()
        .any(|d| d.entries.iter().any(|d| d.term.expression == "素早い")));

    assert!(definitions
        .iter()
        .any(|d| d.entries.iter().any(|d| d.term.expression == "素速い")));

    assert!(definitions
        .iter()
        .any(|d| d.entries.iter().any(|d| d.term.expression == "す早い")));

    assert!(definitions
        .iter()
        .any(|d| d.entries.iter().any(|d| d.term.expression == "す速い")));
}
