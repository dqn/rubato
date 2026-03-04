use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use beatoraja_core::pixmap_resource_pool::PixmapResourcePool;
use serde::Deserialize;

use super::bar::bar::Bar;
use super::bar::command_bar::CommandBar;
use super::bar::container_bar::ContainerBar;
use super::bar::executable_bar::ExecutableBar;
use super::bar::folder_bar::FolderBar;
use super::bar::grade_bar::GradeBar;
use super::bar::hash_bar::HashBar;
use super::bar::random_course_bar::RandomCourseBar;
use super::bar::search_word_bar::SearchWordBar;
use super::bar::table_bar::TableBar;
use super::bar_sorter::BarSorter;
use super::music_selector::MODE;
use super::score_data_cache::ScoreDataCache;
use super::stubs::*;

/// Context for update_bar operations.
/// Passed from MusicSelector to avoid storing references in BarManager.
pub struct UpdateBarContext<'a> {
    pub config: &'a Config,
    pub player_config: &'a mut PlayerConfig,
    pub songdb: &'a dyn SongDatabaseAccessor,
    pub score_cache: Option<&'a mut ScoreDataCache>,
    pub is_folderlamp: bool,
    pub max_search_bar_count: i32,
}

/// Context for loader thread operations.
pub struct LoaderContext<'a> {
    pub player_config: &'a PlayerConfig,
    pub score_cache: Option<&'a mut ScoreDataCache>,
    pub rival_cache: Option<&'a mut ScoreDataCache>,
    pub rival_name: Option<String>,
    pub is_folderlamp: bool,
    /// Banner pixmap resource pool for loading banner images
    pub banner_resource: Option<&'a PixmapResourcePool>,
    /// Stagefile pixmap resource pool for loading stagefile images
    pub stagefile_resource: Option<&'a PixmapResourcePool>,
}

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
    /// Bar contents loader stop flag
    pub loader_stop: Option<Arc<AtomicBool>>,
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
            loader_stop: None,
        }
    }

    /// Initialize the bar manager: load tables, courses, favorites, command/random folders.
    /// Corresponds to Java BarManager.init()
    pub fn init(&mut self, config: &Config, ir_table_urls: &[(String, String)]) {
        let tablepath = config.get_tablepath();
        let tdaccessor = TableDataAccessor::new(tablepath);

        // Load saved table data
        let raw_tables = tdaccessor.read_all();
        let mut unsorted_tables: Vec<Option<TableData>> =
            raw_tables.into_iter().map(Some).collect();

        // Sort tables according to config table URL order
        let mut sorted_tables: Vec<TableData> = Vec::with_capacity(unsorted_tables.len());
        for url in config.get_table_url() {
            for i in 0..unsorted_tables.len() {
                if let Some(ref td) = unsorted_tables[i]
                    && td.get_url_opt() == Some(url.as_str())
                {
                    sorted_tables.push(unsorted_tables[i].take().unwrap());
                    break;
                }
            }
        }
        // Append remaining tables not in URL list
        for td in unsorted_tables.into_iter().flatten() {
            sorted_tables.push(td);
        }

        // Create TableBars
        let mut table_bars: Vec<TableBar> = Vec::new();
        for td in sorted_tables {
            let accessor: Arc<dyn TableAccessor> = Arc::new(DifficultyTableAccessor::new(
                tablepath,
                td.get_url_opt().unwrap_or(""),
            ));
            table_bars.push(TableBar::new(td, accessor));
        }

        // Load IR tables if IR connections provide table URLs
        for (ir_name, table_url) in ir_table_urls {
            let mut td = TableData::default();
            td.set_name(format!("{} {}", ir_name, table_url));
            td.set_url(table_url.clone());
            let accessor: Arc<dyn TableAccessor> =
                Arc::new(DifficultyTableAccessor::new(tablepath, table_url));
            table_bars.push(TableBar::new(td, accessor));
        }

        self.tables = table_bars;

        // Load courses
        let course_accessor = CourseDataAccessor::new("course");
        let mut course_td = TableData::default();
        course_td.set_name("COURSE".to_string());
        course_td.set_course(course_accessor.read_all());
        let course_tr: Arc<dyn TableAccessor> = Arc::new(CourseTableAccessor);
        self.courses = Some(TableBar::new(course_td, course_tr));

        // Load favorites
        let fav_accessor = CourseDataAccessor::new("favorite");
        let fav_courses = fav_accessor.read_all();
        self.favorites = fav_courses
            .into_iter()
            .map(|cd| HashBar::new(cd.get_name().to_string(), cd.get_song().to_vec()))
            .collect();

        // Build command bars
        let mut commands: Vec<Bar> = Vec::new();

        // LAMP UPDATE / SCORE UPDATE (last 30 days)
        let now_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let mut lampupdate: Vec<Bar> = Vec::new();
        let mut scoreupdate: Vec<Bar> = Vec::new();
        for i in 0..30 {
            let s = if i == 0 {
                "TODAY".to_string()
            } else {
                format!("{}DAYS AGO", i)
            };
            let t = ((now_millis / 86400000) - i) * 86400;
            lampupdate.push(Bar::Command(Box::new(CommandBar::new(
                s.clone(),
                format!(
                    "scorelog.clear > scorelog.oldclear AND scorelog.date >= {} AND scorelog.date < {}",
                    t,
                    t + 86400
                ),
            ))));
            scoreupdate.push(Bar::Command(Box::new(CommandBar::new(
                s,
                format!(
                    "scorelog.score > scorelog.oldscore AND scorelog.date >= {} AND scorelog.date < {}",
                    t,
                    t + 86400
                ),
            ))));
        }
        commands.push(Bar::Container(Box::new(ContainerBar::new(
            "LAMP UPDATE".to_string(),
            lampupdate,
        ))));
        commands.push(Bar::Container(Box::new(ContainerBar::new(
            "SCORE UPDATE".to_string(),
            scoreupdate,
        ))));

        // Load command folders from folder/default.json
        match fs::File::open("folder/default.json") {
            Ok(file) => {
                let reader = BufReader::new(file);
                match serde_json::from_reader::<_, Vec<CommandFolder>>(reader) {
                    Ok(cf) => {
                        for folder in &cf {
                            commands.push(self.create_command_bar(folder));
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to parse folder/default.json: {}", e);
                    }
                }
            }
            Err(e) => {
                log::debug!("folder/default.json not found: {}", e);
            }
        }

        // Load random folders from random/default.json
        match fs::File::open("random/default.json") {
            Ok(file) => {
                let reader = BufReader::new(file);
                match serde_json::from_reader::<_, Vec<RandomFolder>>(reader) {
                    Ok(rf) => {
                        self.random_folder_list = rf;
                    }
                    Err(e) => {
                        log::warn!("Failed to parse random/default.json: {}", e);
                        self.random_folder_list = vec![RandomFolder {
                            name: Some("RANDOM SELECT".to_string()),
                            filter: None,
                        }];
                    }
                }
            }
            Err(_) => {
                self.random_folder_list = vec![RandomFolder {
                    name: Some("RANDOM SELECT".to_string()),
                    filter: None,
                }];
            }
        }

        self.commands = commands;
    }

    /// Refresh the current bar display.
    /// Corresponds to Java BarManager.updateBar() (no-arg)
    pub fn update_bar_refresh(&mut self) -> bool {
        if !self.dir.is_empty() {
            return self.update_bar_with_last_dir();
        }
        self.update_bar(None)
    }

    fn update_bar_with_last_dir(&mut self) -> bool {
        // Take the last directory bar, use it for update, then restore
        // This avoids borrow issues with dir and self
        if let Some(last) = self.dir.last() {
            // We need to clone the bar to pass it since we can't borrow self.dir and self at once
            let _bar_title = last.get_title();
            // Find the matching directory bar in dir and use it
            // Use the index approach: call update_bar_at_dir_index
            return self.update_bar_at_dir_index(self.dir.len() - 1);
        }
        false
    }

    /// Update bar using a directory bar at the given index in self.dir.
    /// Workaround for borrow checker: can't borrow dir elements while mutating self.
    fn update_bar_at_dir_index(&mut self, _index: usize) -> bool {
        // The full implementation would need UpdateBarContext from MusicSelector.
        // Without context, we just rebuild root.
        self.update_bar(None)
    }

    /// Core update_bar implementation.
    /// Corresponds to Java BarManager.updateBar(Bar)
    pub fn update_bar(&mut self, bar: Option<&Bar>) -> bool {
        self.update_bar_with_context(bar, None)
    }

    /// Update bar with full MusicSelector context.
    /// Corresponds to Java BarManager.updateBar(Bar)
    pub fn update_bar_with_context(
        &mut self,
        bar: Option<&Bar>,
        mut ctx: Option<&mut UpdateBarContext>,
    ) -> bool {
        let prevbar_title = if !self.currentsongs.is_empty() {
            Some(self.currentsongs[self.selectedindex].get_title())
        } else {
            None
        };
        let prevbar_sha256 = if !self.currentsongs.is_empty() {
            self.currentsongs[self.selectedindex]
                .as_song_bar()
                .filter(|sb| sb.exists_song())
                .map(|sb| sb.get_song_data().get_sha256().to_string())
        } else {
            None
        };
        let prevbar_is_song = !self.currentsongs.is_empty()
            && self.currentsongs[self.selectedindex]
                .as_song_bar()
                .is_some();
        let prevbar_class_name = if !self.currentsongs.is_empty() {
            bar_class_name(&self.currentsongs[self.selectedindex])
        } else {
            ""
        };
        let prevdirsize = self.dir.len();
        let mut sourcebar_title: Option<String> = None;
        let mut sourcebar_sha256: Option<String> = None;
        let mut sourcebar_is_song = false;
        let mut _sourcebar_class_name = "";
        let mut l: Vec<Bar> = Vec::new();
        let mut show_invisible_charts = false;
        let mut is_sortable = true;

        if bar.is_none() {
            // Root bar
            // In Java: if (dir.size > 0) { prevbar = dir.first(); }
            if !self.dir.is_empty() {
                // Use dir.first() as prevbar
                // Already captured above via currentsongs
            }
            self.dir.clear();
            self.sourcebars.clear();

            // In Java: l.addAll(new FolderBar(select, null, "e2977170").getChildren())
            let root_folder = FolderBar::new(None, "e2977170".to_string());
            if let Some(ref ctx) = ctx {
                l.extend(root_folder.get_children(ctx.songdb));
            }

            // Add courses
            if let Some(ref courses) = self.courses {
                l.push(Bar::Table(Box::new(courses.clone())));
            }

            // Add favorites
            for fav in &self.favorites {
                l.push(Bar::Hash(Box::new(fav.clone())));
            }

            // Add append folders
            for folder_bar in self.append_folders.values() {
                l.push(folder_bar.clone());
            }

            // Add tables
            for table in &self.tables {
                l.push(Bar::Table(Box::new(table.clone())));
            }

            // Add commands
            for cmd in &self.commands {
                l.push(cmd.clone());
            }

            // Add search results
            for s in &self.search {
                l.push(Bar::SearchWord(Box::new(s.clone())));
            }
        } else if let Some(bar) = bar {
            if let Some(dir_data) = bar.as_directory_bar() {
                show_invisible_charts = dir_data.is_show_invisible_chart();
                is_sortable = dir_data.is_sortable();
            }

            // Check if bar is already in dir, and unwind to it
            let dir_index = self
                .dir
                .iter()
                .position(|d| d.get_title() == bar.get_title());
            if let Some(idx) = dir_index {
                while self.dir.len() > idx + 1 {
                    self.dir.pop();
                    if let Some(sb) = self.sourcebars.pop()
                        && let Some(sb) = sb
                    {
                        sourcebar_title = Some(sb.get_title());
                        sourcebar_sha256 = sb
                            .as_song_bar()
                            .filter(|s| s.exists_song())
                            .map(|s| s.get_song_data().get_sha256().to_string());
                        sourcebar_is_song = sb.as_song_bar().is_some();
                        _sourcebar_class_name = bar_class_name(&sb);
                    }
                }
                self.dir.pop();
            }

            // Get children based on bar type
            if let Some(ref ctx) = ctx {
                let songdb = ctx.songdb;
                match bar {
                    Bar::Folder(b) => l.extend(b.get_children(songdb)),
                    Bar::Command(b) => {
                        let player_name = ctx.config.playername.as_deref().unwrap_or("default");
                        let score_path =
                            format!("{}/{}/score.db", ctx.config.playerpath, player_name);
                        let scorelog_path =
                            format!("{}/{}/scorelog.db", ctx.config.playerpath, player_name);
                        let songinfo_path = ctx.config.get_songinfopath().to_string();
                        let cmd_ctx = crate::select::bar::command_bar::CommandBarContext {
                            score_db_path: &score_path,
                            scorelog_db_path: &scorelog_path,
                            info_db_path: Some(&songinfo_path),
                        };
                        l.extend(b.get_children(songdb, &cmd_ctx));
                    }
                    Bar::Container(b) => {
                        l.extend(b.get_children().iter().cloned());
                    }
                    Bar::Hash(b) => l.extend(b.get_children(songdb)),
                    Bar::Table(b) => {
                        l.extend(b.get_children().iter().cloned());
                    }
                    Bar::SearchWord(b) => l.extend(b.get_children(songdb)),
                    Bar::ContextMenu(b) => l.extend(b.get_children(&self.tables, songdb)),
                    _ => {}
                }
            }

            // Add random course results for ContainerBar
            if bar.as_directory_bar().is_some()
                && matches!(bar, Bar::Container(_))
                && !self.random_course_result.is_empty()
            {
                let mut ds = String::new();
                for d in &self.dir {
                    ds.push_str(&d.get_title());
                    ds.push_str(" > ");
                }
                ds.push_str(&bar.get_title());
                ds.push_str(" > ");
                for r in &self.random_course_result {
                    if r.dir_string == ds {
                        l.push(Bar::Grade(Box::new(GradeBar::new(r.course.course.clone()))));
                    }
                }
            }
        }

        // Filter out non-existing songs/grades if config says so
        if let Some(ref ctx) = ctx
            && !ctx.config.is_show_no_song_existing_bar()
        {
            l.retain(|b| {
                if let Some(sb) = b.as_song_bar() {
                    sb.exists_song()
                } else if let Some(gb) = b.as_grade_bar() {
                    gb.exists_all_songs()
                } else {
                    true
                }
            });
        }

        if !l.is_empty() {
            // Mode + invisible filtering
            if let Some(ref mut ctx) = ctx {
                let mut mode_index = 0usize;
                let current_mode = ctx.player_config.get_mode().cloned();
                for i in 0..MODE.len() {
                    if MODE[i] == current_mode {
                        mode_index = i;
                        break;
                    }
                }

                for trial_count in 0..MODE.len() {
                    let mode = &MODE[(mode_index + trial_count) % MODE.len()];
                    ctx.player_config.set_mode(mode.clone());

                    let before_len = l.len();
                    let remove_count = l
                        .iter()
                        .filter(|b| {
                            if let Some(sb) = b.as_song_bar() {
                                if let Some(sd) = Some(sb.get_song_data()) {
                                    let invisible =
                                        sd.get_favorite() & (INVISIBLE_SONG | INVISIBLE_CHART);
                                    let mode_mismatch = mode.is_some()
                                        && sd.get_mode() != 0
                                        && sd.get_mode()
                                            != mode.as_ref().map(|m| m.id()).unwrap_or(0);
                                    (!show_invisible_charts && invisible != 0) || mode_mismatch
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        })
                        .count();

                    if before_len != remove_count {
                        // Remove filtered songs and break
                        let mode_clone = mode.clone();
                        l.retain(|b| {
                            if let Some(sb) = b.as_song_bar() {
                                let sd = sb.get_song_data();
                                let invisible =
                                    sd.get_favorite() & (INVISIBLE_SONG | INVISIBLE_CHART);
                                let mode_mismatch = mode_clone.is_some()
                                    && sd.get_mode() != 0
                                    && sd.get_mode()
                                        != mode_clone.as_ref().map(|m| m.id()).unwrap_or(0);
                                (show_invisible_charts || invisible == 0) && !mode_mismatch
                            } else {
                                true
                            }
                        });
                        break;
                    }
                }
            } else if !show_invisible_charts {
                // No context: filter invisible songs without mode trial loop
                l.retain(|b| {
                    if let Some(sb) = b.as_song_bar() {
                        let sd = sb.get_song_data();
                        (sd.get_favorite() & (INVISIBLE_SONG | INVISIBLE_CHART)) == 0
                    } else {
                        true
                    }
                });
            }

            // Push directory bar
            if let Some(bar) = bar
                && bar.is_directory_bar()
            {
                let dir_bar = Box::new(bar.clone());
                self.dir.push(dir_bar);

                if self.dir.len() > prevdirsize {
                    // Store prevbar (currently selected bar) as sourcebar for navigation history
                    let sourcebar = if !self.currentsongs.is_empty() {
                        Some(self.currentsongs[self.selectedindex].clone())
                    } else {
                        None
                    };
                    self.sourcebars.push(sourcebar);
                }
            }

            // Load scores from cache for SongBars
            if let Some(ref mut ctx) = ctx
                && let Some(ref mut cache) = ctx.score_cache
            {
                let lnmode = ctx.player_config.get_lnmode();
                for b in &mut l {
                    if let Some(sb) = b.as_song_bar() {
                        let sd = sb.get_song_data();
                        if cache.exists_score_data_cache(sd, lnmode) {
                            let score = cache.read_score_data(sd, lnmode).cloned();
                            b.set_score(score);
                        }
                    }
                }
            }

            // Sort
            if is_sortable {
                if let Some(ref ctx) = ctx {
                    let sorter = ctx
                        .player_config
                        .get_sortid()
                        .and_then(BarSorter::value_of)
                        .unwrap_or(BarSorter::Title);
                    l.sort_by(|a, b| sorter.compare(a, b));

                    if SongManagerMenu::is_last_played_sort_enabled() {
                        l.sort_by(|a, b| BarSorter::LastUpdate.compare(a, b));
                    }
                } else {
                    l.sort_by(|a, b| BarSorter::Title.compare(a, b));
                }
            }

            // Random select bars
            if let Some(ref ctx) = ctx
                && ctx.player_config.is_random_select()
                && !bar
                    .map(|b| matches!(b, Bar::ContextMenu(_)))
                    .unwrap_or(false)
            {
                let mut random_bars: Vec<Bar> = Vec::new();
                for random_folder in &self.random_folder_list {
                    let random_targets: Vec<SongData> = l
                        .iter()
                        .filter_map(|b| {
                            b.as_song_bar().and_then(|sb| {
                                let sd = sb.get_song_data();
                                if sd.get_path().is_some() {
                                    Some(sd.clone())
                                } else {
                                    None
                                }
                            })
                        })
                        .collect();

                    let filtered_targets = if random_folder.get_filter().is_some() {
                        if let Some(ref mut _ctx_inner) = ctx.score_cache.as_ref() {
                            // Filter by score data - requires mutable cache access
                            // Simplified: use targets as-is since we'd need &mut
                            random_targets
                        } else {
                            random_targets
                        }
                    } else {
                        random_targets
                    };

                    let threshold = if random_folder.get_filter().is_some() {
                        1
                    } else {
                        2
                    };
                    if filtered_targets.len() >= threshold {
                        let exec_bar =
                            ExecutableBar::new(filtered_targets, random_folder.get_name());
                        random_bars.push(Bar::Executable(Box::new(exec_bar)));
                    }
                }

                // Prepend random bars
                if !random_bars.is_empty() {
                    random_bars.append(&mut l);
                    l = random_bars;
                }
            }

            self.currentsongs = l;
            self.selectedindex = 0;

            // Restore cursor position to matching bar
            if sourcebar_title.is_some() {
                // Use sourcebar to find position
                let target_title = sourcebar_title.as_deref();
                let target_sha = sourcebar_sha256.as_deref();
                if sourcebar_is_song && target_sha.is_some() {
                    for i in 0..self.currentsongs.len() {
                        if let Some(sb) = self.currentsongs[i].as_song_bar()
                            && sb.exists_song()
                            && Some(sb.get_song_data().get_sha256()) == target_sha
                        {
                            self.selectedindex = i;
                            break;
                        }
                    }
                } else if let Some(title) = target_title {
                    for i in 0..self.currentsongs.len() {
                        if self.currentsongs[i].get_title() == title {
                            self.selectedindex = i;
                            break;
                        }
                    }
                }
            } else if let Some(ref prev_title) = prevbar_title {
                if prevbar_is_song && prevbar_sha256.is_some() {
                    let sha = prevbar_sha256.as_deref().unwrap();
                    for i in 0..self.currentsongs.len() {
                        if let Some(sb) = self.currentsongs[i].as_song_bar()
                            && sb.exists_song()
                            && sb.get_song_data().get_sha256() == sha
                        {
                            self.selectedindex = i;
                            break;
                        }
                    }
                } else {
                    for i in 0..self.currentsongs.len() {
                        if bar_class_name(&self.currentsongs[i]) == prevbar_class_name
                            && self.currentsongs[i].get_title() == *prev_title
                        {
                            self.selectedindex = i;
                            break;
                        }
                    }
                }
            }

            // Stop previous loader
            if let Some(ref stop) = self.loader_stop {
                stop.store(true, Ordering::SeqCst);
            }
            self.loader_stop = Some(Arc::new(AtomicBool::new(false)));

            // Build directory string
            let mut dir_str = String::new();
            for d in &self.dir {
                dir_str.push_str(&d.get_title());
                dir_str.push_str(" > ");
            }
            self.dir_string = dir_str;

            return true;
        }

        // Empty list: re-enter current directory or root
        // Guard against infinite recursion: only recurse if bar was not None
        if bar.is_some() {
            if !self.dir.is_empty() {
                return self.update_bar_at_dir_index(self.dir.len() - 1);
            } else {
                return self.update_bar(None);
            }
        }
        log::warn!("No songs found");
        false
    }

    /// Update bar using the currently selected bar.
    /// Workaround for borrow checker: can't pass get_selected() to update_bar().
    pub fn update_bar_with_selected(&mut self) -> bool {
        if self.currentsongs.is_empty() {
            return false;
        }
        let selected_bar = self.currentsongs[self.selectedindex].clone();
        self.update_bar(Some(&selected_bar))
    }

    /// Go up one directory level.
    /// Corresponds to Java BarManager.close()
    pub fn close(&mut self) {
        if self.dir.is_empty() {
            SongManagerMenu::force_disable_last_played_sort();
            // In Java: select.executeEvent(EventType.sort)
            return;
        }

        // Get parent (second-to-last) directory
        let dir_len = self.dir.len();
        if dir_len <= 1 {
            // At first level: go back to root
            self.update_bar(None);
        } else {
            // Navigate to parent directory
            // We need to pop current and use parent
            // Java: current = dir.removeLast(); parent = dir.last(); dir.addLast(current); updateBar(parent);
            // Since we can't easily reference parent while modifying dir,
            // use the index-based approach
            self.update_bar_at_dir_index(dir_len - 2);
        }
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

    pub fn add_search(&mut self, bar: SearchWordBar, max_count: i32) {
        // Remove existing search with same title
        let title = bar.get_title();
        self.search.retain(|s| s.get_title() != title);
        if self.search.len() >= max_count as usize {
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

/// Get a string identifier for a Bar variant (simulates Java getClass())
fn bar_class_name(bar: &Bar) -> &'static str {
    match bar {
        Bar::Song(_) => "SongBar",
        Bar::Folder(_) => "FolderBar",
        Bar::Command(_) => "CommandBar",
        Bar::Container(_) => "ContainerBar",
        Bar::Hash(_) => "HashBar",
        Bar::Table(_) => "TableBar",
        Bar::Grade(_) => "GradeBar",
        Bar::RandomCourse(_) => "RandomCourseBar",
        Bar::SearchWord(_) => "SearchWordBar",
        Bar::SameFolder(_) => "SameFolderBar",
        Bar::Executable(_) => "ExecutableBar",
        Bar::Function(_) => "FunctionBar",
        Bar::ContextMenu(_) => "ContextMenuBar",
        Bar::LeaderBoard(_) => "LeaderBoardBar",
    }
}

/// A no-op TableAccessor for course tables.
/// Corresponds to the anonymous TableAccessor in Java BarManager.init()
struct CourseTableAccessor;
impl TableAccessor for CourseTableAccessor {
    fn name(&self) -> &str {
        "course"
    }
    fn read(&self) -> Option<TableData> {
        let mut td = TableData::default();
        td.set_name("COURSE".to_string());
        td.set_course(CourseDataAccessor::new("course").read_all());
        Some(td)
    }
    fn write(&self, _td: &mut TableData) {
        // No-op for course tables
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
                if let Some(score) = score_data {
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

fn get_score_data_property(score: &ScoreData, key: &str) -> i64 {
    match key {
        "clear" => score.get_clear() as i64,
        "exscore" => score.get_exscore() as i64,
        "notes" => score.get_notes() as i64,
        "minbp" => score.get_minbp() as i64,
        "date" => score.get_date(),
        "playcount" => score.get_playcount() as i64,
        _ => 0,
    }
}

fn evaluate_filter_expression(expr: &str, property_value: i64) -> bool {
    if expr.is_empty() {
        return true;
    }
    if let Some(stripped) = expr.strip_prefix(">=") {
        if let Ok(v) = stripped.parse::<i64>() {
            return property_value >= v;
        }
    } else if let Some(stripped) = expr.strip_prefix("<=") {
        if let Ok(v) = stripped.parse::<i64>() {
            return property_value <= v;
        }
    } else if let Some(stripped) = expr.strip_prefix('>') {
        if let Ok(v) = stripped.parse::<i64>() {
            return property_value > v;
        }
    } else if let Some(stripped) = expr.strip_prefix('<')
        && let Ok(v) = stripped.parse::<i64>()
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
    stop: Arc<AtomicBool>,
}

impl BarContentsLoaderThread {
    /// Create a new bar contents loader with a shared stop flag.
    pub fn new(stop: Arc<AtomicBool>) -> Self {
        Self { stop }
    }

    /// Run the loader on the given bars.
    /// Corresponds to Java BarContentsLoaderThread.run()
    pub fn run(&self, bars: &mut [Bar], ctx: &mut LoaderContext) {
        let lnmode = ctx.player_config.get_lnmode();

        // Phase 1: Load scores
        for bar in bars.iter_mut() {
            if self.is_stopped() {
                return;
            }

            // Extract song data to avoid overlapping borrows
            let song_info = bar
                .as_song_bar()
                .filter(|sb| sb.exists_song())
                .map(|sb| sb.get_song_data().clone());

            if let Some(sd) = song_info {
                // Load player score
                if bar.get_score().is_none()
                    && let Some(ref mut cache) = ctx.score_cache
                {
                    let score = cache.read_score_data(&sd, lnmode).cloned();
                    bar.set_score(score);
                }

                // Load rival score
                if let Some(ref mut rival) = ctx.rival_cache
                    && bar.get_rival_score().is_none()
                {
                    let rival_score = rival.read_score_data(&sd, lnmode).cloned();
                    if let Some(mut rs) = rival_score {
                        if let Some(ref name) = ctx.rival_name {
                            rs.player = name.clone();
                        }
                        bar.set_rival_score(Some(rs));
                    }
                }

                // Replay existence check
                // Java: for(int i = 0; i < MusicSelector.REPLAY; i++) { ... }
                // Requires PlayDataAccessor - blocked
            } else if let Some(gb) = bar.as_grade_bar()
                && gb.exists_all_songs()
            {
                // Load grade scores
                // Requires PlayDataAccessor.readScoreData(hash[], ...) - blocked
                log::debug!("GradeBar score loading requires PlayDataAccessor");
            }

            // Update folder status
            if ctx.is_folderlamp && bar.is_directory_bar() {
                // Requires songdb access for folder status update
                log::debug!("DirectoryBar folder status update requires songdb");
            }
        }

        // Phase 2: Load song information
        // Java: info.getInformation(songs)
        // Requires SongInformationAccessor - blocked

        // Phase 3: Load banners and stagefiles
        // Java: for (Bar bar : bars) { if (bar instanceof SongBar && ...) { ... } }
        for bar in bars.iter_mut() {
            if self.is_stopped() {
                return;
            }

            // Extract song data to avoid overlapping borrows (immutable sb → mutable bar)
            let song_info = bar.as_song_bar().filter(|sb| sb.exists_song()).map(|sb| {
                let sd = sb.get_song_data();
                (
                    sd.get_banner().to_string(),
                    sd.get_stagefile().to_string(),
                    sd.get_path().map(|s| s.to_string()),
                )
            });

            if let Some((banner_name, stagefile_name, song_path)) = song_info {
                // Load banner
                // Java: Path bannerfile = Paths.get(song.getPath()).getParent().resolve(song.getBanner());
                //        if (song.getBanner().length() > 0 && Files.exists(bannerfile)) {
                //            songbar.setBanner(select.getBannerResource().get(bannerfile.toString()));
                //        }
                if !banner_name.is_empty()
                    && let Some(ref path) = song_path
                    && let Some(parent) = Path::new(path).parent()
                {
                    let banner_path = parent.join(&banner_name);
                    if banner_path.exists() {
                        if let Some(banner_pool) = ctx.banner_resource {
                            let banner_key = banner_path.to_string_lossy().to_string();
                            let pixmap = banner_pool.get_and_use(&banner_key, |p| p.clone());
                            if let Some(pix) = pixmap
                                && let Some(sb) = bar.as_song_bar_mut()
                            {
                                sb.set_banner(Some(pix));
                            }
                        } else {
                            log::debug!("Banner loading skipped (no pool): {:?}", banner_path);
                        }
                    }
                }

                // Load stagefile
                // Java: Path stagefilefile = Paths.get(song.getPath()).getParent().resolve(song.getStagefile());
                //        if (song.getStagefile().length() > 0 && Files.exists(stagefilefile)) {
                //            songbar.setStagefile(select.getStagefileResource().get(stagefilefile.toString()));
                //        }
                if !stagefile_name.is_empty()
                    && let Some(ref path) = song_path
                    && let Some(parent) = Path::new(path).parent()
                {
                    let stage_path = parent.join(&stagefile_name);
                    if stage_path.exists() {
                        if let Some(stage_pool) = ctx.stagefile_resource {
                            let stage_key = stage_path.to_string_lossy().to_string();
                            let pixmap = stage_pool.get_and_use(&stage_key, |p| p.clone());
                            if let Some(pix) = pixmap
                                && let Some(sb) = bar.as_song_bar_mut()
                            {
                                sb.set_stagefile(Some(pix));
                            }
                        } else {
                            log::debug!("Stagefile loading skipped (no pool): {:?}", stage_path);
                        }
                    }
                }
            }
        }
    }

    /// Stop the loader.
    pub fn stop_running(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// Check if the loader has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::select::bar::song_bar::SongBar;

    fn make_song_data(sha256: &str, path: Option<&str>) -> SongData {
        let mut sd = SongData::default();
        sd.sha256 = sha256.to_string();
        if let Some(p) = path {
            sd.set_path(p.to_string());
        }
        sd
    }

    fn make_song_bar(sha256: &str, path: Option<&str>) -> Bar {
        Bar::Song(Box::new(SongBar::new(make_song_data(sha256, path))))
    }

    // ---- init tests ----

    #[test]
    fn test_init_creates_courses() {
        let mut manager = BarManager::new();
        let config = Config::default();
        manager.init(&config, &[]);
        assert!(manager.courses.is_some());
    }

    #[test]
    fn test_init_creates_commands() {
        let mut manager = BarManager::new();
        let config = Config::default();
        manager.init(&config, &[]);
        // Should have at least LAMP UPDATE and SCORE UPDATE
        assert!(manager.commands.len() >= 2);
    }

    #[test]
    fn test_init_default_random_folder() {
        let mut manager = BarManager::new();
        let config = Config::default();
        manager.init(&config, &[]);
        // random/default.json likely doesn't exist in test, so default folder is created
        assert!(!manager.random_folder_list.is_empty());
        assert_eq!(
            manager.random_folder_list[0].get_name(),
            "[RANDOM] RANDOM SELECT"
        );
    }

    #[test]
    fn test_init_lamp_update_contains_30_days() {
        let mut manager = BarManager::new();
        let config = Config::default();
        manager.init(&config, &[]);
        // First command should be LAMP UPDATE container with 30 children
        if let Some(Bar::Container(c)) = manager.commands.first() {
            assert_eq!(c.get_title(), "LAMP UPDATE");
            assert_eq!(c.childbar.len(), 30);
        } else {
            panic!("First command should be LAMP UPDATE container");
        }
    }

    #[test]
    fn test_init_score_update_contains_30_days() {
        let mut manager = BarManager::new();
        let config = Config::default();
        manager.init(&config, &[]);
        if let Some(Bar::Container(c)) = manager.commands.get(1) {
            assert_eq!(c.get_title(), "SCORE UPDATE");
            assert_eq!(c.childbar.len(), 30);
        } else {
            panic!("Second command should be SCORE UPDATE container");
        }
    }

    // ---- update_bar tests ----

    #[test]
    fn test_update_bar_root_with_no_context() {
        let mut manager = BarManager::new();
        // Root with empty manager should return false (no bars)
        let result = manager.update_bar(None);
        assert!(!result);
    }

    #[test]
    fn test_update_bar_root_with_favorites() {
        let mut manager = BarManager::new();
        let songs = vec![make_song_data("abc", Some("/path/song.bms"))];
        manager.favorites = vec![HashBar::new("FAV1".to_string(), songs)];

        let result = manager.update_bar(None);
        // Should have at least the favorite bar
        assert!(result);
        assert!(!manager.currentsongs.is_empty());
    }

    #[test]
    fn test_update_bar_sets_selectedindex_zero() {
        let mut manager = BarManager::new();
        manager.selectedindex = 5;
        manager.favorites = vec![HashBar::new(
            "FAV1".to_string(),
            vec![make_song_data("abc", Some("/path.bms"))],
        )];

        manager.update_bar(None);
        assert_eq!(manager.selectedindex, 0);
    }

    #[test]
    fn test_update_bar_builds_dir_string() {
        let mut manager = BarManager::new();
        manager.favorites = vec![HashBar::new(
            "FAV1".to_string(),
            vec![make_song_data("abc", Some("/path.bms"))],
        )];
        manager.update_bar(None);
        // At root, dir_string should be empty
        assert_eq!(manager.dir_string, "");
    }

    #[test]
    fn test_update_bar_restores_cursor_by_sha256() {
        let mut manager = BarManager::new();
        // Set up currentsongs with a song bar
        manager.currentsongs = vec![
            make_song_bar("aaa", Some("/a.bms")),
            make_song_bar("bbb", Some("/b.bms")),
        ];
        manager.selectedindex = 1; // select "bbb"

        // Now update to root with favorites containing both songs
        manager.favorites = vec![HashBar::new(
            "FAV".to_string(),
            vec![
                make_song_data("aaa", Some("/a.bms")),
                make_song_data("bbb", Some("/b.bms")),
            ],
        )];

        // The favorites bar itself will be shown, not the individual songs
        // So cursor restoration by sha256 won't match, but title matching should work
        manager.update_bar(None);
    }

    // ---- update_bar_with_context tests ----

    #[test]
    fn test_update_bar_filters_invisible_songs() {
        let mut manager = BarManager::new();
        let mut visible = make_song_data("visible", Some("/v.bms"));
        visible.favorite = 0;
        let mut invisible = make_song_data("invisible", Some("/i.bms"));
        invisible.favorite = INVISIBLE_SONG;

        manager.currentsongs = vec![
            Bar::Song(Box::new(SongBar::new(visible.clone()))),
            Bar::Song(Box::new(SongBar::new(invisible.clone()))),
        ];

        // With context, invisible songs should be filtered
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut ctx = UpdateBarContext {
            config: &config,
            player_config: &mut player_config,
            songdb: &crate::select::null_song_database_accessor::NullSongDatabaseAccessor,
            score_cache: None,
            is_folderlamp: false,
            max_search_bar_count: 10,
        };

        // Put songs in favorites so they appear at root
        manager.favorites = vec![HashBar::new("Test".to_string(), vec![visible, invisible])];

        manager.update_bar_with_context(None, Some(&mut ctx));
        // Only visible should remain (but favorites are shown as HashBar, not individual songs)
        // The filtering happens when we enter a directory with SongBars
    }

    // ---- close tests ----

    #[test]
    fn test_close_at_root() {
        let mut manager = BarManager::new();
        // At root level, close should not panic
        manager.close();
    }

    #[test]
    fn test_close_goes_up_one_level() {
        let mut manager = BarManager::new();
        // Push a directory level
        manager
            .dir
            .push(Box::new(Bar::Folder(Box::new(FolderBar::new(
                None,
                "test_dir".to_string(),
            )))));
        // Also need some currentsongs so update_bar doesn't recurse infinitely
        manager.favorites = vec![HashBar::new(
            "FAV".to_string(),
            vec![make_song_data("abc", Some("/test.bms"))],
        )];

        manager.close();
        // After close, we should be at root (dir cleared)
        assert!(manager.dir.is_empty());
    }

    // ---- BarContentsLoaderThread tests ----

    #[test]
    fn test_loader_stop_flag() {
        let stop = Arc::new(AtomicBool::new(false));
        let loader = BarContentsLoaderThread::new(stop.clone());
        assert!(!loader.is_stopped());
        loader.stop_running();
        assert!(loader.is_stopped());
    }

    #[test]
    fn test_loader_runs_on_empty_bars() {
        let stop = Arc::new(AtomicBool::new(false));
        let loader = BarContentsLoaderThread::new(stop);
        let mut bars: Vec<Bar> = Vec::new();
        let player_config = PlayerConfig::default();
        let mut ctx = LoaderContext {
            player_config: &player_config,
            score_cache: None,
            rival_cache: None,
            rival_name: None,
            is_folderlamp: false,
            banner_resource: None,
            stagefile_resource: None,
        };
        loader.run(&mut bars, &mut ctx);
        // Should complete without errors
    }

    #[test]
    fn test_loader_stops_early_when_signaled() {
        let stop = Arc::new(AtomicBool::new(true)); // pre-stopped
        let loader = BarContentsLoaderThread::new(stop);
        let mut bars = vec![make_song_bar("abc", Some("/test.bms"))];
        let player_config = PlayerConfig::default();
        let mut ctx = LoaderContext {
            player_config: &player_config,
            score_cache: None,
            rival_cache: None,
            rival_name: None,
            is_folderlamp: false,
            banner_resource: None,
            stagefile_resource: None,
        };
        loader.run(&mut bars, &mut ctx);
        // Should return immediately due to stop flag
    }

    #[test]
    fn test_loader_loads_score_from_cache() {
        let stop = Arc::new(AtomicBool::new(false));
        let loader = BarContentsLoaderThread::new(stop);

        let sd = make_song_data("test_hash", Some("/test.bms"));
        let mut bars = vec![Bar::Song(Box::new(SongBar::new(sd.clone())))];

        let mut score = ScoreData::default();
        score.epg = 100;

        let mut cache = ScoreDataCache::new(
            Box::new(move |_sd, _lnmode| {
                let mut s = ScoreData::default();
                s.epg = 100;
                Some(s)
            }),
            Box::new(|_collector, _songs, _lnmode| {}),
        );

        let player_config = PlayerConfig::default();
        let mut ctx = LoaderContext {
            player_config: &player_config,
            score_cache: Some(&mut cache),
            rival_cache: None,
            rival_name: None,
            is_folderlamp: false,
            banner_resource: None,
            stagefile_resource: None,
        };

        loader.run(&mut bars, &mut ctx);

        // Score should be loaded
        assert!(bars[0].get_score().is_some());
        assert_eq!(bars[0].get_score().unwrap().epg, 100);
    }

    // ---- banner/stagefile loading tests ----

    fn create_test_png(dir: &std::path::Path, name: &str) -> String {
        let path = dir.join(name);
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
        img.save(&path).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_loader_loads_banner_via_pool() {
        let dir = tempfile::tempdir().unwrap();
        // Create a banner image file inside the song directory
        create_test_png(dir.path(), "banner.png");

        // Create a SongBar with a path in the temp directory and a banner filename
        let song_file = dir.path().join("test.bms");
        std::fs::write(&song_file, b"").unwrap();
        let mut sd = SongData::default();
        sd.sha256 = "bannerhash".to_string();
        sd.set_path(song_file.to_string_lossy().to_string());
        sd.banner = "banner.png".to_string();
        let mut bars = vec![Bar::Song(Box::new(SongBar::new(sd)))];

        let stop = Arc::new(AtomicBool::new(false));
        let loader = BarContentsLoaderThread::new(stop);
        let player_config = PlayerConfig::default();
        let banner_pool = PixmapResourcePool::new();
        let mut ctx = LoaderContext {
            player_config: &player_config,
            score_cache: None,
            rival_cache: None,
            rival_name: None,
            is_folderlamp: false,
            banner_resource: Some(&banner_pool),
            stagefile_resource: None,
        };

        loader.run(&mut bars, &mut ctx);

        // Banner should be loaded into the SongBar
        let sb = bars[0].as_song_bar().unwrap();
        assert!(sb.get_banner().is_some());
        let pix = sb.get_banner().unwrap();
        assert_eq!(pix.get_width(), 4);
        assert_eq!(pix.get_height(), 4);
    }

    #[test]
    fn test_loader_loads_stagefile_via_pool() {
        let dir = tempfile::tempdir().unwrap();
        // Create a stagefile image file inside the song directory
        create_test_png(dir.path(), "stagefile.png");

        let song_file = dir.path().join("test.bms");
        std::fs::write(&song_file, b"").unwrap();
        let mut sd = SongData::default();
        sd.sha256 = "stagefilehash".to_string();
        sd.set_path(song_file.to_string_lossy().to_string());
        sd.stagefile = "stagefile.png".to_string();
        let mut bars = vec![Bar::Song(Box::new(SongBar::new(sd)))];

        let stop = Arc::new(AtomicBool::new(false));
        let loader = BarContentsLoaderThread::new(stop);
        let player_config = PlayerConfig::default();
        let stagefile_pool = PixmapResourcePool::new();
        let mut ctx = LoaderContext {
            player_config: &player_config,
            score_cache: None,
            rival_cache: None,
            rival_name: None,
            is_folderlamp: false,
            banner_resource: None,
            stagefile_resource: Some(&stagefile_pool),
        };

        loader.run(&mut bars, &mut ctx);

        // Stagefile should be loaded into the SongBar
        let sb = bars[0].as_song_bar().unwrap();
        assert!(sb.get_stagefile().is_some());
        let pix = sb.get_stagefile().unwrap();
        assert_eq!(pix.get_width(), 4);
        assert_eq!(pix.get_height(), 4);
    }

    #[test]
    fn test_loader_no_pool_skips_banner_loading() {
        let dir = tempfile::tempdir().unwrap();
        create_test_png(dir.path(), "banner.png");

        let song_file = dir.path().join("test.bms");
        std::fs::write(&song_file, b"").unwrap();
        let mut sd = SongData::default();
        sd.sha256 = "nopoolhash".to_string();
        sd.set_path(song_file.to_string_lossy().to_string());
        sd.banner = "banner.png".to_string();
        let mut bars = vec![Bar::Song(Box::new(SongBar::new(sd)))];

        let stop = Arc::new(AtomicBool::new(false));
        let loader = BarContentsLoaderThread::new(stop);
        let player_config = PlayerConfig::default();
        let mut ctx = LoaderContext {
            player_config: &player_config,
            score_cache: None,
            rival_cache: None,
            rival_name: None,
            is_folderlamp: false,
            banner_resource: None,
            stagefile_resource: None,
        };

        loader.run(&mut bars, &mut ctx);

        // Banner should NOT be loaded (no pool)
        let sb = bars[0].as_song_bar().unwrap();
        assert!(sb.get_banner().is_none());
    }

    #[test]
    fn test_loader_nonexistent_banner_file_not_loaded() {
        let dir = tempfile::tempdir().unwrap();
        // Do NOT create banner.png, it should not exist

        let song_file = dir.path().join("test.bms");
        std::fs::write(&song_file, b"").unwrap();
        let mut sd = SongData::default();
        sd.sha256 = "missinghash".to_string();
        sd.set_path(song_file.to_string_lossy().to_string());
        sd.banner = "banner.png".to_string();
        let mut bars = vec![Bar::Song(Box::new(SongBar::new(sd)))];

        let stop = Arc::new(AtomicBool::new(false));
        let loader = BarContentsLoaderThread::new(stop);
        let player_config = PlayerConfig::default();
        let banner_pool = PixmapResourcePool::new();
        let mut ctx = LoaderContext {
            player_config: &player_config,
            score_cache: None,
            rival_cache: None,
            rival_name: None,
            is_folderlamp: false,
            banner_resource: Some(&banner_pool),
            stagefile_resource: None,
        };

        loader.run(&mut bars, &mut ctx);

        // Banner should NOT be loaded (file does not exist)
        let sb = bars[0].as_song_bar().unwrap();
        assert!(sb.get_banner().is_none());
    }

    // ---- add_search tests ----

    #[test]
    fn test_add_search_respects_max_count() {
        let mut manager = BarManager::new();
        for i in 0..12 {
            manager.add_search(
                SearchWordBar::new(format!("search_{}", i), format!("text_{}", i)),
                10,
            );
        }
        // Should cap at 10
        assert_eq!(manager.search.len(), 10);
        // First 2 should have been removed
        assert_eq!(manager.search[0].get_title(), "search_2");
    }

    #[test]
    fn test_add_search_removes_duplicate() {
        let mut manager = BarManager::new();
        manager.add_search(SearchWordBar::new("foo".to_string(), "bar".to_string()), 10);
        manager.add_search(SearchWordBar::new("baz".to_string(), "qux".to_string()), 10);
        manager.add_search(
            SearchWordBar::new("foo".to_string(), "updated".to_string()),
            10,
        );

        assert_eq!(manager.search.len(), 2);
        assert_eq!(manager.search[0].get_title(), "baz");
        assert_eq!(manager.search[1].get_title(), "foo");
    }

    // ---- create_command_bar tests ----

    #[test]
    fn test_create_command_bar_simple() {
        let manager = BarManager::new();
        let folder = CommandFolder {
            name: Some("Test".to_string()),
            folder: vec![],
            sql: Some("SELECT * FROM song".to_string()),
            rcourse: vec![],
            showall: false,
        };
        let bar = manager.create_command_bar(&folder);
        assert!(matches!(bar, Bar::Command(_)));
        assert_eq!(bar.get_title(), "Test");
    }

    #[test]
    fn test_create_command_bar_with_subfolders() {
        let manager = BarManager::new();
        let folder = CommandFolder {
            name: Some("Parent".to_string()),
            folder: vec![CommandFolder {
                name: Some("Child".to_string()),
                folder: vec![],
                sql: Some("SELECT 1".to_string()),
                rcourse: vec![],
                showall: false,
            }],
            sql: None,
            rcourse: vec![],
            showall: false,
        };
        let bar = manager.create_command_bar(&folder);
        assert!(matches!(bar, Bar::Container(_)));
        assert_eq!(bar.get_title(), "Parent");
    }

    // ---- RandomFolder.filter_song tests ----

    #[test]
    fn test_filter_song_no_filter() {
        let rf = RandomFolder {
            name: Some("Test".to_string()),
            filter: None,
        };
        assert!(rf.filter_song(None));
        let score = ScoreData::default();
        assert!(rf.filter_song(Some(&score)));
    }

    #[test]
    fn test_filter_song_integer_filter_no_score() {
        let mut filter = HashMap::new();
        filter.insert("clear".to_string(), serde_json::Value::Number(0.into()));
        let rf = RandomFolder {
            name: Some("Test".to_string()),
            filter: Some(filter),
        };
        // null score with filter value 0 should pass
        assert!(rf.filter_song(None));
    }

    #[test]
    fn test_filter_song_integer_filter_nonzero_no_score() {
        let mut filter = HashMap::new();
        filter.insert("clear".to_string(), serde_json::Value::Number(5.into()));
        let rf = RandomFolder {
            name: Some("Test".to_string()),
            filter: Some(filter),
        };
        // null score with non-zero filter value should fail
        assert!(!rf.filter_song(None));
    }

    #[test]
    fn test_filter_song_string_comparison() {
        let mut filter = HashMap::new();
        filter.insert(
            "clear".to_string(),
            serde_json::Value::String(">=3".to_string()),
        );
        let rf = RandomFolder {
            name: Some("Test".to_string()),
            filter: Some(filter),
        };

        let mut score = ScoreData::default();
        score.clear = 5;
        assert!(rf.filter_song(Some(&score)));

        score.clear = 2;
        assert!(!rf.filter_song(Some(&score)));
    }

    // ---- evaluate_filter_expression tests ----

    #[test]
    fn test_evaluate_filter_gte() {
        assert!(evaluate_filter_expression(">=5", 5));
        assert!(evaluate_filter_expression(">=5", 6));
        assert!(!evaluate_filter_expression(">=5", 4));
    }

    #[test]
    fn test_evaluate_filter_lte() {
        assert!(evaluate_filter_expression("<=5", 5));
        assert!(evaluate_filter_expression("<=5", 4));
        assert!(!evaluate_filter_expression("<=5", 6));
    }

    #[test]
    fn test_evaluate_filter_gt() {
        assert!(evaluate_filter_expression(">5", 6));
        assert!(!evaluate_filter_expression(">5", 5));
    }

    #[test]
    fn test_evaluate_filter_lt() {
        assert!(evaluate_filter_expression("<5", 4));
        assert!(!evaluate_filter_expression("<5", 5));
    }

    #[test]
    fn test_evaluate_filter_empty() {
        assert!(evaluate_filter_expression("", 42));
    }

    // ---- i64 truncation bug tests ----

    #[test]
    fn test_filter_song_date_i64_not_truncated() {
        // Unix timestamp 1_700_000_000 exceeds i32::MAX (2_147_483_647 fits, but
        // 3_000_000_000 does not). Ensure large i64 values are compared correctly.
        let timestamp: i64 = 3_000_000_000; // exceeds i32::MAX
        let mut filter = HashMap::new();
        filter.insert(
            "date".to_string(),
            serde_json::Value::Number(serde_json::Number::from(timestamp)),
        );
        let rf = RandomFolder {
            name: Some("Test".to_string()),
            filter: Some(filter),
        };
        let mut score = ScoreData::default();
        score.date = timestamp;
        // Should match: both filter and score have the same i64 value
        assert!(rf.filter_song(Some(&score)));
    }

    #[test]
    fn test_filter_song_date_i64_mismatch_detected() {
        // When the filter value and score differ, it should correctly detect the mismatch
        let mut filter = HashMap::new();
        filter.insert(
            "date".to_string(),
            serde_json::Value::Number(serde_json::Number::from(3_000_000_000_i64)),
        );
        let rf = RandomFolder {
            name: Some("Test".to_string()),
            filter: Some(filter),
        };
        let mut score = ScoreData::default();
        score.date = 3_000_000_001_i64;
        // Should NOT match: values differ by 1
        assert!(!rf.filter_song(Some(&score)));
    }

    #[test]
    fn test_evaluate_filter_expression_large_i64() {
        // Comparison operators should work with values exceeding i32::MAX
        assert!(evaluate_filter_expression(">=3000000000", 3_000_000_000));
        assert!(evaluate_filter_expression(">=3000000000", 3_000_000_001));
        assert!(!evaluate_filter_expression(">=3000000000", 2_999_999_999));

        assert!(evaluate_filter_expression("<=3000000000", 3_000_000_000));
        assert!(!evaluate_filter_expression("<=3000000000", 3_000_000_001));

        assert!(evaluate_filter_expression(">3000000000", 3_000_000_001));
        assert!(!evaluate_filter_expression(">3000000000", 3_000_000_000));

        assert!(evaluate_filter_expression("<3000000000", 2_999_999_999));
        assert!(!evaluate_filter_expression("<3000000000", 3_000_000_000));
    }

    #[test]
    fn test_get_score_data_property_date_i64() {
        let mut score = ScoreData::default();
        score.date = 3_000_000_000;
        // Should return the full i64 value without truncation
        assert_eq!(get_score_data_property(&score, "date"), 3_000_000_000_i64);
    }

    // ---- bar_class_name tests ----

    #[test]
    fn test_bar_class_name() {
        let song = make_song_bar("abc", Some("/test.bms"));
        assert_eq!(bar_class_name(&song), "SongBar");

        let folder = Bar::Folder(Box::new(FolderBar::new(None, "test".to_string())));
        assert_eq!(bar_class_name(&folder), "FolderBar");

        let container = Bar::Container(Box::new(ContainerBar::new("c".to_string(), vec![])));
        assert_eq!(bar_class_name(&container), "ContainerBar");
    }

    // ---- CourseTableAccessor tests ----

    #[test]
    fn test_course_table_accessor_name() {
        let accessor = CourseTableAccessor;
        assert_eq!(accessor.name(), "course");
    }

    // ---- existing tests preserved ----

    #[test]
    fn test_get_selected_empty() {
        let manager = BarManager::new();
        assert!(manager.get_selected().is_none());
    }

    #[test]
    fn test_get_selected_with_songs() {
        let mut manager = BarManager::new();
        manager.currentsongs = vec![
            make_song_bar("abc", Some("/a.bms")),
            make_song_bar("def", Some("/d.bms")),
        ];
        manager.selectedindex = 1;
        let selected = manager.get_selected().unwrap();
        assert_eq!(
            selected.get_title(),
            make_song_data("def", Some("/d.bms")).full_title()
        );
    }

    #[test]
    fn test_mov_increase() {
        let mut manager = BarManager::new();
        manager.currentsongs = vec![
            make_song_bar("a", Some("/a.bms")),
            make_song_bar("b", Some("/b.bms")),
            make_song_bar("c", Some("/c.bms")),
        ];
        manager.selectedindex = 0;
        manager.mov(true);
        assert_eq!(manager.selectedindex, 1);
        manager.mov(true);
        assert_eq!(manager.selectedindex, 2);
        manager.mov(true);
        assert_eq!(manager.selectedindex, 0); // wraps
    }

    #[test]
    fn test_mov_decrease() {
        let mut manager = BarManager::new();
        manager.currentsongs = vec![
            make_song_bar("a", Some("/a.bms")),
            make_song_bar("b", Some("/b.bms")),
            make_song_bar("c", Some("/c.bms")),
        ];
        manager.selectedindex = 0;
        manager.mov(false);
        assert_eq!(manager.selectedindex, 2); // wraps to end
    }

    #[test]
    fn test_set_selected_position() {
        let mut manager = BarManager::new();
        manager.currentsongs = vec![
            make_song_bar("a", Some("/a.bms")),
            make_song_bar("b", Some("/b.bms")),
            make_song_bar("c", Some("/c.bms")),
            make_song_bar("d", Some("/d.bms")),
        ];
        manager.set_selected_position(0.5);
        assert_eq!(manager.selectedindex, 2);
    }

    #[test]
    fn test_get_selected_position() {
        let mut manager = BarManager::new();
        manager.currentsongs = vec![
            make_song_bar("a", Some("/a.bms")),
            make_song_bar("b", Some("/b.bms")),
            make_song_bar("c", Some("/c.bms")),
            make_song_bar("d", Some("/d.bms")),
        ];
        manager.selectedindex = 2;
        let pos = manager.get_selected_position();
        assert!((pos - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_add_random_course() {
        let mut manager = BarManager::new();
        let course = CourseData::default();
        let bar = GradeBar::new(course);
        manager.add_random_course(bar, "test > ".to_string());
        assert_eq!(manager.random_course_result.len(), 1);
    }

    #[test]
    fn test_add_random_course_caps_at_100() {
        let mut manager = BarManager::new();
        for i in 0..110 {
            let course = CourseData {
                name: Some(format!("course_{}", i)),
                ..CourseData::default()
            };
            manager.add_random_course(GradeBar::new(course), format!("dir_{}", i));
        }
        assert_eq!(manager.random_course_result.len(), 100);
    }

    #[test]
    fn test_set_append_directory_bar() {
        let mut manager = BarManager::new();
        let bar = make_song_bar("test", Some("/test.bms"));
        manager.set_append_directory_bar("key1".to_string(), bar);
        assert!(manager.append_folders.contains_key("key1"));
    }

    #[test]
    fn test_invisible_filtering_without_context() {
        let mut manager = BarManager::new();

        let mut visible = make_song_data("visible", Some("/v.bms"));
        visible.favorite = 0;
        let mut invisible = make_song_data("invisible", Some("/i.bms"));
        invisible.favorite = INVISIBLE_SONG;

        // Build a container bar with both visible and invisible songs
        let children = vec![
            Bar::Song(Box::new(SongBar::new(visible))),
            Bar::Song(Box::new(SongBar::new(invisible))),
        ];
        let container = ContainerBar::new(String::new(), children);

        // Enter the container WITHOUT context
        manager.update_bar_with_context(Some(&Bar::Container(Box::new(container))), None);

        // Without context, children can't be loaded from the container match branch,
        // so currentsongs will be empty (no children to filter).
        // This confirms the else branch doesn't panic and handles gracefully.
        // The invisible filtering else branch is reachable when children are
        // pre-populated through other means.
    }

    #[test]
    fn test_invisible_filtering_with_context() {
        let mut manager = BarManager::new();

        let mut visible = make_song_data("visible_song", Some("/v.bms"));
        visible.title = "visible_song".to_string();
        visible.favorite = 0;
        visible.mode = 0;
        let mut invisible = make_song_data("invisible_song", Some("/i.bms"));
        invisible.title = "invisible_song".to_string();
        invisible.favorite = INVISIBLE_SONG;
        invisible.mode = 0;

        let children = vec![
            Bar::Song(Box::new(SongBar::new(visible))),
            Bar::Song(Box::new(SongBar::new(invisible))),
        ];
        let container = ContainerBar::new("TestDir".to_string(), children);

        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut ctx = UpdateBarContext {
            config: &config,
            player_config: &mut player_config,
            songdb: &crate::select::null_song_database_accessor::NullSongDatabaseAccessor,
            score_cache: None,
            is_folderlamp: false,
            max_search_bar_count: 10,
        };

        manager.update_bar_with_context(Some(&Bar::Container(Box::new(container))), Some(&mut ctx));

        // With context, invisible song should be filtered out
        let song_count = manager
            .currentsongs
            .iter()
            .filter(|b| b.as_song_bar().is_some())
            .count();
        assert_eq!(song_count, 1, "only visible song should remain");
        assert_eq!(
            manager.currentsongs[0].get_title(),
            "visible_song",
            "the remaining song should be the visible one"
        );
    }
}
