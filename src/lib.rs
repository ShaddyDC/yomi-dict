use std::{fs, path::{Path, self}, io};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct WordTuple(String, String, Option<String>, String, f32, Vec<String>, i32, String);

#[derive(Debug)]
struct Word{
    text: String,
    reading: String,
    definition_tags: Option<String>, // Make vector
    delinflection: String, // Make vector
    popularity: f32,
    definitions: Vec<String>,
    sequence: u32,
    term_tags: String, // Make vector
}

impl From<WordTuple> for Word {
    fn from(t: WordTuple) -> Self {
        Word { text: t.0, reading: t.1, definition_tags: t.2, delinflection: t.3, popularity: t.4, definitions: t.5, sequence: t.6, term_tags: t.7 }
    }
}

pub fn read(file: fs::File)  {
    // let d: WordTuple = serde_json::from_str(r#"["ヽ","",null,"",2,["ヽ\n〘unc〙\nrepetition mark in katakana.\n→一の字点"],1,""]"#).unwrap();

    let mut archive = zip::ZipArchive::new(&file).unwrap();

    // let index_json = &archive.by_name("index.json").expect("Need index.json");

    let mut words: Vec<Word> = vec![];

    for i in 0..archive.len() {
        let file = archive.by_index(i).unwrap();

        let fname = match file.enclosed_name() {
            Some(path) if path == Path::new("index.json") => continue,
            Some(path) => path.to_owned(),
            None => continue,
        };

        println!("File: {:?}", fname);

        let data: Vec<WordTuple> = serde_json::from_reader(file).unwrap();
        words.extend(data.into_iter().map(|w| Word::from(w)));

    }

    words.sort_by_key(|w| w.sequence);
    println!("Words: {:?}", words);
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn it_works() {
        let fname = std::path::Path::new("dict.zip");
        let file = fs::File::open(&fname).unwrap();

        read(file);
    }
}
