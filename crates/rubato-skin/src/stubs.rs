// External dependency stubs for Phase 6 Skin System
// Rendering stubs (LibGDX types) are in rendering_stubs.rs, re-exported here for compatibility.

// Re-export all rendering stubs (LibGDX graphics types, file types, GL constants)
pub use crate::rendering_stubs::*;

// ============================================================
// beatoraja types (from other crates, stubbed for phase independence)
// ============================================================

/// Stub for beatoraja.MainState
///
/// Extends `SkinRenderContext` (which extends `TimerAccess`) so that all
/// property value, config access, event, gauge, judge, audio, and timer
/// methods are inherited from `SkinRenderContext`.
///
/// Only methods that depend on skin-crate-local types (`MainController`,
/// `PlayerResource`, `TextureRegion`) remain here.
pub trait MainState: rubato_types::skin_render_context::SkinRenderContext {
    fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess;
    fn get_main(&self) -> &MainController;
    fn get_image(&self, id: i32) -> Option<TextureRegion>;
    fn get_resource(&self) -> &PlayerResource;

    /// Select a song with the given play mode.
    /// Only meaningful for MusicSelector.
    /// Note: SkinRenderContext has `select_song_mode(event_id: i32)` with a different signature.
    fn select_song(&mut self, _mode: rubato_core::bms_player_mode::BMSPlayerMode) {
        // default no-op
    }

    // ============================================================
    // Backward-compatibility shims (Phase 3b)
    // These delegate to the renamed SkinRenderContext methods so that
    // existing callers continue to compile until Phase 3c migrates them.
    // ============================================================

    /// Deprecated: use `SkinRenderContext::gauge_value()` instead.
    fn get_gauge_value(&self) -> f32 {
        self.gauge_value()
    }

    /// Deprecated: use `SkinRenderContext::now_judge()` instead.
    fn get_now_judge(&self, player: i32) -> i32 {
        self.now_judge(player)
    }

    /// Deprecated: use `SkinRenderContext::now_combo()` instead.
    fn get_now_combo(&self, player: i32) -> i32 {
        self.now_combo(player)
    }

    /// Deprecated: use `SkinRenderContext::player_config_ref()` instead.
    fn get_player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        self.player_config_ref()
    }

    /// Deprecated: use `SkinRenderContext::config_ref()` instead.
    fn get_config_ref(&self) -> Option<&rubato_types::config::Config> {
        self.config_ref()
    }

    /// Deprecated: use `SkinRenderContext::config_mut()` instead.
    fn get_config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
        self.config_mut()
    }

    /// Deprecated: use `SkinRenderContext::selected_play_config_mut()` instead.
    fn get_selected_play_config_mut(
        &mut self,
    ) -> Option<&mut rubato_types::play_config::PlayConfig> {
        self.selected_play_config_mut()
    }

    /// Deprecated: use `SkinRenderContext::current_play_config_ref()` instead.
    fn get_selected_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        self.current_play_config_ref()
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

    pub fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        let raw = timer_id.as_i32();
        if raw >= 0 && (raw as usize) < self.timers.len() {
            self.timers[raw as usize]
        } else {
            i64::MIN
        }
    }

    pub fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.micro_timer(timer_id) / 1000
    }

    pub fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        if self.is_timer_on(timer_id) {
            (self.now_micro_time - self.micro_timer(timer_id)) / 1000
        } else {
            0
        }
    }

    pub fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
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
    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        Timer::micro_timer(self, timer_id)
    }
    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        Timer::timer(self, timer_id)
    }
    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        Timer::now_time_for(self, timer_id)
    }
    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
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
