#![cfg(target_arch = "wasm32")]

mod test_db;
mod test_translator;

use wasm_bindgen_test::wasm_bindgen_test_configure;

wasm_bindgen_test_configure!(run_in_browser);
