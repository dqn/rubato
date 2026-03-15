//! Configurable test double for [`SongDatabaseAccessor`].
//!
//! Replaces duplicate per-crate MockSongDb implementations with a single,
//! builder-driven test double that covers all existing mock patterns:
//!
//! - Empty stub (returns empty for all queries)
//! - Key-value song/folder lookups
//! - Hash-based lookups (all or filtering)
//! - Text search
//! - SQL query results
//! - Flat "return all" song list

use crate::folder_data::FolderData;
use crate::song_data::SongData;
use crate::song_database_accessor::SongDatabaseAccessor;

/// A key-value pair used for exact-match lookups.
#[derive(Clone, Debug)]
struct KvEntry<T> {
    key: String,
    value: String,
    data: Vec<T>,
}

/// A text-match entry for text search lookups.
#[derive(Clone, Debug)]
struct TextEntry {
    text: String,
    data: Vec<SongData>,
}

/// Configurable test double for [`SongDatabaseAccessor`].
///
/// Build via chained `with_*` methods starting from [`TestSongDb::new()`].
///
/// # Examples
///
/// ```ignore
/// let db = TestSongDb::new()
///     .with_songs("parent", "abc", vec![song1])
///     .with_folders("parent", "abc", vec![folder1]);
/// ```
#[derive(Default)]
pub struct TestSongDb {
    /// Key-value → songs mappings for `song_datas(key, value)`.
    kv_songs: Vec<KvEntry<SongData>>,
    /// Key-value → folders mappings for `folder_datas(key, value)`.
    kv_folders: Vec<KvEntry<FolderData>>,
    /// Songs returned by `song_datas_by_hashes`.
    hash_songs: Vec<SongData>,
    /// When true, `song_datas_by_hashes` filters `hash_songs` by sha256/md5
    /// match. When false, returns all `hash_songs` regardless of the query.
    filter_hashes: bool,
    /// Text → songs mappings for `song_datas_by_text`.
    text_songs: Vec<TextEntry>,
    /// Songs returned by `song_datas_by_sql` for any SQL query.
    sql_songs: Vec<SongData>,
    /// Songs returned by `song_datas(key, value)` when no key-value mapping
    /// matches (flat "return all" mode).
    all_songs: Vec<SongData>,
}

impl TestSongDb {
    /// Create an empty test double that returns empty results for all queries.
    pub fn new() -> Self {
        Self::default()
    }

    // -- Builder methods (consume and return Self for chaining) --

    /// Add a key-value → songs mapping.
    ///
    /// When `song_datas(key, value)` is called with matching key and value,
    /// the configured songs are returned.
    pub fn with_songs(mut self, key: &str, value: &str, songs: Vec<SongData>) -> Self {
        self.kv_songs.push(KvEntry {
            key: key.to_string(),
            value: value.to_string(),
            data: songs,
        });
        self
    }

    /// Add a key-value → folders mapping.
    ///
    /// When `folder_datas(key, value)` is called with matching key and value,
    /// the configured folders are returned.
    pub fn with_folders(mut self, key: &str, value: &str, folders: Vec<FolderData>) -> Self {
        self.kv_folders.push(KvEntry {
            key: key.to_string(),
            value: value.to_string(),
            data: folders,
        });
        self
    }

    /// Set songs returned by `song_datas_by_hashes`.
    ///
    /// By default all configured songs are returned regardless of the queried
    /// hashes. Call [`with_hash_filtering`] to enable sha256/md5 filtering.
    pub fn with_songs_by_hashes(mut self, songs: Vec<SongData>) -> Self {
        self.hash_songs = songs;
        self
    }

    /// Enable or disable hash filtering for `song_datas_by_hashes`.
    ///
    /// When enabled, only songs whose `sha256` or `md5` appears in the
    /// queried hashes slice are returned. When disabled (the default), all
    /// configured hash songs are returned.
    pub fn with_hash_filtering(mut self, enabled: bool) -> Self {
        self.filter_hashes = enabled;
        self
    }

    /// Add a text → songs mapping for `song_datas_by_text`.
    pub fn with_songs_by_text(mut self, text: &str, songs: Vec<SongData>) -> Self {
        self.text_songs.push(TextEntry {
            text: text.to_string(),
            data: songs,
        });
        self
    }

    /// Set songs returned by `song_datas_by_sql` for any SQL query.
    pub fn with_songs_by_sql(mut self, songs: Vec<SongData>) -> Self {
        self.sql_songs = songs;
        self
    }

    /// Set a flat song list returned by `song_datas(key, value)` when no
    /// key-value mapping matches.
    pub fn with_all_songs(mut self, songs: Vec<SongData>) -> Self {
        self.all_songs = songs;
        self
    }
}

impl SongDatabaseAccessor for TestSongDb {
    fn song_datas(&self, key: &str, value: &str) -> Vec<SongData> {
        // Try exact key-value match first.
        for entry in &self.kv_songs {
            if entry.key == key && entry.value == value {
                return entry.data.clone();
            }
        }
        // Fall back to flat "all songs" list.
        self.all_songs.clone()
    }

    fn song_datas_by_hashes(&self, hashes: &[String]) -> Vec<SongData> {
        if self.filter_hashes {
            self.hash_songs
                .iter()
                .filter(|song| {
                    hashes.iter().any(|h| {
                        (!song.file.sha256.is_empty() && song.file.sha256 == *h)
                            || (!song.file.md5.is_empty() && song.file.md5 == *h)
                    })
                })
                .cloned()
                .collect()
        } else {
            self.hash_songs.clone()
        }
    }

    fn song_datas_by_sql(
        &self,
        _sql: &str,
        _score: &str,
        _scorelog: &str,
        _info: Option<&str>,
    ) -> Vec<SongData> {
        self.sql_songs.clone()
    }

    fn set_song_datas(&self, _songs: &[SongData]) -> anyhow::Result<()> {
        // no-op (matches all existing mocks)
        Ok(())
    }

    fn song_datas_by_text(&self, text: &str) -> Vec<SongData> {
        for entry in &self.text_songs {
            if entry.text == text {
                return entry.data.clone();
            }
        }
        Vec::new()
    }

    fn folder_datas(&self, key: &str, value: &str) -> Vec<FolderData> {
        for entry in &self.kv_folders {
            if entry.key == key && entry.value == value {
                return entry.data.clone();
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_song(title: &str, sha256: &str, md5: &str) -> SongData {
        let mut s = SongData::new();
        s.metadata.title = title.to_string();
        s.file.sha256 = sha256.to_string();
        s.file.md5 = md5.to_string();
        s
    }

    fn make_folder(title: &str) -> FolderData {
        FolderData {
            title: title.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn empty_stub_returns_empty() {
        let db = TestSongDb::new();
        assert!(db.song_datas("any", "thing").is_empty());
        assert!(db.song_datas_by_hashes(&["abc".into()]).is_empty());
        assert!(db.song_datas_by_text("hello").is_empty());
        assert!(db.song_datas_by_sql("SELECT 1", "", "", None).is_empty());
        assert!(db.folder_datas("any", "thing").is_empty());
    }

    #[test]
    fn kv_song_lookup() {
        let db = TestSongDb::new()
            .with_songs("parent", "abc", vec![make_song("A", "", "")])
            .with_songs("parent", "def", vec![make_song("B", "", "")]);

        let a = db.song_datas("parent", "abc");
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].metadata.title, "A");

        let b = db.song_datas("parent", "def");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].metadata.title, "B");

        // Non-matching returns empty (no all_songs configured).
        assert!(db.song_datas("parent", "ghi").is_empty());
    }

    #[test]
    fn kv_folder_lookup() {
        let db = TestSongDb::new().with_folders(
            "parent",
            "abc",
            vec![make_folder("Folder1"), make_folder("Folder2")],
        );

        let folders = db.folder_datas("parent", "abc");
        assert_eq!(folders.len(), 2);
        assert_eq!(folders[0].title, "Folder1");

        assert!(db.folder_datas("parent", "xyz").is_empty());
    }

    #[test]
    fn hash_lookup_returns_all() {
        let db = TestSongDb::new().with_songs_by_hashes(vec![
            make_song("X", "sha_x", "md5_x"),
            make_song("Y", "sha_y", "md5_y"),
        ]);

        // Without filtering, all songs are returned regardless of hashes.
        let result = db.song_datas_by_hashes(&["unrelated".into()]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn hash_lookup_with_filtering() {
        let db = TestSongDb::new()
            .with_songs_by_hashes(vec![
                make_song("X", "sha_x", "md5_x"),
                make_song("Y", "sha_y", "md5_y"),
                make_song("Z", "sha_z", "md5_z"),
            ])
            .with_hash_filtering(true);

        // Match by sha256.
        let result = db.song_datas_by_hashes(&["sha_x".into()]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].metadata.title, "X");

        // Match by md5.
        let result = db.song_datas_by_hashes(&["md5_y".into()]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].metadata.title, "Y");

        // Multiple matches.
        let result = db.song_datas_by_hashes(&["sha_x".into(), "sha_z".into()]);
        assert_eq!(result.len(), 2);

        // No match.
        let result = db.song_datas_by_hashes(&["nope".into()]);
        assert!(result.is_empty());
    }

    #[test]
    fn text_search() {
        let db = TestSongDb::new()
            .with_songs_by_text("hello", vec![make_song("H", "", "")])
            .with_songs_by_text("world", vec![make_song("W", "", "")]);

        let h = db.song_datas_by_text("hello");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].metadata.title, "H");

        let w = db.song_datas_by_text("world");
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].metadata.title, "W");

        assert!(db.song_datas_by_text("missing").is_empty());
    }

    #[test]
    fn sql_query() {
        let db = TestSongDb::new()
            .with_songs_by_sql(vec![make_song("SQL1", "", ""), make_song("SQL2", "", "")]);

        let result = db.song_datas_by_sql("SELECT *", "score", "scorelog", Some("info"));
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].metadata.title, "SQL1");
    }

    #[test]
    fn all_songs_fallback() {
        let db = TestSongDb::new()
            .with_songs("parent", "exact", vec![make_song("Exact", "", "")])
            .with_all_songs(vec![make_song("All1", "", ""), make_song("All2", "", "")]);

        // Exact match takes priority.
        let exact = db.song_datas("parent", "exact");
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].metadata.title, "Exact");

        // Non-matching falls back to all_songs.
        let fallback = db.song_datas("parent", "other");
        assert_eq!(fallback.len(), 2);
        assert_eq!(fallback[0].metadata.title, "All1");
    }

    #[test]
    fn set_song_datas_is_noop() {
        let db = TestSongDb::new();
        db.set_song_datas(&[make_song("ignored", "", "")])
            .expect("set_song_datas");
        // Should not affect any queries.
        assert!(db.song_datas("any", "thing").is_empty());
    }

    #[test]
    fn builder_chaining() {
        // Verify all builder methods can be chained in a single expression.
        let db = TestSongDb::new()
            .with_songs("k", "v", vec![])
            .with_folders("k", "v", vec![])
            .with_songs_by_hashes(vec![])
            .with_hash_filtering(true)
            .with_songs_by_text("t", vec![])
            .with_songs_by_sql(vec![])
            .with_all_songs(vec![]);

        assert!(db.song_datas("k", "v").is_empty());
    }
}
