use std::io::Cursor;

use rexie::Rexie;
use wasm_bindgen_test::wasm_bindgen_test;
use yomi_dict::{Dict, IndexedDB, DB};

async fn cleanup_db(name: &str) {
    Rexie::delete(name).await.unwrap();
}

#[wasm_bindgen_test]
async fn test_db_steps() {
    cleanup_db("test_db_steps").await;

    let file = include_bytes!("dict.zip");

    let dict = Dict::new(Cursor::new(file)).unwrap();

    let db = IndexedDB::new("test_db_steps").await.unwrap();

    let steps = db.add_dict_stepwise(dict).await.unwrap();

    assert!(steps.steps.len() >= 3); // Different transactions for tags, terms, kanji

    let mut sum = 0;
    for step in steps.steps {
        sum += step.await.unwrap();
    }
    assert_eq!(sum, steps.total_count);
}
