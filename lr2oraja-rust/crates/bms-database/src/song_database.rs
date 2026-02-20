use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::Connection;
use tracing::{debug, info, warn};

use crate::folder_data::FolderData;
use crate::schema::{FOLDER_TABLE, SCORELOG_TABLE, SONG_TABLE, ensure_table};
use crate::song_data::SongData;

/// BMS file extensions to scan.
const BMS_EXTENSIONS: &[&str] = &["bms", "bme", "bml", "pms", "bmson"];

/// Statistics returned by `update_song_datas`.
#[derive(Debug, Default)]
pub struct UpdateStats {
    pub scanned: usize,
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

/// Whitelist of allowed column names for `get_song_datas(key, value)`.
const ALLOWED_KEYS: &[&str] = &[
    "md5", "sha256", "title", "artist", "genre", "path", "folder", "parent", "favorite",
];
const DISALLOWED_SQL_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "CREATE", "REPLACE", "PRAGMA", "ATTACH",
    "DETACH", "VACUUM", "BEGIN", "COMMIT", "ROLLBACK",
];

/// Song database accessor (song.db).
///
/// Manages the `song` and `folder` tables.
pub struct SongDatabase {
    conn: Connection,
}

impl SongDatabase {
    /// Open (or create) a song database at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
        ensure_table(&conn, &FOLDER_TABLE)?;
        ensure_table(&conn, &SONG_TABLE)?;
        ensure_table(&conn, &SCORELOG_TABLE)?;
        Ok(Self { conn })
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        ensure_table(&conn, &FOLDER_TABLE)?;
        ensure_table(&conn, &SONG_TABLE)?;
        ensure_table(&conn, &SCORELOG_TABLE)?;
        Ok(Self { conn })
    }

    /// Get all song data from the database.
    pub fn get_all_song_datas(&self) -> Result<Vec<SongData>> {
        let sql = "SELECT * FROM song";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], SongData::from_row)?;
        let mut results = Vec::new();
        for r in rows {
            let sd = r?;
            if sd.validate() {
                results.push(sd);
            }
        }
        Ok(results)
    }

    /// Get song data by a single key-value pair.
    ///
    /// `key` must be one of the allowed column names (whitelist-validated).
    pub fn get_song_datas(&self, key: &str, value: &str) -> Result<Vec<SongData>> {
        if !ALLOWED_KEYS.contains(&key) {
            anyhow::bail!("disallowed key for song query: {key}");
        }
        let sql = format!("SELECT * FROM song WHERE [{key}] = ?1");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([value], SongData::from_row)?;
        let mut results = Vec::new();
        for r in rows {
            let sd = r?;
            if sd.validate() {
                results.push(sd);
            }
        }
        Ok(results)
    }

    /// Get song data by multiple hashes (MD5 or SHA256, auto-detected by length).
    pub fn get_song_datas_by_hashes(&self, hashes: &[&str]) -> Result<Vec<SongData>> {
        if hashes.is_empty() {
            return Ok(Vec::new());
        }

        let mut md5_hashes = Vec::new();
        let mut sha256_hashes = Vec::new();
        for &h in hashes {
            if h.len() > 32 {
                sha256_hashes.push(h);
            } else {
                md5_hashes.push(h);
            }
        }

        let mut conditions = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if !md5_hashes.is_empty() {
            let placeholders: Vec<String> = md5_hashes
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", params.len() + i + 1))
                .collect();
            conditions.push(format!("md5 IN ({})", placeholders.join(",")));
            params.extend(md5_hashes.iter().map(|s| s.to_string()));
        }

        if !sha256_hashes.is_empty() {
            let placeholders: Vec<String> = sha256_hashes
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", params.len() + i + 1))
                .collect();
            conditions.push(format!("sha256 IN ({})", placeholders.join(",")));
            params.extend(sha256_hashes.iter().map(|s| s.to_string()));
        }

        let sql = format!("SELECT * FROM song WHERE {}", conditions.join(" OR "));
        let mut stmt = self.conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(param_refs.as_slice(), SongData::from_row)?;

        let mut results = Vec::new();
        for r in rows {
            let sd = r?;
            if sd.validate() {
                results.push(sd);
            }
        }
        Ok(results)
    }

    /// Search songs by text (LIKE match on title, artist, genre, subartist).
    pub fn get_song_datas_by_text(&self, text: &str) -> Result<Vec<SongData>> {
        let pattern = format!("%{text}%");
        let sql = "SELECT * FROM song WHERE \
                   rtrim(title||' '||subtitle||' '||artist||' '||subartist||' '||genre) LIKE ?1 \
                   GROUP BY sha256";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([&pattern], SongData::from_row)?;
        let mut results = Vec::new();
        for r in rows {
            let sd = r?;
            if sd.validate() {
                results.push(sd);
            }
        }
        Ok(results)
    }

    /// Execute a raw SQL query and return matching song data.
    ///
    /// Used by RandomCourse stages that carry user-defined SQL queries.
    /// The query is expected to be a SELECT that returns rows from the song table.
    pub fn get_song_datas_by_sql(&self, sql: &str) -> Result<Vec<SongData>> {
        validate_read_only_select_sql(sql)?;
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], SongData::from_row)?;
        let mut results = Vec::new();
        for r in rows {
            let sd = r?;
            if sd.validate() {
                results.push(sd);
            }
        }
        Ok(results)
    }

    /// Toggle a favorite flag for a song identified by sha256.
    /// `flag` should be one of FAVORITE_SONG, FAVORITE_CHART, INVISIBLE_SONG, INVISIBLE_CHART.
    pub fn update_favorite(&self, sha256: &str, flag: i32) -> Result<()> {
        // SQLite doesn't support ^ (XOR) operator, so we use bitwise formula:
        // XOR(a, b) = (a | b) & ~(a & b)
        let sql = "UPDATE song SET favorite = (favorite | ?1) & ~(favorite & ?1) WHERE sha256 = ?2";
        self.conn.execute(sql, rusqlite::params![flag, sha256])?;
        Ok(())
    }

    /// Insert or replace song data (batch, in a transaction).
    pub fn set_song_datas(&self, songs: &[SongData]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO song \
                 (md5,sha256,title,subtitle,genre,artist,subartist,tag,path,folder,\
                  stagefile,banner,backbmp,preview,parent,level,difficulty,maxbpm,minbpm,\
                  length,mode,judge,feature,content,date,favorite,adddate,notes,charthash) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,\
                         ?11,?12,?13,?14,?15,?16,?17,?18,?19,\
                         ?20,?21,?22,?23,?24,?25,?26,?27,?28,?29)",
            )?;
            for sd in songs {
                stmt.execute(rusqlite::params![
                    sd.md5,
                    sd.sha256,
                    sd.title,
                    sd.subtitle,
                    sd.genre,
                    sd.artist,
                    sd.subartist,
                    sd.tag,
                    sd.path,
                    sd.folder,
                    sd.stagefile,
                    sd.banner,
                    sd.backbmp,
                    sd.preview,
                    sd.parent,
                    sd.level,
                    sd.difficulty,
                    sd.maxbpm,
                    sd.minbpm,
                    sd.length,
                    sd.mode,
                    sd.judge,
                    sd.feature,
                    sd.content,
                    sd.date,
                    sd.favorite,
                    sd.adddate,
                    sd.notes,
                    sd.charthash,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Get folder data by a single key-value pair.
    pub fn get_folder_datas(&self, key: &str, value: &str) -> Result<Vec<FolderData>> {
        if !["path", "parent", "title"].contains(&key) {
            anyhow::bail!("disallowed key for folder query: {key}");
        }
        let sql = format!("SELECT * FROM folder WHERE [{key}] = ?1");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([value], FolderData::from_row)?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    /// Update the song database by scanning BMS root directories.
    ///
    /// `path`: optional specific folder to update (None = all roots).
    /// `bmsroot`: list of BMS root directories to scan.
    /// `update_all`: if true, delete all records and do a full rescan.
    ///
    /// Matches Java `SQLiteSongDatabaseAccessor.updateSongDatas()`.
    pub fn update_song_datas(
        &self,
        path: Option<&Path>,
        bmsroot: &[String],
        update_all: bool,
    ) -> Result<UpdateStats> {
        let mut stats = UpdateStats::default();

        // 1. Preserve tags and favorites (keyed by sha256)
        let mut saved_tags: HashMap<String, String> = HashMap::new();
        let mut saved_favorites: HashMap<String, i32> = HashMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT sha256, tag, favorite FROM song")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, Option<i32>>(2)?.unwrap_or(0),
                ))
            })?;
            for row in rows {
                let (sha256, tag, favorite) = row?;
                if !tag.is_empty() {
                    saved_tags.insert(sha256.clone(), tag);
                }
                if favorite != 0 {
                    saved_favorites.insert(sha256, favorite);
                }
            }
        }

        // 2. Full rescan: delete all records
        if update_all {
            self.conn.execute("DELETE FROM song", [])?;
            self.conn.execute("DELETE FROM folder", [])?;
        }

        // 3. Determine directories to scan
        let scan_dirs: Vec<PathBuf> = if let Some(p) = path {
            vec![p.to_path_buf()]
        } else {
            bmsroot.iter().map(PathBuf::from).collect()
        };

        // 4. Build map of existing records (path -> date) for incremental check
        let existing_records: HashMap<String, i32> = if !update_all {
            let mut map = HashMap::new();
            let mut stmt = self.conn.prepare("SELECT path, date FROM song")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
            })?;
            for row in rows {
                let (p, d) = row?;
                map.insert(p, d);
            }
            map
        } else {
            HashMap::new()
        };

        // 5. Scan directories recursively
        let mut new_songs: Vec<SongData> = Vec::new();
        let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i32)
            .unwrap_or(0);

        for root in &scan_dirs {
            if !root.is_dir() {
                warn!(path = %root.display(), "SongUpdate: root directory does not exist, skipping");
                continue;
            }
            self.scan_directory(
                root,
                &existing_records,
                &saved_tags,
                &saved_favorites,
                now_secs,
                &mut new_songs,
                &mut seen_paths,
                &mut stats,
            );
        }

        // 6. Insert/update scanned songs
        if !new_songs.is_empty() {
            self.set_song_datas(&new_songs)?;
        }

        // 7. Remove records for files that no longer exist (incremental mode)
        if !update_all {
            let mut to_remove: Vec<String> = Vec::new();
            for existing_path in existing_records.keys() {
                // Only remove if the path is under one of the scan directories
                let under_scan = scan_dirs
                    .iter()
                    .any(|root| existing_path.starts_with(root.to_string_lossy().as_ref()));
                if under_scan && !seen_paths.contains(existing_path) {
                    to_remove.push(existing_path.clone());
                }
            }
            if !to_remove.is_empty() {
                let tx = self.conn.unchecked_transaction()?;
                {
                    let mut stmt = tx.prepare("DELETE FROM song WHERE path = ?1")?;
                    for p in &to_remove {
                        stmt.execute([p])?;
                    }
                }
                tx.commit()?;
                stats.removed = to_remove.len();
            }
        }

        info!(
            scanned = stats.scanned,
            added = stats.added,
            updated = stats.updated,
            removed = stats.removed,
            "SongUpdate: scan complete"
        );

        Ok(stats)
    }

    /// Recursively scan a directory for BMS files.
    #[allow(clippy::too_many_arguments)]
    fn scan_directory(
        &self,
        dir: &Path,
        existing: &HashMap<String, i32>,
        saved_tags: &HashMap<String, String>,
        saved_favorites: &HashMap<String, i32>,
        now_secs: i32,
        new_songs: &mut Vec<SongData>,
        seen_paths: &mut std::collections::HashSet<String>,
        stats: &mut UpdateStats,
    ) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                warn!(path = %dir.display(), error = %e, "SongUpdate: failed to read directory");
                return;
            }
        };

        let mut subdirs = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                subdirs.push(path);
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            if !BMS_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }

            stats.scanned += 1;
            let path_str = path.to_string_lossy().to_string();
            seen_paths.insert(path_str.clone());

            // Get file mtime
            let file_date = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i32)
                .unwrap_or(0);

            // Incremental: skip if path exists and mtime matches
            if let Some(&existing_date) = existing.get(&path_str)
                && existing_date == file_date
            {
                continue;
            }

            // Parse the file
            match SongData::from_file(&path) {
                Ok(mut sd) => {
                    // Restore preserved tag/favorite
                    if let Some(tag) = saved_tags.get(&sd.sha256) {
                        sd.tag = tag.clone();
                    }
                    if let Some(&fav) = saved_favorites.get(&sd.sha256) {
                        sd.favorite = fav;
                    }

                    // Set adddate for new entries
                    if !existing.contains_key(&path_str) {
                        sd.adddate = now_secs;
                        stats.added += 1;
                    } else {
                        stats.updated += 1;
                    }

                    new_songs.push(sd);
                }
                Err(e) => {
                    debug!(path = %path.display(), error = %e, "SongUpdate: failed to parse BMS file");
                }
            }
        }

        // Recurse into subdirectories
        for subdir in subdirs {
            self.scan_directory(
                &subdir,
                existing,
                saved_tags,
                saved_favorites,
                now_secs,
                new_songs,
                seen_paths,
                stats,
            );
        }
    }

    /// Insert or replace folder data (batch, in a transaction).
    pub fn set_folder_datas(&self, folders: &[FolderData]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO folder \
                 (title,subtitle,command,path,banner,parent,[type],date,adddate,[max]) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            )?;
            for fd in folders {
                stmt.execute(rusqlite::params![
                    fd.title,
                    fd.subtitle,
                    fd.command,
                    fd.path,
                    fd.banner,
                    fd.parent,
                    fd.r#type,
                    fd.date,
                    fd.adddate,
                    fd.max,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

fn validate_read_only_select_sql(sql: &str) -> Result<()> {
    let trimmed = sql.trim_start();
    if trimmed.is_empty() {
        anyhow::bail!("empty SQL query is not allowed");
    }

    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("SELECT ") && !upper.starts_with("WITH ") {
        anyhow::bail!("only SELECT statements are allowed for random course queries");
    }

    let tokens = upper.split(|c: char| !c.is_ascii_alphanumeric() && c != '_');
    for token in tokens {
        if DISALLOWED_SQL_KEYWORDS.contains(&token) {
            anyhow::bail!("disallowed SQL keyword in random course query: {}", token);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_song() -> SongData {
        SongData {
            md5: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            path: "songs/test.bms".to_string(),
            mode: 7,
            level: 5,
            notes: 500,
            ..Default::default()
        }
    }

    #[test]
    fn song_crud_round_trip() {
        let db = SongDatabase::open_in_memory().unwrap();
        let song = sample_song();

        db.set_song_datas(&[song.clone()]).unwrap();

        let found = db.get_song_datas("path", "songs/test.bms").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Test Song");
        assert_eq!(found[0].artist, "Test Artist");
        assert_eq!(found[0].mode, 7);
        assert_eq!(found[0].notes, 500);
    }

    #[test]
    fn song_get_by_hashes() {
        let db = SongDatabase::open_in_memory().unwrap();
        let song = sample_song();
        db.set_song_datas(&[song]).unwrap();

        // By MD5
        let found = db
            .get_song_datas_by_hashes(&["d41d8cd98f00b204e9800998ecf8427e"])
            .unwrap();
        assert_eq!(found.len(), 1);

        // By SHA256
        let found = db
            .get_song_datas_by_hashes(&[
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ])
            .unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn song_text_search() {
        let db = SongDatabase::open_in_memory().unwrap();
        let song = sample_song();
        db.set_song_datas(&[song]).unwrap();

        let found = db.get_song_datas_by_text("Test").unwrap();
        assert_eq!(found.len(), 1);

        let found = db.get_song_datas_by_text("nonexistent").unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn song_update_existing() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut song = sample_song();
        db.set_song_datas(&[song.clone()]).unwrap();

        song.level = 12;
        db.set_song_datas(&[song]).unwrap();

        let found = db.get_song_datas("path", "songs/test.bms").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].level, 12);
    }

    #[test]
    fn folder_crud_round_trip() {
        let db = SongDatabase::open_in_memory().unwrap();
        let folder = FolderData {
            title: "My Folder".to_string(),
            path: "songs/myfolder/".to_string(),
            parent: "abc123".to_string(),
            date: 1700000000,
            ..Default::default()
        };

        db.set_folder_datas(&[folder]).unwrap();

        let found = db.get_folder_datas("parent", "abc123").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "My Folder");
    }

    #[test]
    fn update_favorite_toggles_song() {
        let db = SongDatabase::open_in_memory().unwrap();
        let song = sample_song();
        db.set_song_datas(&[song]).unwrap();

        let sha = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        // Initial favorite is 0
        let found = db.get_song_datas("sha256", sha).unwrap();
        assert_eq!(found[0].favorite, 0);

        // Toggle FAVORITE_SONG on (0 ^ 1 = 1)
        db.update_favorite(sha, 1).unwrap();
        let found = db.get_song_datas("sha256", sha).unwrap();
        assert_eq!(found[0].favorite, 1);

        // Toggle FAVORITE_SONG off (1 ^ 1 = 0)
        db.update_favorite(sha, 1).unwrap();
        let found = db.get_song_datas("sha256", sha).unwrap();
        assert_eq!(found[0].favorite, 0);
    }

    #[test]
    fn update_favorite_toggles_chart() {
        let db = SongDatabase::open_in_memory().unwrap();
        let song = sample_song();
        db.set_song_datas(&[song]).unwrap();

        let sha = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        // Toggle FAVORITE_CHART on (0 ^ 2 = 2)
        db.update_favorite(sha, 2).unwrap();
        let found = db.get_song_datas("sha256", sha).unwrap();
        assert_eq!(found[0].favorite, 2);

        // Toggle FAVORITE_CHART off (2 ^ 2 = 0)
        db.update_favorite(sha, 2).unwrap();
        let found = db.get_song_datas("sha256", sha).unwrap();
        assert_eq!(found[0].favorite, 0);
    }

    #[test]
    fn update_favorite_combined_flags() {
        let db = SongDatabase::open_in_memory().unwrap();
        let song = sample_song();
        db.set_song_datas(&[song]).unwrap();

        let sha = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        // Toggle both FAVORITE_SONG and FAVORITE_CHART
        db.update_favorite(sha, 1).unwrap(); // 0 ^ 1 = 1
        db.update_favorite(sha, 2).unwrap(); // 1 ^ 2 = 3
        let found = db.get_song_datas("sha256", sha).unwrap();
        assert_eq!(found[0].favorite, 3);

        // Toggle FAVORITE_SONG off (3 ^ 1 = 2, only CHART remains)
        db.update_favorite(sha, 1).unwrap();
        let found = db.get_song_datas("sha256", sha).unwrap();
        assert_eq!(found[0].favorite, 2);
    }

    #[test]
    fn disallowed_key_rejected() {
        let db = SongDatabase::open_in_memory().unwrap();
        let result = db.get_song_datas("DROP TABLE song; --", "x");
        assert!(result.is_err());
    }

    // --- Error case tests ---

    #[test]
    fn empty_table_returns_empty_vec() {
        let db = SongDatabase::open_in_memory().unwrap();
        let all = db.get_all_song_datas().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn query_nonexistent_key_returns_empty() {
        let db = SongDatabase::open_in_memory().unwrap();
        let found = db.get_song_datas("md5", "nonexistent_hash").unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn empty_hashes_returns_empty() {
        let db = SongDatabase::open_in_memory().unwrap();
        let found = db.get_song_datas_by_hashes(&[]).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn text_search_on_empty_table() {
        let db = SongDatabase::open_in_memory().unwrap();
        let found = db.get_song_datas_by_text("anything").unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn duplicate_insert_replaces_existing() {
        let db = SongDatabase::open_in_memory().unwrap();
        let mut song = sample_song();
        song.title = "First".to_string();
        db.set_song_datas(&[song.clone()]).unwrap();

        song.title = "Second".to_string();
        db.set_song_datas(&[song]).unwrap();

        let all = db.get_all_song_datas().unwrap();
        // INSERT OR REPLACE: should have exactly 1 record
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "Second");
    }

    #[test]
    fn extremely_long_strings_in_song_data() {
        let db = SongDatabase::open_in_memory().unwrap();
        let long_string = "x".repeat(10000);
        let song = SongData {
            md5: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            title: long_string.clone(),
            artist: long_string.clone(),
            genre: long_string.clone(),
            path: "songs/long.bms".to_string(),
            mode: 7,
            notes: 1,
            ..Default::default()
        };
        db.set_song_datas(&[song]).unwrap();

        let found = db.get_song_datas("path", "songs/long.bms").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title.len(), 10000);
    }

    #[test]
    fn folder_query_empty_table() {
        let db = SongDatabase::open_in_memory().unwrap();
        let found = db.get_folder_datas("path", "nonexistent").unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn folder_disallowed_key_rejected() {
        let db = SongDatabase::open_in_memory().unwrap();
        let result = db.get_folder_datas("malicious_key", "x");
        assert!(result.is_err());
    }

    #[test]
    fn sql_select_query_allowed() {
        let db = SongDatabase::open_in_memory().unwrap();
        let song = sample_song();
        db.set_song_datas(&[song]).unwrap();

        let found = db
            .get_song_datas_by_sql("SELECT * FROM song WHERE title = 'Test Song'")
            .unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn sql_non_select_query_rejected() {
        let db = SongDatabase::open_in_memory().unwrap();
        let song = sample_song();
        db.set_song_datas(&[song]).unwrap();

        let result = db.get_song_datas_by_sql("DELETE FROM song");
        assert!(result.is_err());

        let found = db.get_all_song_datas().unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn sql_multi_statement_query_rejected() {
        let db = SongDatabase::open_in_memory().unwrap();
        let song = sample_song();
        db.set_song_datas(&[song]).unwrap();

        let result = db.get_song_datas_by_sql("SELECT * FROM song; DELETE FROM song");
        assert!(result.is_err());

        let found = db.get_all_song_datas().unwrap();
        assert_eq!(found.len(), 1);
    }

    // --- update_song_datas tests ---

    #[test]
    fn update_song_datas_scans_test_bms() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("test-bms");
        if !test_dir.is_dir() {
            return; // Skip if test-bms not available
        }

        let db = SongDatabase::open_in_memory().unwrap();
        let roots = vec![test_dir.to_string_lossy().to_string()];
        let stats = db.update_song_datas(None, &roots, false).unwrap();

        assert!(stats.scanned > 0, "should have scanned BMS files");
        assert!(stats.added > 0, "should have added new songs");
        assert_eq!(stats.removed, 0, "first scan should not remove anything");

        // All scanned songs should be in the database
        let all = db.get_all_song_datas().unwrap();
        assert_eq!(all.len(), stats.added);
    }

    #[test]
    fn update_song_datas_incremental_skips_unchanged() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("test-bms");
        if !test_dir.is_dir() {
            return;
        }

        let db = SongDatabase::open_in_memory().unwrap();
        let roots = vec![test_dir.to_string_lossy().to_string()];

        // First scan
        let stats1 = db.update_song_datas(None, &roots, false).unwrap();
        assert!(stats1.added > 0);

        // Second scan: no changes, so added+updated should be 0
        let stats2 = db.update_song_datas(None, &roots, false).unwrap();
        assert_eq!(stats2.added, 0, "no new files should be added");
        assert_eq!(stats2.updated, 0, "no files should be updated");
        assert_eq!(stats2.removed, 0, "no files should be removed");
    }

    #[test]
    fn update_song_datas_full_rescan() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("test-bms");
        if !test_dir.is_dir() {
            return;
        }

        let db = SongDatabase::open_in_memory().unwrap();
        let roots = vec![test_dir.to_string_lossy().to_string()];

        // First scan
        let stats1 = db.update_song_datas(None, &roots, false).unwrap();

        // Full rescan (update_all=true)
        let stats2 = db.update_song_datas(None, &roots, true).unwrap();
        assert_eq!(
            stats2.added, stats1.added,
            "full rescan should re-add all songs"
        );
    }

    #[test]
    fn update_song_datas_preserves_favorites() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("test-bms");
        if !test_dir.is_dir() {
            return;
        }

        let db = SongDatabase::open_in_memory().unwrap();
        let roots = vec![test_dir.to_string_lossy().to_string()];

        // Scan and set a favorite
        db.update_song_datas(None, &roots, false).unwrap();
        let all = db.get_all_song_datas().unwrap();
        if all.is_empty() {
            return;
        }
        let sha = all[0].sha256.clone();
        db.update_favorite(&sha, 1).unwrap();

        // Full rescan should preserve favorite
        db.update_song_datas(None, &roots, true).unwrap();
        let found = db.get_song_datas("sha256", &sha).unwrap();
        assert!(!found.is_empty());
        assert_eq!(found[0].favorite, 1, "favorite should be preserved");
    }

    #[test]
    fn update_song_datas_nonexistent_root() {
        let db = SongDatabase::open_in_memory().unwrap();
        let roots = vec!["/nonexistent/path/to/bms".to_string()];
        let stats = db.update_song_datas(None, &roots, false).unwrap();
        assert_eq!(stats.scanned, 0);
        assert_eq!(stats.added, 0);
    }
}
