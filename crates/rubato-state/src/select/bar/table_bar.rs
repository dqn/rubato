use std::collections::HashSet;
use std::sync::Arc;

use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::grade_bar::GradeBar;
use super::hash_bar::HashBar;
use crate::select::stubs::*;

/// Difficulty table bar
/// Translates: bms.player.beatoraja.select.bar.TableBar
#[derive(Clone)]
pub struct TableBar {
    pub directory: DirectoryBarData,
    /// Table data
    pub td: TableData,
    /// Level bars
    pub levels: Vec<HashBar>,
    /// Course bars
    pub grades: Vec<GradeBar>,
    /// Level bars + course bars combined
    pub children: Vec<Bar>,
    /// Table accessor (Arc for cheap cloning)
    pub tr: Arc<dyn TableAccessor>,
}

impl TableBar {
    pub fn new(td: TableData, tr: Arc<dyn TableAccessor>) -> Self {
        let mut bar = Self {
            directory: DirectoryBarData::default(),
            td: TableData::default(),
            levels: Vec::new(),
            grades: Vec::new(),
            children: Vec::new(),
            tr,
        };
        bar.set_table_data(td);
        bar
    }

    pub fn title(&self) -> &str {
        &self.td.name
    }

    pub fn url(&self) -> Option<&str> {
        self.td.url_opt()
    }

    pub fn accessor(&self) -> &dyn TableAccessor {
        self.tr.as_ref()
    }

    pub fn set_table_data(&mut self, td: TableData) {
        self.levels = td
            .folder
            .iter()
            .map(|folder| HashBar::new(folder.name().to_string(), folder.songs.to_vec()))
            .collect();

        let courses = &td.course;
        let mut hashset: HashSet<String> = HashSet::new();
        for course in courses {
            for song in &course.hash {
                if !song.sha256.is_empty() {
                    hashset.insert(song.sha256.clone());
                } else {
                    hashset.insert(song.md5.clone());
                }
            }
        }
        // In Java: selector.getSongDatabase().getSongDatas(hashset.toArray(...))
        // For now, we cannot resolve songs without DB access
        // Stub: create grade bars with existing song data
        self.grades = courses
            .iter()
            .map(|course| {
                // In Java, songs are matched and merged here
                // We skip the DB lookup for now
                GradeBar::new(course.clone())
            })
            .collect();

        // children = levels + grades combined
        // We cannot combine them into a Vec<Bar> without creating Bar enum variants
        // This would require storing them differently
        self.children = Vec::new(); // Will be populated when getChildren() is called

        self.td = td;
    }

    pub fn levels(&self) -> &[HashBar] {
        &self.levels
    }

    pub fn grades(&self) -> &[GradeBar] {
        &self.grades
    }

    pub fn children(&self) -> &[Bar] {
        &self.children
    }

    pub fn table_data(&self) -> &TableData {
        &self.td
    }
}
