use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use beatoraja_core::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};
use beatoraja_core::validatable::remove_invalid_elements_vec;
use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::{BMSModel, LNTYPE_LONGNOTE};
use bms_model::bmson_decoder::BMSONDecoder;
use bms_model::osu_decoder::OSUDecoder;
use rayon::prelude::*;
use rusqlite::Connection;

use crate::folder_data::FolderData;
use crate::song_data::SongData;
use crate::song_database_accessor::SongDatabaseAccessor;
use crate::song_database_update_listener::SongDatabaseUpdateListener;
use crate::song_utils;
use beatoraja_types::song_information_db::SongInformationDb;

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
    checked_parent: HashSet<String>,
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
            "PRAGMA shared_cache = ON; PRAGMA synchronous = OFF; PRAGMA recursive_triggers = ON;",
        )?;
        let root = PathBuf::from(".");

        let accessor = Self {
            base,
            conn: Mutex::new(conn),
            root,
            plugins: Vec::new(),
            checked_parent: HashSet::new(),
        };
        accessor.create_table()?;
        Ok(accessor)
    }

    pub fn add_plugin(&mut self, plugin: Box<dyn SongDatabaseAccessorPlugin>) {
        self.plugins.push(plugin);
    }

    fn create_table(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
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
            conn.execute("ALTER TABLE [song] RENAME TO [old_song]", [])?;
            self.base.validate(&conn)?;
            conn.execute(
                "INSERT INTO song SELECT \
                 md5, sha256, title, subtitle, genre, artist, subartist, tag, path,\
                 folder, stagefile, banner, backbmp, preview, parent, level, difficulty,\
                 maxbpm, minbpm, length, mode, judge, feature, content,\
                 date, favorite, notes, adddate, charthash \
                 FROM old_song GROUP BY path HAVING MAX(adddate)",
                [],
            )?;
            conn.execute("DROP TABLE old_song", [])?;
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
        let conn = self.conn.lock().unwrap();
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
            sd.md5 = row.get::<_, String>(0).unwrap_or_default();
            sd.sha256 = row.get::<_, String>(1).unwrap_or_default();
            sd.title = row.get::<_, String>(2).unwrap_or_default();
            sd.subtitle = row.get::<_, String>(3).unwrap_or_default();
            sd.genre = row.get::<_, String>(4).unwrap_or_default();
            sd.artist = row.get::<_, String>(5).unwrap_or_default();
            sd.subartist = row.get::<_, String>(6).unwrap_or_default();
            sd.tag = row.get::<_, String>(7).unwrap_or_default();
            let path: String = row.get::<_, String>(8).unwrap_or_default();
            sd.set_path(path);
            sd.folder = row.get::<_, String>(9).unwrap_or_default();
            sd.stagefile = row.get::<_, String>(10).unwrap_or_default();
            sd.banner = row.get::<_, String>(11).unwrap_or_default();
            sd.backbmp = row.get::<_, String>(12).unwrap_or_default();
            sd.preview = row.get::<_, String>(13).unwrap_or_default();
            sd.parent = row.get::<_, String>(14).unwrap_or_default();
            sd.level = row.get::<_, i32>(15).unwrap_or(0);
            sd.difficulty = row.get::<_, i32>(16).unwrap_or(0);
            sd.maxbpm = row.get::<_, i32>(17).unwrap_or(0);
            sd.minbpm = row.get::<_, i32>(18).unwrap_or(0);
            sd.length = row.get::<_, i32>(19).unwrap_or(0);
            sd.mode = row.get::<_, i32>(20).unwrap_or(0);
            sd.judge = row.get::<_, i32>(21).unwrap_or(0);
            sd.feature = row.get::<_, i32>(22).unwrap_or(0);
            sd.content = row.get::<_, i32>(23).unwrap_or(0);
            sd.date = row.get::<_, i32>(24).unwrap_or(0);
            sd.favorite = row.get::<_, i32>(25).unwrap_or(0);
            sd.adddate = row.get::<_, i32>(26).unwrap_or(0);
            sd.notes = row.get::<_, i32>(27).unwrap_or(0);
            sd.charthash = row.get::<_, Option<String>>(28).unwrap_or(None);
            Ok(sd)
        })?;
        let mut result = Vec::new();
        for sd in rows.flatten() {
            result.push(sd);
        }
        Ok(result)
    }

    fn query_folders(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql]) -> Vec<FolderData> {
        let conn = self.conn.lock().unwrap();
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
                date: row.get::<_, i32>(7).unwrap_or(0),
                adddate: row.get::<_, i32>(8).unwrap_or(0),
                max: row.get::<_, i32>(9).unwrap_or(0),
            })
        })?;
        let mut result = Vec::new();
        for fd in rows.flatten() {
            result.push(fd);
        }
        Ok(result)
    }

    fn insert_song(&self, sd: &SongData) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        self.base
            .insert_with_values(&conn, "song", &|name: &str| -> rusqlite::types::Value {
                match name {
                    "md5" => rusqlite::types::Value::Text(sd.md5.clone()),
                    "sha256" => rusqlite::types::Value::Text(sd.sha256.clone()),
                    "title" => rusqlite::types::Value::Text(sd.title.clone()),
                    "subtitle" => rusqlite::types::Value::Text(sd.subtitle.clone()),
                    "genre" => rusqlite::types::Value::Text(sd.genre.clone()),
                    "artist" => rusqlite::types::Value::Text(sd.artist.clone()),
                    "subartist" => rusqlite::types::Value::Text(sd.subartist.clone()),
                    "tag" => rusqlite::types::Value::Text(sd.tag.clone()),
                    "path" => rusqlite::types::Value::Text(sd.get_path().unwrap_or("").to_string()),
                    "folder" => rusqlite::types::Value::Text(sd.folder.clone()),
                    "stagefile" => rusqlite::types::Value::Text(sd.stagefile.clone()),
                    "banner" => rusqlite::types::Value::Text(sd.banner.clone()),
                    "backbmp" => rusqlite::types::Value::Text(sd.backbmp.clone()),
                    "preview" => rusqlite::types::Value::Text(sd.preview.clone()),
                    "parent" => rusqlite::types::Value::Text(sd.parent.clone()),
                    "level" => rusqlite::types::Value::Integer(sd.level as i64),
                    "difficulty" => rusqlite::types::Value::Integer(sd.difficulty as i64),
                    "maxbpm" => rusqlite::types::Value::Integer(sd.maxbpm as i64),
                    "minbpm" => rusqlite::types::Value::Integer(sd.minbpm as i64),
                    "length" => rusqlite::types::Value::Integer(sd.length as i64),
                    "mode" => rusqlite::types::Value::Integer(sd.mode as i64),
                    "judge" => rusqlite::types::Value::Integer(sd.judge as i64),
                    "feature" => rusqlite::types::Value::Integer(sd.feature as i64),
                    "content" => rusqlite::types::Value::Integer(sd.content as i64),
                    "date" => rusqlite::types::Value::Integer(sd.date as i64),
                    "favorite" => rusqlite::types::Value::Integer(sd.favorite as i64),
                    "adddate" => rusqlite::types::Value::Integer(sd.adddate as i64),
                    "notes" => rusqlite::types::Value::Integer(sd.notes as i64),
                    "charthash" => match &sd.charthash {
                        Some(h) => rusqlite::types::Value::Text(h.clone()),
                        None => rusqlite::types::Value::Null,
                    },
                    _ => rusqlite::types::Value::Null,
                }
            })
    }

    fn insert_folder(&self, fd: &FolderData) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        self.base
            .insert_with_values(&conn, "folder", &|name: &str| -> rusqlite::types::Value {
                match name {
                    "title" => rusqlite::types::Value::Text(fd.title.clone()),
                    "subtitle" => rusqlite::types::Value::Text(fd.subtitle.clone()),
                    "command" => rusqlite::types::Value::Text(fd.command.clone()),
                    "path" => rusqlite::types::Value::Text(fd.path.clone()),
                    "banner" => rusqlite::types::Value::Text(fd.banner.clone()),
                    "parent" => rusqlite::types::Value::Text(fd.parent.clone()),
                    "type" => rusqlite::types::Value::Integer(fd.folder_type as i64),
                    "date" => rusqlite::types::Value::Integer(fd.date as i64),
                    "adddate" => rusqlite::types::Value::Integer(fd.adddate as i64),
                    "max" => rusqlite::types::Value::Integer(fd.max as i64),
                    _ => rusqlite::types::Value::Null,
                }
            })
    }
}

impl SongDatabaseAccessor for SQLiteSongDatabaseAccessor {
    fn get_song_datas(&self, key: &str, value: &str) -> Vec<SongData> {
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

    fn get_song_datas_by_hashes(&self, hashes: &[String]) -> Vec<SongData> {
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

        // Build parameterized IN clause
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut conditions = Vec::new();

        if !md5_hashes.is_empty() {
            let placeholders: Vec<String> = md5_hashes
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", params.len() + i + 1))
                .collect();
            conditions.push(format!("md5 IN ({})", placeholders.join(",")));
            for h in &md5_hashes {
                params.push(Box::new(h.to_string()));
            }
        }

        if !sha256_hashes.is_empty() {
            let placeholders: Vec<String> = sha256_hashes
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", params.len() + i + 1))
                .collect();
            conditions.push(format!("sha256 IN ({})", placeholders.join(",")));
            for h in &sha256_hashes {
                params.push(Box::new(h.to_string()));
            }
        }

        let sql = format!("SELECT * FROM song WHERE {}", conditions.join(" OR "));
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        let m = self.query_songs(&sql, &param_refs);

        // Preserve search order
        let mut sorted = m;
        sorted.sort_by(|a, b| {
            let mut a_index_sha256 = -1i32;
            let mut a_index_md5 = -1i32;
            let mut b_index_sha256 = -1i32;
            let mut b_index_md5 = -1i32;
            for (i, hash) in hashes.iter().enumerate() {
                if hash == &a.sha256 {
                    a_index_sha256 = i as i32;
                }
                if hash == a.get_md5() {
                    a_index_md5 = i as i32;
                }
                if hash == &b.sha256 {
                    b_index_sha256 = i as i32;
                }
                if hash == b.get_md5() {
                    b_index_md5 = i as i32;
                }
            }
            let a_index = std::cmp::min(
                if a_index_sha256 == -1 {
                    i32::MAX
                } else {
                    a_index_sha256
                },
                if a_index_md5 == -1 {
                    i32::MAX
                } else {
                    a_index_md5
                },
            );
            let b_index = std::cmp::min(
                if b_index_sha256 == -1 {
                    i32::MAX
                } else {
                    b_index_sha256
                },
                if b_index_md5 == -1 {
                    i32::MAX
                } else {
                    b_index_md5
                },
            );
            // Java: return bIndex - aIndex (descending)
            b_index.cmp(&a_index)
        });

        remove_invalid_elements_vec(sorted)
    }

    fn get_song_datas_by_sql(
        &self,
        sql: &str,
        score: &str,
        scorelog: &str,
        info: Option<&str>,
    ) -> Vec<SongData> {
        let conn = self.conn.lock().unwrap();
        let result: anyhow::Result<Vec<SongData>> = (|| {
            // ATTACH DATABASE doesn't support parameterized paths; escape single quotes
            let score_escaped = score.replace('\'', "''");
            let scorelog_escaped = scorelog.replace('\'', "''");
            conn.execute(
                &format!("ATTACH DATABASE '{}' as scoredb", score_escaped),
                [],
            )?;
            conn.execute(
                &format!("ATTACH DATABASE '{}' as scorelogdb", scorelog_escaped),
                [],
            )?;

            let songs = if let Some(info_path) = info {
                let info_escaped = info_path.replace('\'', "''");
                conn.execute(&format!("ATTACH DATABASE '{}' as infodb", info_escaped), [])?;
                let query = format!(
                    "SELECT DISTINCT md5, song.sha256 AS sha256, title, subtitle, genre, artist, subartist,path,folder,stagefile,banner,backbmp,parent,level,difficulty,\
                     maxbpm,minbpm,song.mode AS mode, judge, feature, content, song.date AS date, favorite, song.notes AS notes, adddate, preview, length, charthash\
                     FROM song INNER JOIN (information LEFT OUTER JOIN (score LEFT OUTER JOIN scorelog ON score.sha256 = scorelog.sha256) ON information.sha256 = score.sha256) \
                     ON song.sha256 = information.sha256 WHERE {}",
                    sql
                );
                let songs = Self::query_songs_with_conn(&conn, &query, &[]).unwrap_or_default();
                let _ = conn.execute("DETACH DATABASE infodb", []);
                songs
            } else {
                let query = format!(
                    "SELECT DISTINCT md5, song.sha256 AS sha256, title, subtitle, genre, artist, subartist,path,folder,stagefile,banner,backbmp,parent,level,difficulty,\
                     maxbpm,minbpm,song.mode AS mode, judge, feature, content, song.date AS date, favorite, song.notes AS notes, adddate, preview, length, charthash\
                     FROM song LEFT OUTER JOIN (score LEFT OUTER JOIN scorelog ON score.sha256 = scorelog.sha256) ON song.sha256 = score.sha256 WHERE {}",
                    sql
                );
                Self::query_songs_with_conn(&conn, &query, &[]).unwrap_or_default()
            };

            let _ = conn.execute("DETACH DATABASE scorelogdb", []);
            let _ = conn.execute("DETACH DATABASE scoredb", []);

            Ok(remove_invalid_elements_vec(songs))
        })();

        match result {
            Ok(songs) => songs,
            Err(e) => {
                log::error!("Error in getSongDatas with SQL: {}", e);
                Vec::new()
            }
        }
    }

    fn get_song_datas_by_text(&self, text: &str) -> Vec<SongData> {
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
        let sql = "SELECT * FROM song WHERE rtrim(title||' '||subtitle||' '||artist||' '||subartist||' '||genre) LIKE ?1 GROUP BY sha256";
        let pattern = format!("%{}%", text);
        let songs = self.query_songs(sql, &[&pattern as &dyn rusqlite::types::ToSql]);
        remove_invalid_elements_vec(songs)
    }

    fn get_folder_datas(&self, key: &str, value: &str) -> Vec<FolderData> {
        // Whitelist valid column names to prevent SQL injection via key parameter
        const VALID_COLUMNS: &[&str] = &["path", "parent", "title", "type", "date"];
        if !VALID_COLUMNS.contains(&key) {
            log::warn!("Invalid column name for folder query: {}", key);
            return Vec::new();
        }
        let sql = format!("SELECT * FROM folder WHERE [{}] = ?1", key);
        self.query_folders(&sql, &[&value as &dyn rusqlite::types::ToSql])
    }

    fn set_song_datas(&self, songs: &[SongData]) {
        {
            let conn = self.conn.lock().unwrap();
            if let Err(e) = conn.execute_batch("BEGIN TRANSACTION") {
                log::error!("Error starting transaction: {}", e);
                return;
            }
        }

        for sd in songs {
            if let Err(e) = self.insert_song(sd) {
                log::error!("Error inserting song: {}", e);
            }
        }

        let conn = self.conn.lock().unwrap();
        if let Err(e) = conn.execute_batch("COMMIT") {
            log::error!("Error committing transaction: {}", e);
        }
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
            if !self.checked_parent.contains(&parent) {
                let query = "SELECT * FROM folder WHERE path = ?1";
                let folders = self.query_folders(query, &[&parent as &dyn rusqlite::types::ToSql]);
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

        let count = listener.get_bms_files_count();
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

        // Acquire lock for transaction setup and tag/favorite preservation
        {
            let conn = accessor.conn.lock().unwrap();
            if let Err(e) = conn.execute_batch("BEGIN TRANSACTION") {
                log::error!("Error starting transaction: {}", e);
                return;
            }

            // Preserve tags and favorites
            {
                let mut stmt = match conn.prepare("SELECT sha256, tag, favorite FROM song") {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Error preparing tag/favorite query: {}", e);
                        return;
                    }
                };
                let rows = match stmt.query_map([], |row| {
                    let sha256: String = row.get::<_, String>(0).unwrap_or_default();
                    let tag: String = row.get::<_, String>(1).unwrap_or_default();
                    let favorite: i32 = row.get::<_, i32>(2).unwrap_or(0);
                    Ok((sha256, tag, favorite))
                }) {
                    Ok(r) => r,
                    Err(e) => {
                        log::error!("Error querying tags/favorites: {}", e);
                        return;
                    }
                };
                for row in rows.flatten() {
                    let (sha256, tag, favorite) = row;
                    if !tag.is_empty() {
                        property.tags.insert(sha256.clone(), tag);
                    }
                    if favorite > 0 {
                        property.favorites.insert(sha256, favorite);
                    }
                }
            }

            if self.update_all {
                let _ = conn.execute("DELETE FROM folder", []);
                let _ = conn.execute("DELETE FROM song", []);
            } else {
                // Delete folders not contained in root directories
                let mut dsql = String::new();
                let mut params: Vec<String> = Vec::new();
                for (i, root) in self.bmsroot.iter().enumerate() {
                    dsql.push_str("path NOT LIKE ?");
                    params.push(format!("{}%", root));
                    if i < self.bmsroot.len() - 1 {
                        dsql.push_str(" AND ");
                    }
                }

                let delete_folder_sql = format!(
                    "DELETE FROM folder WHERE path NOT LIKE 'LR2files%' AND path NOT LIKE '%.lr2folder' AND {}",
                    dsql
                );
                let delete_song_sql = format!("DELETE FROM song WHERE {}", dsql);

                // Execute with dynamic params
                let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
                    .iter()
                    .map(|p| p as &dyn rusqlite::types::ToSql)
                    .collect();
                let _ = conn.execute(&delete_folder_sql, param_refs.as_slice());
                let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
                    .iter()
                    .map(|p| p as &dyn rusqlite::types::ToSql)
                    .collect();
                let _ = conn.execute(&delete_song_sql, param_refs.as_slice());
            }
        } // Release lock before parallel section

        // Parallel processing of root paths (matches Java: paths.parallel().forEach(...))
        paths.par_iter().for_each(|p| {
            let folder = BMSFolder::new(p.clone(), &self.bmsroot);
            if let Err(e) = folder.process_directory(accessor, &property) {
                log::error!("Error during song database update: {}", e);
            }
        });

        let conn = accessor.conn.lock().unwrap();
        let _ = conn.execute_batch("COMMIT");

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
        property: &SongDatabaseUpdaterProperty,
    ) -> anyhow::Result<()> {
        let root_str = accessor.root.to_string_lossy().to_string();
        let bmsroot_strs: Vec<String> = self.bmsroot.clone();

        let crc = song_utils::crc32(&self.path.to_string_lossy(), &bmsroot_strs, &root_str);

        let records_sql = "SELECT * FROM song WHERE folder = ?1";
        let mut records: Vec<Option<SongData>> = accessor
            .query_songs(records_sql, &[&crc as &dyn rusqlite::types::ToSql])
            .into_iter()
            .map(Some)
            .collect();

        let folders_sql = "SELECT * FROM folder WHERE parent = ?1";
        let mut folders: Vec<Option<FolderData>> = accessor
            .query_folders(folders_sql, &[&crc as &dyn rusqlite::types::ToSql])
            .into_iter()
            .map(Some)
            .collect();

        // Scan directory
        let mut auto_preview_file: Option<String> = None;

        if let Ok(entries) = fs::read_dir(&self.path) {
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
                            auto_preview_file = Some(filename.clone());
                        } else {
                            self.previewpath = Some(filename.clone());
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
            .add_bms_files_count(self.bmsfiles.len() as i32);

        let (skip_count, new_count) = self.process_bms_folder(&mut records, accessor, property);
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
                            .as_secs() as i32;
                        if record_date == modified_secs {
                            bf.update_folder = false;
                        }
                    }
                    break;
                }
            }
        }

        if !contains_bms {
            // Parallel subdirectory recursion (matches Java: dirs.parallelStream().forEach(...))
            let dirs = std::mem::take(&mut self.dirs);
            dirs.into_par_iter().for_each(|bf| {
                if let Err(e) = bf.process_directory(accessor, property) {
                    log::error!("Error during song database update: {}", e);
                }
            });
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
                .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i32)
                .unwrap_or(0);

            let folder = FolderData {
                title: self
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: s,
                parent: song_utils::crc32(&parentpath.to_string_lossy(), &bmsroot_strs, &root_str),
                date: folder_date,
                adddate: property.updatetime as i32,
                ..Default::default()
            };

            if let Err(e) = accessor.insert_folder(&folder) {
                log::error!("Error inserting folder: {}", e);
            }
        }

        // Delete folder records that no longer exist in directory
        // (matches Java: folders.parallelStream().filter(Objects::nonNull).forEach(...))
        folders.into_par_iter().flatten().for_each(|folder| {
            let delete_path = format!("{}%", folder.path);
            let conn = accessor.conn.lock().unwrap();
            let _ = conn.execute(
                "DELETE FROM folder WHERE path LIKE ?1",
                rusqlite::params![delete_path],
            );
            let _ = conn.execute(
                "DELETE FROM song WHERE path LIKE ?1",
                rusqlite::params![delete_path],
            );
        });

        Ok(())
    }

    fn process_bms_folder(
        &self,
        records: &mut [Option<SongData>],
        accessor: &SQLiteSongDatabaseAccessor,
        property: &SongDatabaseUpdaterProperty,
    ) -> (i32, i32) {
        let mut skip_count = 0i32;
        let mut new_count = 0i32;
        let mut bmsdecoder: Option<BMSDecoder> = None;
        let mut bmsondecoder: Option<BMSONDecoder> = None;
        let mut osudecoder: Option<OSUDecoder> = None;
        let root_str = accessor.root.to_string_lossy().to_string();
        let bmsroot_strs: Vec<String> = self.bmsroot.clone();

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
                    rec.get_path() == Some(&pathname)
                } else {
                    false
                };
                if matched {
                    if let Some(rec) = record.as_ref()
                        && rec.date == last_modified_time as i32
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

            let model: Option<BMSModel> = if pathname.to_lowercase().ends_with(".bmson") {
                if bmsondecoder.is_none() {
                    bmsondecoder = Some(BMSONDecoder::new(LNTYPE_LONGNOTE));
                }
                match bmsondecoder.as_mut().unwrap().decode_path(bmsfile_path) {
                    Some(m) => Some(m),
                    None => {
                        log::error!("Error while decoding bmson at path: {}", pathname);
                        None
                    }
                }
            } else if pathname.to_lowercase().ends_with(".osu") {
                if osudecoder.is_none() {
                    osudecoder = Some(OSUDecoder::new(LNTYPE_LONGNOTE));
                }
                match osudecoder.as_mut().unwrap().decode_path(bmsfile_path) {
                    Some(m) => Some(m),
                    None => {
                        log::error!("Error while decoding osu at path: {}", pathname);
                        None
                    }
                }
            } else {
                if bmsdecoder.is_none() {
                    bmsdecoder = Some(BMSDecoder::new_with_lntype(LNTYPE_LONGNOTE));
                }
                match bmsdecoder.as_mut().unwrap().decode_path(bmsfile_path) {
                    Some(m) => Some(m),
                    None => {
                        log::error!("Error while decoding bms at path: {}", pathname);
                        None
                    }
                }
            };

            let model = match model {
                Some(m) => m,
                None => continue,
            };

            let mut sd = SongData::new_from_model(model, self.txt);

            if sd.notes != 0
                || !sd
                    .model
                    .as_ref()
                    .is_none_or(|m| m.get_wav_list().is_empty())
            {
                if sd.difficulty == 0 {
                    let fulltitle = format!("{}{}", sd.title, sd.subtitle).to_lowercase();
                    let diffname = sd.subtitle.to_lowercase();
                    if diffname.contains("beginner") {
                        sd.difficulty = 1;
                    } else if diffname.contains("normal") {
                        sd.difficulty = 2;
                    } else if diffname.contains("hyper") {
                        sd.difficulty = 3;
                    } else if diffname.contains("another") {
                        sd.difficulty = 4;
                    } else if diffname.contains("insane") || diffname.contains("leggendaria") {
                        sd.difficulty = 5;
                    } else if fulltitle.contains("beginner") {
                        sd.difficulty = 1;
                    } else if fulltitle.contains("normal") {
                        sd.difficulty = 2;
                    } else if fulltitle.contains("hyper") {
                        sd.difficulty = 3;
                    } else if fulltitle.contains("another") {
                        sd.difficulty = 4;
                    } else if fulltitle.contains("insane") || fulltitle.contains("leggendaria") {
                        sd.difficulty = 5;
                    } else if sd.notes < 250 {
                        sd.difficulty = 1;
                    } else if sd.notes < 600 {
                        sd.difficulty = 2;
                    } else if sd.notes < 1000 {
                        sd.difficulty = 3;
                    } else if sd.notes < 2000 {
                        sd.difficulty = 4;
                    } else {
                        sd.difficulty = 5;
                    }
                }

                if sd.preview.is_empty()
                    && let Some(ref preview) = self.previewpath
                {
                    sd.preview = preview.clone();
                }

                let tag = property.tags.get(&sd.sha256).cloned().unwrap_or_default();
                let favorite = property.favorites.get(&sd.sha256).copied().unwrap_or(0);

                // Plugin updates
                for plugin in &accessor.plugins {
                    if let Some(ref model) = sd.model {
                        let mut sd_clone = sd.clone();
                        plugin.update(model, &mut sd_clone);
                        sd = sd_clone;
                    }
                }

                sd.tag = tag;
                sd.set_path(pathname.clone());

                if let Some(parent_path) = bmsfile_path.parent() {
                    sd.folder =
                        song_utils::crc32(&parent_path.to_string_lossy(), &bmsroot_strs, &root_str);
                    if let Some(grandparent) = parent_path.parent() {
                        sd.parent = song_utils::crc32(
                            &grandparent.to_string_lossy(),
                            &bmsroot_strs,
                            &root_str,
                        );
                    }
                }
                sd.date = last_modified_time as i32;
                sd.favorite = favorite;
                sd.adddate = property.updatetime as i32;

                if let Err(e) = accessor.insert_song(&sd) {
                    log::error!("Error inserting song: {}", e);
                }

                if let Some(info) = property.info
                    && let Some(ref model) = sd.model
                {
                    info.update(model);
                }

                new_count += 1;
            } else {
                let conn = accessor.conn.lock().unwrap();
                let _ = conn.execute(
                    "DELETE FROM song WHERE path = ?1",
                    rusqlite::params![pathname],
                );
            }
        }

        // Delete records that no longer exist in directory
        // (matches Java: records.parallelStream().filter(Objects::nonNull).forEach(...))
        records.par_iter().flatten().for_each(|record| {
            if let Some(path) = record.get_path() {
                let conn = accessor.conn.lock().unwrap();
                let _ = conn.execute("DELETE FROM song WHERE path = ?1", rusqlite::params![path]);
            }
        });

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
mod tests {
    use super::*;

    fn create_test_accessor() -> SQLiteSongDatabaseAccessor {
        SQLiteSongDatabaseAccessor::new(":memory:", &[]).unwrap()
    }

    fn make_test_song(md5: &str, sha256: &str, title: &str) -> SongData {
        let mut sd = SongData::new();
        sd.md5 = md5.to_string();
        sd.sha256 = sha256.to_string();
        sd.title = title.to_string();
        sd.set_path(format!("test/{}.bms", title));
        sd
    }

    #[test]
    fn test_new_creates_tables() {
        let accessor = create_test_accessor();
        // Verify tables exist by querying them
        let songs = accessor.get_song_datas("md5", "nonexistent");
        assert!(songs.is_empty());
        let folders = accessor.get_folder_datas("path", "nonexistent");
        assert!(folders.is_empty());
    }

    #[test]
    fn test_insert_and_get_song_by_md5() {
        let accessor = create_test_accessor();
        let song = make_test_song("abc123", "sha_abc123", "Test Song");
        accessor.insert_song(&song).unwrap();

        let results = accessor.get_song_datas("md5", "abc123");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Test Song");
        assert_eq!(results[0].md5, "abc123");
    }

    #[test]
    fn test_insert_and_get_song_by_sha256() {
        let accessor = create_test_accessor();
        let song = make_test_song("md5_xyz", "sha256_xyz", "SHA Test");
        accessor.insert_song(&song).unwrap();

        let results = accessor.get_song_datas("sha256", "sha256_xyz");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "SHA Test");
    }

    #[test]
    fn test_get_song_datas_empty() {
        let accessor = create_test_accessor();
        let results = accessor.get_song_datas("md5", "nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_get_song_datas_by_hashes() {
        let accessor = create_test_accessor();
        // SHA256 hashes must be > 32 chars to be classified as sha256
        let sha1 = "a".repeat(64);
        let sha2 = "b".repeat(64);
        let sha3 = "c".repeat(64);
        let song1 = make_test_song("md5_1", &sha1, "Song 1");
        let song2 = make_test_song("md5_2", &sha2, "Song 2");
        let song3 = make_test_song("md5_3", &sha3, "Song 3");
        accessor.insert_song(&song1).unwrap();
        accessor.insert_song(&song2).unwrap();
        accessor.insert_song(&song3).unwrap();

        // Query by sha256 hashes (> 32 chars)
        let hashes = vec![sha1, sha3];
        let results = accessor.get_song_datas_by_hashes(&hashes);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_get_song_datas_by_hashes_md5() {
        let accessor = create_test_accessor();
        let song1 = make_test_song("md5_short_1", "sha1", "Song Short 1");
        let song2 = make_test_song("md5_short_2", "sha2", "Song Short 2");
        accessor.insert_song(&song1).unwrap();
        accessor.insert_song(&song2).unwrap();

        // Query by md5 hashes (<= 32 chars)
        let hashes = vec!["md5_short_1".to_string()];
        let results = accessor.get_song_datas_by_hashes(&hashes);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Song Short 1");
    }

    #[test]
    fn test_get_song_datas_by_text() {
        let accessor = create_test_accessor();
        let mut song = make_test_song("m1", "s1", "Rhythm Action");
        song.artist = "DJ Test".to_string();
        accessor.insert_song(&song).unwrap();

        let results = accessor.get_song_datas_by_text("Rhythm");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rhythm Action");

        let results = accessor.get_song_datas_by_text("DJ Test");
        assert_eq!(results.len(), 1);

        let results = accessor.get_song_datas_by_text("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_set_song_datas_batch() {
        let accessor = create_test_accessor();
        let songs = vec![
            make_test_song("batch_1", "sbatch_1", "Batch Song 1"),
            make_test_song("batch_2", "sbatch_2", "Batch Song 2"),
            make_test_song("batch_3", "sbatch_3", "Batch Song 3"),
        ];

        accessor.set_song_datas(&songs);

        let results = accessor.get_song_datas("md5", "batch_1");
        assert_eq!(results.len(), 1);
        let results = accessor.get_song_datas("md5", "batch_2");
        assert_eq!(results.len(), 1);
        let results = accessor.get_song_datas("md5", "batch_3");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_insert_and_get_folder() {
        let accessor = create_test_accessor();
        let folder = FolderData {
            title: "Test Folder".to_string(),
            path: "/test/folder/".to_string(),
            parent: "parent_crc".to_string(),
            date: 1000,
            adddate: 2000,
            ..Default::default()
        };
        accessor.insert_folder(&folder).unwrap();

        let results = accessor.get_folder_datas("path", "/test/folder/");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Test Folder");
        assert_eq!(results[0].date, 1000);
    }

    #[test]
    fn test_get_folder_datas_empty() {
        let accessor = create_test_accessor();
        let results = accessor.get_folder_datas("path", "nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_add_plugin() {
        let mut accessor = create_test_accessor();
        struct TestPlugin;
        impl SongDatabaseAccessorPlugin for TestPlugin {
            fn update(&self, _model: &BMSModel, song: &mut SongData) {
                song.tag = "plugin_tag".to_string();
            }
        }
        accessor.add_plugin(Box::new(TestPlugin));
        assert_eq!(accessor.plugins.len(), 1);
    }

    #[test]
    fn test_update_song_datas_scans_bms_files() {
        let tmpdir = tempfile::tempdir().unwrap();
        let bms_dir = tmpdir.path().join("songs").join("testpack");
        fs::create_dir_all(&bms_dir).unwrap();

        // Write a minimal BMS file
        let bms_content = "\
#PLAYER 1\n\
#GENRE Test\n\
#TITLE Update Test Song\n\
#ARTIST tester\n\
#BPM 120\n\
#PLAYLEVEL 3\n\
#RANK 2\n\
#TOTAL 300\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
        fs::write(bms_dir.join("test.bms"), bms_content).unwrap();

        let db_path = tmpdir.path().join("song.db");
        let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
        let accessor =
            SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

        accessor.update_song_datas(None, &bmsroot, true, false, None);

        // Verify the song was inserted
        let songs = accessor.get_song_datas("title", "Update Test Song");
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].artist, "tester");
        assert!(songs[0].notes > 0);
    }

    #[test]
    fn test_update_song_datas_incremental_skips_unchanged() {
        let tmpdir = tempfile::tempdir().unwrap();
        let bms_dir = tmpdir.path().join("songs").join("testpack");
        fs::create_dir_all(&bms_dir).unwrap();

        let bms_content = "\
#PLAYER 1\n\
#TITLE Incremental Test\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
        fs::write(bms_dir.join("incr.bms"), bms_content).unwrap();

        let db_path = tmpdir.path().join("song.db");
        let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
        let accessor =
            SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

        // First update
        let listener1 = SongDatabaseUpdateListener::new();
        accessor.update_song_datas_with_listener(None, &bmsroot, false, false, None, &listener1);
        assert_eq!(listener1.get_new_bms_files_count(), 1);

        // Second update (no changes) - should skip
        let listener2 = SongDatabaseUpdateListener::new();
        accessor.update_song_datas_with_listener(None, &bmsroot, false, false, None, &listener2);
        assert_eq!(listener2.get_new_bms_files_count(), 0);
        assert_eq!(listener2.get_bms_files_count(), 1);
    }

    #[test]
    fn test_update_song_datas_creates_folder_records() {
        let tmpdir = tempfile::tempdir().unwrap();
        let bms_dir = tmpdir.path().join("songs").join("pack1");
        fs::create_dir_all(&bms_dir).unwrap();

        let bms_content = "\
#TITLE Folder Test\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
        fs::write(bms_dir.join("folder_test.bms"), bms_content).unwrap();

        let db_path = tmpdir.path().join("song.db");
        let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
        let accessor =
            SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

        accessor.update_song_datas(None, &bmsroot, true, false, None);

        // Check that folder records were created (at least root and pack1)
        let all_folders: Vec<FolderData> = {
            let conn = accessor.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT * FROM folder").unwrap();
            let rows = stmt
                .query_map([], |row| {
                    Ok(FolderData {
                        title: row.get::<_, String>(0).unwrap_or_default(),
                        subtitle: row.get::<_, String>(1).unwrap_or_default(),
                        command: row.get::<_, String>(2).unwrap_or_default(),
                        path: row.get::<_, String>(3).unwrap_or_default(),
                        banner: row.get::<_, String>(4).unwrap_or_default(),
                        parent: row.get::<_, String>(5).unwrap_or_default(),
                        folder_type: row.get::<_, i32>(6).unwrap_or(0),
                        date: row.get::<_, i32>(7).unwrap_or(0),
                        adddate: row.get::<_, i32>(8).unwrap_or(0),
                        max: row.get::<_, i32>(9).unwrap_or(0),
                    })
                })
                .unwrap();
            rows.flatten().collect()
        };
        assert!(
            !all_folders.is_empty(),
            "Folder records should be created during update"
        );
    }

    #[test]
    fn test_update_song_datas_empty_bmsroot() {
        let accessor = create_test_accessor();
        // Should not panic, just log warning and return
        accessor.update_song_datas(None, &[], true, false, None);
    }

    #[test]
    fn test_update_song_datas_preserves_favorites() {
        let tmpdir = tempfile::tempdir().unwrap();
        let bms_dir = tmpdir.path().join("songs").join("favpack");
        fs::create_dir_all(&bms_dir).unwrap();

        let bms_content = "\
#TITLE Favorite Test\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
        fs::write(bms_dir.join("fav.bms"), bms_content).unwrap();

        let db_path = tmpdir.path().join("song.db");
        let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
        let accessor =
            SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

        // First update
        accessor.update_song_datas(None, &bmsroot, true, false, None);

        // Set favorite on the song
        let songs = accessor.get_song_datas("title", "Favorite Test");
        assert_eq!(songs.len(), 1);
        let sha256 = songs[0].sha256.clone();
        let conn = accessor.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE song SET favorite = 3 WHERE sha256 = ?1",
            rusqlite::params![sha256],
        );
        drop(conn);

        // Full re-update (updateAll=true)
        accessor.update_song_datas(None, &bmsroot, true, false, None);

        // Verify favorite is preserved
        let songs = accessor.get_song_datas("title", "Favorite Test");
        assert_eq!(songs.len(), 1);
        assert_eq!(
            songs[0].favorite, 3,
            "Favorite should be preserved across updates"
        );
    }

    #[test]
    fn test_update_song_datas_auto_difficulty() {
        let tmpdir = tempfile::tempdir().unwrap();
        let bms_dir = tmpdir.path().join("songs").join("diffpack");
        fs::create_dir_all(&bms_dir).unwrap();

        // "beginner" in subtitle -> difficulty 1
        let bms_content = "\
#TITLE Test\n\
#SUBTITLE beginner\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
        fs::write(bms_dir.join("diff.bms"), bms_content).unwrap();

        let db_path = tmpdir.path().join("song.db");
        let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
        let accessor =
            SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

        accessor.update_song_datas(None, &bmsroot, true, false, None);

        let songs = accessor.get_song_datas("title", "Test");
        assert_eq!(songs.len(), 1);
        assert_eq!(
            songs[0].difficulty, 1,
            "Beginner subtitle should set difficulty to 1"
        );
    }
}
