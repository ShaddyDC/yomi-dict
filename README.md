# Yomi-Dict

Rust library for parsing and using [Yomichan](https://github.com/FooSoft/yomichan/) [dictionaries](https://github.com/FooSoft/yomichan/#dictionaries).

## Features

- Parse dictionaries
- Add dictionaries to database
- Get possible word deinflections (`聞かれました` → `聞く`)
- Get database matches for word

Note that the only implementation is currently limited to a WASM context with [IndexedDB](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API).

Lookup for kanji and tags is currently not implemented.
Deletion of dictionaries is not currently implemented.
Errors during import can leave the database in an incomplete state.
In that case, reinitialising the database is the best recourse.

## Usage

```rust
use std::io::Cursor;
use yomi_dict::{inflection_reasons, Dict, IndexedDB, DB};

let dict = include_bytes!("dict.zip");

let dict = Dict::new(Cursor::new(dict)).unwrap();
let reasons = inflection_reasons();

let db = IndexedDB::new("my_db").await.unwrap();

db.add_dict(dict).await.unwrap();

let definitions = db.find_terms("聞かれましたか", &reasons).await.unwrap();

assert!(definitions
    .iter()
    .any(|d| d.entries.iter().any(|d| d.term.expression == "聞く")));
```
