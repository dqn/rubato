use rusqlite::Connection;

use crate::clear_type::ClearType;
use crate::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};
use crate::validatable::Validatable;

/// Score log database accessor.
/// Translated from Java: ScoreLogDatabaseAccessor extends SQLiteDatabaseAccessor
pub struct ScoreLogDatabaseAccessor {
    conn: Connection,
    base: SQLiteDatabaseAccessor,
}

impl ScoreLogDatabaseAccessor {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "synchronous", "OFF")?;
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

    pub fn get_connection(&self) -> &Connection {
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

    pub fn get_sha256(&self) -> Option<&str> {
        self.sha256.as_deref()
    }

    pub fn set_sha256(&mut self, sha256: &str) {
        self.sha256 = Some(sha256.to_string());
    }

    pub fn get_mode(&self) -> i32 {
        self.mode
    }

    pub fn set_mode(&mut self, mode: i32) {
        self.mode = mode;
    }

    pub fn get_clear(&self) -> i32 {
        self.clear
    }

    pub fn set_clear(&mut self, clear: i32) {
        self.clear = clear;
    }

    pub fn get_oldclear(&self) -> i32 {
        self.oldclear
    }

    pub fn set_oldclear(&mut self, oldclear: i32) {
        self.oldclear = oldclear;
    }

    pub fn get_score(&self) -> i32 {
        self.score
    }

    pub fn set_score(&mut self, score: i32) {
        self.score = score;
    }

    pub fn get_oldscore(&self) -> i32 {
        self.oldscore
    }

    pub fn set_oldscore(&mut self, oldscore: i32) {
        self.oldscore = oldscore;
    }

    pub fn get_combo(&self) -> i32 {
        self.combo
    }

    pub fn set_combo(&mut self, combo: i32) {
        self.combo = combo;
    }

    pub fn get_oldcombo(&self) -> i32 {
        self.oldcombo
    }

    pub fn set_oldcombo(&mut self, oldcombo: i32) {
        self.oldcombo = oldcombo;
    }

    pub fn get_minbp(&self) -> i32 {
        self.minbp
    }

    pub fn set_minbp(&mut self, minbp: i32) {
        self.minbp = minbp;
    }

    pub fn get_oldminbp(&self) -> i32 {
        self.oldminbp
    }

    pub fn set_oldminbp(&mut self, oldminbp: i32) {
        self.oldminbp = oldminbp;
    }

    pub fn get_date(&self) -> i64 {
        self.date
    }

    pub fn set_date(&mut self, date: i64) {
        self.date = date;
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
