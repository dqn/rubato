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
        // TODO: parallel download requires DifficultyTableParser
        for url in urls {
            if let Some(mut td) = self.read_from_url(url) {
                self.write(&mut td);
            }
        }
    }

    pub fn load_new_table_data(&self, urls: &[&str]) {
        let local_tables = self.get_local_table_filenames();
        for url in urls {
            let filename = format!("{}.bmt", Self::get_file_name(url));
            if let Some(ref locals) = local_tables
                && locals.contains(&filename)
            {
                continue;
            }
            if let Some(mut td) = self.read_from_url(url) {
                self.write(&mut td);
            }
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
                        file_name_to_table_name.insert(filename, td.get_name().to_string());
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
        let path = PathBuf::from(&self.tabledir)
            .join(format!("{}.bmt", Self::get_file_name(td.get_url())));
        TableData::write_to_path(&path, td);
    }

    pub fn write_with_filename(&self, td: &mut TableData, filename: &str) {
        let path = PathBuf::from(&self.tabledir).join(filename);
        TableData::write_to_path(&path, td);
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
            dt.table.set_head_url(url);
        } else {
            dt.table.set_source_url(url);
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
            dt.table.set_head_url(&self.url);
        } else {
            dt.table.set_source_url(&self.url);
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

impl beatoraja_types::table_update_source::TableUpdateSource for TableAccessorUpdateSource {
    fn source_name(&self) -> String {
        self.accessor.name().to_string()
    }

    fn refresh(&self) {
        if let Some(mut td) = self.accessor.read() {
            self.accessor.write(&mut td);
        }
    }
}
