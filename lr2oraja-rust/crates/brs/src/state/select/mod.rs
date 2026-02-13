// MusicSelect state — song selection browser.
//
// Loads the song list from the database, allows cursor navigation,
// and transitions to Decide when a song is selected.

pub mod bar_manager;
mod select_skin_state;

use std::collections::HashMap;

use tracing::info;

use bms_database::SongInformation;
use bms_database::song_data::{FAVORITE_CHART, FAVORITE_SONG};
use bms_input::control_keys::ControlKeys;
use bms_input::key_command::KeyCommand;
use bms_skin::property_id::{TIMER_FADEOUT, TIMER_STARTINPUT};

use crate::app_state::AppStateType;
use crate::state::{GameStateHandler, StateContext};

use bar_manager::{Bar, BarManager, SortMode};

/// Default input delay in milliseconds.
const DEFAULT_INPUT_DELAY_MS: i64 = 500;
/// Default fadeout duration in milliseconds.
const DEFAULT_FADEOUT_DURATION_MS: i64 = 500;

/// Default scroll animation duration in milliseconds.
const DEFAULT_SCROLL_DURATION_MS: i64 = 150;

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
    /// Whether the score cache needs refresh.
    score_cache_dirty: bool,
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
            score_cache_dirty: true,
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
        info!("MusicSelect: create");

        // If a BMS model is already loaded (via CLI --bms), skip to Decide immediately
        if ctx.resource.bms_model.is_some() {
            info!("MusicSelect: BMS already loaded, transitioning to Decide");
            *ctx.transition = Some(AppStateType::Decide);
            return;
        }

        // Load song list from database
        if let Some(db) = ctx.database {
            self.bar_manager.load_root(&db.song_db);
            self.score_cache_dirty = true;
            info!(
                songs = self.bar_manager.bar_count(),
                "MusicSelect: loaded song list"
            );
        }
    }

    fn render(&mut self, ctx: &mut StateContext) {
        let now = ctx.timer.now_time();

        // Enable input after initial delay
        if now > DEFAULT_INPUT_DELAY_MS {
            ctx.timer.switch_timer(TIMER_STARTINPUT, true);
        }

        // Check fadeout -> transition
        if ctx.timer.is_timer_on(TIMER_FADEOUT)
            && ctx.timer.now_time_of(TIMER_FADEOUT) > DEFAULT_FADEOUT_DURATION_MS
        {
            info!("MusicSelect: transition to Decide");
            *ctx.transition = Some(AppStateType::Decide);
        }

        // Compute scroll interpolation
        let scroll_duration_us = DEFAULT_SCROLL_DURATION_MS * 1000;
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
                if let Ok(scores) = db.score_db.get_score_datas(&sha256_refs, mode) {
                    for sd in scores {
                        let lamp = sd.clear.id() as i32;
                        // Keep best (highest) clear per sha256
                        let entry = self.score_lamp_cache.entry(sd.sha256).or_insert(0);
                        if lamp > *entry {
                            *entry = lamp;
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
            select_skin_state::sync_select_state(shared, &self.bar_manager, has_ln, true);
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
            // Process key commands (F9/F10 favorite toggle)
            for cmd in &input_state.commands {
                match cmd {
                    KeyCommand::AddFavoriteSong => {
                        self.toggle_favorite(ctx, FAVORITE_SONG);
                    }
                    KeyCommand::AddFavoriteChart => {
                        self.toggle_favorite(ctx, FAVORITE_CHART);
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
                        return;
                    }
                    ControlKeys::Down => {
                        self.bar_manager.move_cursor(1);
                        self.scroll_angle = 1;
                        self.scroll_start_us = Some(ctx.timer.now_micro_time());
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
                        self.sort_mode = self.sort_mode.next();
                        self.bar_manager.sort(self.sort_mode);
                        self.score_cache_dirty = true;
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
                            self.bar_manager.sort(self.sort_mode);
                            self.score_cache_dirty = true;
                        }
                        info!(filter = ?self.mode_filter, "MusicSelect: mode filter changed");
                        return;
                    }
                    ControlKeys::Num3 => {
                        // Cycle gauge type
                        ctx.player_config.gauge = (ctx.player_config.gauge + 1) % 6;
                        info!(
                            gauge = ctx.player_config.gauge,
                            "MusicSelect: gauge changed"
                        );
                        return;
                    }
                    ControlKeys::Num4 => {
                        // Cycle random type
                        ctx.player_config.random = (ctx.player_config.random + 1) % 10;
                        info!(
                            random = ctx.player_config.random,
                            "MusicSelect: random changed"
                        );
                        return;
                    }
                    ControlKeys::Num5 => {
                        // Cycle DP option
                        ctx.player_config.doubleoption = (ctx.player_config.doubleoption + 1) % 4;
                        info!(
                            dp = ctx.player_config.doubleoption,
                            "MusicSelect: DP option changed"
                        );
                        return;
                    }
                    ControlKeys::Num6 => {
                        // Cycle hi-speed (placeholder for future integration)
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

    fn shutdown(&mut self, _ctx: &mut StateContext) {
        info!("MusicSelect: shutdown");
    }
}

impl MusicSelectState {
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
        // Clone course data if needed to avoid borrow checker issues
        let course_data_opt = if let Some(Bar::Course(course_data)) = self.bar_manager.current() {
            Some((**course_data).clone())
        } else {
            None
        };

        match self.bar_manager.current() {
            Some(Bar::Song(song_data)) => {
                // Load BMS file
                let path = std::path::PathBuf::from(&song_data.path);
                match bms_model::BmsDecoder::decode(&path) {
                    Ok(model) => {
                        ctx.resource.play_mode = model.mode;
                        ctx.resource.bms_dir = path.parent().map(|p| p.to_path_buf());
                        ctx.resource.bms_model = Some(model);
                        // Start fadeout -> Decide
                        self.fadeout_started = true;
                        ctx.timer.set_timer_on(TIMER_FADEOUT);
                    }
                    Err(e) => {
                        tracing::warn!(path = %path.display(), "MusicSelect: failed to load BMS: {e}");
                    }
                }
            }
            Some(Bar::Folder { .. }) => {
                if let Some(db) = ctx.database {
                    self.bar_manager.enter_folder(&db.song_db);
                    self.score_cache_dirty = true;
                }
            }
            Some(Bar::Course(_)) => {
                if let Some(course_data) = course_data_opt {
                    self.select_course(ctx, &course_data);
                }
            }
            None => {}
        }
    }

    fn select_course(&mut self, ctx: &mut StateContext, course_data: &bms_database::CourseData) {
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
            ctx.resource.start_course(course_data.clone(), models, dirs);
            ctx.resource.load_course_stage();
            self.fadeout_started = true;
            ctx.timer.set_timer_on(TIMER_FADEOUT);
        }
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
        }
    }

    fn setup_input_ready(timer: &mut TimerManager) {
        timer.set_now_micro_time(1_000_000);
        timer.switch_timer(TIMER_STARTINPUT, true);
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
            ..Default::default()
        };
        db.song_db.set_song_datas(&[song]).unwrap();
        (db, sha)
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

        // Load songs into bar_manager
        state.bar_manager.load_root(&db.song_db);
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
        state.bar_manager.load_root(&db.song_db);

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
            };
            state.input(&mut ctx);
        }
        // Should NOT have pushed into folder stack
        assert!(!state.bar_manager().is_in_folder());
    }
}
