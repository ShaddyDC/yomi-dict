use gluesql::{
    core::{
        ast_builder::{col, null, num, table, text, Execute, ExprNode},
        store::{GStore, GStoreMut},
    },
    prelude::Glue,
};
use itertools::Itertools;

use crate::{Dict, YomiDictError};

pub struct DB<T: GStore + GStoreMut> {
    glue: Glue<T>,
}

impl<T: GStore + GStoreMut> DB<T> {
    pub async fn new(storage: T) -> Result<Self, YomiDictError> {
        let mut glue = Glue::new(storage);

        let setup = vec![
            "CREATE TABLE IF NOT EXISTS dictionaries (title TEXT NOT NULL, revision TEXT NOT NULL, sequenced BOOLEAN, format INT8 NOT NULL, author TEXT, url TEXT, description TEXT, attribution TEXT, frequency_mode TEXT);",
            "CREATE TABLE IF NOT EXISTS tags (name TEXT NOT NULL, category TEXT NOT NULL, order FLOAT NOT NULL, notes TEXT NOT NULL, score FLOAT NOT NULL);",
            "CREATE TABLE IF NOT EXISTS terms (expression TEXT NOT NULL, reading TEXT NOT NULL, definition_tags TEXT, rules INT8, score FLOAT NOT NULL, glossary TEXT NOT NULL, sequence INT32 NOT NULL, term_tags TEXT NOT NULL);",
            "CREATE TABLE IF NOT EXISTS kanji (character TEXT NOT NULL, onyomi TEXT NOT NULL, kunyomi TEXT NOT NULL, tags TEXT NOT NULL, meanings TEXT NOT NULL, stats TEXT NOT NULL);"
        ];

        for query in setup {
            glue.execute_async(query)
                .await
                .map_err(YomiDictError::DbError)?;
        }

        Ok(DB { glue })
    }

    pub async fn add_dict(&mut self, dict: Dict) -> Result<(), YomiDictError> {
        let rows = match table("dictionaries")
            .select()
            .filter(col("title").eq(text(&dict.index.title)))
            .limit(1)
            .execute(&mut self.glue)
            .await
            .map_err(YomiDictError::DbError)?
        {
            gluesql::prelude::Payload::Select { labels: _, rows } => rows,
            _ => panic!("Unexpected results!"),
        };

        // TODO Override or error
        if rows.len() > 0 {
            return Ok(());
        }

        let index = dict.index;
        table("dictionaries")
            .insert()
            .values(vec![vec![
                text(index.title),
                text(index.revision),
                index.sequenced.map_or(null(), |b| ExprNode::from(b)),
                num(index.format as i64),
                index.author.map_or(null(), |s| text(s)),
                index.url.map_or(null(), |s| text(s)),
                index.description.map_or(null(), |s| text(s)),
                index.attribution.map_or(null(), |s| text(s)),
                index.frequency_mode.map_or(null(), |m| num(m as i64)),
            ]])
            .execute(&mut self.glue)
            .await
            .map_err(YomiDictError::DbError)?;

        table("tags")
            .insert()
            .values(
                dict.tags
                    .into_iter()
                    .map(|tag| {
                        vec![
                            text(tag.name),
                            text(tag.category),
                            num(tag.order as i64), // TODO Fix
                            text(tag.notes),
                            num(tag.score as i64),
                        ]
                    })
                    .collect_vec(),
            )
            .execute(&mut self.glue)
            .await
            .map_err(YomiDictError::DbError)?;

        table("terms")
            .insert()
            .values(
                dict.terms
                    .into_iter()
                    .map(|term| {
                        vec![
                            text(term.expression),
                            text(term.reading),
                            term.definition_tags.map_or(null(), |tags| text(tags)),
                            num(term.rules.0.bits() as i64),
                            num(term.score.0 as i64),
                            text(serde_json::to_string(&term.glossary).unwrap()), // TODO get rid of unwraps
                            num(term.sequence as i64),
                            text(term.term_tags),
                        ]
                    })
                    .collect_vec(),
            )
            .execute(&mut self.glue)
            .await
            .map_err(YomiDictError::DbError)?;

        table("kanji")
            .insert()
            .values(
                dict.kanji
                    .into_iter()
                    .map(|kanji| {
                        vec![
                            text(kanji.character),
                            text(kanji.onyomi),
                            text(kanji.kunyomi),
                            text(kanji.tags),
                            text(serde_json::to_string(&kanji.meanings).unwrap()),
                            text(serde_json::to_string(&kanji.stats).unwrap()),
                        ]
                    })
                    .collect_vec(),
            )
            .execute(&mut self.glue)
            .await
            .map_err(YomiDictError::DbError)?;

        Ok(())
    }
}
