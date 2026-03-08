use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::table_data::TableData;
use crate::table_data_bridge::difficulty_table_to_table_data;
use crate::validatable::Validatable;

/// Table data accessor.
/// Translated from Java: TableDataAccessor
pub struct TableDataAccessor {
    tabledir: String,
}

impl TableDataAccessor {
    pub fn new(tabledir: &str) -> Self {
        Self {
            tabledir: tabledir.to_string(),
        }
    }

    pub fn update_table_data(&self, urls: &[&str]) {
        let results: Vec<Option<TableData>> = std::thread::scope(|s| {
            let handles: Vec<_> = urls
                .iter()
                .map(|url| s.spawn(move || self.read_from_url(url)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().ok().flatten())
                .collect()
        });
        for mut td in results.into_iter().flatten() {
            self.write(&mut td);
        }
    }

    pub fn load_new_table_data(&self, urls: &[&str]) {
        let local_tables = self.get_local_table_filenames();
        let urls_to_download: Vec<&str> = urls
            .iter()
            .filter(|url| {
                let filename = format!("{}.bmt", Self::get_file_name(url));
                match local_tables {
                    Some(ref locals) => !locals.contains(&filename),
                    None => true,
                }
            })
            .copied()
            .collect();

        let results: Vec<Option<TableData>> = std::thread::scope(|s| {
            let handles: Vec<_> = urls_to_download
                .iter()
                .map(|url| s.spawn(move || self.read_from_url(url)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().ok().flatten())
                .collect()
        });
        for mut td in results.into_iter().flatten() {
            self.write(&mut td);
        }
    }

    fn get_local_table_filenames(&self) -> Option<std::collections::HashSet<String>> {
        let dir = Path::new(&self.tabledir);
        if !dir.exists() {
            return None;
        }
        match fs::read_dir(dir) {
            Ok(entries) => {
                let mut set = std::collections::HashSet::new();
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.to_lowercase().ends_with(".bmt") {
                        set.insert(name);
                    }
                }
                Some(set)
            }
            Err(_) => None,
        }
    }

    pub fn read_local_table_names(
        &self,
        urls: &[&str],
    ) -> Option<std::collections::HashMap<String, String>> {
        let dir = Path::new(&self.tabledir);
        if !dir.exists() {
            return None;
        }
        let mut file_name_to_table_name: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if !filename.ends_with(".bmt") {
                        continue;
                    }
                    if let Some(td) = TableData::read_from_path(&entry.path()) {
                        file_name_to_table_name.insert(filename, td.name.clone());
                    }
                }
            }
            Err(_) => return None,
        }

        let mut url_to_table_name: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for url in urls {
            let key = format!("{}.bmt", Self::get_file_name(url));
            if let Some(name) = file_name_to_table_name.get(&key) {
                url_to_table_name.insert(url.to_string(), name.clone());
            }
        }
        Some(url_to_table_name)
    }

    pub fn write(&self, td: &mut TableData) {
        let path =
            PathBuf::from(&self.tabledir).join(format!("{}.bmt", Self::get_file_name(&td.url)));
        if let Err(e) = TableData::write_to_path(&path, td) {
            log::warn!("Failed to write table data to {}: {:#}", path.display(), e);
        }
    }

    pub fn write_with_filename(&self, td: &mut TableData, filename: &str) {
        let path = PathBuf::from(&self.tabledir).join(filename);
        if let Err(e) = TableData::write_to_path(&path, td) {
            log::warn!("Failed to write table data to {}: {:#}", path.display(), e);
        }
    }

    pub fn read_all(&self) -> Vec<TableData> {
        let dir = Path::new(&self.tabledir);
        if !dir.exists() {
            return Vec::new();
        }
        match fs::read_dir(dir) {
            Ok(entries) => entries
                .flatten()
                .filter_map(|entry| TableData::read_from_path(&entry.path()))
                .collect(),
            Err(e) => {
                log::error!("Failed to read table directory: {}", e);
                Vec::new()
            }
        }
    }

    pub fn read_cache(&self, url: &str) -> Option<TableData> {
        let filename = format!("{}.bmt", Self::get_file_name(url));
        let path = PathBuf::from(&self.tabledir).join(filename);
        if path.exists() {
            TableData::read_from_path(&path)
        } else {
            None
        }
    }

    pub fn read(&self, filename: &str) -> Option<TableData> {
        let path = PathBuf::from(&self.tabledir).join(filename);
        TableData::read_from_path(&path)
    }

    fn get_file_name(name: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        let result = hasher.finalize();
        result.iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn read_from_url(&self, url: &str) -> Option<TableData> {
        let mut dtp = bms_table::difficulty_table_parser::DifficultyTableParser::new();
        let mut dt = bms_table::difficulty_table::DifficultyTable::new();
        if url.ends_with(".json") {
            dt.table.head_url = url.to_string();
        } else {
            dt.table.source_url = url.to_string();
        }
        match dtp.decode(true, &mut dt) {
            Ok(()) => {
                let mut td = difficulty_table_to_table_data(&dt, url);
                if td.validate() {
                    Some(td)
                } else {
                    log::warn!("Difficulty table validation failed: {}", url);
                    None
                }
            }
            Err(e) => {
                log::warn!("Failed to read difficulty table {}: {}", url, e);
                None
            }
        }
    }
}

/// Table accessor trait.
/// Translated from Java: TableDataAccessor.TableAccessor
pub trait TableAccessor: Send + Sync {
    fn name(&self) -> &str;
    fn read(&self) -> Option<TableData>;
    fn write(&self, td: &mut TableData);
}

/// Difficulty table accessor.
/// Translated from Java: TableDataAccessor.DifficultyTableAccessor
pub struct DifficultyTableAccessor {
    pub name: String,
    pub tabledir: String,
    pub url: String,
}

impl DifficultyTableAccessor {
    pub fn new(tabledir: &str, url: &str) -> Self {
        Self {
            name: url.to_string(),
            tabledir: tabledir.to_string(),
            url: url.to_string(),
        }
    }
}

impl TableAccessor for DifficultyTableAccessor {
    fn name(&self) -> &str {
        &self.name
    }

    fn read(&self) -> Option<TableData> {
        let mut dtp = bms_table::difficulty_table_parser::DifficultyTableParser::new();
        let mut dt = bms_table::difficulty_table::DifficultyTable::new();
        if self.url.ends_with(".json") {
            dt.table.head_url = self.url.clone();
        } else {
            dt.table.source_url = self.url.clone();
        }
        match dtp.decode(true, &mut dt) {
            Ok(()) => {
                let mut td = difficulty_table_to_table_data(&dt, &self.url);
                if td.validate() {
                    Some(td)
                } else {
                    log::warn!("Difficulty table validation failed: {}", self.url);
                    None
                }
            }
            Err(e) => {
                log::warn!("Failed to read difficulty table {}: {}", self.url, e);
                None
            }
        }
    }

    fn write(&self, td: &mut TableData) {
        TableDataAccessor::new(&self.tabledir).write(td);
    }
}

/// Adapter from `Arc<dyn TableAccessor>` to `TableUpdateSource`.
/// Allows passing table accessors through `MainControllerAccess` trait
/// without beatoraja-types knowing about `TableAccessor`.
pub struct TableAccessorUpdateSource {
    accessor: std::sync::Arc<dyn TableAccessor>,
}

impl TableAccessorUpdateSource {
    pub fn new(accessor: std::sync::Arc<dyn TableAccessor>) -> Self {
        Self { accessor }
    }
}

impl rubato_types::table_update_source::TableUpdateSource for TableAccessorUpdateSource {
    fn source_name(&self) -> String {
        self.accessor.name().to_string()
    }

    fn refresh(&self) {
        if let Some(mut td) = self.accessor.read() {
            self.accessor.write(&mut td);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table_data::TableFolder;

    #[test]
    fn test_get_file_name_sha256() {
        // SHA-256 of "https://example.com/table" should be deterministic
        let hash = TableDataAccessor::get_file_name("https://example.com/table");
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
        // Verify it's all lowercase hex
        assert!(
            hash.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        // Same input produces same output
        let hash2 = TableDataAccessor::get_file_name("https://example.com/table");
        assert_eq!(hash, hash2);
        // Different input produces different output
        let hash3 = TableDataAccessor::get_file_name("https://example.com/other");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_write_and_read_cache_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        let mut td = TableData {
            url: "https://example.com/table".to_string(),
            name: "Test Table".to_string(),
            tag: "T".to_string(),
            folder: vec![TableFolder {
                name: Some("T1".to_string()),
                songs: vec![{
                    let mut s = crate::stubs::SongData::new();
                    s.md5 = "abcdef0123456789abcdef0123456789".to_string();
                    s.title = "Test Song".to_string();
                    s
                }],
            }],
            course: vec![],
        };

        accessor.write(&mut td);

        let cached = accessor.read_cache("https://example.com/table");
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.name, "Test Table");
        assert_eq!(cached.url, "https://example.com/table");
        assert_eq!(cached.tag, "T");
        assert_eq!(cached.folder.len(), 1);
        assert_eq!(cached.folder[0].name(), "T1");
    }

    #[test]
    fn test_read_cache_nonexistent_url() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        let cached = accessor.read_cache("https://nonexistent.example.com/table");
        assert!(cached.is_none());
    }

    #[test]
    fn test_read_all_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        let all = accessor.read_all();
        assert!(all.is_empty());
    }

    #[test]
    fn test_read_all_with_tables() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        // Write two tables
        for i in 0..2 {
            let mut td = TableData {
                url: format!("https://example.com/table{}", i),
                name: format!("Table {}", i),
                tag: format!("T{}", i),
                folder: vec![TableFolder {
                    name: Some(format!("T{}1", i)),
                    songs: vec![{
                        let mut s = crate::stubs::SongData::new();
                        s.md5 = format!("md5hash{:032}", i);
                        s.title = format!("Song {}", i);
                        s
                    }],
                }],
                course: vec![],
            };
            accessor.write(&mut td);
        }

        let all = accessor.read_all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_write_with_filename() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        let mut td = TableData {
            url: "https://example.com/custom".to_string(),
            name: "Custom Table".to_string(),
            tag: "C".to_string(),
            folder: vec![TableFolder {
                name: Some("C1".to_string()),
                songs: vec![{
                    let mut s = crate::stubs::SongData::new();
                    s.md5 = "custom_md5_hash_01234567890123".to_string();
                    s.title = "Custom Song".to_string();
                    s
                }],
            }],
            course: vec![],
        };

        accessor.write_with_filename(&mut td, "custom.bmt");

        let read_back = accessor.read("custom.bmt");
        assert!(read_back.is_some());
        assert_eq!(read_back.unwrap().name, "Custom Table");
    }

    #[test]
    fn test_read_local_table_names() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        let url = "https://example.com/named";
        let mut td = TableData {
            url: url.to_string(),
            name: "Named Table".to_string(),
            tag: "N".to_string(),
            folder: vec![TableFolder {
                name: Some("N1".to_string()),
                songs: vec![{
                    let mut s = crate::stubs::SongData::new();
                    s.md5 = "named_md5_hash_012345678901234".to_string();
                    s.title = "Named Song".to_string();
                    s
                }],
            }],
            course: vec![],
        };
        accessor.write(&mut td);

        let names = accessor.read_local_table_names(&[url]);
        assert!(names.is_some());
        let names = names.unwrap();
        assert_eq!(names.get(url).map(|s| s.as_str()), Some("Named Table"));
    }

    #[test]
    fn test_get_local_table_filenames() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        // Initially empty
        let filenames = accessor.get_local_table_filenames();
        assert!(filenames.is_some());
        assert!(filenames.unwrap().is_empty());

        // Write a table
        let mut td = TableData {
            url: "https://example.com/local".to_string(),
            name: "Local Table".to_string(),
            tag: "L".to_string(),
            folder: vec![TableFolder {
                name: Some("L1".to_string()),
                songs: vec![{
                    let mut s = crate::stubs::SongData::new();
                    s.md5 = "local_md5_hash_01234567890123".to_string();
                    s.title = "Local Song".to_string();
                    s
                }],
            }],
            course: vec![],
        };
        accessor.write(&mut td);

        let filenames = accessor.get_local_table_filenames();
        assert!(filenames.is_some());
        let filenames = filenames.unwrap();
        assert_eq!(filenames.len(), 1);
        let expected_name = format!(
            "{}.bmt",
            TableDataAccessor::get_file_name("https://example.com/local")
        );
        assert!(filenames.contains(&expected_name));
    }

    #[test]
    fn test_update_table_data_with_invalid_urls_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        // Invalid URLs will fail to download but should not panic
        accessor.update_table_data(&[
            "http://invalid.test.example/table1",
            "http://invalid.test.example/table2",
        ]);

        // No tables should be written
        let all = accessor.read_all();
        assert!(all.is_empty());
    }

    #[test]
    fn test_load_new_table_data_skips_existing() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        let url = "https://example.com/existing";

        // Pre-populate a cached table
        let mut td = TableData {
            url: url.to_string(),
            name: "Existing Table".to_string(),
            tag: "E".to_string(),
            folder: vec![TableFolder {
                name: Some("E1".to_string()),
                songs: vec![{
                    let mut s = crate::stubs::SongData::new();
                    s.md5 = "existing_md5_hash_0123456789012".to_string();
                    s.title = "Existing Song".to_string();
                    s
                }],
            }],
            course: vec![],
        };
        accessor.write(&mut td);

        // load_new_table_data should skip this URL since it already exists locally
        // (no network call should be made for it)
        accessor.load_new_table_data(&[url]);

        // The existing table should still be there, unchanged
        let cached = accessor.read_cache(url);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().name, "Existing Table");
    }

    #[test]
    fn test_update_table_data_empty_urls() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        // Should not panic with empty input
        accessor.update_table_data(&[]);

        let all = accessor.read_all();
        assert!(all.is_empty());
    }

    #[test]
    fn test_load_new_table_data_empty_urls() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = TableDataAccessor::new(dir.path().to_str().unwrap());

        // Should not panic with empty input
        accessor.load_new_table_data(&[]);

        let all = accessor.read_all();
        assert!(all.is_empty());
    }
}
