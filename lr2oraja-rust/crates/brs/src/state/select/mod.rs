// MusicSelect state — song selection browser.
//
// Loads the song list from the database, allows cursor navigation,
// and transitions to Decide when a song is selected.

pub mod bar_manager;
pub mod command;
pub mod leaderboard;
mod select_skin_state;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::{info, warn};

use bms_config::SongPreview;
use bms_database::SongInformation;
use bms_database::song_data::{FAVORITE_CHART, FAVORITE_SONG};
use bms_input::control_keys::ControlKeys;
use bms_input::key_command::KeyCommand;
use bms_rule::ScoreData;
use bms_skin::property_id::{TIMER_FADEOUT, TIMER_STARTINPUT};

use crate::app_state::AppStateType;
use crate::preview_music::PREVIEW_DELAY_MS;
use crate::skin_manager::SkinType;
use crate::state::{GameStateHandler, StateContext};
use crate::system_sound::SystemSound;

use bar_manager::{Bar, BarManager, SortMode};
use command::{
    CommandResult, MusicSelectCommand, build_song_context_menu, build_table_context_menu,
    build_table_folder_context_menu,
};

/// Default input delay in milliseconds.
const DEFAULT_INPUT_DELAY_MS: i64 = 500;
/// Default fadeout duration in milliseconds.
const DEFAULT_FADEOUT_DURATION_MS: i64 = 500;

/// Fallback scroll animation duration in milliseconds (used when config is 0).
const FALLBACK_SCROLL_DURATION_MS: i64 = 150;

static TABLE_UPDATE_JOB_RUNNING: AtomicBool = AtomicBool::new(false);

fn try_start_table_update_job() -> bool {
    TABLE_UPDATE_JOB_RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

fn finish_table_update_job() {
    TABLE_UPDATE_JOB_RUNNING.store(false, Ordering::Release);
}

#[cfg(test)]
fn finish_table_update_job_for_test() {
    finish_table_update_job();
}

struct TableUpdateJobGuard;

impl Drop for TableUpdateJobGuard {
    fn drop(&mut self) {
        finish_table_update_job();
    }
}

/// Music select state — song browser and selection.
pub struct MusicSelectState {
    bar_manager: BarManager,
    fadeout_started: bool,
    sort_mode: SortMode,
    mode_filter: Option<i32>,
    search_mode: bool,
    search_text: String,
    /// Center bar index from skin config (BAR_CENTER).
    center_bar: usize,
    /// Scroll animation start time (microseconds, from timer).
    scroll_start_us: Option<i64>,
    /// Scroll direction (-1 = up, 0 = idle, 1 = down).
    scroll_angle: i32,
    /// Cached song information keyed by sha256 to avoid repeated DB lookups.
    cached_song_info: Option<(String, SongInformation)>,
    /// Score lamp cache: sha256 → ClearType ID (0-10).
    score_lamp_cache: HashMap<String, i32>,
    /// Score data cache: sha256 → ScoreData (for sorting by score-related fields).
    score_data_cache: HashMap<String, ScoreData>,
    /// Whether the score cache needs refresh.
    score_cache_dirty: bool,
    /// Microsecond timestamp of the last cursor change (for preview delay).
    songbar_change_time: Option<i64>,
    /// Whether the preview for the current selection has already been triggered.
    preview_triggered: bool,
    /// Receiver for asynchronous IR leaderboard fetch results.
    /// Wrapped in `Mutex` to satisfy the `Sync` bound on `GameStateHandler`.
    ir_fetch_receiver: Option<parking_lot::Mutex<std::sync::mpsc::Receiver<Vec<Bar>>>>,
    /// Command executor for music select commands (clipboard, replay, etc.).
    command_executor: command::CommandExecutor,
    /// Currently selected rival index (cycles through available rivals).
    selected_rival: usize,
}

impl MusicSelectState {
    pub fn new() -> Self {
        Self {
            bar_manager: BarManager::new(),
            fadeout_started: false,
            sort_mode: SortMode::Default,
            mode_filter: None,
            search_mode: false,
            search_text: String::new(),
            center_bar: 0,
            scroll_start_us: None,
            scroll_angle: 0,
            cached_song_info: None,
            score_lamp_cache: HashMap::new(),
            score_data_cache: HashMap::new(),
            score_cache_dirty: true,
            songbar_change_time: None,
            preview_triggered: false,
            ir_fetch_receiver: None,
            command_executor: command::CommandExecutor::new(),
            selected_rival: 0,
        }
    }
}

impl Default for MusicSelectState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateHandler for MusicSelectState {
    fn create(&mut self, ctx: &mut StateContext) {
        self.fadeout_started = false;
        self.songbar_change_time = None;
        self.preview_triggered = false;
        info!("MusicSelect: create");

        if let Some(skin_mgr) = ctx.skin_manager.as_deref_mut() {
            skin_mgr.request_load(SkinType::MusicSelect);
        }

        // If a BMS model is already loaded (via CLI --bms), skip to Decide immediately
        if ctx.resource.bms_model.is_some() {
            info!("MusicSelect: BMS already loaded, transitioning to Decide");
            *ctx.transition = Some(AppStateType::Decide);
            return;
        }

        // Initialize preview music volume and default BGM
        if let Some(pm) = &mut ctx.preview_music {
            pm.set_volume(ctx.config.audio.systemvolume as f64);
            // Try to load select screen BGM from the bgmpath config
            let bgm_dir = PathBuf::from(&ctx.config.bgmpath);
            if let Some(bgm_path) = bms_audio::decode::resolve_audio_path(&bgm_dir, "select") {
                pm.set_default(&bgm_path);
                info!(path = %bgm_path.display(), "MusicSelect: loaded select BGM");
            }
        }

        // Configure bar manager from config
        self.bar_manager
            .set_max_search_bar_count(ctx.config.max_search_bar_count as usize);
        self.bar_manager
            .set_show_no_song_existing_bar(ctx.config.show_no_song_existing_bar);

        // Load song list from database
        if let Some(db) = ctx.database {
            self.bar_manager.load_root(&db.song_db);

            // Load table data from cache
            let table_accessor = bms_database::TableDataAccessor::new(&ctx.config.tablepath);
            if let Ok(accessor) = table_accessor
                && let Ok(tables) = accessor.read_all()
            {
                self.bar_manager.load_tables(&tables);
                info!(tables = tables.len(), "MusicSelect: loaded table data");
            }

            // Spawn background HTTP table update (results cached for next startup)
            if !ctx.config.table_url.is_empty() {
                let urls = ctx.config.table_url.clone();
                let table_dir = ctx.config.tablepath.clone();
                if try_start_table_update_job() {
                    std::thread::spawn(move || {
                        let _job_guard = TableUpdateJobGuard;
                        let rt = match tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                        {
                            Ok(rt) => rt,
                            Err(e) => {
                                warn!("MusicSelect: failed to build runtime for table update: {e}");
                                return;
                            }
                        };
                        rt.block_on(crate::table_updater::update_all(&urls, &table_dir));
                    });
                    info!("MusicSelect: background table update started");
                } else {
                    info!("MusicSelect: background table update already running");
                }
            }

            // Load built-in containers (LAMP UPDATE, SCORE UPDATE)
            self.bar_manager.load_builtin_containers();

            // Load course data from "course" directory
            self.bar_manager.load_courses("course");

            // Load favorite playlists from "favorite" directory
            self.bar_manager.load_favorites("favorite");

            // Load custom command folders from "folder/default.json"
            self.bar_manager.load_command_folders("folder/default.json");

            // Load sort mode from player config (Java parity: BarManager L384)
            self.sort_mode = ctx
                .player_config
                .sortid
                .as_deref()
                .map(SortMode::from_id)
                .unwrap_or(SortMode::Default);
            if self.sort_mode != SortMode::Default {
                self.bar_manager
                    .sort(self.sort_mode, &self.score_data_cache);
            }

            self.score_cache_dirty = true;
            info!(
                songs = self.bar_manager.bar_count(),
                sort = ?self.sort_mode,
                "MusicSelect: loaded song list"
            );
        }
    }

    fn render(&mut self, ctx: &mut StateContext) {
        let now = ctx.timer.now_time();

        // Check for completed IR leaderboard fetch
        let received_bars = self
            .ir_fetch_receiver
            .as_ref()
            .and_then(|mutex| mutex.lock().try_recv().ok());
        if let Some(bars) = received_bars {
            self.bar_manager.replace_current_bars(bars);
            self.score_cache_dirty = true;
            self.ir_fetch_receiver = None;
        }

        // Enable input after initial delay
        if now > DEFAULT_INPUT_DELAY_MS {
            ctx.timer.switch_timer(TIMER_STARTINPUT, true);
        }

        // Check fadeout -> transition
        if ctx.timer.is_timer_on(TIMER_FADEOUT)
            && ctx.timer.now_time_of(TIMER_FADEOUT) > DEFAULT_FADEOUT_DURATION_MS
        {
            if ctx.config.skip_decide_screen {
                info!("MusicSelect: transition to Play (skipDecideScreen)");
                *ctx.transition = Some(AppStateType::Play);
            } else {
                info!("MusicSelect: transition to Decide");
                *ctx.transition = Some(AppStateType::Decide);
            }
        }

        // Preview music: trigger after PREVIEW_DELAY_MS since last cursor change
        if ctx.config.song_preview != SongPreview::None {
            if let Some(change_time) = self.songbar_change_time {
                let elapsed_ms = (ctx.timer.now_micro_time() - change_time) / 1000;
                if elapsed_ms >= PREVIEW_DELAY_MS && !self.preview_triggered {
                    self.preview_triggered = true;
                    self.trigger_preview(ctx);
                }
            }
            if let Some(pm) = &mut ctx.preview_music {
                pm.update();
            }
        }

        // Compute scroll interpolation (use config scrolldurationlow; Java parity)
        let scroll_duration_ms = if ctx.config.scrolldurationlow > 0 {
            ctx.config.scrolldurationlow as i64
        } else {
            FALLBACK_SCROLL_DURATION_MS
        };
        let scroll_duration_us = scroll_duration_ms * 1000;
        let (angle_lerp, angle) = if let Some(start_us) = self.scroll_start_us {
            let elapsed_us = ctx.timer.now_micro_time() - start_us;
            if elapsed_us >= scroll_duration_us {
                // Scroll animation complete
                self.scroll_start_us = None;
                self.scroll_angle = 0;
                (0.0, 0)
            } else {
                let t = elapsed_us as f32 / scroll_duration_us as f32;
                (t * self.scroll_angle as f32, self.scroll_angle)
            }
        } else {
            (0.0, 0)
        };

        // Refresh score lamp cache when bar list changed
        if self.score_cache_dirty {
            if let Some(db) = ctx.database {
                let sha256_list: Vec<String> = self
                    .bar_manager
                    .bars()
                    .iter()
                    .filter_map(|bar| match bar {
                        Bar::Song(song_data) => Some(song_data.sha256.clone()),
                        _ => None,
                    })
                    .collect();
                let sha256_refs: Vec<&str> = sha256_list.iter().map(String::as_str).collect();
                let mode = ctx.resource.play_mode.mode_id();
                self.score_lamp_cache.clear();
                self.score_data_cache.clear();
                if let Ok(scores) = db.score_db.get_score_datas(&sha256_refs, mode) {
                    for sd in scores {
                        let lamp = sd.clear.id() as i32;
                        // Keep best (highest) clear per sha256
                        let lamp_entry =
                            self.score_lamp_cache.entry(sd.sha256.clone()).or_insert(0);
                        if lamp > *lamp_entry {
                            *lamp_entry = lamp;
                        }
                        // Keep best score data per sha256 (by clear type)
                        let score_entry =
                            self.score_data_cache.entry(sd.sha256.clone()).or_default();
                        if sd.clear.id() > score_entry.clear.id() {
                            *score_entry = sd;
                        }
                    }
                }
            }
            self.score_cache_dirty = false;
        }

        // Sync select state to shared game state for skin rendering
        if let Some(shared) = &mut ctx.shared_state {
            // Determine if current song has LN (from song data metadata)
            let has_ln = matches!(
                self.bar_manager.current(),
                Some(Bar::Song(s)) if s.has_any_long_note()
            );
            let is_preview_playing = ctx
                .preview_music
                .as_ref()
                .is_some_and(|p| p.is_playing_preview());
            select_skin_state::sync_select_state(
                shared,
                &self.bar_manager,
                has_ln,
                true,
                is_preview_playing,
                self.command_executor.selected_replay(),
            );
            select_skin_state::sync_bar_scroll_state(
                shared,
                &self.bar_manager,
                self.center_bar,
                angle_lerp,
                angle,
                &self.score_lamp_cache,
            );

            // Sync song information for the currently selected song
            match self.bar_manager.current() {
                Some(Bar::Song(song_data)) => {
                    let sha = &song_data.sha256;
                    let cached_matches = self
                        .cached_song_info
                        .as_ref()
                        .is_some_and(|(cached_sha, _)| cached_sha == sha);
                    if !cached_matches {
                        let info = ctx
                            .database
                            .and_then(|db| db.info_db.get_information(sha).ok().flatten());
                        self.cached_song_info = info.map(|i| (sha.clone(), i));
                    }
                    let info_ref = self.cached_song_info.as_ref().map(|(_, i)| i);
                    select_skin_state::sync_song_information(shared, info_ref);
                }
                _ => {
                    self.cached_song_info = None;
                    select_skin_state::sync_song_information(shared, None);
                }
            }
        }
    }

    fn input(&mut self, ctx: &mut StateContext) {
        if ctx.timer.is_timer_on(TIMER_FADEOUT) || !ctx.timer.is_timer_on(TIMER_STARTINPUT) {
            return;
        }

        // Search mode: handle text input separately
        if self.search_mode {
            self.input_search_mode(ctx);
            return;
        }

        if let Some(input_state) = ctx.input_state {
            // Process key commands
            for cmd in &input_state.commands {
                match cmd {
                    KeyCommand::AddFavoriteSong => {
                        self.toggle_favorite(ctx, FAVORITE_SONG);
                    }
                    KeyCommand::AddFavoriteChart => {
                        self.toggle_favorite(ctx, FAVORITE_CHART);
                    }
                    KeyCommand::CopySongMd5Hash => {
                        let result = self
                            .command_executor
                            .execute(MusicSelectCommand::CopyMd5Hash, &self.bar_manager);
                        self.handle_command_result(result, ctx);
                    }
                    KeyCommand::CopySongSha256Hash => {
                        let result = self
                            .command_executor
                            .execute(MusicSelectCommand::CopySha256Hash, &self.bar_manager);
                        self.handle_command_result(result, ctx);
                    }
                    KeyCommand::CopyHighlightedMenuText => {
                        let result = self.command_executor.execute(
                            MusicSelectCommand::CopyHighlightedMenuText,
                            &self.bar_manager,
                        );
                        self.handle_command_result(result, ctx);
                    }
                    _ => {}
                }
            }

            for key in &input_state.pressed_keys {
                match key {
                    ControlKeys::Up => {
                        self.bar_manager.move_cursor(-1);
                        self.scroll_angle = -1;
                        self.scroll_start_us = Some(ctx.timer.now_micro_time());
                        self.on_cursor_change(ctx.timer.now_micro_time());
                        if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                            sm.play(SystemSound::Select);
                        }
                        return;
                    }
                    ControlKeys::Down => {
                        self.bar_manager.move_cursor(1);
                        self.scroll_angle = 1;
                        self.scroll_start_us = Some(ctx.timer.now_micro_time());
                        self.on_cursor_change(ctx.timer.now_micro_time());
                        if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                            sm.play(SystemSound::Select);
                        }
                        return;
                    }
                    ControlKeys::Enter => {
                        self.select_current(ctx);
                        return;
                    }
                    ControlKeys::Escape => {
                        if self.bar_manager.is_in_folder() {
                            self.bar_manager.leave_folder();
                            self.score_cache_dirty = true;
                            self.on_cursor_change(ctx.timer.now_micro_time());
                            if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                                sm.play(SystemSound::Folder);
                            }
                        }
                        return;
                    }
                    ControlKeys::Insert => {
                        *ctx.transition = Some(AppStateType::SkinConfig);
                        return;
                    }
                    ControlKeys::Num0 => {
                        // Enter search mode
                        self.search_mode = true;
                        self.search_text.clear();
                        info!("MusicSelect: search mode ON");
                        return;
                    }
                    ControlKeys::Num2 => {
                        // Cycle sort mode
                        self.sort_mode = self.sort_mode.next(self.bar_manager.has_rival());
                        self.bar_manager
                            .sort(self.sort_mode, &self.score_data_cache);
                        self.score_cache_dirty = true;
                        // Persist sort mode to player config
                        ctx.player_config.sortid = Some(self.sort_mode.to_id().to_string());
                        if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                            sm.play(SystemSound::OptionChange);
                        }
                        info!(sort = ?self.sort_mode, "MusicSelect: sort changed");
                        return;
                    }
                    ControlKeys::Num1 => {
                        // Cycle mode filter
                        self.mode_filter = match self.mode_filter {
                            None => Some(7),      // Beat7K
                            Some(7) => Some(14),  // Beat14K
                            Some(14) => Some(9),  // PopN9K
                            Some(9) => Some(5),   // Beat5K
                            Some(5) => Some(10),  // Beat10K
                            Some(10) => Some(25), // 24K
                            _ => None,            // All
                        };
                        if let Some(db) = ctx.database {
                            self.bar_manager.load_root(&db.song_db);
                            if let Some(mode_id) = self.mode_filter {
                                self.bar_manager.filter_by_mode(Some(mode_id));
                            }
                            self.bar_manager
                                .sort(self.sort_mode, &self.score_data_cache);
                            self.score_cache_dirty = true;
                        }
                        if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                            sm.play(SystemSound::OptionChange);
                        }
                        info!(filter = ?self.mode_filter, "MusicSelect: mode filter changed");
                        return;
                    }
                    ControlKeys::Num3 => {
                        // Cycle gauge type
                        ctx.player_config.gauge = (ctx.player_config.gauge + 1) % 6;
                        if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                            sm.play(SystemSound::OptionChange);
                        }
                        info!(
                            gauge = ctx.player_config.gauge,
                            "MusicSelect: gauge changed"
                        );
                        return;
                    }
                    ControlKeys::Num4 => {
                        // Cycle random type
                        ctx.player_config.random = (ctx.player_config.random + 1) % 10;
                        if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                            sm.play(SystemSound::OptionChange);
                        }
                        info!(
                            random = ctx.player_config.random,
                            "MusicSelect: random changed"
                        );
                        return;
                    }
                    ControlKeys::Num5 => {
                        // Cycle DP option
                        ctx.player_config.doubleoption = (ctx.player_config.doubleoption + 1) % 4;
                        if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                            sm.play(SystemSound::OptionChange);
                        }
                        info!(
                            dp = ctx.player_config.doubleoption,
                            "MusicSelect: DP option changed"
                        );
                        return;
                    }
                    ControlKeys::F2 => {
                        // Practice mode: load song and start play with practice flag
                        if let Some(Bar::Song(song_data)) = self.bar_manager.current() {
                            let path = std::path::PathBuf::from(&song_data.path);
                            match bms_model::BmsDecoder::decode(&path) {
                                Ok(model) => {
                                    ctx.resource.play_mode = model.mode;
                                    ctx.resource.bms_dir = path.parent().map(|p| p.to_path_buf());
                                    ctx.resource.bms_path = Some(path);
                                    ctx.resource.bms_model = Some(model);
                                    ctx.resource.is_practice = true;
                                    self.fadeout_started = true;
                                    ctx.timer.set_timer_on(TIMER_FADEOUT);
                                    info!("MusicSelect: practice mode start");
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "MusicSelect: failed to load BMS for practice: {e}"
                                    );
                                }
                            }
                        }
                        return;
                    }
                    ControlKeys::Num6 => {
                        // Cycle rival selection
                        let result = self
                            .command_executor
                            .execute(MusicSelectCommand::NextRival, &self.bar_manager);
                        self.handle_command_result(result, ctx);
                        return;
                    }
                    ControlKeys::Num7 => {
                        let result = self
                            .command_executor
                            .execute(MusicSelectCommand::ShowContextMenu, &self.bar_manager);
                        self.handle_command_result(result, ctx);
                        return;
                    }
                    ControlKeys::Num8 => {
                        let result = self
                            .command_executor
                            .execute(MusicSelectCommand::ShowSongsOnSameFolder, &self.bar_manager);
                        self.handle_command_result(result, ctx);
                        return;
                    }
                    ControlKeys::Num9 => {
                        // Cycle replay slot forward
                        self.command_executor
                            .execute(MusicSelectCommand::NextReplay, &self.bar_manager);
                        if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                            sm.play(SystemSound::OptionChange);
                        }
                        info!(
                            replay = self.command_executor.selected_replay(),
                            "MusicSelect: replay slot changed (next)"
                        );
                        return;
                    }
                    ControlKeys::F3 => {
                        // Cycle replay slot backward
                        self.command_executor
                            .execute(MusicSelectCommand::PrevReplay, &self.bar_manager);
                        if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                            sm.play(SystemSound::OptionChange);
                        }
                        info!(
                            replay = self.command_executor.selected_replay(),
                            "MusicSelect: replay slot changed (prev)"
                        );
                        return;
                    }
                    ControlKeys::Del => {
                        // Transition to KeyConfig
                        *ctx.transition = Some(AppStateType::KeyConfig);
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    fn shutdown(&mut self, ctx: &mut StateContext) {
        if let Some(pm) = &mut ctx.preview_music {
            pm.stop();
        }
        info!("MusicSelect: shutdown");
    }
}

impl MusicSelectState {
    /// Handle a command result from the executor, performing broader state changes.
    fn handle_command_result(&mut self, result: CommandResult, ctx: &mut StateContext) {
        match result {
            CommandResult::None => {}
            CommandResult::ShowSameFolder { title, folder_crc } => {
                if let Some(db) = ctx.database {
                    self.bar_manager.push_and_set_bars(vec![Bar::SameFolder {
                        name: title,
                        folder_crc,
                    }]);
                    self.bar_manager.enter_folder(&db.song_db);
                    self.score_cache_dirty = true;
                }
            }
            CommandResult::ShowContextMenu => {
                let (source_bar, items) = match self.bar_manager.current() {
                    Some(bar @ Bar::Song(song_data)) => {
                        (Box::new(bar.clone()), build_song_context_menu(song_data))
                    }
                    Some(bar @ Bar::TableRoot { name, url, .. }) => (
                        Box::new(bar.clone()),
                        build_table_context_menu(name, url.as_deref()),
                    ),
                    Some(bar @ Bar::HashFolder { name, .. }) => {
                        (Box::new(bar.clone()), build_table_folder_context_menu(name))
                    }
                    _ => return,
                };
                if !items.is_empty() {
                    let cm = Bar::ContextMenu(Box::new(bar_manager::ContextMenuData {
                        source_bar,
                        items,
                    }));
                    self.bar_manager.push_and_set_bars(vec![cm]);
                    // ContextMenu enter_folder doesn't need song_db but method requires it
                    if let Some(db) = ctx.database {
                        self.bar_manager.enter_folder(&db.song_db);
                    }
                    self.score_cache_dirty = true;
                }
            }
            CommandResult::NextRival => {
                let rival_count = ctx.database.map(|db| db.rival.rival_count()).unwrap_or(0);
                if rival_count == 0 {
                    info!("MusicSelect: no rivals available");
                    return;
                }
                self.selected_rival = (self.selected_rival + 1) % rival_count;
                self.load_rival_scores(ctx);
                if self.bar_manager.has_rival() {
                    self.bar_manager
                        .sort(self.sort_mode, &self.score_data_cache);
                }
                if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                    sm.play(SystemSound::OptionChange);
                }
                let rival_name = ctx
                    .database
                    .and_then(|db| db.rival.get_rival(self.selected_rival))
                    .map(|r| r.info.name.as_str())
                    .unwrap_or("unknown");
                info!(
                    rival = %rival_name,
                    index = self.selected_rival,
                    "MusicSelect: rival changed"
                );
            }
        }
    }

    /// Record that the cursor changed, resetting the preview delay timer.
    ///
    /// Also resets the replay selection (Java parity: cursor movement resets
    /// the replay slot to the first available).
    fn on_cursor_change(&mut self, now_us: i64) {
        self.songbar_change_time = Some(now_us);
        self.preview_triggered = false;
        self.command_executor
            .execute(MusicSelectCommand::ResetReplay, &self.bar_manager);
    }

    /// Trigger preview playback for the currently selected bar.
    fn trigger_preview(&mut self, ctx: &mut StateContext) {
        let pm = match &mut ctx.preview_music {
            Some(pm) => pm,
            None => return,
        };

        let loop_play = ctx.config.song_preview == SongPreview::Loop;

        match self.bar_manager.current() {
            Some(Bar::Song(song_data)) if !song_data.preview.is_empty() => {
                // Resolve preview path relative to the BMS file's directory
                let song_path = std::path::Path::new(&song_data.path);
                if let Some(parent) = song_path.parent() {
                    let resolved =
                        bms_audio::decode::resolve_audio_path(parent, &song_data.preview);
                    pm.start_preview(resolved.as_deref(), loop_play);
                } else {
                    pm.start_preview(None, loop_play);
                }
            }
            _ => {
                // Not a song bar or no preview — fall back to default BGM
                pm.start_preview(None, loop_play);
            }
        }
    }

    fn toggle_favorite(&self, ctx: &mut StateContext, flag: i32) {
        if let Some(Bar::Song(song_data)) = self.bar_manager.current()
            && let Some(db) = ctx.database
        {
            if let Err(e) = db.song_db.update_favorite(&song_data.sha256, flag) {
                tracing::warn!("Failed to toggle favorite: {e}");
            } else {
                info!(sha256 = %song_data.sha256, flag, "MusicSelect: favorite toggled");
            }
        }
    }

    fn input_search_mode(&mut self, ctx: &mut StateContext) {
        // Accept character input
        for &ch in ctx.received_chars {
            if ch.is_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' {
                self.search_text.push(ch);
            }
        }

        if let Some(input_state) = ctx.input_state {
            for key in &input_state.pressed_keys {
                match key {
                    ControlKeys::Enter => {
                        // Execute search
                        if !self.search_text.is_empty()
                            && let Some(db) = ctx.database
                        {
                            self.bar_manager.search(&db.song_db, &self.search_text);
                            self.score_cache_dirty = true;
                        }
                        self.search_mode = false;
                        self.search_text.clear();
                        return;
                    }
                    ControlKeys::Escape => {
                        // Cancel search
                        self.search_mode = false;
                        self.search_text.clear();
                        return;
                    }
                    ControlKeys::Del => {
                        // Backspace
                        self.search_text.pop();
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    fn select_current(&mut self, ctx: &mut StateContext) {
        // Pre-extract data from bar variants that need ownership to avoid borrow checker issues
        // with self.bar_manager (immutable borrow) vs self (mutable borrow).
        enum BarAction {
            Song {
                path: String,
            },
            Directory,
            Course(bms_database::CourseData),
            LeaderBoard {
                song_data: Box<bms_database::SongData>,
                from_lr2ir: bool,
            },
            Function(bar_manager::FunctionAction),
            Grade(bar_manager::GradeBarData),
            RandomCourse(bms_database::RandomCourseData),
            None,
        }

        let action = match self.bar_manager.current() {
            Some(Bar::Song(song_data)) => BarAction::Song {
                path: song_data.path.clone(),
            },
            Some(Bar::Folder { .. })
            | Some(Bar::TableRoot { .. })
            | Some(Bar::HashFolder { .. })
            | Some(Bar::Container { .. })
            | Some(Bar::SameFolder { .. })
            | Some(Bar::SearchWord { .. })
            | Some(Bar::Command { .. })
            | Some(Bar::Executable { .. })
            | Some(Bar::ContextMenu(_)) => BarAction::Directory,
            Some(Bar::Course(course_data)) => BarAction::Course((**course_data).clone()),
            Some(Bar::LeaderBoard {
                song_data,
                from_lr2ir,
            }) => BarAction::LeaderBoard {
                song_data: Box::new((**song_data).clone()),
                from_lr2ir: *from_lr2ir,
            },
            Some(Bar::Function { action, .. }) => BarAction::Function(action.clone()),
            Some(Bar::Grade(grade_data)) => BarAction::Grade((**grade_data).clone()),
            Some(Bar::RandomCourse(rc_data)) => BarAction::RandomCourse((**rc_data).clone()),
            std::option::Option::None => BarAction::None,
        };

        match action {
            BarAction::Song { path } => {
                let path = std::path::PathBuf::from(&path);
                match bms_model::BmsDecoder::decode(&path) {
                    Ok(model) => {
                        ctx.resource.play_mode = model.mode;
                        ctx.resource.bms_dir = path.parent().map(|p| p.to_path_buf());
                        ctx.resource.bms_path = Some(path.clone());
                        ctx.resource.bms_model = Some(model);
                        self.fadeout_started = true;
                        ctx.timer.set_timer_on(TIMER_FADEOUT);
                    }
                    Err(e) => {
                        tracing::warn!(path = %path.display(), "MusicSelect: failed to load BMS: {e}");
                    }
                }
            }
            BarAction::Directory => {
                if let Some(db) = ctx.database {
                    self.bar_manager.enter_folder(&db.song_db);
                    self.score_cache_dirty = true;
                    if let Some(sm) = ctx.sound_manager.as_deref_mut() {
                        sm.play(SystemSound::Folder);
                    }
                }
            }
            BarAction::Course(course_data) => {
                self.select_course(ctx, &course_data);
            }
            BarAction::LeaderBoard {
                song_data,
                from_lr2ir,
            } => {
                self.enter_leaderboard(*song_data, from_lr2ir);
            }
            // Executable is now treated as Directory (expanded via enter_folder)
            BarAction::Function(func_action) => {
                self.execute_function_action(ctx, func_action);
            }
            BarAction::Grade(grade_data) => {
                self.select_course_with_constraints(
                    ctx,
                    &grade_data.course,
                    grade_data.constraints.clone(),
                );
            }
            BarAction::RandomCourse(rc_data) => {
                self.select_random_course(ctx, &rc_data);
            }
            BarAction::None => {}
        }
    }

    /// Load rival scores from the selected rival's DB into BarManager.
    fn load_rival_scores(&mut self, ctx: &StateContext) {
        let db = match ctx.database {
            Some(db) => db,
            None => return,
        };

        let rival_count = db.rival.rival_count();
        if rival_count == 0 || self.selected_rival >= rival_count {
            self.bar_manager.set_rival_scores(HashMap::new());
            return;
        }

        let rival = &db.rival.rivals()[self.selected_rival];
        let mode = ctx.resource.play_mode.mode_id();
        match bms_database::RivalDataAccessor::get_all_rival_scores(&rival.db_path, mode) {
            Ok(scores) => {
                let map: HashMap<String, ScoreData> =
                    scores.into_iter().map(|s| (s.sha256.clone(), s)).collect();
                info!(
                    rival = %rival.info.name,
                    scores = map.len(),
                    "MusicSelect: loaded rival scores"
                );
                self.bar_manager.set_rival_scores(map);
            }
            Err(e) => {
                warn!("MusicSelect: failed to load rival scores: {e}");
                self.bar_manager.set_rival_scores(HashMap::new());
            }
        }
    }

    /// Enter the leaderboard for a song.
    ///
    /// Pushes the current bar list onto the folder stack, shows a loading
    /// placeholder, and spawns a background thread to fetch IR rankings.
    fn enter_leaderboard(&mut self, song_data: bms_database::SongData, from_lr2ir: bool) {
        use bar_manager::FunctionAction;

        // Push current bars and show loading placeholder
        self.bar_manager.push_and_set_bars(vec![Bar::Function {
            title: "Loading leaderboard...".to_string(),
            subtitle: None,
            display_bar_type: 5,
            action: FunctionAction::None,
            lamp: 0,
        }]);

        // Spawn background IR fetch
        let (tx, rx) = std::sync::mpsc::channel();
        self.ir_fetch_receiver = Some(parking_lot::Mutex::new(rx));

        let song = song_data.clone();
        std::thread::spawn(move || {
            let bars = if from_lr2ir {
                fetch_lr2ir_leaderboard(&song)
            } else {
                leaderboard::error_to_bars("Leaderboard source not available")
            };
            let _ = tx.send(bars);
        });

        info!(
            title = %song_data.title,
            from_lr2ir,
            "MusicSelect: entering leaderboard"
        );
    }

    fn select_course(&mut self, ctx: &mut StateContext, course_data: &bms_database::CourseData) {
        self.select_course_with_constraints(ctx, course_data, Vec::new());
    }

    fn select_course_with_constraints(
        &mut self,
        ctx: &mut StateContext,
        course_data: &bms_database::CourseData,
        constraints: Vec<bms_database::CourseDataConstraint>,
    ) {
        let db = match ctx.database {
            Some(db) => db,
            None => {
                tracing::warn!("MusicSelect: no database available for course lookup");
                return;
            }
        };

        let mut models = Vec::new();
        let mut dirs = Vec::new();

        for (i, song_ref) in course_data.hash.iter().enumerate() {
            // Look up song by hash (prefer sha256, fall back to md5)
            let found = if !song_ref.sha256.is_empty() {
                db.song_db.get_song_datas("sha256", &song_ref.sha256)
            } else if !song_ref.md5.is_empty() {
                db.song_db.get_song_datas("md5", &song_ref.md5)
            } else {
                tracing::warn!(stage = i, "MusicSelect: course stage has no hash");
                return;
            };

            let song_data = match found {
                Ok(songs) if !songs.is_empty() => songs.into_iter().next().unwrap(),
                Ok(_) => {
                    tracing::warn!(stage = i, "MusicSelect: course stage song not found in DB");
                    return;
                }
                Err(e) => {
                    tracing::warn!(stage = i, "MusicSelect: course stage DB lookup failed: {e}");
                    return;
                }
            };

            let path = std::path::PathBuf::from(&song_data.path);
            match bms_model::BmsDecoder::decode(&path) {
                Ok(model) => {
                    dirs.push(path.parent().map(|p| p.to_path_buf()).unwrap_or_default());
                    models.push(model);
                }
                Err(e) => {
                    tracing::warn!(
                        stage = i,
                        path = %path.display(),
                        "MusicSelect: failed to load course BMS: {e}"
                    );
                    return;
                }
            }
        }

        if !models.is_empty() {
            ctx.resource
                .start_course(course_data.clone(), models, dirs, constraints);
            ctx.resource.load_course_stage();
            self.fadeout_started = true;
            ctx.timer.set_timer_on(TIMER_FADEOUT);
        }
    }

    /// Execute a FunctionAction from a Function bar selection.
    fn execute_function_action(
        &mut self,
        ctx: &mut StateContext,
        action: bar_manager::FunctionAction,
    ) {
        use bar_manager::FunctionAction;
        match action {
            FunctionAction::Autoplay(song_data) => {
                let path = std::path::PathBuf::from(&song_data.path);
                match bms_model::BmsDecoder::decode(&path) {
                    Ok(model) => {
                        ctx.resource.play_mode = model.mode;
                        ctx.resource.bms_dir = path.parent().map(|p| p.to_path_buf());
                        ctx.resource.bms_path = Some(path);
                        ctx.resource.bms_model = Some(model);
                        ctx.resource.player_mode = crate::player_resource::PlayerMode::Autoplay;
                        self.fadeout_started = true;
                        ctx.timer.set_timer_on(TIMER_FADEOUT);
                    }
                    Err(e) => {
                        tracing::warn!("MusicSelect: failed to load BMS for autoplay: {e}");
                    }
                }
            }
            FunctionAction::Practice(song_data) => {
                let path = std::path::PathBuf::from(&song_data.path);
                match bms_model::BmsDecoder::decode(&path) {
                    Ok(model) => {
                        ctx.resource.play_mode = model.mode;
                        ctx.resource.bms_dir = path.parent().map(|p| p.to_path_buf());
                        ctx.resource.bms_path = Some(path);
                        ctx.resource.bms_model = Some(model);
                        ctx.resource.is_practice = true;
                        self.fadeout_started = true;
                        ctx.timer.set_timer_on(TIMER_FADEOUT);
                    }
                    Err(e) => {
                        tracing::warn!("MusicSelect: failed to load BMS for practice: {e}");
                    }
                }
            }
            FunctionAction::ShowSameFolder { title, folder_crc } => {
                info!(title = %title, folder_crc = %folder_crc, "MusicSelect: show same folder via function");
                if let Some(db) = ctx.database {
                    self.bar_manager.push_and_set_bars(vec![Bar::SameFolder {
                        name: title,
                        folder_crc,
                    }]);
                    self.bar_manager.enter_folder(&db.song_db);
                    self.score_cache_dirty = true;
                }
            }
            FunctionAction::CopyToClipboard(text) => {
                self.command_executor.set_clipboard(&text);
            }
            FunctionAction::OpenUrl(url) => {
                #[cfg(target_os = "macos")]
                let result = std::process::Command::new("open").arg(&url).spawn();
                #[cfg(target_os = "linux")]
                let result = std::process::Command::new("xdg-open").arg(&url).spawn();
                #[cfg(target_os = "windows")]
                let result = std::process::Command::new("cmd")
                    .args(["/C", "start", &url])
                    .spawn();
                #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
                let result: Result<std::process::Child, std::io::Error> = Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "unsupported OS",
                ));

                if let Err(e) = result {
                    tracing::warn!("MusicSelect: failed to open URL '{url}': {e}");
                }
            }
            FunctionAction::ToggleFavorite { sha256, flag } => {
                if let Some(db) = ctx.database {
                    if let Err(e) = db.song_db.update_favorite(&sha256, flag) {
                        tracing::warn!("MusicSelect: failed to toggle favorite: {e}");
                    } else {
                        info!(sha256 = %sha256, flag, "MusicSelect: favorite toggled via function");
                    }
                }
            }
            FunctionAction::PlayReplay {
                song_data,
                replay_index,
            } => {
                let path = std::path::PathBuf::from(&song_data.path);
                match bms_model::BmsDecoder::decode(&path) {
                    Ok(model) => {
                        ctx.resource.play_mode = model.mode;
                        ctx.resource.bms_dir = path.parent().map(|p| p.to_path_buf());
                        ctx.resource.bms_path = Some(path);
                        ctx.resource.bms_model = Some(model);
                        ctx.resource.player_mode =
                            crate::player_resource::PlayerMode::Replay(replay_index as u8);
                        self.fadeout_started = true;
                        ctx.timer.set_timer_on(TIMER_FADEOUT);
                    }
                    Err(e) => {
                        tracing::warn!("MusicSelect: failed to load BMS for replay: {e}");
                    }
                }
            }
            FunctionAction::GhostBattle { song_data, lr2_id } => {
                let path = std::path::PathBuf::from(&song_data.path);
                match bms_model::BmsDecoder::decode(&path) {
                    Ok(model) => {
                        ctx.resource.play_mode = model.mode;
                        ctx.resource.bms_dir = path.parent().map(|p| p.to_path_buf());
                        ctx.resource.bms_path = Some(path);
                        ctx.resource.bms_model = Some(model);
                        ctx.resource.ghost_battle =
                            Some(crate::player_resource::GhostBattleSettings {
                                random_seed: lr2_id,
                                lane_sequence: 0,
                            });
                        self.fadeout_started = true;
                        ctx.timer.set_timer_on(TIMER_FADEOUT);
                    }
                    Err(e) => {
                        tracing::warn!("MusicSelect: failed to load BMS for ghost battle: {e}");
                    }
                }
            }
            FunctionAction::ViewLeaderboard { song_data } => {
                self.enter_leaderboard(*song_data, true);
            }
            FunctionAction::None => {}
        }
    }

    /// Execute a random course selection using the RandomCourseData lottery system.
    fn select_random_course(
        &mut self,
        ctx: &mut StateContext,
        rc_data: &bms_database::RandomCourseData,
    ) {
        let db = match ctx.database {
            Some(db) => db,
            None => {
                tracing::warn!("MusicSelect: no database available for random course");
                return;
            }
        };

        // Query candidates for each stage
        let mut candidates_per_stage = Vec::new();
        for (i, stage) in rc_data.stage.iter().enumerate() {
            if stage.sql.is_empty() {
                tracing::warn!(stage = i, "RandomCourse: stage has empty SQL");
                return;
            }
            match db.song_db.get_song_datas_by_sql(&stage.sql) {
                Ok(songs) => {
                    let course_songs: Vec<bms_database::CourseSongData> = songs
                        .iter()
                        .map(|s| bms_database::CourseSongData {
                            sha256: s.sha256.clone(),
                            md5: s.md5.clone(),
                            title: s.title.clone(),
                        })
                        .collect();
                    if course_songs.is_empty() {
                        tracing::warn!(stage = i, sql = %stage.sql, "RandomCourse: no songs matched query");
                        return;
                    }
                    candidates_per_stage.push(course_songs);
                }
                Err(e) => {
                    tracing::warn!(stage = i, sql = %stage.sql, "RandomCourse: query failed: {e}");
                    return;
                }
            }
        }

        if candidates_per_stage.is_empty() {
            tracing::warn!("RandomCourse: no stages defined");
            return;
        }

        // Run the lottery to pick one song per stage
        let mut rng = rand::rng();
        let picked = rc_data.lottery(&candidates_per_stage, &mut rng);

        // Build course data from lottery results
        let selected_songs: Vec<bms_database::CourseSongData> =
            picked.into_iter().flatten().collect();
        if selected_songs.len() != rc_data.stage.len() {
            tracing::warn!(
                expected = rc_data.stage.len(),
                got = selected_songs.len(),
                "RandomCourse: lottery did not fill all stages"
            );
            return;
        }

        let course_data = rc_data.create_course_data(&selected_songs);

        // Load BMS models for all selected songs
        let mut models = Vec::new();
        let mut dirs = Vec::new();
        for (i, song_ref) in selected_songs.iter().enumerate() {
            let found = if !song_ref.sha256.is_empty() {
                db.song_db.get_song_datas("sha256", &song_ref.sha256)
            } else if !song_ref.md5.is_empty() {
                db.song_db.get_song_datas("md5", &song_ref.md5)
            } else {
                tracing::warn!(stage = i, "RandomCourse: selected song has no hash");
                return;
            };

            let song_data = match found {
                Ok(songs) if !songs.is_empty() => songs.into_iter().next().unwrap(),
                Ok(_) => {
                    tracing::warn!(stage = i, "RandomCourse: selected song not found in DB");
                    return;
                }
                Err(e) => {
                    tracing::warn!(stage = i, "RandomCourse: DB lookup failed: {e}");
                    return;
                }
            };

            let path = std::path::PathBuf::from(&song_data.path);
            match bms_model::BmsDecoder::decode(&path) {
                Ok(model) => {
                    dirs.push(path.parent().map(|p| p.to_path_buf()).unwrap_or_default());
                    models.push(model);
                }
                Err(e) => {
                    tracing::warn!(
                        stage = i,
                        path = %path.display(),
                        "RandomCourse: failed to load BMS: {e}"
                    );
                    return;
                }
            }
        }

        if !models.is_empty() {
            // RandomCourse constraints come from the CourseData itself
            let constraints = course_data.constraint.clone();
            ctx.resource
                .start_course(course_data, models, dirs, constraints);
            ctx.resource.load_course_stage();
            self.fadeout_started = true;
            ctx.timer.set_timer_on(TIMER_FADEOUT);
            info!(name = %rc_data.name, "MusicSelect: random course started");
        }
    }
}

/// Fetch leaderboard entries from LR2IR in a blocking context.
///
/// Creates a short-lived tokio runtime to execute the async API call.
fn fetch_lr2ir_leaderboard(song_data: &bms_database::SongData) -> Vec<bar_manager::Bar> {
    use bms_ir::{IRChartData, LR2IRConnection};

    let lr2ir = LR2IRConnection::new();
    let chart = IRChartData::from(song_data);

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => return leaderboard::error_to_bars(&format!("Runtime error: {e}")),
    };

    match rt.block_on(lr2ir.get_score_data(&chart)) {
        Ok(entries) => leaderboard::entries_to_bars(&entries, song_data),
        Err(e) => leaderboard::error_to_bars(&format!("{e}")),
    }
}

#[cfg(test)]
impl MusicSelectState {
    pub(crate) fn bar_manager(&self) -> &BarManager {
        &self.bar_manager
    }

    pub(crate) fn is_fadeout_started(&self) -> bool {
        self.fadeout_started
    }

    pub(crate) fn sort_mode(&self) -> SortMode {
        self.sort_mode
    }

    pub(crate) fn mode_filter(&self) -> Option<i32> {
        self.mode_filter
    }

    pub(crate) fn search_mode(&self) -> bool {
        self.search_mode
    }

    pub(crate) fn search_text(&self) -> &str {
        &self.search_text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database_manager::DatabaseManager;
    use crate::input_mapper::InputState;
    use crate::player_resource::PlayerResource;
    use crate::timer_manager::TimerManager;
    use bms_config::{Config, PlayerConfig};
    use bms_database::SongData;
    use bms_model::BmsModel;

    fn make_ctx<'a>(
        timer: &'a mut TimerManager,
        resource: &'a mut PlayerResource,
        config: &'a Config,
        player_config: &'a mut PlayerConfig,
        transition: &'a mut Option<AppStateType>,
    ) -> StateContext<'a> {
        StateContext {
            timer,
            resource,
            config,
            player_config,
            transition,
            keyboard_backend: None,
            database: None,
            input_state: None,
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        }
    }

    /// Create a context with input enabled (TIMER_STARTINPUT on) and a key pressed.
    fn make_input_ctx<'a>(
        timer: &'a mut TimerManager,
        resource: &'a mut PlayerResource,
        config: &'a Config,
        player_config: &'a mut PlayerConfig,
        transition: &'a mut Option<AppStateType>,
        input_state: &'a InputState,
    ) -> StateContext<'a> {
        StateContext {
            timer,
            resource,
            config,
            player_config,
            transition,
            keyboard_backend: None,
            database: None,
            input_state: Some(input_state),
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        }
    }

    fn setup_input_ready(timer: &mut TimerManager) {
        timer.set_now_micro_time(1_000_000);
        timer.switch_timer(TIMER_STARTINPUT, true);
    }

    #[test]
    fn table_update_job_guard_is_single_flight() {
        finish_table_update_job_for_test();
        assert!(try_start_table_update_job());
        assert!(!try_start_table_update_job());
        finish_table_update_job_for_test();
        assert!(try_start_table_update_job());
        finish_table_update_job_for_test();
    }

    #[test]
    fn create_with_bms_loaded_transitions_to_decide() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(BmsModel::default());
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.create(&mut ctx);
        assert_eq!(transition, Some(AppStateType::Decide));
    }

    #[test]
    fn create_without_bms_stays() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.create(&mut ctx);
        assert_eq!(transition, None);
    }

    #[test]
    fn render_enables_input_after_delay() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Before delay
        timer.set_now_micro_time(400_000);
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert!(!timer.is_timer_on(TIMER_STARTINPUT));

        // After delay
        timer.set_now_micro_time(501_000);
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert!(timer.is_timer_on(TIMER_STARTINPUT));
    }

    #[test]
    fn render_fadeout_transitions_to_decide() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(1_000_000);
        timer.set_timer_on(TIMER_FADEOUT);
        timer.set_now_micro_time(1_501_000);

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(transition, Some(AppStateType::Decide));
    }

    #[test]
    fn sort_mode_cycles_on_num2() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        assert_eq!(state.sort_mode(), SortMode::Default);

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Num2],
        };
        let mut ctx = make_input_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
            &input,
        );
        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::Title);

        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::Artist);

        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::Level);

        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::Bpm);

        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::Length);

        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::Clear);

        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::Score);

        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::MissCount);

        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::Duration);

        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::LastUpdate);

        state.input(&mut ctx);
        assert_eq!(state.sort_mode(), SortMode::Default);
    }

    #[test]
    fn mode_filter_cycles_on_num1() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        assert_eq!(state.mode_filter(), None);

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Num1],
        };
        // Without database, filter changes but no reload happens
        let mut ctx = make_input_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
            &input,
        );
        state.input(&mut ctx);
        assert_eq!(state.mode_filter(), Some(7));

        state.input(&mut ctx);
        assert_eq!(state.mode_filter(), Some(14));

        state.input(&mut ctx);
        assert_eq!(state.mode_filter(), Some(9));

        state.input(&mut ctx);
        assert_eq!(state.mode_filter(), Some(5));

        state.input(&mut ctx);
        assert_eq!(state.mode_filter(), Some(10));

        state.input(&mut ctx);
        assert_eq!(state.mode_filter(), Some(25));

        state.input(&mut ctx);
        assert_eq!(state.mode_filter(), None);
    }

    #[test]
    fn gauge_cycles_on_num3() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        assert_eq!(player_config.gauge, 0);

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Num3],
        };

        // Press Num3: gauge 0 -> 1
        {
            let mut ctx = make_input_ctx(
                &mut timer,
                &mut resource,
                &config,
                &mut player_config,
                &mut transition,
                &input,
            );
            state.input(&mut ctx);
        }
        assert_eq!(player_config.gauge, 1);

        // Press Num3: gauge 1 -> 2
        {
            let mut ctx = make_input_ctx(
                &mut timer,
                &mut resource,
                &config,
                &mut player_config,
                &mut transition,
                &input,
            );
            state.input(&mut ctx);
        }
        assert_eq!(player_config.gauge, 2);

        // Test wrap-around: 5 -> 0
        player_config.gauge = 5;
        {
            let mut ctx = make_input_ctx(
                &mut timer,
                &mut resource,
                &config,
                &mut player_config,
                &mut transition,
                &input,
            );
            state.input(&mut ctx);
        }
        assert_eq!(player_config.gauge, 0);
    }

    #[test]
    fn random_cycles_on_num4() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        assert_eq!(player_config.random, 0);

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Num4],
        };

        // Press Num4: random 0 -> 1
        {
            let mut ctx = make_input_ctx(
                &mut timer,
                &mut resource,
                &config,
                &mut player_config,
                &mut transition,
                &input,
            );
            state.input(&mut ctx);
        }
        assert_eq!(player_config.random, 1);

        // Test wrap-around: 9 -> 0
        player_config.random = 9;
        {
            let mut ctx = make_input_ctx(
                &mut timer,
                &mut resource,
                &config,
                &mut player_config,
                &mut transition,
                &input,
            );
            state.input(&mut ctx);
        }
        assert_eq!(player_config.random, 0);
    }

    #[test]
    fn dp_option_cycles_on_num5() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        assert_eq!(player_config.doubleoption, 0);

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Num5],
        };

        // Press Num5: doubleoption 0 -> 1
        {
            let mut ctx = make_input_ctx(
                &mut timer,
                &mut resource,
                &config,
                &mut player_config,
                &mut transition,
                &input,
            );
            state.input(&mut ctx);
        }
        assert_eq!(player_config.doubleoption, 1);

        // Test wrap-around: 3 -> 0
        player_config.doubleoption = 3;
        {
            let mut ctx = make_input_ctx(
                &mut timer,
                &mut resource,
                &config,
                &mut player_config,
                &mut transition,
                &input,
            );
            state.input(&mut ctx);
        }
        assert_eq!(player_config.doubleoption, 0);
    }

    #[test]
    fn del_transitions_to_key_config() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Del],
        };
        let mut ctx = make_input_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
            &input,
        );
        state.input(&mut ctx);
        assert_eq!(transition, Some(AppStateType::KeyConfig));
    }

    // --- Favorite toggle tests ---

    fn make_db_with_song() -> (DatabaseManager, String) {
        let db = DatabaseManager::open_in_memory().unwrap();
        let sha = "aaa_sha256_hash".to_string();
        let song = SongData {
            md5: "aaa_md5".to_string(),
            sha256: sha.clone(),
            title: "Test Song".to_string(),
            path: "test.bms".to_string(),
            folder: "test_folder_crc".to_string(),
            ..Default::default()
        };
        db.song_db.set_song_datas(&[song]).unwrap();
        (db, sha)
    }

    /// Load a Song bar directly into bar_manager for tests that need a song at cursor.
    ///
    /// `load_root` now groups songs into Folder bars. Use this helper when the test
    /// needs a Song bar at cursor position 0.
    fn load_song_bar_for_test(state: &mut MusicSelectState, sha256: &str) {
        state
            .bar_manager
            .set_bars_for_test(vec![Bar::Song(Box::new(SongData {
                md5: "aaa_md5".to_string(),
                sha256: sha256.to_string(),
                title: "Test Song".to_string(),
                path: "test.bms".to_string(),
                folder: "test_folder_crc".to_string(),
                ..Default::default()
            }))]);
    }

    #[test]
    fn f9_toggles_favorite_song() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let (db, sha) = make_db_with_song();
        setup_input_ready(&mut timer);

        // Place a Song bar directly (load_root produces Folder bars, not Song bars).
        load_song_bar_for_test(&mut state, &sha);
        assert_eq!(state.bar_manager.bar_count(), 1);

        let input = InputState {
            commands: vec![KeyCommand::AddFavoriteSong],
            pressed_keys: vec![ControlKeys::F9],
        };
        {
            let mut ctx = StateContext {
                timer: &mut timer,
                resource: &mut resource,
                config: &config,
                player_config: &mut player_config,
                transition: &mut transition,
                keyboard_backend: None,
                database: Some(&db),
                input_state: Some(&input),
                skin_manager: None,
                sound_manager: None,
                received_chars: &[],
                bevy_images: None,
                shared_state: None,
                preview_music: None,
            };
            state.input(&mut ctx);
        }

        // Check DB was updated (0 ^ 1 = 1)
        let found = db.song_db.get_song_datas("sha256", &sha).unwrap();
        assert_eq!(found[0].favorite, 1);

        // Toggle again (1 ^ 1 = 0)
        {
            let mut ctx = StateContext {
                timer: &mut timer,
                resource: &mut resource,
                config: &config,
                player_config: &mut player_config,
                transition: &mut transition,
                keyboard_backend: None,
                database: Some(&db),
                input_state: Some(&input),
                skin_manager: None,
                sound_manager: None,
                received_chars: &[],
                bevy_images: None,
                shared_state: None,
                preview_music: None,
            };
            state.input(&mut ctx);
        }
        let found = db.song_db.get_song_datas("sha256", &sha).unwrap();
        assert_eq!(found[0].favorite, 0);
    }

    #[test]
    fn f10_toggles_favorite_chart() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let (db, sha) = make_db_with_song();
        setup_input_ready(&mut timer);
        // Place a Song bar directly (load_root produces Folder bars, not Song bars).
        load_song_bar_for_test(&mut state, &sha);

        let input = InputState {
            commands: vec![KeyCommand::AddFavoriteChart],
            pressed_keys: vec![ControlKeys::F10],
        };
        {
            let mut ctx = StateContext {
                timer: &mut timer,
                resource: &mut resource,
                config: &config,
                player_config: &mut player_config,
                transition: &mut transition,
                keyboard_backend: None,
                database: Some(&db),
                input_state: Some(&input),
                skin_manager: None,
                sound_manager: None,
                received_chars: &[],
                bevy_images: None,
                shared_state: None,
                preview_music: None,
            };
            state.input(&mut ctx);
        }

        let found = db.song_db.get_song_datas("sha256", &sha).unwrap();
        assert_eq!(found[0].favorite, 2); // FAVORITE_CHART = 2
    }

    #[test]
    fn favorite_toggle_no_song_is_noop() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        // No songs loaded, bar_manager is empty

        let input = InputState {
            commands: vec![KeyCommand::AddFavoriteSong],
            pressed_keys: vec![ControlKeys::F9],
        };
        let mut ctx = make_input_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
            &input,
        );
        // Should not panic
        state.input(&mut ctx);
    }

    // --- Search mode tests ---

    #[test]
    fn num0_enters_search_mode() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        assert!(!state.search_mode());

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Num0],
        };
        let mut ctx = make_input_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
            &input,
        );
        state.input(&mut ctx);
        assert!(state.search_mode());
        assert!(state.search_text().is_empty());
    }

    #[test]
    fn search_mode_escape_cancels() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        state.search_mode = true;
        state.search_text = "abc".to_string();

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Escape],
        };
        let mut ctx = make_input_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
            &input,
        );
        state.input(&mut ctx);
        assert!(!state.search_mode());
        assert!(state.search_text().is_empty());
    }

    #[test]
    fn search_mode_enter_executes_search() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let (db, _sha) = make_db_with_song();
        setup_input_ready(&mut timer);
        state.bar_manager.load_root(&db.song_db);
        state.search_mode = true;
        state.search_text = "Test".to_string();

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Enter],
        };
        {
            let mut ctx = StateContext {
                timer: &mut timer,
                resource: &mut resource,
                config: &config,
                player_config: &mut player_config,
                transition: &mut transition,
                keyboard_backend: None,
                database: Some(&db),
                input_state: Some(&input),
                skin_manager: None,
                sound_manager: None,
                received_chars: &[],
                bevy_images: None,
                shared_state: None,
                preview_music: None,
            };
            state.input(&mut ctx);
        }

        assert!(!state.search_mode());
        // Search results pushed, bar_manager should be in folder (search results)
        assert!(state.bar_manager.is_in_folder());
        assert_eq!(state.bar_manager.bar_count(), 1);
    }

    #[test]
    fn search_mode_accepts_chars() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        state.search_mode = true;

        let chars = ['h', 'i'];
        let input = InputState {
            commands: vec![],
            pressed_keys: vec![],
        };
        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: Some(&input),
            skin_manager: None,
            sound_manager: None,
            received_chars: &chars,
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        };
        state.input(&mut ctx);
        assert_eq!(state.search_text(), "hi");
    }

    #[test]
    fn search_mode_del_removes_char() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        state.search_mode = true;
        state.search_text = "abc".to_string();

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Del],
        };
        let mut ctx = make_input_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
            &input,
        );
        state.input(&mut ctx);
        assert_eq!(state.search_text(), "ab");
    }

    #[test]
    fn search_mode_blocks_cursor_movement() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);
        state.search_mode = true;

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Up],
        };
        let mut ctx = make_input_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
            &input,
        );
        // In search mode, cursor movement should be ignored
        state.input(&mut ctx);
        assert!(state.search_mode()); // Still in search mode
    }

    // --- Course bar tests ---

    fn sample_course(name: &str, sha256: &str) -> bms_database::CourseData {
        use bms_database::CourseSongData;
        bms_database::CourseData {
            name: name.to_string(),
            hash: vec![CourseSongData {
                sha256: sha256.to_string(),
                md5: String::new(),
                title: "Stage 1".to_string(),
            }],
            constraint: Vec::new(),
            trophy: Vec::new(),
            release: true,
        }
    }

    #[test]
    fn course_select_no_db_is_noop() {
        // Selecting a course without a database should not panic and should not start fadeout.
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_input_ready(&mut timer);

        state.bar_manager = bar_manager::BarManager::new();
        let course = sample_course("Test Course", "nonexistent_sha");
        state.bar_manager.add_courses(&[course]);

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Enter],
        };
        let mut ctx = make_input_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
            &input,
        );
        state.input(&mut ctx);
        // No database => should not start fadeout
        assert!(!state.is_fadeout_started());
        assert!(!resource.is_course());
    }

    #[test]
    fn course_select_song_not_in_db_is_noop() {
        // Selecting a course whose songs aren't in the DB should not start fadeout.
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let db = DatabaseManager::open_in_memory().unwrap();
        setup_input_ready(&mut timer);

        let course = sample_course("Test Course", "nonexistent_sha");
        state.bar_manager = bar_manager::BarManager::new();
        state.bar_manager.add_courses(&[course]);

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Enter],
        };
        {
            let mut ctx = StateContext {
                timer: &mut timer,
                resource: &mut resource,
                config: &config,
                player_config: &mut player_config,
                transition: &mut transition,
                keyboard_backend: None,
                database: Some(&db),
                input_state: Some(&input),
                skin_manager: None,
                sound_manager: None,
                received_chars: &[],
                bevy_images: None,
                shared_state: None,
                preview_music: None,
            };
            state.input(&mut ctx);
        }
        // Song not found => should not start fadeout
        assert!(!state.is_fadeout_started());
        assert!(!resource.is_course());
    }

    #[test]
    fn course_bar_enter_does_not_enter_folder() {
        // Pressing Enter on a Course bar should not call enter_folder.
        // Instead it attempts course selection (which may fail without BMS files).
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let db = DatabaseManager::open_in_memory().unwrap();
        setup_input_ready(&mut timer);

        let course = sample_course("Test Course", "some_sha");
        state.bar_manager = bar_manager::BarManager::new();
        state.bar_manager.add_courses(&[course]);

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Enter],
        };
        {
            let mut ctx = StateContext {
                timer: &mut timer,
                resource: &mut resource,
                config: &config,
                player_config: &mut player_config,
                transition: &mut transition,
                keyboard_backend: None,
                database: Some(&db),
                input_state: Some(&input),
                skin_manager: None,
                sound_manager: None,
                received_chars: &[],
                bevy_images: None,
                shared_state: None,
                preview_music: None,
            };
            state.input(&mut ctx);
        }
        // Should NOT have pushed into folder stack
        assert!(!state.bar_manager().is_in_folder());
    }

    // --- MusicSelectCommand keyboard shortcut tests ---

    #[test]
    fn num8_triggers_show_same_folder() {
        let mut state = MusicSelectState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let (db, sha) = make_db_with_song();
        setup_input_ready(&mut timer);
        // Place a Song bar directly so ShowSongsOnSameFolder can extract the folder_crc.
        load_song_bar_for_test(&mut state, &sha);
        assert_eq!(state.bar_manager.bar_count(), 1);
        assert!(!state.bar_manager.is_in_folder());

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Num8],
        };
        {
            let mut ctx = StateContext {
                timer: &mut timer,
                resource: &mut resource,
                config: &config,
                player_config: &mut player_config,
                transition: &mut transition,
                keyboard_backend: None,
                database: Some(&db),
                input_state: Some(&input),
                skin_manager: None,
                sound_manager: None,
                received_chars: &[],
                bevy_images: None,
                shared_state: None,
                preview_music: None,
            };
            state.input(&mut ctx);
        }
        // Should have pushed same-folder results into bar manager
        assert!(state.bar_manager().is_in_folder());
    }

    #[test]
    fn cursor_change_resets_replay() {
        let mut state = MusicSelectState::new();
        // Manually set replay to non-zero
        state.command_executor = command::CommandExecutor::new();
        let bm_ref = &state.bar_manager;
        state
            .command_executor
            .execute(command::MusicSelectCommand::NextReplay, bm_ref);
        assert_eq!(state.command_executor.selected_replay(), 1);

        // Trigger cursor change
        state.on_cursor_change(1_000_000);

        // Replay should be reset to 0
        assert_eq!(state.command_executor.selected_replay(), 0);
    }
}
