use anyhow::Result;
use rusqlite::Connection;

/// Column definition for schema management.
pub struct ColumnDef {
    pub name: &'static str,
    pub sql_type: &'static str,
    pub not_null: bool,
    pub primary_key: bool,
    pub default_val: Option<&'static str>,
}

/// Table definition for schema management.
pub struct TableDef {
    pub name: &'static str,
    pub columns: &'static [ColumnDef],
}

/// Ensure table exists with all defined columns.
///
/// - If table doesn't exist, CREATE TABLE with all columns.
/// - If table exists but missing columns, ALTER TABLE ADD COLUMN.
pub fn ensure_table(conn: &Connection, table: &TableDef) -> Result<()> {
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE name = ?1 AND type = 'table'",
        [table.name],
        |row| row.get(0),
    )?;

    if !table_exists {
        let mut sql = format!("CREATE TABLE [{}] (", table.name);
        let mut pk_columns = Vec::new();

        for (i, col) in table.columns.iter().enumerate() {
            if i > 0 {
                sql.push(',');
            }
            sql.push_str(&format!("[{}] {}", col.name, col.sql_type));
            if col.not_null {
                sql.push_str(" NOT NULL");
            }
            if let Some(default) = col.default_val {
                sql.push_str(&format!(" DEFAULT {default}"));
            }
            if col.primary_key {
                pk_columns.push(col.name);
            }
        }

        if !pk_columns.is_empty() {
            sql.push_str(",PRIMARY KEY(");
            for (i, name) in pk_columns.iter().enumerate() {
                if i > 0 {
                    sql.push(',');
                }
                sql.push_str(name);
            }
            sql.push(')');
        }
        sql.push_str(");");

        conn.execute_batch(&sql)?;
    } else {
        // Check for missing columns
        let mut existing: Vec<String> = Vec::new();
        let mut stmt = conn.prepare(&format!("PRAGMA table_info('{}')", table.name))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for name in rows {
            existing.push(name?);
        }

        for col in table.columns {
            if !existing.iter().any(|e| e == col.name) {
                let mut alter = format!(
                    "ALTER TABLE {} ADD COLUMN [{}] {}",
                    table.name, col.name, col.sql_type,
                );
                if col.not_null {
                    alter.push_str(" NOT NULL");
                }
                if let Some(default) = col.default_val {
                    alter.push_str(&format!(" DEFAULT {default}"));
                }
                conn.execute_batch(&alter)?;
            }
        }
    }

    Ok(())
}

// ============================================================
// Table definitions (matching Java schema exactly)
// ============================================================

pub static FOLDER_TABLE: TableDef = TableDef {
    name: "folder",
    columns: &[
        ColumnDef {
            name: "title",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "subtitle",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "command",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "path",
            sql_type: "TEXT",
            not_null: false,
            primary_key: true,
            default_val: None,
        },
        ColumnDef {
            name: "banner",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "parent",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "type",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "date",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "adddate",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "max",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
    ],
};

pub static SONG_TABLE: TableDef = TableDef {
    name: "song",
    columns: &[
        ColumnDef {
            name: "md5",
            sql_type: "TEXT",
            not_null: true,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "sha256",
            sql_type: "TEXT",
            not_null: true,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "title",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "subtitle",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "genre",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "artist",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "subartist",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "tag",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "path",
            sql_type: "TEXT",
            not_null: false,
            primary_key: true,
            default_val: None,
        },
        ColumnDef {
            name: "folder",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "stagefile",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "banner",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "backbmp",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "preview",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "parent",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "level",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "difficulty",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "maxbpm",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "minbpm",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "length",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "mode",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "judge",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "feature",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "content",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "date",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "favorite",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "adddate",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "notes",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "charthash",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
    ],
};

pub static INFO_TABLE: TableDef = TableDef {
    name: "info",
    columns: &[
        ColumnDef {
            name: "id",
            sql_type: "TEXT",
            not_null: true,
            primary_key: true,
            default_val: None,
        },
        ColumnDef {
            name: "name",
            sql_type: "TEXT",
            not_null: true,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "rank",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
    ],
};

pub static PLAYER_TABLE: TableDef = TableDef {
    name: "player",
    columns: &[
        ColumnDef {
            name: "date",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: true,
            default_val: None,
        },
        ColumnDef {
            name: "playcount",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "clear",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "epg",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lpg",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "egr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lgr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "egd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lgd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "ebd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lbd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "epr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lpr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "ems",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lms",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "playtime",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "maxcombo",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
    ],
};

pub static SCORE_TABLE: TableDef = TableDef {
    name: "score",
    columns: &[
        ColumnDef {
            name: "sha256",
            sql_type: "TEXT",
            not_null: true,
            primary_key: true,
            default_val: None,
        },
        ColumnDef {
            name: "mode",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: true,
            default_val: None,
        },
        ColumnDef {
            name: "clear",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "epg",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lpg",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "egr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lgr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "egd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lgd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "ebd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lbd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "epr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lpr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "ems",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lms",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "notes",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "combo",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "minbp",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "avgjudge",
            sql_type: "INTEGER",
            not_null: true,
            primary_key: false,
            default_val: Some("2147483647"),
        },
        ColumnDef {
            name: "playcount",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "clearcount",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "trophy",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "ghost",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "option",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "seed",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "random",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "date",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "state",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "scorehash",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
    ],
};

pub static INFORMATION_TABLE: TableDef = TableDef {
    name: "information",
    columns: &[
        ColumnDef {
            name: "sha256",
            sql_type: "TEXT",
            not_null: true,
            primary_key: true,
            default_val: None,
        },
        ColumnDef {
            name: "n",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "ln",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "s",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "ls",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "total",
            sql_type: "REAL",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "density",
            sql_type: "REAL",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "peakdensity",
            sql_type: "REAL",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "enddensity",
            sql_type: "REAL",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "mainbpm",
            sql_type: "REAL",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "distribution",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "speedchange",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lanenotes",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
    ],
};

/// Score update log table (tracks clear lamp / score improvements).
///
/// Java parity: `ScoreLogDatabaseAccessor.SCORELOG_TABLE`.
/// Used by LAMP UPDATE / SCORE UPDATE built-in containers.
pub static SCORELOG_TABLE: TableDef = TableDef {
    name: "scorelog",
    columns: &[
        ColumnDef {
            name: "sha256",
            sql_type: "TEXT",
            not_null: true,
            primary_key: true,
            default_val: None,
        },
        ColumnDef {
            name: "mode",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "clear",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "oldclear",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "score",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "oldscore",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "combo",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "oldcombo",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "minbp",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "oldminbp",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "date",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
    ],
};

pub static SCOREDATALOG_TABLE: TableDef = TableDef {
    name: "scoredatalog",
    columns: &[
        ColumnDef {
            name: "sha256",
            sql_type: "TEXT",
            not_null: true,
            primary_key: true,
            default_val: None,
        },
        ColumnDef {
            name: "mode",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: true,
            default_val: None,
        },
        ColumnDef {
            name: "clear",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "epg",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lpg",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "egr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lgr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "egd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lgd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "ebd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lbd",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "epr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lpr",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "ems",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "lms",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "notes",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "combo",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "minbp",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "avgjudge",
            sql_type: "INTEGER",
            not_null: true,
            primary_key: false,
            default_val: Some("2147483647"),
        },
        ColumnDef {
            name: "playcount",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "clearcount",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "trophy",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "ghost",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "option",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "seed",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "random",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "date",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "state",
            sql_type: "INTEGER",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
        ColumnDef {
            name: "scorehash",
            sql_type: "TEXT",
            not_null: false,
            primary_key: false,
            default_val: None,
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_table(&conn, &FOLDER_TABLE).unwrap();
        ensure_table(&conn, &SONG_TABLE).unwrap();
        ensure_table(&conn, &INFO_TABLE).unwrap();
        ensure_table(&conn, &PLAYER_TABLE).unwrap();
        ensure_table(&conn, &SCORE_TABLE).unwrap();
        ensure_table(&conn, &SCORELOG_TABLE).unwrap();
        ensure_table(&conn, &SCOREDATALOG_TABLE).unwrap();
        ensure_table(&conn, &INFORMATION_TABLE).unwrap();

        // Verify tables exist
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 8);
    }

    #[test]
    fn ensure_table_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_table(&conn, &SONG_TABLE).unwrap();
        // Running again should not error
        ensure_table(&conn, &SONG_TABLE).unwrap();
    }

    #[test]
    fn add_missing_column() {
        let conn = Connection::open_in_memory().unwrap();
        // Create a partial table
        conn.execute_batch("CREATE TABLE [song] ([md5] TEXT, [path] TEXT, PRIMARY KEY(path));")
            .unwrap();
        // ensure_table should add missing columns
        ensure_table(&conn, &SONG_TABLE).unwrap();

        // Verify sha256 column was added
        let mut stmt = conn.prepare("PRAGMA table_info('song')").unwrap();
        let names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(names.contains(&"sha256".to_string()));
        assert!(names.contains(&"title".to_string()));
    }
}
