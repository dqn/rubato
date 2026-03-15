mod helpers;
mod mutations;
mod queries;

use rusqlite::Connection;

use crate::player_data::PlayerData;
use crate::score_data::ScoreData;
use crate::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};

use helpers::player_data_to_value;

// Re-export SongData from rubato_types for use by other accessors
pub use rubato_types::SongData;

pub trait ScoreDataCollector {
    fn collect(&mut self, song: &SongData, score: Option<&ScoreData>);
}

const LOAD_CHUNK_SIZE: usize = 1000;

/// Score database accessor.
/// Translated from Java: ScoreDatabaseAccessor extends SQLiteDatabaseAccessor
pub struct ScoreDatabaseAccessor {
    conn: Connection,
    base: SQLiteDatabaseAccessor,
}

impl ScoreDatabaseAccessor {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA shared_cache = ON")?;
        conn.pragma_update(None, "synchronous", "OFF")?;
        conn.pragma_update(None, "cache_size", 2000)?;

        let tables = vec![
            Table::new(
                "info",
                vec![
                    Column::with_pk("id", "TEXT", 1, 1),
                    Column::with_pk("name", "TEXT", 1, 0),
                    Column::new("rank", "TEXT"),
                ],
            ),
            Table::new(
                "player",
                vec![
                    Column::with_pk("date", "INTEGER", 0, 1),
                    Column::new("playcount", "INTEGER"),
                    Column::new("clear", "INTEGER"),
                    Column::new("epg", "INTEGER"),
                    Column::new("lpg", "INTEGER"),
                    Column::new("egr", "INTEGER"),
                    Column::new("lgr", "INTEGER"),
                    Column::new("egd", "INTEGER"),
                    Column::new("lgd", "INTEGER"),
                    Column::new("ebd", "INTEGER"),
                    Column::new("lbd", "INTEGER"),
                    Column::new("epr", "INTEGER"),
                    Column::new("lpr", "INTEGER"),
                    Column::new("ems", "INTEGER"),
                    Column::new("lms", "INTEGER"),
                    Column::new("playtime", "INTEGER"),
                    Column::new("maxcombo", "INTEGER"),
                ],
            ),
            Table::new(
                "score",
                vec![
                    Column::with_pk("sha256", "TEXT", 1, 1),
                    Column::with_pk("mode", "INTEGER", 0, 1),
                    Column::new("clear", "INTEGER"),
                    Column::new("epg", "INTEGER"),
                    Column::new("lpg", "INTEGER"),
                    Column::new("egr", "INTEGER"),
                    Column::new("lgr", "INTEGER"),
                    Column::new("egd", "INTEGER"),
                    Column::new("lgd", "INTEGER"),
                    Column::new("ebd", "INTEGER"),
                    Column::new("lbd", "INTEGER"),
                    Column::new("epr", "INTEGER"),
                    Column::new("lpr", "INTEGER"),
                    Column::new("ems", "INTEGER"),
                    Column::new("lms", "INTEGER"),
                    Column::new("notes", "INTEGER"),
                    Column::new("combo", "INTEGER"),
                    Column::new("minbp", "INTEGER"),
                    Column::with_default("avgjudge", "INTEGER", 1, 0, &i32::MAX.to_string()),
                    Column::new("playcount", "INTEGER"),
                    Column::new("clearcount", "INTEGER"),
                    Column::new("trophy", "TEXT"),
                    Column::new("ghost", "TEXT"),
                    Column::new("option", "INTEGER"),
                    Column::new("seed", "INTEGER"),
                    Column::new("random", "INTEGER"),
                    Column::new("date", "INTEGER"),
                    Column::new("state", "INTEGER"),
                    Column::new("scorehash", "TEXT"),
                ],
            ),
        ];

        let base = SQLiteDatabaseAccessor::new(tables);

        Ok(Self { conn, base })
    }

    pub fn create_table(&self) -> anyhow::Result<()> {
        self.base.validate(&self.conn)?;
        if self.player_datas(1).is_empty() {
            let pd = PlayerData::default();
            self.base
                .insert_with_values(&self.conn, "player", &|col_name| {
                    player_data_to_value(&pd, col_name)
                })?;
        }
        Ok(())
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

impl rubato_types::score_database_access::ScoreDatabaseAccess for ScoreDatabaseAccessor {
    fn create_table(&self) -> anyhow::Result<()> {
        ScoreDatabaseAccessor::create_table(self)
    }

    fn score_data(&self, sha256: &str, mode: i32) -> Option<ScoreData> {
        ScoreDatabaseAccessor::score_data(self, sha256, mode)
    }

    fn set_score_data_slice(&self, scores: &[ScoreData]) {
        let refs: Vec<&ScoreData> = scores.iter().collect();
        self.set_score_data_batch(&refs);
    }
}

#[cfg(test)]
mod tests;
