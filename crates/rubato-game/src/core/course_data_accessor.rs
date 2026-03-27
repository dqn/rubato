use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use crate::core::course_data::CourseData;
use crate::core::validatable::Validatable;

/// Course data accessor.
/// Translated from Java: CourseDataAccessor
pub struct CourseDataAccessor {
    coursedir: String,
}

impl CourseDataAccessor {
    pub fn new(path: &str) -> Self {
        let _ = fs::create_dir_all(path);
        Self {
            coursedir: path.to_string(),
        }
    }

    /// Read all course data.
    pub fn read_all(&self) -> Vec<CourseData> {
        self.read_all_names()
            .iter()
            .flat_map(|name| self.read(name))
            .collect()
    }

    pub fn read_all_names(&self) -> Vec<String> {
        let dir = Path::new(&self.coursedir);
        match fs::read_dir(dir) {
            Ok(entries) => entries
                .flatten()
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    name.rfind('.').map(|idx| name[..idx].to_string())
                })
                .collect(),
            Err(e) => {
                log::error!("Failed to read course directory: {}", e);
                Vec::new()
            }
        }
    }

    pub fn read(&self, name: &str) -> Vec<CourseData> {
        let path = PathBuf::from(&self.coursedir).join(format!("{}.json", name));

        // Try reading as array first
        if let Ok(file) = fs::File::open(&path) {
            let reader = BufReader::new(file);
            if let Ok(courses) = serde_json::from_reader::<_, Vec<CourseData>>(reader) {
                let valid: Vec<CourseData> = courses
                    .into_iter()
                    .filter_map(|mut c| if c.validate() { Some(c) } else { None })
                    .collect();
                return valid;
            }
        }

        // Try reading as single object
        if let Ok(file) = fs::File::open(&path) {
            let reader = BufReader::new(file);
            if let Ok(mut course) = serde_json::from_reader::<_, CourseData>(reader)
                && course.validate()
            {
                return vec![course];
            }
        }

        Vec::new()
    }

    /// Write course data.
    pub fn write(&self, name: &str, cd: &[CourseData]) {
        let path = PathBuf::from(&self.coursedir).join(format!("{}.json", name));
        match fs::File::create(&path) {
            Ok(file) => {
                let writer = BufWriter::new(file);
                if let Err(e) = serde_json::to_writer_pretty(writer, cd) {
                    log::error!("Failed to write course data: {}", e);
                }
            }
            Err(e) => {
                log::error!("Failed to create course file: {}", e);
            }
        }
    }
}
