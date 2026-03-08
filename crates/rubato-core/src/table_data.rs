use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

use crate::course_data::CourseData;
use crate::stubs::SongData;
use crate::validatable::{Validatable, remove_invalid_elements_vec};

/// Table data (difficulty table)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TableData {
    pub url: String,
    pub name: String,
    pub tag: String,
    pub folder: Vec<TableFolder>,
    pub course: Vec<CourseData>,
}

impl TableData {
    pub fn url_opt(&self) -> Option<&str> {
        if self.url.is_empty() {
            None
        } else {
            Some(&self.url)
        }
    }
    pub fn shrink(&mut self) {
        for c in &mut self.course {
            c.shrink();
        }
        for tf in &mut self.folder {
            tf.shrink();
        }
    }

    pub fn read_from_path(p: &Path) -> Option<TableData> {
        let path_str = p.to_string_lossy();
        let data: Option<Vec<u8>> = if path_str.ends_with(".bmt") {
            let file = std::fs::File::open(p).ok()?;
            let mut gz = GzDecoder::new(BufReader::new(file));
            let mut buf = Vec::new();
            gz.read_to_end(&mut buf).ok()?;
            Some(buf)
        } else if path_str.ends_with(".json") {
            std::fs::read(p).ok()
        } else {
            None
        };

        if let Some(data) = data {
            let mut td: TableData = serde_json::from_slice(&data).ok()?;
            if td.validate() {
                return Some(td);
            }
        }
        None
    }

    pub fn write_to_path(p: &Path, td: &TableData) -> Result<()> {
        let mut td = td.clone();
        td.shrink();
        let path_str = p.to_string_lossy();
        let json = serde_json::to_string_pretty(&td).context("failed to serialize table data")?;

        if path_str.ends_with(".bmt") {
            let file = std::fs::File::create(p)
                .with_context(|| format!("failed to create file: {}", p.display()))?;
            let mut encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
            encoder
                .write_all(json.as_bytes())
                .with_context(|| format!("failed to write gzip data: {}", p.display()))?;
            encoder
                .finish()
                .with_context(|| format!("failed to finish gzip encoding: {}", p.display()))?;
        } else if path_str.ends_with(".json") {
            let mut file = std::fs::File::create(p)
                .with_context(|| format!("failed to create file: {}", p.display()))?;
            file.write_all(json.as_bytes())
                .with_context(|| format!("failed to write JSON data: {}", p.display()))?;
        } else {
            bail!("unsupported file extension: {}", p.display());
        }
        Ok(())
    }
}

impl Validatable for TableData {
    fn validate(&mut self) -> bool {
        if self.name.is_empty() {
            return false;
        }
        self.folder = remove_invalid_elements_vec(std::mem::take(&mut self.folder));
        self.course = remove_invalid_elements_vec(std::mem::take(&mut self.course));
        self.folder.len() + self.course.len() > 0
    }
}

/// Table folder
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TableFolder {
    pub name: Option<String>,
    pub songs: Vec<SongData>,
}

impl TableFolder {
    pub fn name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
    pub fn shrink(&mut self) {
        for song in &mut self.songs {
            song.shrink();
        }
    }
}

impl Validatable for TableFolder {
    fn validate(&mut self) -> bool {
        self.songs.retain_mut(|s| s.validate());
        self.name.as_ref().is_some_and(|n| !n.is_empty()) && !self.songs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::SongData;

    /// Helper to create a minimal valid TableData for testing.
    fn make_valid_table_data() -> TableData {
        TableData {
            name: "test-table".to_string(),
            url: "http://example.com/table".to_string(),
            tag: String::new(),
            folder: vec![TableFolder {
                name: Some("Normal".to_string()),
                songs: vec![{
                    let mut s = SongData::default();
                    s.title = "test-song".to_string();
                    s.md5 = "d41d8cd98f00b204e9800998ecf8427e".to_string();
                    s
                }],
            }],
            course: Vec::new(),
        }
    }

    #[test]
    fn write_and_read_roundtrip_bmt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.bmt");
        let td = make_valid_table_data();

        TableData::write_to_path(&path, &td).unwrap();

        let loaded = TableData::read_from_path(&path).expect("should read back .bmt");
        assert_eq!(loaded.name, "test-table");
        assert_eq!(loaded.folder.len(), 1);
        assert_eq!(loaded.folder[0].songs.len(), 1);
    }

    #[test]
    fn write_and_read_roundtrip_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");
        let td = make_valid_table_data();

        TableData::write_to_path(&path, &td).unwrap();

        let loaded = TableData::read_from_path(&path).expect("should read back .json");
        assert_eq!(loaded.name, "test-table");
        assert_eq!(loaded.folder.len(), 1);
    }

    #[test]
    fn write_to_path_returns_error_for_unsupported_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let td = make_valid_table_data();

        let result = TableData::write_to_path(&path, &td);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unsupported file extension"),
            "error message should mention unsupported extension"
        );
    }

    #[test]
    fn write_to_path_returns_error_for_nonexistent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no/such/dir/test.bmt");
        let td = make_valid_table_data();

        let result = TableData::write_to_path(&path, &td);
        assert!(
            result.is_err(),
            "writing to a nonexistent directory should fail"
        );
    }
}
