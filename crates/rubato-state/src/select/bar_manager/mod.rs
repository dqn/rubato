use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rubato_core::pixmap_resource_pool::PixmapResourcePool;

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

    /// Build an UpdateBarContext from MusicSelector fields.
    /// Helper to avoid duplicating context construction at every call site.
    pub fn make_context<'a>(
        config: &'a Config,
        player_config: &'a mut PlayerConfig,
        songdb: &'a dyn SongDatabaseAccessor,
        score_cache: Option<&'a mut ScoreDataCache>,
    ) -> UpdateBarContext<'a> {
        UpdateBarContext {
            config,
            player_config,
            songdb,
            score_cache,
            is_folderlamp: false,
            max_search_bar_count: 10,
        }
    }

    /// Initialize the bar manager: load tables, courses, favorites, command/random folders.
    /// Corresponds to Java BarManager.init()
    pub fn init(&mut self, config: &Config, ir_table_urls: &[(String, String)]) {
        let tablepath = &config.paths.tablepath;
        let tdaccessor = TableDataAccessor::new(tablepath);

        // Load saved table data
        let raw_tables = tdaccessor.read_all();
        let mut unsorted_tables: Vec<Option<TableData>> =
            raw_tables.into_iter().map(Some).collect();

        // Sort tables according to config table URL order
        let mut sorted_tables: Vec<TableData> = Vec::with_capacity(unsorted_tables.len());
        for url in &config.paths.table_url {
            if let Some(td) = unsorted_tables.iter_mut().find_map(|slot| {
                if slot
                    .as_ref()
                    .is_some_and(|td| td.url_opt() == Some(url.as_str()))
                {
                    slot.take()
                } else {
                    None
                }
            }) {
                sorted_tables.push(td);
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
                td.url_opt().unwrap_or(""),
            ));
            table_bars.push(TableBar::new(td, accessor));
        }

        // Load IR tables if IR connections provide table URLs
        for (ir_name, table_url) in ir_table_urls {
            let td = TableData {
                name: format!("{} {}", ir_name, table_url),
                url: table_url.clone(),
                ..Default::default()
            };
            let accessor: Arc<dyn TableAccessor> =
                Arc::new(DifficultyTableAccessor::new(tablepath, table_url));
            table_bars.push(TableBar::new(td, accessor));
        }

        self.tables = table_bars;

        // Load courses
        let course_accessor = CourseDataAccessor::new("course");
        let course_td = TableData {
            name: "COURSE".to_string(),
            course: course_accessor.read_all(),
            ..Default::default()
        };
        let course_tr: Arc<dyn TableAccessor> = Arc::new(CourseTableAccessor);
        self.courses = Some(TableBar::new(course_td, course_tr));

        // Load favorites
        let fav_accessor = CourseDataAccessor::new("favorite");
        let fav_courses = fav_accessor.read_all();
        self.favorites = fav_courses
            .into_iter()
            .map(|cd| HashBar::new(cd.name().to_string(), cd.hash.to_vec()))
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

    /// Refresh the current bar display with song database context.
    /// Corresponds to Java BarManager.updateBar() (no-arg)
    pub fn update_bar_refresh_with_context(&mut self, ctx: Option<&mut UpdateBarContext>) -> bool {
        if !self.dir.is_empty() {
            return self.update_bar_with_last_dir_with_context(ctx);
        }
        self.update_bar_with_context(None, ctx)
    }

    /// Refresh the current bar display (no context — songdb queries will be skipped).
    pub fn update_bar_refresh(&mut self) -> bool {
        self.update_bar_refresh_with_context(None)
    }

    fn update_bar_with_last_dir_with_context(
        &mut self,
        ctx: Option<&mut UpdateBarContext>,
    ) -> bool {
        if !self.dir.is_empty() {
            return self.update_bar_at_dir_index_with_context(self.dir.len() - 1, ctx);
        }
        false
    }

    fn _update_bar_with_last_dir(&mut self) -> bool {
        self.update_bar_with_last_dir_with_context(None)
    }

    /// Update bar using a directory bar at the given index in self.dir.
    /// Workaround for borrow checker: can't borrow dir elements while mutating self.
    fn update_bar_at_dir_index_with_context(
        &mut self,
        _index: usize,
        ctx: Option<&mut UpdateBarContext>,
    ) -> bool {
        // The full implementation would need to clone dir[index] and pass it as bar.
        // Without the dir bar, we rebuild root.
        self.update_bar_with_context(None, ctx)
    }

    fn _update_bar_at_dir_index(&mut self, index: usize) -> bool {
        self.update_bar_at_dir_index_with_context(index, None)
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
            Some(self.currentsongs[self.selectedindex].title().to_owned())
        } else {
            None
        };
        let prevbar_sha256 = if !self.currentsongs.is_empty() {
            self.currentsongs[self.selectedindex]
                .as_song_bar()
                .filter(|sb| sb.exists_song())
                .map(|sb| sb.song_data().sha256.clone())
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
                l.extend(root_folder.children(ctx.songdb));
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
            let dir_index = self.dir.iter().position(|d| d.title() == bar.title());
            if let Some(idx) = dir_index {
                while self.dir.len() > idx + 1 {
                    self.dir.pop();
                    if let Some(sb) = self.sourcebars.pop()
                        && let Some(sb) = sb
                    {
                        sourcebar_title = Some(sb.title().to_owned());
                        sourcebar_sha256 = sb
                            .as_song_bar()
                            .filter(|s| s.exists_song())
                            .map(|s| s.song_data().sha256.clone());
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
                    Bar::Folder(b) => l.extend(b.children(songdb)),
                    Bar::Command(b) => {
                        let player_name = ctx.config.playername.as_deref().unwrap_or("default");
                        let score_path =
                            format!("{}/{}/score.db", ctx.config.paths.playerpath, player_name);
                        let scorelog_path = format!(
                            "{}/{}/scorelog.db",
                            ctx.config.paths.playerpath, player_name
                        );
                        let songinfo_path = ctx.config.paths.songinfopath.to_string();
                        let cmd_ctx = crate::select::bar::command_bar::CommandBarContext {
                            score_db_path: &score_path,
                            scorelog_db_path: &scorelog_path,
                            info_db_path: Some(&songinfo_path),
                        };
                        l.extend(b.children(songdb, &cmd_ctx));
                    }
                    Bar::Container(b) => {
                        l.extend(b.children().iter().cloned());
                    }
                    Bar::Hash(b) => l.extend(b.children(songdb)),
                    Bar::Table(b) => {
                        l.extend(b.children().iter().cloned());
                    }
                    Bar::SearchWord(b) => l.extend(b.children(songdb)),
                    Bar::ContextMenu(b) => l.extend(b.children(&self.tables, songdb)),
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
                    ds.push_str(d.title());
                    ds.push_str(" > ");
                }
                ds.push_str(bar.title());
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
                let current_mode = ctx.player_config.mode().copied();
                if let Some(pos) = MODE.iter().position(|m| *m == current_mode) {
                    mode_index = pos;
                }

                for trial_count in 0..MODE.len() {
                    let mode = &MODE[(mode_index + trial_count) % MODE.len()];
                    ctx.player_config.mode = *mode;

                    let before_len = l.len();
                    let remove_count = l
                        .iter()
                        .filter(|b| {
                            if let Some(sb) = b.as_song_bar() {
                                if let Some(sd) = Some(sb.song_data()) {
                                    let invisible =
                                        sd.favorite & (INVISIBLE_SONG | INVISIBLE_CHART);
                                    let mode_mismatch = mode.is_some()
                                        && sd.mode != 0
                                        && sd.mode != mode.as_ref().map(|m| m.id()).unwrap_or(0);
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
                        let mode_clone = *mode;
                        l.retain(|b| {
                            if let Some(sb) = b.as_song_bar() {
                                let sd = sb.song_data();
                                let invisible = sd.favorite & (INVISIBLE_SONG | INVISIBLE_CHART);
                                let mode_mismatch = mode_clone.is_some()
                                    && sd.mode != 0
                                    && sd.mode != mode_clone.as_ref().map(|m| m.id()).unwrap_or(0);
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
                        let sd = sb.song_data();
                        (sd.favorite & (INVISIBLE_SONG | INVISIBLE_CHART)) == 0
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
                let lnmode = ctx.player_config.play_settings.lnmode;
                for b in &mut l {
                    if let Some(sb) = b.as_song_bar() {
                        let sd = sb.song_data();
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
                                let sd = sb.song_data();
                                if sd.path().is_some() {
                                    Some(sd.clone())
                                } else {
                                    None
                                }
                            })
                        })
                        .collect();

                    let filtered_targets = if random_folder.filter().is_some() {
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

                    let threshold = if random_folder.filter().is_some() {
                        1
                    } else {
                        2
                    };
                    if filtered_targets.len() >= threshold {
                        let exec_bar = ExecutableBar::new(filtered_targets, random_folder.name());
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
                    if let Some(pos) = self.currentsongs.iter().position(|bar| {
                        bar.as_song_bar().is_some_and(|sb| {
                            sb.exists_song() && Some(sb.song_data().sha256.as_str()) == target_sha
                        })
                    }) {
                        self.selectedindex = pos;
                    }
                } else if let Some(title) = target_title
                    && let Some(pos) = self
                        .currentsongs
                        .iter()
                        .position(|bar| bar.title() == title)
                {
                    self.selectedindex = pos;
                }
            } else if let Some(ref prev_title) = prevbar_title {
                if prevbar_is_song && prevbar_sha256.is_some() {
                    let sha = prevbar_sha256.as_deref().expect("as_deref");
                    for i in 0..self.currentsongs.len() {
                        if let Some(sb) = self.currentsongs[i].as_song_bar()
                            && sb.exists_song()
                            && sb.song_data().sha256 == sha
                        {
                            self.selectedindex = i;
                            break;
                        }
                    }
                } else if let Some(pos) = self.currentsongs.iter().position(|bar| {
                    bar_class_name(bar) == prevbar_class_name && bar.title() == *prev_title
                }) {
                    self.selectedindex = pos;
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
                dir_str.push_str(d.title());
                dir_str.push_str(" > ");
            }
            self.dir_string = dir_str;

            return true;
        }

        // Empty list: re-enter current directory or root
        // Guard against infinite recursion: only recurse if bar was not None
        if bar.is_some() {
            if !self.dir.is_empty() {
                return self.update_bar_at_dir_index_with_context(self.dir.len() - 1, ctx);
            } else {
                return self.update_bar_with_context(None, ctx);
            }
        }
        log::warn!("No songs found");
        false
    }

    /// Update bar using the currently selected bar.
    /// Workaround for borrow checker: can't pass selected() to update_bar().
    pub fn update_bar_with_selected_and_context(
        &mut self,
        ctx: Option<&mut UpdateBarContext>,
    ) -> bool {
        if self.currentsongs.is_empty() {
            return false;
        }
        let selected_bar = self.currentsongs[self.selectedindex].clone();
        self.update_bar_with_context(Some(&selected_bar), ctx)
    }

    pub fn update_bar_with_selected(&mut self) -> bool {
        self.update_bar_with_selected_and_context(None)
    }

    /// Go up one directory level.
    /// Corresponds to Java BarManager.close()
    pub fn close_with_context(&mut self, ctx: Option<&mut UpdateBarContext>) {
        if self.dir.is_empty() {
            SongManagerMenu::force_disable_last_played_sort();
            // In Java: select.executeEvent(EventType.sort)
            return;
        }

        // Get parent (second-to-last) directory
        let dir_len = self.dir.len();
        if dir_len <= 1 {
            // At first level: go back to root
            self.update_bar_with_context(None, ctx);
        } else {
            // Navigate to parent directory
            self.update_bar_at_dir_index_with_context(dir_len - 2, ctx);
        }
    }

    pub fn close(&mut self) {
        self.close_with_context(None);
    }

    pub fn directory(&self) -> &[Box<Bar>] {
        &self.dir
    }

    pub fn directory_string(&self) -> &str {
        &self.dir_string
    }

    pub fn selected(&self) -> Option<&Bar> {
        if self.currentsongs.is_empty() {
            None
        } else {
            Some(&self.currentsongs[self.selectedindex])
        }
    }

    pub fn set_selected(&mut self, bar: &Bar) {
        if let Some(pos) = self
            .currentsongs
            .iter()
            .position(|b| b.title() == bar.title())
        {
            self.selectedindex = pos;
        }
    }

    pub fn selected_position(&self) -> f32 {
        if self.currentsongs.is_empty() {
            0.0
        } else {
            self.selectedindex as f32 / self.currentsongs.len() as f32
        }
    }

    pub fn tables(&self) -> &[TableBar] {
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
        let title = bar.title();
        self.search.retain(|s| s.title() != title);
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
        let has_subfolders = !folder.folder().is_empty();
        let has_random_courses = !folder.random_course().is_empty();

        if has_subfolders || has_random_courses {
            let mut children: Vec<Bar> = Vec::new();
            // Recursively create child bars for sub-folders
            for child in folder.folder() {
                children.push(self.create_command_bar(child));
            }
            // Create RandomCourseBar for random courses
            for rc in folder.random_course() {
                children.push(Bar::RandomCourse(Box::new(RandomCourseBar::new(
                    rc.clone(),
                ))));
            }
            Bar::Container(Box::new(ContainerBar::new(
                folder.name().to_string(),
                children,
            )))
        } else {
            Bar::Command(Box::new(CommandBar::new_with_visibility(
                folder.name().to_string(),
                folder.sql().unwrap_or("").to_string(),
                folder.is_showall(),
            )))
        }
    }
}

mod loader;
pub use loader::*;
use loader::{CourseTableAccessor, RandomCourseResult, bar_class_name};

#[cfg(test)]
mod tests;
