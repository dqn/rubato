use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::validatable::remove_invalid_elements_vec;
use bms::model::bms_decoder::BMSDecoder;
use bms::model::bms_model::{BMSModel, LNTYPE_LONGNOTE};
use bms::model::bmson_decoder::BMSONDecoder;
use bms::model::osu_decoder::OSUDecoder;
use rayon::prelude::*;
use rubato_db::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};
use rubato_types::sync_utils::lock_or_recover;
use rusqlite::Connection;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

use crate::song::folder_data::FolderData;
use crate::song::song_data::SongData;
use crate::song::song_database_accessor::SongDatabaseAccessor;
use crate::song::song_database_update_listener::SongDatabaseUpdateListener;
use crate::song::song_utils;
use rubato_types::song_information_db::SongInformationDb;

/// Escape SQL LIKE wildcard characters (`%`, `_`, `\`) so that they are
/// treated as literal characters in a `LIKE ... ESCAPE '\'` clause.
fn escape_sql_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '%' | '_' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

/// SQLite authorizer callback that only allows read-only operations.
/// Used to guard queries that interpolate untrusted SQL (e.g. course file WHERE clauses).
fn read_only_authorizer(ctx: AuthContext<'_>) -> Authorization {
    match ctx.action {
        AuthAction::Select
        | AuthAction::Read { .. }
        | AuthAction::Function { .. }
        | AuthAction::Recursive => Authorization::Allow,
        _ => Authorization::Deny,
    }
}

/// Plugin interface for song database accessor
pub trait SongDatabaseAccessorPlugin: Send + Sync {
    fn update(&self, model: &BMSModel, song: &mut SongData);
}

/// SQLite song database accessor
pub struct SQLiteSongDatabaseAccessor {
    base: SQLiteDatabaseAccessor,
    conn: Mutex<Connection>,
    root: PathBuf,
    plugins: Vec<Box<dyn SongDatabaseAccessorPlugin>>,
    checked_parent: Mutex<HashSet<String>>,
}

impl SQLiteSongDatabaseAccessor {
    pub fn new(filepath: &str, _bmsroot: &[String]) -> anyhow::Result<Self> {
        let base = SQLiteDatabaseAccessor::new(vec![
            Table::new(
                "folder",
                vec![
                    Column::new("title", "TEXT"),
                    Column::new("subtitle", "TEXT"),
                    Column::new("command", "TEXT"),
                    Column::with_pk("path", "TEXT", 0, 1),
                    Column::new("banner", "TEXT"),
                    Column::new("parent", "TEXT"),
                    Column::new("type", "INTEGER"),
                    Column::new("date", "INTEGER"),
                    Column::new("adddate", "INTEGER"),
                    Column::new("max", "INTEGER"),
                ],
            ),
            Table::new(
                "song",
                vec![
                    Column::with_pk("md5", "TEXT", 1, 0),
                    Column::with_pk("sha256", "TEXT", 1, 0),
                    Column::new("title", "TEXT"),
                    Column::new("subtitle", "TEXT"),
                    Column::new("genre", "TEXT"),
                    Column::new("artist", "TEXT"),
                    Column::new("subartist", "TEXT"),
                    Column::new("tag", "TEXT"),
                    Column::with_pk("path", "TEXT", 0, 1),
                    Column::new("folder", "TEXT"),
                    Column::new("stagefile", "TEXT"),
                    Column::new("banner", "TEXT"),
                    Column::new("backbmp", "TEXT"),
                    Column::new("preview", "TEXT"),
                    Column::new("parent", "TEXT"),
                    Column::new("level", "INTEGER"),
                    Column::new("difficulty", "INTEGER"),
                    Column::new("maxbpm", "INTEGER"),
                    Column::new("minbpm", "INTEGER"),
                    Column::new("length", "INTEGER"),
                    Column::new("mode", "INTEGER"),
                    Column::new("judge", "INTEGER"),
                    Column::new("feature", "INTEGER"),
                    Column::new("content", "INTEGER"),
                    Column::new("date", "INTEGER"),
                    Column::new("favorite", "INTEGER"),
                    Column::new("adddate", "INTEGER"),
                    Column::new("notes", "INTEGER"),
                    Column::new("charthash", "TEXT"),
                ],
            ),
        ]);

        let conn = Connection::open(filepath)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA shared_cache = ON; PRAGMA synchronous = NORMAL; PRAGMA recursive_triggers = ON;",
        )?;
        let root = PathBuf::from(".");

        let accessor = Self {
            base,
            conn: Mutex::new(conn),
            root,
            plugins: Vec::new(),
            checked_parent: Mutex::new(HashSet::new()),
        };
        accessor.create_table()?;
        Ok(accessor)
    }

    pub fn add_plugin(&mut self, plugin: Box<dyn SongDatabaseAccessorPlugin>) {
        self.plugins.push(plugin);
    }

    fn create_table(&self) -> anyhow::Result<()> {
        let conn = lock_or_recover(&self.conn);
        self.base.validate(&conn)?;

        // Check if sha256 is primary key in song table (migration check)
        let mut stmt = conn.prepare("PRAGMA TABLE_INFO(song)")?;
        let has_sha256_pk = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                let pk: i32 = row.get(5)?;
                Ok((name, pk))
            })?
            .filter_map(|r| r.ok())
            .any(|(name, pk)| name == "sha256" && pk == 1);

        if has_sha256_pk {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            let migration_result = (|| -> anyhow::Result<()> {
                conn.execute("ALTER TABLE [song] RENAME TO [old_song]", [])?;
                self.base.validate(&conn)?;
                conn.execute(
                    "INSERT INTO song SELECT \
                     s.md5, s.sha256, s.title, s.subtitle, s.genre, s.artist, s.subartist, s.tag, s.path,\
                     s.folder, s.stagefile, s.banner, s.backbmp, s.preview, s.parent, s.level, s.difficulty,\
                     s.maxbpm, s.minbpm, s.length, s.mode, s.judge, s.feature, s.content,\
                     s.date, s.favorite, s.adddate, s.notes, s.charthash \
                     FROM old_song s \
                     INNER JOIN (SELECT path, MAX(adddate) AS max_adddate FROM old_song GROUP BY path) g \
                     ON s.path = g.path AND s.adddate = g.max_adddate",
                    [],
                )?;
                conn.execute("DROP TABLE old_song", [])?;
                Ok(())
            })();
            match migration_result {
                Ok(()) => conn.execute_batch("COMMIT")?,
                Err(e) => {
                    let _ = conn.execute_batch("ROLLBACK");
                    return Err(e);
                }
            }
        }

        // FTS5 full-text search index for song text search
        Self::create_fts_table(&conn)?;

        Ok(())
    }

    /// Create the FTS5 virtual table and sync triggers for full-text search.
    /// Uses content-sync: the FTS table references the song table's rowid.
    /// Requires PRAGMA recursive_triggers = ON for INSERT OR REPLACE support.
    fn create_fts_table(conn: &Connection) -> anyhow::Result<()> {
        let fts_exists: bool = {
            let mut stmt = conn
                .prepare("SELECT 1 FROM sqlite_master WHERE name = 'song_fts' AND type='table'")?;
            stmt.query_map([], |_| Ok(()))?.count() > 0
        };

        if !fts_exists {
            conn.execute_batch(
                "CREATE VIRTUAL TABLE song_fts USING fts5(\
                     title, subtitle, artist, subartist, genre, \
                     content='song', content_rowid='rowid'\
                 );\
                 CREATE TRIGGER song_fts_ai AFTER INSERT ON song BEGIN \
                     INSERT INTO song_fts(rowid, title, subtitle, artist, subartist, genre) \
                     VALUES (new.rowid, new.title, new.subtitle, new.artist, new.subartist, new.genre); \
                 END;\
                 CREATE TRIGGER song_fts_ad AFTER DELETE ON song BEGIN \
                     INSERT INTO song_fts(song_fts, rowid, title, subtitle, artist, subartist, genre) \
                     VALUES ('delete', old.rowid, old.title, old.subtitle, old.artist, old.subartist, old.genre); \
                 END;\
                 CREATE TRIGGER song_fts_au AFTER UPDATE ON song BEGIN \
                     INSERT INTO song_fts(song_fts, rowid, title, subtitle, artist, subartist, genre) \
                     VALUES ('delete', old.rowid, old.title, old.subtitle, old.artist, old.subartist, old.genre); \
                     INSERT INTO song_fts(rowid, title, subtitle, artist, subartist, genre) \
                     VALUES (new.rowid, new.title, new.subtitle, new.artist, new.subartist, new.genre); \
                 END;",
            )?;

            // Populate FTS from existing data (handles migration and pre-existing databases)
            conn.execute_batch("INSERT INTO song_fts(song_fts) VALUES('rebuild')")?;
        }

        Ok(())
    }

    fn query_songs(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql]) -> Vec<SongData> {
        let conn = lock_or_recover(&self.conn);
        match Self::query_songs_with_conn(&conn, sql, params) {
            Ok(songs) => songs,
            Err(e) => {
                log::error!("Error querying songs: {}", e);
                Vec::new()
            }
        }
    }

    fn query_songs_with_conn(
        conn: &Connection,
        sql: &str,
        params: &[&dyn rusqlite::types::ToSql],
    ) -> anyhow::Result<Vec<SongData>> {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params, |row| {
            let mut sd = SongData::new();
            sd.file.md5 = row.get::<_, String>(0).unwrap_or_default();
            sd.file.sha256 = row.get::<_, String>(1).unwrap_or_default();
            sd.metadata.title = row.get::<_, String>(2).unwrap_or_default();
            sd.metadata.subtitle = row.get::<_, String>(3).unwrap_or_default();
            sd.metadata.genre = row.get::<_, String>(4).unwrap_or_default();
            sd.metadata.artist = row.get::<_, String>(5).unwrap_or_default();
            sd.metadata.subartist = row.get::<_, String>(6).unwrap_or_default();
            sd.metadata.tag = row.get::<_, String>(7).unwrap_or_default();
            let path: String = row.get::<_, String>(8).unwrap_or_default();
            sd.file.set_path(path);
            sd.folder = row.get::<_, String>(9).unwrap_or_default();
            sd.file.stagefile = row.get::<_, String>(10).unwrap_or_default();
            sd.file.banner = row.get::<_, String>(11).unwrap_or_default();
            sd.file.backbmp = row.get::<_, String>(12).unwrap_or_default();
            sd.file.preview = row.get::<_, String>(13).unwrap_or_default();
            sd.parent = row.get::<_, String>(14).unwrap_or_default();
            sd.chart.level = row.get::<_, i32>(15).unwrap_or(0);
            sd.chart.difficulty = row.get::<_, i32>(16).unwrap_or(0);
            sd.chart.maxbpm = row.get::<_, i32>(17).unwrap_or(0);
            sd.chart.minbpm = row.get::<_, i32>(18).unwrap_or(0);
            sd.chart.length = row.get::<_, i32>(19).unwrap_or(0);
            sd.chart.mode = row.get::<_, i32>(20).unwrap_or(0);
            sd.chart.judge = row.get::<_, i32>(21).unwrap_or(0);
            sd.chart.feature = row.get::<_, i32>(22).unwrap_or(0);
            sd.chart.content = row.get::<_, i32>(23).unwrap_or(0);
            sd.chart.date = row.get::<_, i64>(24).unwrap_or(0);
            sd.favorite = row.get::<_, i32>(25).unwrap_or(0);
            sd.chart.adddate = row.get::<_, i64>(26).unwrap_or(0);
            sd.chart.notes = row.get::<_, i32>(27).unwrap_or(0);
            sd.file.charthash = row.get::<_, Option<String>>(28).unwrap_or(None);
            Ok(sd)
        })?;
        Ok(rows.flatten().collect())
    }

    fn query_folders(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql]) -> Vec<FolderData> {
        let conn = lock_or_recover(&self.conn);
        match Self::query_folders_with_conn(&conn, sql, params) {
            Ok(folders) => folders,
            Err(e) => {
                log::error!("Error querying folders: {}", e);
                Vec::new()
            }
        }
    }

    fn query_folders_with_conn(
        conn: &Connection,
        sql: &str,
        params: &[&dyn rusqlite::types::ToSql],
    ) -> anyhow::Result<Vec<FolderData>> {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params, |row| {
            Ok(FolderData {
                title: row.get::<_, String>(0).unwrap_or_default(),
                subtitle: row.get::<_, String>(1).unwrap_or_default(),
                command: row.get::<_, String>(2).unwrap_or_default(),
                path: row.get::<_, String>(3).unwrap_or_default(),
                banner: row.get::<_, String>(4).unwrap_or_default(),
                parent: row.get::<_, String>(5).unwrap_or_default(),
                folder_type: row.get::<_, i32>(6).unwrap_or(0),
                date: row.get::<_, i64>(7).unwrap_or(0),
                adddate: row.get::<_, i64>(8).unwrap_or(0),
                max: row.get::<_, i32>(9).unwrap_or(0),
            })
        })?;
        Ok(rows.flatten().collect())
    }

    #[cfg(test)]
    fn insert_song(&self, sd: &SongData) -> anyhow::Result<()> {
        let conn = lock_or_recover(&self.conn);
        Self::insert_song_with_conn(&self.base, &conn, sd)
    }

    fn insert_song_with_conn(
        base: &SQLiteDatabaseAccessor,
        conn: &Connection,
        sd: &SongData,
    ) -> anyhow::Result<()> {
        base.insert_with_values(conn, "song", &|name: &str| -> rusqlite::types::Value {
            match name {
                "md5" => rusqlite::types::Value::Text(sd.file.md5.clone()),
                "sha256" => rusqlite::types::Value::Text(sd.file.sha256.clone()),
                "title" => rusqlite::types::Value::Text(sd.metadata.title.clone()),
                "subtitle" => rusqlite::types::Value::Text(sd.metadata.subtitle.clone()),
                "genre" => rusqlite::types::Value::Text(sd.metadata.genre.clone()),
                "artist" => rusqlite::types::Value::Text(sd.metadata.artist.clone()),
                "subartist" => rusqlite::types::Value::Text(sd.metadata.subartist.clone()),
                "tag" => rusqlite::types::Value::Text(sd.metadata.tag.clone()),
                "path" => rusqlite::types::Value::Text(sd.file.path().unwrap_or("").to_string()),
                "folder" => rusqlite::types::Value::Text(sd.folder.clone()),
                "stagefile" => rusqlite::types::Value::Text(sd.file.stagefile.clone()),
                "banner" => rusqlite::types::Value::Text(sd.file.banner.clone()),
                "backbmp" => rusqlite::types::Value::Text(sd.file.backbmp.clone()),
                "preview" => rusqlite::types::Value::Text(sd.file.preview.clone()),
                "parent" => rusqlite::types::Value::Text(sd.parent.clone()),
                "level" => rusqlite::types::Value::Integer(sd.chart.level as i64),
                "difficulty" => rusqlite::types::Value::Integer(sd.chart.difficulty as i64),
                "maxbpm" => rusqlite::types::Value::Integer(sd.chart.maxbpm as i64),
                "minbpm" => rusqlite::types::Value::Integer(sd.chart.minbpm as i64),
                "length" => rusqlite::types::Value::Integer(sd.chart.length as i64),
                "mode" => rusqlite::types::Value::Integer(sd.chart.mode as i64),
                "judge" => rusqlite::types::Value::Integer(sd.chart.judge as i64),
                "feature" => rusqlite::types::Value::Integer(sd.chart.feature as i64),
                "content" => rusqlite::types::Value::Integer(sd.chart.content as i64),
                "date" => rusqlite::types::Value::Integer(sd.chart.date),
                "favorite" => rusqlite::types::Value::Integer(sd.favorite as i64),
                "adddate" => rusqlite::types::Value::Integer(sd.chart.adddate),
                "notes" => rusqlite::types::Value::Integer(sd.chart.notes as i64),
                "charthash" => match &sd.file.charthash {
                    Some(h) => rusqlite::types::Value::Text(h.clone()),
                    None => rusqlite::types::Value::Null,
                },
                _ => rusqlite::types::Value::Null,
            }
        })
    }

    #[cfg(test)]
    fn insert_folder(&self, fd: &FolderData) -> anyhow::Result<()> {
        let conn = lock_or_recover(&self.conn);
        Self::insert_folder_with_conn(&self.base, &conn, fd)
    }

    fn insert_folder_with_conn(
        base: &SQLiteDatabaseAccessor,
        conn: &Connection,
        fd: &FolderData,
    ) -> anyhow::Result<()> {
        base.insert_with_values(conn, "folder", &|name: &str| -> rusqlite::types::Value {
            match name {
                "title" => rusqlite::types::Value::Text(fd.title.clone()),
                "subtitle" => rusqlite::types::Value::Text(fd.subtitle.clone()),
                "command" => rusqlite::types::Value::Text(fd.command.clone()),
                "path" => rusqlite::types::Value::Text(fd.path.clone()),
                "banner" => rusqlite::types::Value::Text(fd.banner.clone()),
                "parent" => rusqlite::types::Value::Text(fd.parent.clone()),
                "type" => rusqlite::types::Value::Integer(fd.folder_type as i64),
                "date" => rusqlite::types::Value::Integer(fd.date),
                "adddate" => rusqlite::types::Value::Integer(fd.adddate),
                "max" => rusqlite::types::Value::Integer(fd.max as i64),
                _ => rusqlite::types::Value::Null,
            }
        })
    }
}

impl SongDatabaseAccessor for SQLiteSongDatabaseAccessor {
    fn song_datas(&self, key: &str, value: &str) -> Vec<SongData> {
        // Whitelist valid column names to prevent SQL injection via key parameter
        const VALID_COLUMNS: &[&str] = &[
            "md5",
            "sha256",
            "title",
            "subtitle",
            "genre",
            "artist",
            "subartist",
            "path",
            "folder",
            "parent",
            "level",
            "difficulty",
            "mode",
        ];
        if !VALID_COLUMNS.contains(&key) {
            log::warn!("Invalid column name for song query: {}", key);
            return Vec::new();
        }
        let sql = format!("SELECT * FROM song WHERE [{}] = ?1", key);
        let songs = self.query_songs(&sql, &[&value as &dyn rusqlite::types::ToSql]);
        remove_invalid_elements_vec(songs)
    }

    fn song_datas_by_hashes(&self, hashes: &[String]) -> Vec<SongData> {
        let mut md5_hashes: Vec<&str> = Vec::new();
        let mut sha256_hashes: Vec<&str> = Vec::new();
        for hash in hashes {
            if hash.len() > 32 {
                sha256_hashes.push(hash);
            } else {
                md5_hashes.push(hash);
            }
        }

        if md5_hashes.is_empty() && sha256_hashes.is_empty() {
            return Vec::new();
        }

        // SQLite has a 999 bind parameter limit. Batch each hash type
        // separately in chunks of 900 (leaving headroom) and collect results.
        const BATCH_SIZE: usize = 900;

        let mut songs: Vec<SongData> = Vec::new();

        for chunk in md5_hashes.chunks(BATCH_SIZE) {
            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            let placeholders: Vec<String> = chunk
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect();
            for h in chunk {
                params.push(Box::new(h.to_string()));
            }
            let sql = format!(
                "SELECT * FROM song WHERE md5 IN ({})",
                placeholders.join(",")
            );
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            songs.extend(self.query_songs(&sql, &param_refs));
        }

        for chunk in sha256_hashes.chunks(BATCH_SIZE) {
            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
            let placeholders: Vec<String> = chunk
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect();
            for h in chunk {
                params.push(Box::new(h.to_string()));
            }
            let sql = format!(
                "SELECT * FROM song WHERE sha256 IN ({})",
                placeholders.join(",")
            );
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            songs.extend(self.query_songs(&sql, &param_refs));
        }

        // Performance note: O(n*m) linear scan matches Java parity. Consider HashMap<&str, &ScoreData>
        // lookup for large libraries if profiling shows this as a bottleneck.
        songs.sort_by(|a, b| {
            let mut a_index_sha256 = -1i64;
            let mut a_index_md5 = -1i64;
            let mut b_index_sha256 = -1i64;
            let mut b_index_md5 = -1i64;
            for (i, hash) in hashes.iter().enumerate() {
                if hash == &a.file.sha256 {
                    a_index_sha256 = i as i64;
                }
                if hash == &a.file.md5 {
                    a_index_md5 = i as i64;
                }
                if hash == &b.file.sha256 {
                    b_index_sha256 = i as i64;
                }
                if hash == &b.file.md5 {
                    b_index_md5 = i as i64;
                }
            }
            let a_index = std::cmp::min(
                if a_index_sha256 == -1 {
                    i64::MAX
                } else {
                    a_index_sha256
                },
                if a_index_md5 == -1 {
                    i64::MAX
                } else {
                    a_index_md5
                },
            );
            let b_index = std::cmp::min(
                if b_index_sha256 == -1 {
                    i64::MAX
                } else {
                    b_index_sha256
                },
                if b_index_md5 == -1 {
                    i64::MAX
                } else {
                    b_index_md5
                },
            );
            // Java: return bIndex - aIndex (descending)
            b_index.cmp(&a_index)
        });

        remove_invalid_elements_vec(songs)
    }

    /// Query song data using a raw SQL WHERE clause from course files (.lr2crs).
    ///
    /// # Attack surface
    ///
    /// The `sql` parameter is interpolated directly into a WHERE clause. This raw SQL
    /// originates from `.lr2crs` course definition files, matching Java beatoraja behavior.
    ///
    /// Defense layers:
    /// - **Read-only authorizer** (primary): A SQLite authorizer callback is installed before
    ///   executing the query, blocking all write operations (INSERT, UPDATE, DELETE, DROP,
    ///   ALTER, CREATE, ATTACH, DETACH, REINDEX). Only SELECT/READ operations are permitted.
    /// - **SQL length limit** (defense-in-depth): Rejects SQL strings exceeding 4096 characters.
    ///   Legitimate course file WHERE clauses are short; oversized strings are likely malformed
    ///   or malicious.
    ///
    /// # Known limitations
    ///
    /// - Reading from attached databases (scoredb, scorelogdb, infodb) is permitted by design.
    /// - A crafted WHERE clause could read arbitrary data from these attached databases.
    /// - This matches Java beatoraja behavior, which has the same exposure.
    fn song_datas_by_sql(
        &self,
        sql: &str,
        score: &str,
        scorelog: &str,
        info: Option<&str>,
    ) -> Vec<SongData> {
        // Defense-in-depth: reject oversized SQL from course files.
        // Legitimate .lr2crs WHERE clauses are typically short (< 500 chars).
        const MAX_COURSE_SQL_LENGTH: usize = 4096;
        if sql.len() > MAX_COURSE_SQL_LENGTH {
            log::warn!(
                "Rejecting oversized course SQL ({} chars, limit {}): {:?}",
                sql.len(),
                MAX_COURSE_SQL_LENGTH,
                &sql[..80.min(sql.len())]
            );
            return Vec::new();
        }

        log::debug!("song_datas_by_sql: executing course SQL: {:?}", sql);

        let conn = lock_or_recover(&self.conn);

        // Track which databases are attached so we can detach them on any exit path.
        let mut attached_score = false;
        let mut attached_scorelog = false;
        let mut attached_info = false;

        let result: anyhow::Result<Vec<SongData>> = (|| {
            // ATTACH DATABASE with parameterized binding (e.g., ?1) is supported by SQLite but
            // not well-tested with rusqlite. Single-quote escaping prevents SQL injection via the
            // path string. Additional metacharacters (semicolons, newlines) are not exploitable
            // because ATTACH parses only a single string expression, not multiple statements.
            let score_escaped = score.replace('\'', "''");
            let scorelog_escaped = scorelog.replace('\'', "''");
            conn.execute(
                &format!("ATTACH DATABASE '{}' as scoredb", score_escaped),
                [],
            )?;
            attached_score = true;
            conn.execute(
                &format!("ATTACH DATABASE '{}' as scorelogdb", scorelog_escaped),
                [],
            )?;
            attached_scorelog = true;

            let songs = if let Some(info_path) = info {
                let info_escaped = info_path.replace('\'', "''");
                conn.execute(&format!("ATTACH DATABASE '{}' as infodb", info_escaped), [])?;
                attached_info = true;
                let query = format!(
                    "SELECT DISTINCT md5, song.sha256 AS sha256, title, subtitle, genre, artist, subartist, \
                     tag, path, folder, stagefile, banner, backbmp, preview, parent, level, difficulty, \
                     maxbpm, minbpm, length, song.mode AS mode, judge, feature, content, \
                     song.date AS date, favorite, adddate, song.notes AS notes, charthash \
                     FROM song INNER JOIN (information LEFT OUTER JOIN (score LEFT OUTER JOIN scorelog ON score.sha256 = scorelog.sha256) ON information.sha256 = score.sha256) \
                     ON song.sha256 = information.sha256 WHERE {}",
                    sql
                );
                // Guard untrusted SQL with read-only authorizer
                conn.authorizer(Some(read_only_authorizer));
                let result = Self::query_songs_with_conn(&conn, &query, &[]).unwrap_or_default();
                conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
                result
            } else {
                let query = format!(
                    "SELECT DISTINCT md5, song.sha256 AS sha256, title, subtitle, genre, artist, subartist, \
                     tag, path, folder, stagefile, banner, backbmp, preview, parent, level, difficulty, \
                     maxbpm, minbpm, length, song.mode AS mode, judge, feature, content, \
                     song.date AS date, favorite, adddate, song.notes AS notes, charthash \
                     FROM song LEFT OUTER JOIN (score LEFT OUTER JOIN scorelog ON score.sha256 = scorelog.sha256) ON song.sha256 = score.sha256 WHERE {}",
                    sql
                );
                // Guard untrusted SQL with read-only authorizer
                conn.authorizer(Some(read_only_authorizer));
                let result = Self::query_songs_with_conn(&conn, &query, &[]).unwrap_or_default();
                conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
                result
            };

            Ok(remove_invalid_elements_vec(songs))
        })();

        // Always detach in reverse order, regardless of success or failure.
        if attached_info {
            let _ = conn.execute("DETACH DATABASE infodb", []);
        }
        if attached_scorelog {
            let _ = conn.execute("DETACH DATABASE scorelogdb", []);
        }
        if attached_score {
            let _ = conn.execute("DETACH DATABASE scoredb", []);
        }

        match result {
            Ok(songs) => songs,
            Err(e) => {
                log::error!("Error in getSongDatas with SQL: {}", e);
                Vec::new()
            }
        }
    }

    fn song_datas_by_text(&self, text: &str) -> Vec<SongData> {
        // Try FTS5 first: convert search terms to prefix-match query
        let fts_query = Self::build_fts5_query(text);
        if !fts_query.is_empty() {
            let sql = "SELECT song.* FROM song JOIN song_fts ON song.rowid = song_fts.rowid \
                       WHERE song_fts MATCH ?1 GROUP BY sha256";
            let songs = self.query_songs(sql, &[&fts_query as &dyn rusqlite::types::ToSql]);
            if !songs.is_empty() {
                return remove_invalid_elements_vec(songs);
            }
        }

        // Fallback to LIKE for substring-within-word matches
        let sql = "SELECT * FROM song WHERE rtrim(title||' '||subtitle||' '||artist||' '||subartist||' '||genre) LIKE ?1 ESCAPE '\\' GROUP BY sha256";
        let escaped = escape_sql_like(text);
        let pattern = format!("%{}%", escaped);
        let songs = self.query_songs(sql, &[&pattern as &dyn rusqlite::types::ToSql]);
        remove_invalid_elements_vec(songs)
    }

    fn folder_datas(&self, key: &str, value: &str) -> Vec<FolderData> {
        // Whitelist valid column names to prevent SQL injection via key parameter
        const VALID_COLUMNS: &[&str] = &["path", "parent", "title", "type", "date"];
        if !VALID_COLUMNS.contains(&key) {
            log::warn!("Invalid column name for folder query: {}", key);
            return Vec::new();
        }
        let sql = format!("SELECT * FROM folder WHERE [{}] = ?1", key);
        self.query_folders(&sql, &[&value as &dyn rusqlite::types::ToSql])
    }

    fn set_song_datas(&self, songs: &[SongData]) -> anyhow::Result<()> {
        let mut conn = lock_or_recover(&self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| anyhow::anyhow!("Error starting transaction: {e}"))?;

        for sd in songs {
            if let Err(e) = Self::insert_song_with_conn(&self.base, &tx, sd) {
                log::error!("Error inserting song, rolling back: {}", e);
                tx.rollback().map_err(|re| {
                    anyhow::anyhow!("Rollback failed after insert error ({e}): {re}")
                })?;
                return Err(anyhow::anyhow!("Error inserting song: {e}"));
            }
        }

        tx.commit()
            .map_err(|e| anyhow::anyhow!("Error committing transaction: {e}"))?;
        Ok(())
    }

    fn update_song_datas(
        &self,
        update_path: Option<&str>,
        bmsroot: &[String],
        update_all: bool,
        update_parent_when_missing: bool,
    ) {
        // Delegate to inherent method with info: None
        SQLiteSongDatabaseAccessor::update_song_datas(
            self,
            update_path,
            bmsroot,
            update_all,
            update_parent_when_missing,
            None,
        );
    }
}

impl SQLiteSongDatabaseAccessor {
    /// Build an FTS5 MATCH query from user search text.
    /// Each whitespace-separated token becomes a prefix query (token*).
    /// FTS5 special characters are escaped by double-quoting each token.
    fn build_fts5_query(text: &str) -> String {
        text.split_whitespace()
            .map(|token| {
                let escaped = token.replace('"', "\"\"");
                format!("\"{}\"*", escaped)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

// Update methods kept as inherent methods (not part of the trait in beatoraja-types)
// because they depend on SongDatabaseUpdateListener/SongInformationAccessor from beatoraja-song.
impl SQLiteSongDatabaseAccessor {
    pub fn update_song_datas(
        &self,
        update_path: Option<&str>,
        bmsroot: &[String],
        update_all: bool,
        update_parent_when_missing: bool,
        info: Option<&dyn SongInformationDb>,
    ) {
        let listener = SongDatabaseUpdateListener::new();
        self.update_song_datas_with_listener(
            update_path,
            bmsroot,
            update_all,
            update_parent_when_missing,
            info,
            &listener,
        );
    }

    pub fn update_song_datas_with_listener(
        &self,
        update_path: Option<&str>,
        bmsroot: &[String],
        update_all: bool,
        update_parent_when_missing: bool,
        info: Option<&dyn SongInformationDb>,
        listener: &SongDatabaseUpdateListener,
    ) {
        if bmsroot.is_empty() {
            log::warn!("No BMS root folders registered");
            return;
        }

        let mut path = update_path.map(|s| s.to_string());

        if update_parent_when_missing && let Some(ref p) = path {
            let parent = Path::new(p)
                .parent()
                .map(|pp| pp.to_string_lossy().to_string())
                .unwrap_or_default();
            // Release checked_parent lock before calling query_folders(), which
            // acquires the conn lock, to avoid nested Mutex acquisition deadlock.
            let needs_parent_check = {
                let checked = lock_or_recover(&self.checked_parent);
                !checked.contains(&parent)
            };
            if needs_parent_check {
                let query = "SELECT * FROM folder WHERE path = ?1";
                let folders = self.query_folders(query, &[&parent as &dyn rusqlite::types::ToSql]);
                let mut checked = lock_or_recover(&self.checked_parent);
                checked.insert(parent.clone());
                if folders.is_empty() {
                    path = Some(parent);
                }
            }
        }

        let updater = SongDatabaseUpdater {
            update_all,
            bmsroot: bmsroot.to_vec(),
            info,
        };

        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let paths: Vec<PathBuf> = if let Some(p) = &path {
            vec![PathBuf::from(p)]
        } else {
            bmsroot.iter().map(PathBuf::from).collect()
        };

        updater.update_song_datas(self, &paths, listener);

        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let count = listener.bms_files_count();
        if count > 0 {
            log::info!(
                "Song update completed: Time - {}ms, per song - {}ms",
                end_time - start_time,
                (end_time - start_time) / (count as u128)
            );
        } else {
            log::info!(
                "Song update completed: Time - {}ms, per song - unknown",
                end_time - start_time
            );
        }
    }
}

struct SongDatabaseUpdater<'a> {
    update_all: bool,
    bmsroot: Vec<String>,
    info: Option<&'a dyn SongInformationDb>,
}

impl<'a> SongDatabaseUpdater<'a> {
    fn update_song_datas(
        &self,
        accessor: &SQLiteSongDatabaseAccessor,
        paths: &[PathBuf],
        listener: &SongDatabaseUpdateListener,
    ) {
        let updatetime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut property = SongDatabaseUpdaterProperty {
            tags: HashMap::new(),
            favorites: HashMap::new(),
            info: self.info,
            updatetime,
            listener,
        };

        if let Some(info) = self.info {
            let _ = info.start_update();
        }

        // Hold the lock for the entire transaction to prevent interleaving.
        // Connection is passed through to all DB operations via _with_conn methods.
        let mut conn = lock_or_recover(&accessor.conn);
        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(e) => {
                log::error!("Error starting transaction: {}", e);
                if let Some(info) = self.info {
                    info.end_update();
                }
                return;
            }
        };

        // Preserve tags and favorites.
        // On error, dropping `tx` auto-rolls-back the transaction.
        {
            let preserve_result: anyhow::Result<()> = (|| {
                let mut stmt = tx.prepare("SELECT sha256, tag, favorite FROM song")?;
                let rows = stmt.query_map([], |row| {
                    let sha256: String = row.get::<_, String>(0).unwrap_or_default();
                    let tag: String = row.get::<_, String>(1).unwrap_or_default();
                    let favorite: i32 = row.get::<_, i32>(2).unwrap_or(0);
                    Ok((sha256, tag, favorite))
                })?;
                for row in rows.flatten() {
                    let (sha256, tag, favorite) = row;
                    if !tag.is_empty() {
                        property.tags.insert(sha256.clone(), tag);
                    }
                    if favorite > 0 {
                        property.favorites.insert(sha256, favorite);
                    }
                }
                Ok(())
            })();
            if let Err(e) = preserve_result {
                log::error!("Error preserving tags/favorites: {}", e);
                drop(tx);
                if let Some(info) = self.info {
                    info.end_update();
                }
                return;
            }
        }

        if self.update_all {
            if let Err(e) = tx.execute("DELETE FROM folder", []) {
                log::warn!("Failed to delete all folder entries: {}", e);
            }
            if let Err(e) = tx.execute("DELETE FROM song", []) {
                log::warn!("Failed to delete all song entries: {}", e);
            }
        } else {
            // Filter out empty bmsroot entries: an empty string produces
            // LIKE '%' which matches ALL rows and would delete everything.
            let roots: Vec<&str> = self
                .bmsroot
                .iter()
                .filter(|r| !r.is_empty())
                .map(|r| r.as_str())
                .collect();

            if !roots.is_empty() {
                // Delete folders not contained in root directories
                let mut dsql = String::new();
                let mut params: Vec<String> = Vec::new();
                for (i, root) in roots.iter().enumerate() {
                    dsql.push_str("path NOT LIKE ? ESCAPE '\\'");
                    params.push(format!("{}%", escape_sql_like(root)));
                    if i < roots.len() - 1 {
                        dsql.push_str(" AND ");
                    }
                }

                let delete_folder_sql = format!(
                    "DELETE FROM folder WHERE path NOT LIKE 'LR2files%' AND path NOT LIKE '%.lr2folder' AND {}",
                    dsql
                );
                let delete_song_sql = format!(
                    "DELETE FROM song WHERE path NOT LIKE 'LR2files%' AND path NOT LIKE '%.lr2folder' AND {}",
                    dsql
                );

                // Execute with dynamic params
                let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
                    .iter()
                    .map(|p| p as &dyn rusqlite::types::ToSql)
                    .collect();
                if let Err(e) = tx.execute(&delete_folder_sql, param_refs.as_slice()) {
                    log::warn!("Failed to delete stale folder entries: {}", e);
                }
                let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
                    .iter()
                    .map(|p| p as &dyn rusqlite::types::ToSql)
                    .collect();
                if let Err(e) = tx.execute(&delete_song_sql, param_refs.as_slice()) {
                    log::warn!("Failed to delete stale song entries: {}", e);
                }
            }
        }

        // Process all paths serially while holding the transaction lock.
        // BMS file decoding (CPU-bound) is parallelized within each directory
        // via par_iter, but DB writes are serialized under the held transaction.
        let mut had_error = false;
        for p in paths {
            let folder = BMSFolder::new(p.clone(), &self.bmsroot);
            if let Err(e) = folder.process_directory(accessor, &tx, &property, &mut had_error) {
                log::error!("Error during song database update: {}", e);
                had_error = true;
            }
        }

        if had_error {
            log::error!("Rolling back song database refresh due to worker errors");
            if let Err(e) = tx.rollback() {
                log::error!("Error rolling back transaction: {}", e);
            }
        } else if let Err(e) = tx.commit() {
            log::error!("Error committing transaction: {}", e);
        }

        if let Some(info) = self.info {
            info.end_update();
        }
    }
}

struct BMSFolder {
    path: PathBuf,
    update_folder: bool,
    txt: bool,
    bmsfiles: Vec<PathBuf>,
    dirs: Vec<BMSFolder>,
    previewpath: Option<String>,
    bmsroot: Vec<String>,
}

impl BMSFolder {
    fn new(path: PathBuf, bmsroot: &[String]) -> Self {
        Self {
            path,
            update_folder: true,
            txt: false,
            bmsfiles: Vec::new(),
            dirs: Vec::new(),
            previewpath: None,
            bmsroot: bmsroot.to_vec(),
        }
    }

    fn process_directory(
        mut self,
        accessor: &SQLiteSongDatabaseAccessor,
        conn: &Connection,
        property: &SongDatabaseUpdaterProperty,
        had_error: &mut bool,
    ) -> anyhow::Result<()> {
        let root_str = accessor.root.to_string_lossy().to_string();

        let crc = song_utils::crc32(&self.path.to_string_lossy(), &self.bmsroot, &root_str);

        let records_sql = "SELECT * FROM song WHERE folder = ?1";
        let mut records: Vec<Option<SongData>> = SQLiteSongDatabaseAccessor::query_songs_with_conn(
            conn,
            records_sql,
            &[&crc as &dyn rusqlite::types::ToSql],
        )
        .unwrap_or_else(|e| {
            log::error!("Error querying songs: {}", e);
            Vec::new()
        })
        .into_iter()
        .map(Some)
        .collect();

        let folders_sql = "SELECT * FROM folder WHERE parent = ?1";
        let mut folders: Vec<Option<FolderData>> =
            SQLiteSongDatabaseAccessor::query_folders_with_conn(
                conn,
                folders_sql,
                &[&crc as &dyn rusqlite::types::ToSql],
            )
            .unwrap_or_else(|e| {
                log::error!("Error querying folders: {}", e);
                Vec::new()
            })
            .into_iter()
            .map(Some)
            .collect();

        // Scan directory
        let mut auto_preview_file: Option<String> = None;

        let read_dir_result = fs::read_dir(&self.path);
        if let Err(ref e) = read_dir_result {
            log::error!(
                "Cannot read directory {:?}, preserving existing DB records: {}",
                self.path,
                e
            );
            // Mark all records as matched so they are not deleted as "missing"
            for record in records.iter_mut() {
                *record = None;
            }
            for folder in folders.iter_mut() {
                *folder = None;
            }
        }
        if let Ok(entries) = read_dir_result {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    self.dirs.push(BMSFolder::new(entry_path, &self.bmsroot));
                } else {
                    let filename = entry_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let s = filename.to_lowercase();

                    if !self.txt && s.ends_with(".txt") {
                        self.txt = true;
                    }
                    if self.previewpath.is_none()
                        && s.starts_with("preview")
                        && (s.ends_with(".wav")
                            || s.ends_with(".ogg")
                            || s.ends_with(".mp3")
                            || s.ends_with(".flac"))
                    {
                        if s.starts_with("preview_auto_generator") {
                            auto_preview_file = Some(filename);
                        } else {
                            self.previewpath = Some(filename);
                        }
                    }
                    if s.ends_with(".bms")
                        || s.ends_with(".bme")
                        || s.ends_with(".bml")
                        || s.ends_with(".pms")
                        || s.ends_with(".bmson")
                        || s.ends_with(".osu")
                    {
                        self.bmsfiles.push(entry_path);
                    }
                }
            }
        }

        if self.previewpath.is_none() && auto_preview_file.is_some() {
            self.previewpath = auto_preview_file;
        }

        let contains_bms = !self.bmsfiles.is_empty();
        property
            .listener
            .add_bms_files_count(self.bmsfiles.len().min(i32::MAX as usize) as i32);

        let (skip_count, new_count) =
            self.process_bms_folder(&mut records, accessor, conn, property);
        property
            .listener
            .add_processed_bms_files_count(skip_count + new_count);
        property.listener.add_new_bms_files_count(new_count);

        // Match existing folders with dir entries
        let folders_len = folders.len();
        for bf in &mut self.dirs {
            let s = if bf.path.starts_with(&accessor.root) {
                let rel = accessor.root.as_path();
                let relative = bf.path.strip_prefix(rel).unwrap_or(&bf.path);
                format!("{}{}", relative.display(), std::path::MAIN_SEPARATOR)
            } else {
                format!("{}{}", bf.path.display(), std::path::MAIN_SEPARATOR)
            };

            for folder_opt in folders.iter_mut().take(folders_len) {
                let matched = if let Some(record) = folder_opt.as_ref() {
                    if record.path == s {
                        Some(record.date)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(record_date) = matched {
                    *folder_opt = None;
                    if let Ok(metadata) = fs::metadata(&bf.path)
                        && let Ok(modified) = metadata.modified()
                    {
                        let modified_secs = modified
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64;
                        if record_date == modified_secs {
                            bf.update_folder = false;
                        }
                    }
                    break;
                }
            }
        }

        if !contains_bms {
            // Serial subdirectory recursion with connection passed through.
            // Connection is held for the entire transaction to prevent interleaving.
            let dirs = std::mem::take(&mut self.dirs);
            for bf in dirs {
                if let Err(e) = bf.process_directory(accessor, conn, property, had_error) {
                    log::error!("Error during song database update: {}", e);
                    *had_error = true;
                }
            }
        }

        // Update folder table
        if self.update_folder {
            let s = if self.path.starts_with(&accessor.root) {
                let relative = self.path.strip_prefix(&accessor.root).unwrap_or(&self.path);
                format!("{}{}", relative.display(), std::path::MAIN_SEPARATOR)
            } else {
                format!("{}{}", self.path.display(), std::path::MAIN_SEPARATOR)
            };

            let parentpath = self
                .path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| {
                    std::fs::canonicalize(&self.path)
                        .unwrap_or_else(|_| self.path.clone())
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .to_path_buf()
                });

            let folder_date = fs::metadata(&self.path)
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
                .unwrap_or(0);

            let folder = FolderData {
                title: self
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: s,
                parent: song_utils::crc32(&parentpath.to_string_lossy(), &self.bmsroot, &root_str),
                date: folder_date,
                adddate: property.updatetime,
                ..Default::default()
            };

            if let Err(e) =
                SQLiteSongDatabaseAccessor::insert_folder_with_conn(&accessor.base, conn, &folder)
            {
                log::error!("Error inserting folder: {}", e);
                *had_error = true;
            }
        }

        // Delete folder records that no longer exist in directory
        for folder in folders.into_iter().flatten() {
            let delete_path = format!("{}%", escape_sql_like(&folder.path));
            let _ = conn.execute(
                "DELETE FROM folder WHERE path LIKE ?1 ESCAPE '\\'",
                rusqlite::params![delete_path],
            );
            let _ = conn.execute(
                "DELETE FROM song WHERE path LIKE ?1 ESCAPE '\\'",
                rusqlite::params![delete_path],
            );
        }

        Ok(())
    }

    fn process_bms_folder(
        &self,
        records: &mut [Option<SongData>],
        accessor: &SQLiteSongDatabaseAccessor,
        conn: &Connection,
        property: &SongDatabaseUpdaterProperty,
    ) -> (i32, i32) {
        let mut skip_count = 0i32;
        let mut new_count = 0i32;
        let root_str = accessor.root.to_string_lossy().to_string();

        // Phase 1: Determine which files need parsing (check against existing records).
        struct FileToProcess {
            bmsfile_path: PathBuf,
            pathname: String,
            last_modified_time: i64,
        }
        let mut files_to_process: Vec<FileToProcess> = Vec::new();

        for bmsfile_path in &self.bmsfiles {
            let last_modified_time: i64 = fs::metadata(bmsfile_path)
                .and_then(|m| m.modified())
                .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
                .unwrap_or(-1);

            let pathname = if bmsfile_path.starts_with(&accessor.root) {
                accessor
                    .root
                    .as_path()
                    .strip_prefix(&accessor.root)
                    .ok()
                    .and_then(|_| bmsfile_path.strip_prefix(&accessor.root).ok())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| bmsfile_path.to_string_lossy().to_string())
            } else {
                bmsfile_path.to_string_lossy().to_string()
            };

            let mut update = true;
            for record in records.iter_mut() {
                let matched = if let Some(rec) = record.as_ref() {
                    rec.file.path() == Some(&pathname)
                } else {
                    false
                };
                if matched {
                    // Accepted trade-off: skip re-parsing when chart mtime is unchanged,
                    // matching Java's incremental scan. Preview audio changes without chart
                    // edits require a full rescan.
                    if let Some(rec) = record.as_ref()
                        && rec.chart.date == last_modified_time
                    {
                        update = false;
                    }
                    *record = None;
                    break;
                }
            }

            if !update {
                skip_count += 1;
                continue;
            }

            files_to_process.push(FileToProcess {
                bmsfile_path: bmsfile_path.clone(),
                pathname,
                last_modified_time,
            });
        }

        // Phase 2: Parallel BMS file decoding (CPU-bound, no DB access).
        let decoded: Vec<(String, i64, Option<SongData>)> = files_to_process
            .par_iter()
            .map(|file_info| {
                let model: Option<BMSModel> = if file_info
                    .pathname
                    .to_lowercase()
                    .ends_with(".bmson")
                {
                    let mut decoder = BMSONDecoder::new(LNTYPE_LONGNOTE);
                    match decoder.decode_path(&file_info.bmsfile_path) {
                        Some(m) => Some(m),
                        None => {
                            log::error!(
                                "Error while decoding bmson at path: {}",
                                file_info.pathname
                            );
                            None
                        }
                    }
                } else if file_info.pathname.to_lowercase().ends_with(".osu") {
                    let mut decoder = OSUDecoder::new(LNTYPE_LONGNOTE);
                    match decoder.decode_path(&file_info.bmsfile_path) {
                        Some(m) => Some(m),
                        None => {
                            log::error!("Error while decoding osu at path: {}", file_info.pathname);
                            None
                        }
                    }
                } else {
                    let mut decoder = BMSDecoder::new_with_lntype(LNTYPE_LONGNOTE);
                    match decoder.decode_path(&file_info.bmsfile_path) {
                        Some(m) => Some(m),
                        None => {
                            log::error!("Error while decoding bms at path: {}", file_info.pathname);
                            None
                        }
                    }
                };

                let sd = model.map(|m| SongData::new_from_model(m, self.txt));
                (file_info.pathname.clone(), file_info.last_modified_time, sd)
            })
            .collect();

        // Phase 3: Serial DB inserts under the held connection.
        for (pathname, last_modified_time, sd_opt) in &decoded {
            let mut sd = match sd_opt {
                Some(sd) => sd.clone(),
                None => continue,
            };

            let bmsfile_path = Path::new(pathname);

            if sd.chart.notes != 0 || !sd.model.as_ref().is_none_or(|m| m.wavmap.is_empty()) {
                if sd.chart.difficulty == 0 {
                    let fulltitle =
                        format!("{}{}", sd.metadata.title, sd.metadata.subtitle).to_lowercase();
                    let diffname = sd.metadata.subtitle.to_lowercase();
                    if diffname.contains("beginner") {
                        sd.chart.difficulty = 1;
                    } else if diffname.contains("normal") {
                        sd.chart.difficulty = 2;
                    } else if diffname.contains("hyper") {
                        sd.chart.difficulty = 3;
                    } else if diffname.contains("another") {
                        sd.chart.difficulty = 4;
                    } else if diffname.contains("insane") || diffname.contains("leggendaria") {
                        sd.chart.difficulty = 5;
                    } else if fulltitle.contains("beginner") {
                        sd.chart.difficulty = 1;
                    } else if fulltitle.contains("normal") {
                        sd.chart.difficulty = 2;
                    } else if fulltitle.contains("hyper") {
                        sd.chart.difficulty = 3;
                    } else if fulltitle.contains("another") {
                        sd.chart.difficulty = 4;
                    } else if fulltitle.contains("insane") || fulltitle.contains("leggendaria") {
                        sd.chart.difficulty = 5;
                    } else if sd.chart.notes < 250 {
                        sd.chart.difficulty = 1;
                    } else if sd.chart.notes < 600 {
                        sd.chart.difficulty = 2;
                    } else if sd.chart.notes < 1000 {
                        sd.chart.difficulty = 3;
                    } else if sd.chart.notes < 2000 {
                        sd.chart.difficulty = 4;
                    } else {
                        sd.chart.difficulty = 5;
                    }
                }

                if sd.file.preview.is_empty()
                    && let Some(ref preview) = self.previewpath
                {
                    sd.file.preview = preview.clone();
                }

                let tag = property
                    .tags
                    .get(&sd.file.sha256)
                    .cloned()
                    .unwrap_or_default();
                let favorite = property
                    .favorites
                    .get(&sd.file.sha256)
                    .copied()
                    .unwrap_or(0);

                // Plugin updates
                for plugin in &accessor.plugins {
                    if let Some(ref model) = sd.model {
                        let mut sd_clone = sd.clone();
                        plugin.update(model, &mut sd_clone);
                        sd = sd_clone;
                    }
                }

                sd.metadata.tag = tag;
                sd.file.set_path(pathname.clone());

                if let Some(parent_path) = bmsfile_path.parent() {
                    sd.folder =
                        song_utils::crc32(&parent_path.to_string_lossy(), &self.bmsroot, &root_str);
                    if let Some(grandparent) = parent_path.parent() {
                        sd.parent = song_utils::crc32(
                            &grandparent.to_string_lossy(),
                            &self.bmsroot,
                            &root_str,
                        );
                    }
                }
                sd.chart.date = *last_modified_time;
                sd.favorite = favorite;
                sd.chart.adddate = property.updatetime;

                if let Err(e) =
                    SQLiteSongDatabaseAccessor::insert_song_with_conn(&accessor.base, conn, &sd)
                {
                    log::error!("Error inserting song: {}", e);
                }

                if let Some(info) = property.info
                    && let Some(ref model) = sd.model
                {
                    info.update(model);
                }

                new_count += 1;
            } else {
                let _ = conn.execute(
                    "DELETE FROM song WHERE path = ?1",
                    rusqlite::params![pathname],
                );
            }
        }

        // Delete records that no longer exist in directory
        for record in records.iter().flatten() {
            if let Some(path) = record.file.path() {
                let _ = conn.execute("DELETE FROM song WHERE path = ?1", rusqlite::params![path]);
            }
        }

        (skip_count, new_count)
    }
}

struct SongDatabaseUpdaterProperty<'a> {
    tags: HashMap<String, String>,
    favorites: HashMap<String, i32>,
    info: Option<&'a dyn SongInformationDb>,
    updatetime: i64,
    listener: &'a SongDatabaseUpdateListener,
}

#[cfg(test)]
mod tests;
