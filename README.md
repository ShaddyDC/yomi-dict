# Yomi-Dict

Rust library for parsing [Yomichan](https://github.com/FooSoft/yomichan/) [dictionaries](https://github.com/FooSoft/yomichan/#dictionaries).

## Features

- Parse dictionary zip
- Add dictionary to database
- Get possible word deinflections (`聞かれました` → `聞く`)
- Get database matches for word

Note that most of the functionality is currently limited to a WASM context with [IndexedDB](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API).
