use rusqlite::Connection;

use crate::core::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};
use rubato_types::clear_type::ClearType;
use rubato_types::validatable::Validatable;

/// Score log database accessor.
/// Translated from Java: ScoreLogDatabaseAccessor extends SQLiteDatabaseAccessor
pub struct ScoreLogDatabaseAccessor {
    conn: Connection,
    base: SQLiteDatabaseAccessor,
}

impl ScoreLogDatabaseAccessor {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        conn.pragma_update(None, "cache_size", 2000)?;

        let tables = vec![Table::new(
            "scorelog",
            vec![
                Column::with_pk("sha256", "TEXT", 1, 0),
                Column::new("mode", "INTEGER"),
                Column::new("clear", "INTEGER"),
                Column::new("oldclear", "INTEGER"),
                Column::new("score", "INTEGER"),
                Column::new("oldscore", "INTEGER"),
                Column::new("combo", "INTEGER"),
                Column::new("oldcombo", "INTEGER"),
                Column::new("minbp", "INTEGER"),
                Column::new("oldminbp", "INTEGER"),
                Column::new("date", "INTEGER"),
            ],
        )];

        let base = SQLiteDatabaseAccessor::new(tables);
        base.validate(&conn)?;

        Ok(Self { conn, base })
    }

    pub fn set_score_log(&self, log: &ScoreLog) {
        if let Err(e) = self
            .base
            .insert_with_values(&self.conn, "scorelog", &|col_name| match col_name {
                "sha256" => match &log.sha256 {
                    Some(s) => rusqlite::types::Value::Text(s.clone()),
                    None => rusqlite::types::Value::Null,
                },
                "mode" => rusqlite::types::Value::Integer(log.mode as i64),
                "clear" => rusqlite::types::Value::Integer(log.clear as i64),
                "oldclear" => rusqlite::types::Value::Integer(log.oldclear as i64),
                "score" => rusqlite::types::Value::Integer(log.score as i64),
                "oldscore" => rusqlite::types::Value::Integer(log.oldscore as i64),
                "combo" => rusqlite::types::Value::Integer(log.combo as i64),
                "oldcombo" => rusqlite::types::Value::Integer(log.oldcombo as i64),
                "minbp" => rusqlite::types::Value::Integer(log.minbp as i64),
                "oldminbp" => rusqlite::types::Value::Integer(log.oldminbp as i64),
                "date" => rusqlite::types::Value::Integer(log.date),
                _ => rusqlite::types::Value::Null,
            })
        {
            log::error!("Exception setting score log: {}", e);
        }
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

/// Score log entry.
/// Translated from Java: ScoreLogDatabaseAccessor.ScoreLog
#[derive(Clone, Debug, Default)]
pub struct ScoreLog {
    pub sha256: Option<String>,
    pub mode: i32,
    pub clear: i32,
    pub oldclear: i32,
    pub score: i32,
    pub oldscore: i32,
    pub combo: i32,
    pub oldcombo: i32,
    pub minbp: i32,
    pub oldminbp: i32,
    pub date: i64,
}

impl ScoreLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sha256(&self) -> Option<&str> {
        self.sha256.as_deref()
    }

    pub fn set_sha256(&mut self, sha256: &str) {
        self.sha256 = Some(sha256.to_string());
    }
}

impl Validatable for ScoreLog {
    fn validate(&mut self) -> bool {
        self.mode >= 0
            && self.clear >= 0
            && self.clear <= ClearType::Max.id()
            && self.oldclear >= 0
            && self.oldclear <= self.clear
            && self.score >= 0
            && self.oldscore <= self.score
            && self.combo >= 0
            && self.oldcombo <= self.combo
            && self.minbp >= 0
            && self.oldminbp >= self.minbp
            && self.date >= 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_has_wal_and_synchronous_normal() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_scorelog.db");
        let accessor = ScoreLogDatabaseAccessor::new(db_path.to_str().unwrap()).unwrap();

        let journal_mode: String = accessor
            .connection()
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            journal_mode, "wal",
            "journal_mode should be WAL for crash safety, got {}",
            journal_mode
        );

        let synchronous: i64 = accessor
            .connection()
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        // synchronous = NORMAL is 1
        assert_eq!(
            synchronous, 1,
            "synchronous should be NORMAL (1), got {}",
            synchronous
        );
    }
}
