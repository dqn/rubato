//! Table (difficulty table) data structures and file I/O.
//!
//! Port of Java `TableData.java`.
//! Supports .bmt (GZIP compressed JSON) and .json formats.

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use anyhow::{Result, anyhow};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

use crate::course_data::{CourseData, CourseSongData};

/// A folder within a difficulty table, containing songs at a specific level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableFolder {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub songs: Vec<CourseSongData>,
}

impl TableFolder {
    /// Validate the folder: must have a name and at least one valid song.
    pub fn validate(&mut self) -> bool {
        self.songs.retain(|s| s.validate());
        !self.name.is_empty() && !self.songs.is_empty()
    }

    /// Clear transient fields from songs.
    pub fn shrink(&mut self) {
        for song in &mut self.songs {
            song.shrink();
        }
    }
}

/// Difficulty table data containing folders and optional courses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableData {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub folder: Vec<TableFolder>,
    #[serde(default)]
    pub course: Vec<CourseData>,
}

impl TableData {
    /// Read table data from a file path.
    ///
    /// - `.bmt` files are GZIP-compressed JSON.
    /// - `.json` files are plain JSON.
    pub fn read(path: &Path) -> Result<Self> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let mut td: TableData = match ext {
            "bmt" => {
                let file = File::open(path)?;
                let reader = BufReader::new(GzDecoder::new(file));
                serde_json::from_reader(reader)?
            }
            "json" => {
                let file = File::open(path)?;
                let reader = BufReader::new(file);
                serde_json::from_reader(reader)?
            }
            _ => {
                return Err(anyhow!(
                    "unsupported table data extension: {}",
                    path.display()
                ));
            }
        };

        if !td.validate() {
            return Err(anyhow!("invalid table data in {}", path.display()));
        }

        Ok(td)
    }

    /// Write table data to a file path.
    ///
    /// - `.bmt` files are GZIP-compressed JSON.
    /// - `.json` files are plain JSON.
    pub fn write(path: &Path, data: &TableData) -> Result<()> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let mut data = data.clone();
        data.shrink();

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("invalid table data filename: {}", path.display()))?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let tmp_path = parent.join(format!(".{file_name}.tmp-{nonce}"));

        let write_result = match ext {
            "bmt" => {
                let file = File::create(&tmp_path)?;
                let writer = BufWriter::new(file);
                let mut gz = GzEncoder::new(writer, Compression::default());
                serde_json::to_writer_pretty(&mut gz, &data)?;
                gz.try_finish()?;
                Ok(())
            }
            "json" => {
                let file = File::create(&tmp_path)?;
                let mut writer = BufWriter::new(file);
                serde_json::to_writer_pretty(&mut writer, &data)?;
                writer.flush()?;
                Ok(())
            }
            _ => Err(anyhow!(
                "unsupported table data extension: {}",
                path.display()
            )),
        };

        if let Err(e) = write_result {
            let _ = fs::remove_file(&tmp_path);
            return Err(e);
        }

        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    /// Validate the table data.
    ///
    /// Name must be non-empty, and must have at least one valid folder or course.
    pub fn validate(&mut self) -> bool {
        if self.name.is_empty() {
            return false;
        }
        self.folder.retain_mut(|f| f.validate());
        self.course.retain_mut(|c| c.validate());
        !self.folder.is_empty() || !self.course.is_empty()
    }

    /// Clear transient fields for serialization.
    pub fn shrink(&mut self) {
        for course in &mut self.course {
            course.shrink();
        }
        for folder in &mut self.folder {
            folder.shrink();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course_data::CourseDataConstraint;

    fn sample_song(sha: &str) -> CourseSongData {
        CourseSongData {
            sha256: sha.to_string(),
            md5: String::new(),
            title: format!("Song {sha}"),
        }
    }

    fn sample_table() -> TableData {
        TableData {
            url: "https://example.com/table".to_string(),
            name: "Test Table".to_string(),
            tag: "TT".to_string(),
            folder: vec![TableFolder {
                name: "Level 1".to_string(),
                songs: vec![sample_song("aaa"), sample_song("bbb")],
            }],
            course: vec![CourseData {
                name: "Dan Course".to_string(),
                hash: vec![sample_song("ccc")],
                constraint: vec![CourseDataConstraint::Class],
                trophy: Vec::new(),
                release: true,
            }],
        }
    }

    #[test]
    fn validate_empty_name() {
        let mut td = TableData {
            name: String::new(),
            folder: vec![TableFolder {
                name: "Level 1".to_string(),
                songs: vec![sample_song("a")],
            }],
            ..Default::default()
        };
        assert!(!td.validate());
    }

    #[test]
    fn validate_no_content() {
        let mut td = TableData {
            name: "Empty".to_string(),
            ..Default::default()
        };
        assert!(!td.validate());
    }

    #[test]
    fn validate_with_folders_only() {
        let mut td = TableData {
            name: "Test".to_string(),
            folder: vec![TableFolder {
                name: "Level 1".to_string(),
                songs: vec![sample_song("a")],
            }],
            ..Default::default()
        };
        assert!(td.validate());
    }

    #[test]
    fn validate_with_courses_only() {
        let mut td = TableData {
            name: "Test".to_string(),
            course: vec![CourseData {
                name: "Dan".to_string(),
                hash: vec![sample_song("a")],
                constraint: Vec::new(),
                trophy: Vec::new(),
                release: true,
            }],
            ..Default::default()
        };
        assert!(td.validate());
    }

    #[test]
    fn validate_removes_invalid_folders() {
        let mut td = TableData {
            name: "Test".to_string(),
            folder: vec![
                TableFolder {
                    name: "Valid".to_string(),
                    songs: vec![sample_song("a")],
                },
                TableFolder {
                    name: String::new(), // invalid: empty name
                    songs: vec![sample_song("b")],
                },
                TableFolder {
                    name: "Empty".to_string(),
                    songs: vec![], // invalid: no songs
                },
            ],
            ..Default::default()
        };
        assert!(td.validate());
        assert_eq!(td.folder.len(), 1);
        assert_eq!(td.folder[0].name, "Valid");
    }

    #[test]
    fn write_and_read_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");
        let td = sample_table();

        TableData::write(&path, &td).unwrap();
        let read = TableData::read(&path).unwrap();

        assert_eq!(read.name, "Test Table");
        assert_eq!(read.tag, "TT");
        assert_eq!(read.folder.len(), 1);
        assert_eq!(read.course.len(), 1);
    }

    #[test]
    fn write_and_read_bmt() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.bmt");
        let td = sample_table();

        TableData::write(&path, &td).unwrap();
        let read = TableData::read(&path).unwrap();

        assert_eq!(read.name, "Test Table");
        assert_eq!(read.url, "https://example.com/table");
        assert_eq!(read.folder.len(), 1);
        assert_eq!(read.folder[0].songs.len(), 2);
    }

    #[test]
    fn read_unsupported_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "{}").unwrap();
        assert!(TableData::read(&path).is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let td = sample_table();
        let json = serde_json::to_string_pretty(&td).unwrap();
        let parsed: TableData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, td.name);
        assert_eq!(parsed.url, td.url);
        assert_eq!(parsed.folder.len(), td.folder.len());
        assert_eq!(parsed.course.len(), td.course.len());
    }

    #[test]
    fn table_folder_validate() {
        let mut valid = TableFolder {
            name: "Level 1".to_string(),
            songs: vec![sample_song("a")],
        };
        assert!(valid.validate());

        let mut no_name = TableFolder {
            name: String::new(),
            songs: vec![sample_song("a")],
        };
        assert!(!no_name.validate());

        let mut no_songs = TableFolder {
            name: "Level 1".to_string(),
            songs: vec![],
        };
        assert!(!no_songs.validate());
    }

    #[test]
    fn table_folder_validate_removes_invalid_songs() {
        let mut folder = TableFolder {
            name: "Level 1".to_string(),
            songs: vec![
                sample_song("valid"),
                CourseSongData {
                    sha256: String::new(),
                    md5: String::new(),
                    title: "invalid".to_string(),
                },
            ],
        };
        assert!(folder.validate());
        assert_eq!(folder.songs.len(), 1);
    }
}
