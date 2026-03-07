use rusqlite::Connection;

use crate::score_data::ScoreData;
use crate::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};

/// Score data log database accessor.
/// Translated from Java: ScoreDataLogDatabaseAccessor extends SQLiteDatabaseAccessor
pub struct ScoreDataLogDatabaseAccessor {
    conn: Connection,
    base: SQLiteDatabaseAccessor,
}

impl ScoreDataLogDatabaseAccessor {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "synchronous", "OFF")?;
        conn.pragma_update(None, "cache_size", 2000)?;

        let tables = vec![Table::new(
            "scoredatalog",
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
        )];

        let base = SQLiteDatabaseAccessor::new(tables);
        base.validate(&conn)?;

        Ok(Self { conn, base })
    }

    pub fn set_score_data_log(&self, score: &ScoreData) {
        self.set_score_data_log_batch(&[score]);
    }

    pub fn set_score_data_log_batch(&self, scores: &[&ScoreData]) {
        let result: anyhow::Result<()> = (|| {
            let tx = self.conn.unchecked_transaction()?;
            for score in scores {
                self.base
                    .insert_with_values(&self.conn, "scoredatalog", &|col_name| {
                        score_data_to_value(score, col_name)
                    })?;
            }
            tx.commit()?;
            Ok(())
        })();
        if let Err(e) = result {
            log::error!("Exception updating score data log: {}", e);
        }
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

fn score_data_to_value(score: &ScoreData, col_name: &str) -> rusqlite::types::Value {
    match col_name {
        "sha256" => rusqlite::types::Value::Text(score.sha256.clone()),
        "mode" => rusqlite::types::Value::Integer(score.mode as i64),
        "clear" => rusqlite::types::Value::Integer(score.clear as i64),
        "epg" => rusqlite::types::Value::Integer(score.judge_counts.epg as i64),
        "lpg" => rusqlite::types::Value::Integer(score.judge_counts.lpg as i64),
        "egr" => rusqlite::types::Value::Integer(score.judge_counts.egr as i64),
        "lgr" => rusqlite::types::Value::Integer(score.judge_counts.lgr as i64),
        "egd" => rusqlite::types::Value::Integer(score.judge_counts.egd as i64),
        "lgd" => rusqlite::types::Value::Integer(score.judge_counts.lgd as i64),
        "ebd" => rusqlite::types::Value::Integer(score.judge_counts.ebd as i64),
        "lbd" => rusqlite::types::Value::Integer(score.judge_counts.lbd as i64),
        "epr" => rusqlite::types::Value::Integer(score.judge_counts.epr as i64),
        "lpr" => rusqlite::types::Value::Integer(score.judge_counts.lpr as i64),
        "ems" => rusqlite::types::Value::Integer(score.judge_counts.ems as i64),
        "lms" => rusqlite::types::Value::Integer(score.judge_counts.lms as i64),
        "notes" => rusqlite::types::Value::Integer(score.notes as i64),
        "combo" => rusqlite::types::Value::Integer(score.maxcombo as i64),
        "minbp" => rusqlite::types::Value::Integer(score.minbp as i64),
        "avgjudge" => rusqlite::types::Value::Integer(score.timing_stats.avgjudge),
        "playcount" => rusqlite::types::Value::Integer(score.playcount as i64),
        "clearcount" => rusqlite::types::Value::Integer(score.clearcount as i64),
        "trophy" => rusqlite::types::Value::Text(score.trophy.clone()),
        "ghost" => rusqlite::types::Value::Text(score.ghost.clone()),
        "option" => rusqlite::types::Value::Integer(score.play_option.option as i64),
        "seed" => rusqlite::types::Value::Integer(score.play_option.seed),
        "random" => rusqlite::types::Value::Integer(score.play_option.random as i64),
        "date" => rusqlite::types::Value::Integer(score.date),
        "state" => rusqlite::types::Value::Integer(score.state as i64),
        "scorehash" => rusqlite::types::Value::Text(score.scorehash.clone()),
        _ => rusqlite::types::Value::Null,
    }
}
