use std::collections::HashMap;

use serde::Deserialize;

use crate::bar::bar::Bar;
use crate::bar::command_bar::CommandBar;
use crate::bar::container_bar::ContainerBar;
use crate::bar::directory_bar::DirectoryBarData;
use crate::bar::grade_bar::GradeBar;
use crate::bar::hash_bar::HashBar;
use crate::bar::random_course_bar::RandomCourseBar;
use crate::bar::search_word_bar::SearchWordBar;
use crate::bar::song_bar::SongBar;
use crate::bar::table_bar::TableBar;
use crate::bar_sorter::BarSorter;
use crate::stubs::*;

/// Bar manager for managing the song bar hierarchy
/// Translates: bms.player.beatoraja.select.BarManager
pub struct BarManager {
    /// Difficulty table bars
    pub tables: Vec<TableBar>,
    /// Command bars
    pub commands: Vec<Bar>,
    /// Course bar
    pub courses: Option<TableBar>,
    /// Favorite bars
    pub favorites: Vec<HashBar>,
    /// Current folder hierarchy
    pub dir: Vec<Box<Bar>>,
    pub dir_string: String,
    /// Currently displayed bars
    pub currentsongs: Vec<Bar>,
    /// Selected bar index
    pub selectedindex: usize,
    /// Source bars for each directory level
    sourcebars: Vec<Option<Bar>>,
    /// Random folder definitions
    random_folder_list: Vec<RandomFolder>,
    /// System-inserted root folders
    append_folders: HashMap<String, Bar>,
    /// Search result bars
    search: Vec<SearchWordBar>,
    /// Random course result bars
    random_course_result: Vec<RandomCourseResult>,
    /// Bar contents loader is running
    pub loader_running: bool,
}

impl Default for BarManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BarManager {
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            commands: Vec::new(),
            courses: None,
            favorites: Vec::new(),
            dir: Vec::new(),
            dir_string: String::new(),
            currentsongs: Vec::new(),
            selectedindex: 0,
            sourcebars: Vec::new(),
            random_folder_list: Vec::new(),
            append_folders: HashMap::new(),
            search: Vec::new(),
            random_course_result: Vec::new(),
            loader_running: false,
        }
    }

    pub fn init(&mut self) {
        // In Java: loads tables, courses, favorites, command folders, random folders
        // This requires: TableDataAccessor, CourseDataAccessor, SongDatabaseAccessor,
        // IRConnection, BMSSearchAccessor, JSON parsing, etc.
        log::warn!(
            "not yet implemented: BarManager.init - requires MusicSelector and TableDataAccessor context"
        );
    }

    pub fn update_bar_refresh(&mut self) -> bool {
        if !self.dir.is_empty() {
            // In Java: updateBar(dir.last())
            return self.update_bar_with_last_dir();
        }
        self.update_bar(None)
    }

    fn update_bar_with_last_dir(&mut self) -> bool {
        // Stub: would re-enter last directory
        log::warn!("not yet implemented: BarManager.updateBar(dir.last()) - requires full context");
        false
    }

    pub fn update_bar(&mut self, _bar: Option<&Bar>) -> bool {
        // In Java: complex method that rebuilds currentsongs based on the bar argument
        // - null = root level (tables, commands, favorites, search)
        // - DirectoryBar = open folder, get children, filter, sort
        // - Handles mode filtering, invisible charts, sorting, random select
        // - Starts BarContentsLoaderThread for score/banner/stagefile loading
        log::warn!(
            "not yet implemented: BarManager.updateBar - requires full MusicSelector context"
        );
        false
    }

    pub fn close(&mut self) {
        // In Java: goes up one directory level
        if self.dir.is_empty() {
            // At root level: toggle sort
            SongManagerMenu::force_disable_last_played_sort();
            return;
        }
        // In Java: removes last dir, updates to parent
        log::warn!("not yet implemented: BarManager.close - requires MusicSelector context");
    }

    pub fn get_directory(&self) -> &[Box<Bar>] {
        &self.dir
    }

    pub fn get_directory_string(&self) -> &str {
        &self.dir_string
    }

    pub fn get_selected(&self) -> Option<&Bar> {
        if self.currentsongs.is_empty() {
            None
        } else {
            Some(&self.currentsongs[self.selectedindex])
        }
    }

    pub fn set_selected(&mut self, bar: &Bar) {
        for i in 0..self.currentsongs.len() {
            if self.currentsongs[i].get_title() == bar.get_title() {
                self.selectedindex = i;
                break;
            }
        }
    }

    pub fn get_selected_position(&self) -> f32 {
        if self.currentsongs.is_empty() {
            0.0
        } else {
            self.selectedindex as f32 / self.currentsongs.len() as f32
        }
    }

    pub fn get_tables(&self) -> &[TableBar] {
        &self.tables
    }

    pub fn set_selected_position(&mut self, value: f32) {
        if (0.0..1.0).contains(&value) && !self.currentsongs.is_empty() {
            self.selectedindex = (self.currentsongs.len() as f32 * value) as usize;
        }
    }

    pub fn mov(&mut self, increase: bool) {
        if self.currentsongs.is_empty() {
            return;
        }
        if increase {
            self.selectedindex += 1;
        } else {
            self.selectedindex += self.currentsongs.len() - 1;
        }
        self.selectedindex %= self.currentsongs.len();
    }

    pub fn add_search(&mut self, bar: SearchWordBar) {
        // Remove existing search with same title
        self.search.retain(|s| s.get_title() != bar.get_title());
        // In Java: checks max search bar count from config
        if self.search.len() >= 10 {
            self.search.remove(0);
        }
        self.search.push(bar);
    }

    pub fn add_random_course(&mut self, bar: GradeBar, dir_string: String) {
        if self.random_course_result.len() >= 100 {
            self.random_course_result.remove(0);
        }
        self.random_course_result.push(RandomCourseResult {
            course: bar,
            dir_string,
        });
    }

    pub fn set_append_directory_bar(&mut self, key: String, bar: Bar) {
        self.append_folders.insert(key, bar);
    }

    /// Create a command bar from a CommandFolder definition.
    /// Corresponds to Java BarManager.createCommandBar(MusicSelector, CommandFolder)
    fn create_command_bar(&self, folder: &CommandFolder) -> Bar {
        let has_subfolders = !folder.get_folder().is_empty();
        let has_random_courses = !folder.get_random_course().is_empty();

        if has_subfolders || has_random_courses {
            let mut children: Vec<Bar> = Vec::new();
            // Recursively create child bars for sub-folders
            for child in folder.get_folder() {
                children.push(self.create_command_bar(child));
            }
            // Create RandomCourseBar for random courses
            for rc in folder.get_random_course() {
                children.push(Bar::RandomCourse(Box::new(RandomCourseBar::new(
                    rc.clone(),
                ))));
            }
            Bar::Container(Box::new(ContainerBar::new(
                folder.get_name().to_string(),
                children,
            )))
        } else {
            Bar::Command(Box::new(CommandBar::new_with_visibility(
                folder.get_name().to_string(),
                folder.get_sql().unwrap_or("").to_string(),
                folder.is_showall(),
            )))
        }
    }
}

/// Command folder definition (loaded from JSON)
/// Translates: bms.player.beatoraja.select.BarManager.CommandFolder
#[derive(Clone, Debug, Default, Deserialize)]
pub struct CommandFolder {
    pub name: Option<String>,
    #[serde(default)]
    pub folder: Vec<CommandFolder>,
    pub sql: Option<String>,
    #[serde(default)]
    pub rcourse: Vec<RandomCourseData>,
    #[serde(default)]
    pub showall: bool,
}

impl CommandFolder {
    pub fn get_name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
    pub fn get_folder(&self) -> &[CommandFolder] {
        &self.folder
    }
    pub fn get_sql(&self) -> Option<&str> {
        self.sql.as_deref()
    }
    pub fn get_random_course(&self) -> &[RandomCourseData] {
        &self.rcourse
    }
    pub fn is_showall(&self) -> bool {
        self.showall
    }
}

/// Random folder definition (loaded from JSON)
/// Translates: bms.player.beatoraja.select.BarManager.RandomFolder
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RandomFolder {
    pub name: Option<String>,
    pub filter: Option<HashMap<String, serde_json::Value>>,
}

impl RandomFolder {
    pub fn get_name(&self) -> String {
        format!("[RANDOM] {}", self.name.as_deref().unwrap_or(""))
    }

    pub fn get_filter(&self) -> Option<&HashMap<String, serde_json::Value>> {
        self.filter.as_ref()
    }

    pub fn filter_song(&self, score_data: Option<&ScoreData>) -> bool {
        let filter = match &self.filter {
            Some(f) => f,
            None => return true,
        };

        for (key, value) in filter {
            // In Java: uses reflection to call getters on ScoreData
            // This is a simplified version that handles integer comparison
            if let Some(int_value) = value.as_i64() {
                let int_value = int_value as i32;
                if let Some(score) = score_data {
                    // In Java: uses reflection. In Rust, we match on field name.
                    let property_value = get_score_data_property(score, key);
                    if property_value != int_value {
                        return false;
                    }
                } else if int_value != 0 {
                    return false;
                }
                return true;
            }

            // String filter with comparison operators
            if let Some(str_value) = value.as_str() {
                let parts: Vec<&str> = str_value.split("&&").collect();
                for part in parts {
                    let part = part.trim();
                    if let Some(score) = score_data {
                        let property_value = get_score_data_property(score, key);
                        if !evaluate_filter_expression(part, property_value) {
                            return false;
                        }
                    } else if !part.is_empty() && !part.starts_with('<') {
                        return false;
                    }
                }
            }
        }
        true
    }
}

fn get_score_data_property(score: &ScoreData, key: &str) -> i32 {
    match key {
        "clear" => score.get_clear(),
        "exscore" => score.get_exscore(),
        "notes" => score.get_notes(),
        "minbp" => score.get_minbp(),
        _ => 0,
    }
}

fn evaluate_filter_expression(expr: &str, property_value: i32) -> bool {
    if expr.is_empty() {
        return true;
    }
    if let Some(stripped) = expr.strip_prefix(">=") {
        if let Ok(v) = stripped.parse::<i32>() {
            return property_value >= v;
        }
    } else if let Some(stripped) = expr.strip_prefix("<=") {
        if let Ok(v) = stripped.parse::<i32>() {
            return property_value <= v;
        }
    } else if let Some(stripped) = expr.strip_prefix('>') {
        if let Ok(v) = stripped.parse::<i32>() {
            return property_value > v;
        }
    } else if let Some(stripped) = expr.strip_prefix('<')
        && let Ok(v) = stripped.parse::<i32>()
    {
        return property_value < v;
    }
    true
}

/// Random course result
struct RandomCourseResult {
    pub course: GradeBar,
    pub dir_string: String,
}

/// Thread for loading score data, banners, and stagefiles for bar contents.
/// Corresponds to Java BarManager.BarContentsLoaderThread
pub struct BarContentsLoaderThread {
    bars: Vec<Bar>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl BarContentsLoaderThread {
    /// Create a new bar contents loader thread.
    /// Corresponds to Java BarContentsLoaderThread(MusicSelector, Bar[])
    pub fn new(bars: Vec<Bar>) -> Self {
        Self {
            bars,
            stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Run the loader (loads scores, song info, banners, stagefiles).
    /// Corresponds to Java BarContentsLoaderThread.run()
    pub fn run(&self) {
        // In Java: iterates bars, loads scores for SongBar/GradeBar,
        // updates folder status for DirectoryBar, loads song information,
        // then loads banners and stagefiles
        // Requires MusicSelector, ScoreDataCache, PlayDataAccessor, SongInformationAccessor,
        // PixmapResourcePool (banners/stagefiles)
        log::warn!(
            "not yet implemented: BarContentsLoaderThread.run - requires full MusicSelector context"
        );
    }

    /// Stop the loader thread.
    /// Corresponds to Java BarContentsLoaderThread.stopRunning()
    pub fn stop_running(&self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if the loader has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.stop.load(std::sync::atomic::Ordering::SeqCst)
    }
}
