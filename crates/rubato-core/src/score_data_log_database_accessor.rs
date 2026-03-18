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
        // Normalize sentinel: write i32::MAX (not i64::MAX) for Java DB compatibility.
        "avgjudge" => rusqlite::types::Value::Integer(if score.timing_stats.avgjudge == i64::MAX {
            i32::MAX as i64
        } else {
            score.timing_stats.avgjudge
        }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_types::score_data::TimingStats;

    #[test]
    fn score_data_log_avgjudge_sentinel_normalized_to_i32_max() {
        // When avgjudge is i64::MAX (sentinel), it should be written as i32::MAX
        // for consistency with the scoredata table and Java compatibility.
        let mut sd = ScoreData::default();
        sd.timing_stats.avgjudge = i64::MAX;

        let value = score_data_to_value(&sd, "avgjudge");
        assert_eq!(
            value,
            rusqlite::types::Value::Integer(i32::MAX as i64),
            "i64::MAX sentinel should be normalized to i32::MAX on write"
        );
    }

    #[test]
    fn score_data_log_avgjudge_normal_value_preserved() {
        // Normal avgjudge values should be written as-is.
        let mut sd = ScoreData::default();
        sd.timing_stats = TimingStats {
            avgjudge: 42,
            ..Default::default()
        };

        let value = score_data_to_value(&sd, "avgjudge");
        assert_eq!(
            value,
            rusqlite::types::Value::Integer(42),
            "normal avgjudge values should be preserved"
        );
    }

    #[test]
    fn score_data_log_avgjudge_sentinel_roundtrip_via_db() {
        // Write a score with sentinel avgjudge to DB, read it back,
        // and verify the sentinel is preserved through the roundtrip.
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_scoredatalog.db");
        let accessor = ScoreDataLogDatabaseAccessor::new(db_path.to_str().unwrap()).unwrap();

        let mut sd = ScoreData::default();
        sd.sha256 = "test_hash".to_string();
        sd.timing_stats.avgjudge = i64::MAX;
        accessor.set_score_data_log(&sd);

        // Verify that the raw DB value is i32::MAX (not i64::MAX)
        let raw: i64 = accessor
            .connection()
            .query_row(
                "SELECT avgjudge FROM scoredatalog WHERE sha256 = ?1",
                rusqlite::params!["test_hash"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            raw,
            i32::MAX as i64,
            "DB should store i32::MAX, not i64::MAX"
        );
    }
}
