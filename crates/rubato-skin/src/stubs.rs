// External dependency stubs for Phase 6 Skin System
// Rendering stubs (LibGDX types) are in rendering_stubs.rs, re-exported here for compatibility.

// Re-export all rendering stubs (LibGDX graphics types, file types, GL constants)
pub use crate::rendering_stubs::*;

// ============================================================
// beatoraja types (from other crates, stubbed for phase independence)
// ============================================================

/// Stub for beatoraja.MainState
pub trait MainState {
    fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess;
    fn get_offset_value(&self, id: i32) -> Option<&SkinOffset>;
    fn get_main(&self) -> &MainController;
    fn get_image(&self, id: i32) -> Option<TextureRegion>;
    fn get_resource(&self) -> &PlayerResource;

    /// Returns the integer property value for the given ID.
    /// Used by IntegerPropertyFactory delegate to look up pre-computed values.
    fn integer_value(&self, _id: i32) -> i32 {
        0
    }

    /// Returns the image-index property value for the given ID.
    /// This is separate from `integer_value()` because some LR2 IDs collide
    /// between numeric refs and image selector refs.
    fn image_index_value(&self, id: i32) -> i32 {
        self.integer_value(id)
    }

    /// Returns the string property value for the given ID.
    /// Used by StringPropertyFactory delegate to look up pre-computed values.
    fn string_value(&self, _id: i32) -> String {
        String::new()
    }

    /// Returns the boolean property value for the given ID.
    /// Used by BooleanPropertyFactory delegate to look up pre-computed values.
    fn boolean_value(&self, _id: i32) -> bool {
        false
    }

    /// Returns the float property value for the given ID.
    /// Used by FloatPropertyFactory delegate to look up pre-computed values.
    fn float_value(&self, _id: i32) -> f32 {
        0.0
    }

    /// Sets the float property value for the given ID.
    /// Used by FloatWriter delegate to write values back to state.
    fn set_float_value(&mut self, _id: i32, _value: f32) {
        // default no-op
    }

    // ============================================================
    // Event-facing methods (Phase 41h)
    // These provide mutable config access for EventFactory events.
    // Default implementations log warnings; real implementations
    // are provided by concrete MainState types (MusicSelector, etc.)
    // ============================================================

    /// Returns true if this state is a MusicSelector.
    fn is_music_selector(&self) -> bool {
        false
    }

    /// Returns true if this state is a result screen (MusicResult or CourseResult).
    fn is_result_state(&self) -> bool {
        false
    }

    /// Returns mutable reference to the player config.
    /// Returns None if config is not available (e.g., stub state).
    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        None
    }

    /// Returns immutable reference to the player config.
    /// Returns None if config is not available (e.g., stub state).
    fn get_player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        None
    }

    /// Returns mutable reference to the global config.
    /// Returns None if config is not available.
    fn get_config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
        None
    }

    /// Returns immutable reference to the global config.
    fn get_config_ref(&self) -> Option<&rubato_types::config::Config> {
        None
    }

    /// Returns mutable reference to the currently selected bar's PlayConfig.
    /// Only available for MusicSelector; returns None for other states.
    fn get_selected_play_config_mut(
        &mut self,
    ) -> Option<&mut rubato_types::play_config::PlayConfig> {
        None
    }

    /// Returns immutable reference to the currently selected bar's PlayConfig.
    fn get_selected_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        None
    }

    /// Returns the distribution data (lamps/ranks) for the currently selected directory bar.
    /// Used by SkinDistributionGraph to render folder lamp/rank graphs.
    /// Only meaningful for MusicSelector; returns None for other states.
    fn get_distribution_data(&self) -> Option<rubato_types::distribution_data::DistributionData> {
        None
    }

    /// Play the OPTION_CHANGE system sound.
    fn play_option_change_sound(&mut self) {
        // default no-op
    }

    /// Update the bar manager after a config change (e.g., mode filter, sort).
    /// Only meaningful for MusicSelector.
    fn update_bar_after_change(&mut self) {
        // default no-op
    }

    /// Execute a custom event by ID with arguments.
    /// Delegates to skin.executeCustomEvent(this, id, arg1, arg2) in Java.
    /// Default no-op — real implementations live in concrete MainState types.
    /// SkinRenderContext now carries this capability at the SkinDrawable level.
    fn execute_event(&mut self, _id: i32, _arg1: i32, _arg2: i32) {
        // default no-op
    }

    /// Change the application state (e.g., to CONFIG, SKINCONFIG).
    /// Default no-op — real implementations live in concrete MainState types.
    /// SkinRenderContext now carries this capability at the SkinDrawable level.
    fn change_state(&mut self, _state_type: rubato_types::main_state_type::MainStateType) {
        // default no-op
    }

    /// Select a song with the given play mode.
    /// Only meaningful for MusicSelector.
    fn select_song(&mut self, _mode: rubato_core::bms_player_mode::BMSPlayerMode) {
        // default no-op
    }

    // ============================================================
    // Lua MainStateAccessor methods (Phase 45)
    // These provide read access to score/judge/gauge data and
    // write access to timers, volumes, audio, and events.
    // ============================================================

    /// Returns the ScoreDataProperty for the current state.
    /// Used by Lua rate/exscore/rate_best/exscore_best/rate_rival/exscore_rival functions.
    fn score_data_property(&self) -> &rubato_core::score_data_property::ScoreDataProperty {
        static DEFAULT: std::sync::OnceLock<rubato_core::score_data_property::ScoreDataProperty> =
            std::sync::OnceLock::new();
        DEFAULT.get_or_init(rubato_core::score_data_property::ScoreDataProperty::default)
    }

    /// Returns the total judge count for the given judge index (fast + slow).
    /// Used by Lua `judge(id)` function.
    fn judge_count(&self, _judge: i32, _fast: bool) -> i32 {
        0
    }

    /// Returns the gauge value (0.0-1.0). Only meaningful for BMSPlayer states.
    /// Used by Lua `gauge()` function.
    fn get_gauge_value(&self) -> f32 {
        0.0
    }

    /// Returns the gauge type ID. Only meaningful for BMSPlayer states.
    /// Used by Lua `gauge_type()` function.
    fn gauge_type(&self) -> i32 {
        0
    }

    /// Returns true if this state is a BMSPlayer (gameplay state).
    fn is_bms_player(&self) -> bool {
        false
    }

    /// Returns the recent judge timing offsets (milliseconds).
    /// 100-element circular buffer from JudgeManager.
    fn recent_judges(&self) -> &[i64] {
        &[]
    }

    /// Returns the current write index into the recent judges circular buffer.
    fn recent_judges_index(&self) -> usize {
        0
    }

    /// Returns the current judge type for the given player (1-indexed, 0 = no judge).
    /// Used by SkinJudge to determine which judge image to display.
    fn get_now_judge(&self, _player: i32) -> i32 {
        0
    }

    /// Returns the current combo count for the given player.
    /// Used by SkinJudge to display the combo number.
    fn get_now_combo(&self, _player: i32) -> i32 {
        0
    }

    /// Returns whether the current gauge is at max value.
    /// Used by SkinJudge to display the MAX PG variant.
    fn is_gauge_max(&self) -> bool {
        false
    }

    /// Returns true if the media (audio/BGA) has finished loading.
    /// Used by PracticeConfiguration to show the "PRESS 1KEY TO PLAY" prompt.
    fn is_media_load_finished(&self) -> bool {
        false
    }

    /// Returns true if this is a practice mode play.
    /// Used by SkinBGA to decide whether to draw practice UI or BGA.
    fn is_practice_mode(&self) -> bool {
        false
    }

    /// Set a timer value by ID. Only writable timers (custom timers) are allowed.
    /// Used by Lua `set_timer(id, value)` function.
    /// Default no-op — SkinRenderContext now carries this capability.
    fn set_timer_micro(&mut self, _timer_id: i32, _micro_time: i64) {
        // default no-op
    }

    /// Play an audio file at the given path with volume and loop flag.
    /// Used by Lua `audio_play` and `audio_loop` functions.
    /// Default no-op — SkinRenderContext now carries this capability.
    fn audio_play(&mut self, _path: &str, _volume: f32, _is_loop: bool) {
        // default no-op
    }

    /// Stop an audio file at the given path.
    /// Used by Lua `audio_stop` function.
    /// Default no-op — SkinRenderContext now carries this capability.
    fn audio_stop(&mut self, _path: &str) {
        // default no-op
    }
}

/// Stub for beatoraja.MainController
pub struct MainController {
    pub debug: bool,
}

impl MainController {
    pub fn input_processor(&self) -> &InputProcessor {
        static INPUT: std::sync::OnceLock<InputProcessor> = std::sync::OnceLock::new();
        INPUT.get_or_init(|| InputProcessor)
    }

    pub fn config(&self) -> &rubato_core::config::Config {
        static CONFIG: std::sync::OnceLock<rubato_core::config::Config> =
            std::sync::OnceLock::new();
        CONFIG.get_or_init(rubato_core::config::Config::default)
    }
}

/// Stub for input processor
pub struct InputProcessor;

// SAFETY: InputProcessor is a stateless unit struct with no fields.
// It contains no non-Send/Sync types; the impls are needed because
// it is stored behind OnceLock which requires Send + Sync.
unsafe impl Send for InputProcessor {}
unsafe impl Sync for InputProcessor {}

impl InputProcessor {
    pub fn mouse_x(&self) -> f32 {
        0.0
    }
    pub fn mouse_y(&self) -> f32 {
        0.0
    }
}

// SkinOffset — re-exported from beatoraja-types (Phase 25d-2)
pub use rubato_types::skin_offset::SkinOffset;

/// Timer data carrier for skin rendering — implements TimerAccess from beatoraja-types.
///
/// Holds current time and per-timer-id activation times (snapshot from TimerManager).
/// Previously returned 0 for all per-timer queries (frozen animations).
#[derive(Clone, Debug, Default)]
pub struct Timer {
    pub now_time: i64,
    pub now_micro_time: i64,
    /// Per-timer-id activation times. Index = timer_id, value = micro-time when set
    /// (i64::MIN = OFF). Populated from TimerManager's timer array.
    timers: Vec<i64>,
}

impl Timer {
    /// Create a Timer with time values and a timer array snapshot.
    pub fn with_timers(now_time: i64, now_micro_time: i64, timers: Vec<i64>) -> Self {
        Self {
            now_time,
            now_micro_time,
            timers,
        }
    }

    /// Set the activation time for a specific timer ID.
    /// Grows the timers array as needed (new entries default to i64::MIN = OFF).
    pub fn set_timer_value(&mut self, timer_id: i32, micro_time: i64) {
        if timer_id < 0 {
            return;
        }
        let idx = timer_id as usize;
        if idx >= self.timers.len() {
            self.timers.resize(idx + 1, i64::MIN);
        }
        self.timers[idx] = micro_time;
    }

    pub fn now_time(&self) -> i64 {
        self.now_time
    }

    pub fn now_micro_time(&self) -> i64 {
        self.now_micro_time
    }

    pub fn micro_timer(&self, timer_id: i32) -> i64 {
        if timer_id >= 0 && (timer_id as usize) < self.timers.len() {
            self.timers[timer_id as usize]
        } else {
            i64::MIN
        }
    }

    pub fn timer(&self, timer_id: i32) -> i64 {
        self.micro_timer(timer_id) / 1000
    }

    pub fn now_time_for(&self, timer_id: i32) -> i64 {
        if self.is_timer_on(timer_id) {
            (self.now_micro_time - self.micro_timer(timer_id)) / 1000
        } else {
            0
        }
    }

    pub fn is_timer_on(&self, timer_id: i32) -> bool {
        self.micro_timer(timer_id) != i64::MIN
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for Timer {}

impl rubato_types::timer_access::TimerAccess for Timer {
    fn now_time(&self) -> i64 {
        self.now_time
    }
    fn now_micro_time(&self) -> i64 {
        self.now_micro_time
    }
    fn micro_timer(&self, timer_id: i32) -> i64 {
        Timer::micro_timer(self, timer_id)
    }
    fn timer(&self, timer_id: i32) -> i64 {
        Timer::timer(self, timer_id)
    }
    fn now_time_for(&self, timer_id: i32) -> i64 {
        Timer::now_time_for(self, timer_id)
    }
    fn is_timer_on(&self, timer_id: i32) -> bool {
        Timer::is_timer_on(self, timer_id)
    }
}

/// Stub for beatoraja.Resolution
#[derive(Clone, Debug, Default)]
pub struct Resolution {
    pub width: f32,
    pub height: f32,
}

/// Stub for beatoraja.SkinConfig.Offset
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SkinConfigOffset {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub r: f32,
    pub a: f32,
    pub enabled: bool,
}

// ============================================================
// beatoraja.play types (stubs)
// ============================================================

/// Stub for beatoraja.play.BMSPlayer
pub struct BMSPlayer {
    pub judge_manager: JudgeManager,
}

impl BMSPlayer {
    pub fn skin_type(&self) -> crate::skin_type::SkinType {
        crate::skin_type::SkinType::Play7Keys
    }

    pub fn past_notes(&self) -> i32 {
        0
    }

    pub fn judge_manager(&self) -> &JudgeManager {
        &self.judge_manager
    }
}

/// Stub for beatoraja.play.JudgeManager (minimal for visualizers)
pub struct JudgeManager {
    pub recent_judges: Vec<i64>,
    pub recent_judges_index: usize,
}

impl JudgeManager {
    pub fn recent_judges_index(&self) -> usize {
        self.recent_judges_index
    }

    pub fn recent_judges(&self) -> &[i64] {
        &self.recent_judges
    }
}

/// Stub for beatoraja.result.MusicResult
pub struct MusicResult {
    pub resource: MusicResultResource,
}

impl MusicResult {
    pub fn timing_distribution(&self) -> &TimingDistribution {
        static DEFAULT: std::sync::OnceLock<TimingDistribution> = std::sync::OnceLock::new();
        DEFAULT.get_or_init(|| TimingDistribution {
            distribution: vec![],
            array_center: 0,
            average: 0.0,
            std_dev: 0.0,
        })
    }
}

/// Stub for PlayerResource within MusicResult context
pub struct MusicResultResource;

impl MusicResultResource {
    pub fn bms_model(&self) -> &bms_model::bms_model::BMSModel {
        static MODEL: std::sync::OnceLock<bms_model::bms_model::BMSModel> =
            std::sync::OnceLock::new();
        MODEL.get_or_init(bms_model::bms_model::BMSModel::default)
    }

    pub fn original_mode(&self) -> bms_model::mode::Mode {
        bms_model::mode::Mode::BEAT_7K
    }

    pub fn player_config(&self) -> &rubato_core::player_config::PlayerConfig {
        static PC: std::sync::OnceLock<rubato_core::player_config::PlayerConfig> =
            std::sync::OnceLock::new();
        PC.get_or_init(rubato_core::player_config::PlayerConfig::default)
    }

    pub fn constraint(&self) -> Vec<rubato_core::course_data::CourseDataConstraint> {
        vec![]
    }
}

// TimingDistribution — re-exported from beatoraja-types (Phase 25d-2)
pub use rubato_types::timing_distribution::TimingDistribution;

// beatoraja.song types (re-exports)
pub use rubato_song::song_data::SongData;
pub use rubato_song::song_information::SongInformation;

/// Stub for beatoraja.PlayerResource
pub struct PlayerResource;

impl PlayerResource {
    pub fn songdata(&self) -> Option<&SongData> {
        None
    }

    pub fn bms_model(&self) -> &bms_model::bms_model::BMSModel {
        static MODEL: std::sync::OnceLock<bms_model::bms_model::BMSModel> =
            std::sync::OnceLock::new();
        MODEL.get_or_init(bms_model::bms_model::BMSModel::default)
    }

    pub fn original_mode(&self) -> bms_model::mode::Mode {
        bms_model::mode::Mode::BEAT_7K
    }

    pub fn player_config(&self) -> &rubato_core::player_config::PlayerConfig {
        static PC: std::sync::OnceLock<rubato_core::player_config::PlayerConfig> =
            std::sync::OnceLock::new();
        PC.get_or_init(rubato_core::player_config::PlayerConfig::default)
    }

    pub fn config(&self) -> &rubato_core::config::Config {
        static CFG: std::sync::OnceLock<rubato_core::config::Config> = std::sync::OnceLock::new();
        CFG.get_or_init(rubato_core::config::Config::default)
    }

    pub fn constraint(&self) -> Vec<rubato_core::course_data::CourseDataConstraint> {
        vec![]
    }
}

/// Stub for beatoraja.play.PlaySkin
pub struct PlaySkinStub {
    pub pomyu: rubato_play::pomyu_chara_processor::PomyuCharaProcessor,
}

impl Default for PlaySkinStub {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaySkinStub {
    pub fn new() -> Self {
        Self {
            pomyu: rubato_play::pomyu_chara_processor::PomyuCharaProcessor::new(),
        }
    }

    pub fn add(&mut self, _obj: crate::skin_image::SkinImage) {
        // stub
    }
}

/// Stub for beatoraja.skin.SkinLoader (static methods)
pub struct SkinLoaderStub;

impl SkinLoaderStub {
    pub fn texture(path: &str, usecim: bool) -> Option<Texture> {
        crate::skin_loader::texture(path, usecim)
    }
}
