pub(crate) use std::sync::{Arc, Mutex};

pub(crate) use crate::core::bms_player_mode::BMSPlayerMode;
pub(crate) use crate::core::main_state::{MainState, MainStateData, MainStateType};
pub(crate) use crate::core::pattern::autoplay_modifier::AutoplayModifier;
pub(crate) use crate::core::pattern::extra_note_modifier::ExtraNoteModifier;
pub(crate) use crate::core::pattern::lane_shuffle_modifier::{
    PlayerBattleModifier, PlayerFlipModifier,
};
pub(crate) use crate::core::pattern::long_note_modifier::LongNoteModifier;
pub(crate) use crate::core::pattern::mine_note_modifier::MineNoteModifier;
pub(crate) use crate::core::pattern::mode_modifier::ModeModifier;
pub(crate) use crate::core::pattern::pattern_modifier::{AssistLevel, PatternModifier};
pub(crate) use crate::core::pattern::scroll_speed_modifier::ScrollSpeedModifier;
pub(crate) use crate::core::player_config::PlayerConfig;
pub(crate) use crate::core::score_data::ScoreData;
pub(crate) use crate::core::timer_manager::TimerManager;
pub(crate) use crate::play::bga::bga_processor::BGAProcessor;
pub(crate) use crate::play::bms_player_rule::BMSPlayerRule;
pub(crate) use crate::play::groove_gauge::GrooveGauge;
pub(crate) use crate::play::input::control_input::ControlInputProcessor;
pub(crate) use crate::play::input::key_input::KeyInputProccessor;
pub(crate) use crate::play::input::key_sound::KeySoundProcessor;
pub(crate) use crate::play::judge::algorithm::JudgeAlgorithm;
pub(crate) use crate::play::judge::manager::{JudgeConfig, JudgeManager};
pub(crate) use crate::play::lane_property::LaneProperty;
pub(crate) use crate::play::lane_renderer::{LaneGroupRegion, LaneRenderer};
pub(crate) use crate::play::play_skin::PlaySkin;
pub(crate) use crate::play::practice_configuration::PracticeConfiguration;
pub(crate) use crate::play::rhythm_timer_processor::RhythmTimerProcessor;
pub(crate) use bms::model::bms_model::{BMSModel, LNTYPE_LONGNOTE};
pub(crate) use bms::model::bms_model_utils;
pub(crate) use bms::model::mode::Mode;
pub(crate) use bms::model::note::{Note, TYPE_LONGNOTE, TYPE_UNDEFINED};
pub(crate) use rubato_input::bms_player_input_processor::{BMSPlayerInputProcessor, KEYSTATE_SIZE};
pub(crate) use rubato_input::keyboard_input_processor::ControlKeys;
pub(crate) use rubato_types::audio_config::FrequencyType;
pub(crate) use rubato_types::clear_type::ClearType;
pub(crate) use rubato_types::course_data::CourseDataConstraint;
pub(crate) use rubato_types::play_config::PlayConfig;
pub(crate) use rubato_types::replay_data::ReplayData;
pub(crate) use rubato_types::skin_type::SkinType;

pub static TIME_MARGIN: i64 = 5000;

/// Key state flags for replay mode.
/// Corresponds to Java `main.getInputProcessor().getKeyState(N)` checks.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReplayKeyState {
    /// Key1 held: replay pattern mode (copy options + seeds + rand)
    pub pattern_key: bool,
    /// Key2 held: replay option mode (copy options only, no seeds)
    pub option_key: bool,
    /// Key4 held: replay HS option mode (save replay config)
    pub hs_key: bool,
    /// Key3 held: gauge shift +2
    pub gauge_shift_key3: bool,
    /// Key5 held: gauge shift +1
    pub gauge_shift_key5: bool,
}

/// Result of replay data restoration.
#[derive(Clone, Debug)]
pub struct ReplayRestoreResult {
    /// Whether the player should remain in REPLAY mode.
    /// If false, playmode should be switched to PLAY.
    pub stay_replay: bool,
    /// The replay data to use for keylog playback (None if switched to PLAY mode).
    pub replay: Option<ReplayData>,
    /// HS replay config to apply (from Key4 held).
    pub hs_replay_config: Option<PlayConfig>,
}

/// Result of frequency trainer application.
#[derive(Clone, Debug)]
pub struct FreqTrainerResult {
    /// Whether frequency training is active.
    pub freq_on: bool,
    /// Formatted frequency string (e.g., "[1.50x]").
    pub freq_string: String,
    /// Whether IR score submission should be blocked.
    pub force_no_ir_send: bool,
    /// Global audio pitch to set (Some if freq_option == FREQUENCY).
    pub global_pitch: Option<f32>,
}

/// Action the caller should take to configure the input processor after create().
///
/// Translated from: BMSPlayer.create() Java lines 526-531
/// ```java
/// if (autoplay.mode == PLAY || autoplay.mode == PRACTICE) {
///     input.setPlayConfig(config.getPlayConfig(model.getMode()));
/// } else if (autoplay.mode == AUTOPLAY || autoplay.mode == REPLAY) {
///     input.setEnable(false);
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputModeAction {
    /// PLAY or PRACTICE mode: caller should call `input.set_play_config(mode)` with the
    /// BMS model mode.
    SetPlayConfig(Mode),
    /// AUTOPLAY or REPLAY mode: caller should call `input.set_enable(false)`.
    DisableInput,
    /// No action needed (play mode not set on BMSPlayer).
    None,
}

/// Side effects produced by `BMSPlayer::create()` that the caller must apply
/// to external systems (audio processor, input processor).
///
/// Since `create()` is a `MainState` trait method taking only `&mut self`,
/// it cannot directly access the audio driver or input processor. Instead,
/// it populates this struct and the caller retrieves it via
/// `take_create_side_effects()`.
///
/// Guide SE path resolution:
///   The caller should use `BMSPlayer::build_guide_se_config(is_guide_se, sound_manager)`
///   to resolve the actual file paths, then apply them to the audio driver.
#[derive(Clone, Debug)]
pub struct CreateSideEffects {
    /// Whether guide SE is enabled. The caller should resolve paths via
    /// `build_guide_se_config()` using the SystemSoundManager.
    pub is_guide_se: bool,

    /// Input processor mode action to apply.
    pub input_mode_action: InputModeAction,

    /// Skin type to load (if determined from the model).
    pub skin_type: Option<SkinType>,
}

/// Play state machine states for BMSPlayer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayState {
    Preload = 0,
    Practice = 1,
    PracticeFinished = 2,
    Ready = 3,
    Play = 4,
    Failed = 5,
    Finished = 6,
    Aborted = 7,
}

// SkinProperty timer constants used in BMSPlayer
pub(crate) use rubato_types::timer_id::TimerId;

const TIMER_STARTINPUT: TimerId = TimerId(1);
const TIMER_FADEOUT: TimerId = TimerId(2);
const TIMER_FAILED: TimerId = TimerId(3);
const TIMER_READY: TimerId = TimerId(40);
const TIMER_PLAY: TimerId = TimerId(41);
const TIMER_GAUGE_MAX_1P: TimerId = TimerId(44);
const TIMER_FULLCOMBO_1P: TimerId = TimerId(48);
const TIMER_RHYTHM: TimerId = TimerId(140);
const TIMER_ENDOFNOTE_1P: TimerId = TimerId(143);
const TIMER_SCORE_A: TimerId = TimerId(348);
const TIMER_SCORE_AA: TimerId = TimerId(349);
const TIMER_SCORE_AAA: TimerId = TimerId(350);
const TIMER_SCORE_BEST: TimerId = TimerId(351);
const TIMER_SCORE_TARGET: TimerId = TimerId(352);
const TIMER_PM_CHARA_1P_NEUTRAL: TimerId = TimerId(900);
const TIMER_PM_CHARA_2P_NEUTRAL: TimerId = TimerId(905);
const TIMER_PM_CHARA_2P_BAD: TimerId = TimerId(907);
const TIMER_MUSIC_END: TimerId = TimerId(908);
const TIMER_PM_CHARA_DANCE: TimerId = TimerId(909);

/// Pending side-effect requests produced during BMSPlayer render/state transitions.
///
/// Consumed by MainController each frame via the corresponding `take_*` / `drain_*` methods.
pub struct PendingActions {
    /// Pending state change to request from MainController.
    pub pending_state_change: Option<MainStateType>,
    /// Pending system sound requests.
    pub pending_sounds: Vec<(rubato_types::sound_type::SoundType, bool)>,
    /// Pending score handoff for Result state.
    pub pending_score_handoff: Option<rubato_types::score_handoff::ScoreHandoff>,
    /// Pending BMS file reload request (for quick retry).
    pub pending_reload_bms: bool,
    /// Pending global pitch to apply to the audio driver.
    pub pending_global_pitch: Option<f32>,
    /// When true, stop all currently playing keysound notes via `audio.stop_note(None)`.
    ///
    /// Set at Failed/Aborted transitions to match Java `main.getAudioProcessor().stop(null)`.
    pub pending_stop_all_notes: bool,
    /// Pending play config update to push back to MainController's PlayerConfig.
    ///
    /// In Java, BMSPlayer writes directly to `main.getPlayerConfig()` (shared reference).
    /// In Rust, BMSPlayer owns a clone, so save_config() changes are local-only unless
    /// we push them back via this outbox field.
    pub pending_play_config_update: Option<(Mode, PlayConfig)>,
    /// Pending keysound play commands from JudgeManager.
    /// Each entry is a (Note, volume) pair resolved from JudgeNote indices.
    /// Consumed by `sync_audio_impl` via `AudioDriver::play_note(note, volume, 0)`.
    pub pending_keysound_plays: Vec<(Note, f32)>,
    /// Pending keysound volume-set commands from JudgeManager.
    /// Each entry is a (Note, volume) pair resolved from JudgeNote indices.
    /// Consumed by `sync_audio_impl` via `AudioDriver::set_volume_note(note, volume)`.
    pub pending_keysound_volume_sets: Vec<(Note, f32)>,
    /// When true, reset `resource.replay.randomoptionseed` to -1 before BMS reload.
    ///
    /// Set during quick retry (Failed/Aborted) when the player presses START or when
    /// assist mode is active. Causes the next play to re-randomize the chart pattern.
    /// Translated from: Java `resource.getReplayData().randomoptionseed = -1`
    pub pending_replay_seed_reset: bool,
    /// Pending score data to store on PlayerResource during quick retry (SELECT key).
    ///
    /// Unlike the full `pending_score_handoff`, this only sets `resource.score_data`
    /// without modifying combo, gauge, or replay fields.
    /// Translated from: Java `resource.setScoreData(createScoreData())` in quick retry.
    pub pending_quick_retry_score: Option<rubato_types::score_data::ScoreData>,
    /// Pending replay data to store on PlayerResource during quick retry (SELECT key).
    ///
    /// Preserves the current session's lane_shuffle_pattern and randomoptionseed so the
    /// next play session inherits the correct replay data instead of stale data.
    /// Built via `build_replay_data()` in the SELECT quick-retry path.
    pub pending_quick_retry_replay: Option<rubato_types::replay_data::ReplayData>,
    /// Pending audio config update to propagate volume changes to MainController.
    ///
    /// Set by PlayMouseContext when volume sliders (set_float_value IDs 17-19) or
    /// notify_audio_config_changed() are called. Drained by MainController each frame
    /// via take_pending_audio_config().
    pub pending_audio_config: Option<rubato_types::audio_config::AudioConfig>,
    /// Pending audio path play requests from skin scripts (audio_play).
    ///
    /// Each entry is (path, volume, is_loop). Drained by MainController each frame
    /// via drain_pending_audio_path_plays().
    pub pending_audio_path_plays: Vec<(String, f32, bool)>,
    /// Pending audio path stop requests from skin scripts (audio_stop).
    ///
    /// Each entry is a path string. Drained by MainController each frame
    /// via drain_pending_audio_path_stops().
    pub pending_audio_path_stops: Vec<String>,
}

impl PendingActions {
    pub fn new() -> Self {
        Self {
            pending_state_change: None,
            pending_sounds: Vec::new(),
            pending_score_handoff: None,
            pending_reload_bms: false,
            pending_global_pitch: None,
            pending_stop_all_notes: false,
            pending_play_config_update: None,
            pending_keysound_plays: Vec::new(),
            pending_keysound_volume_sets: Vec::new(),
            pending_replay_seed_reset: false,
            pending_quick_retry_score: None,
            pending_quick_retry_replay: None,
            pending_audio_config: None,
            pending_audio_path_plays: Vec::new(),
            pending_audio_path_stops: Vec::new(),
        }
    }
}

impl Default for PendingActions {
    fn default() -> Self {
        Self::new()
    }
}

/// Input state snapshot copied from BMSPlayerInputProcessor each frame.
///
/// Updated by the caller before calling render(). Contains both key/button
/// states and controller-specific analog input data.
pub struct PlayerInputState {
    pub keyinput: Option<KeyInputProccessor>,
    pub control: Option<ControlInputProcessor>,
    pub input_start_pressed: bool,
    pub input_select_pressed: bool,
    pub input_key_states: Vec<bool>,
    pub control_key_up: bool,
    pub control_key_down: bool,
    pub control_key_left: bool,
    pub control_key_right: bool,
    pub control_key_escape_pressed: bool,
    pub control_key_num1: bool,
    pub control_key_num2: bool,
    pub control_key_num3: bool,
    pub control_key_num4: bool,
    pub input_scroll: i32,
    pub input_key_changed_times: Vec<i64>,
    pub input_is_analog: Vec<bool>,
    pub input_analog_diff_ticks: Vec<i32>,
    pub input_analog_recent_ms: Vec<i64>,
    pub pending_analog_resets: Vec<usize>,
    /// Pending start time to set on BMSPlayerInputProcessor (enables key logging).
    /// Set when transitioning to PlayState::Play; consumed by sync_input_back_to.
    pub pending_input_start_time: Option<i64>,
    /// Pending key log margin time (in milliseconds) to set on BMSPlayerInputProcessor.
    pub pending_key_log_margin_time: Option<i64>,
}

impl PlayerInputState {
    pub fn new() -> Self {
        Self {
            keyinput: None,
            control: None,
            input_start_pressed: false,
            input_select_pressed: false,
            input_key_states: Vec::new(),
            control_key_up: false,
            control_key_down: false,
            control_key_left: false,
            control_key_right: false,
            control_key_escape_pressed: false,
            control_key_num1: false,
            control_key_num2: false,
            control_key_num3: false,
            control_key_num4: false,
            input_scroll: 0,
            input_key_changed_times: Vec::new(),
            input_is_analog: Vec::new(),
            input_analog_diff_ticks: Vec::new(),
            input_analog_recent_ms: Vec::new(),
            pending_analog_resets: Vec::new(),
            pending_input_start_time: None,
            pending_key_log_margin_time: None,
        }
    }
}

impl Default for PlayerInputState {
    fn default() -> Self {
        Self::new()
    }
}

/// Score, replay, and analysis state for the current play session.
pub struct PlayerScoreState {
    pub playinfo: ReplayData,
    pub replay_config: Option<rubato_types::play_config::PlayConfig>,
    pub active_replay: Option<ReplayData>,
    pub db_score: Option<ScoreData>,
    pub rival_score: Option<ScoreData>,
    pub target_score: Option<ScoreData>,
    pub analysis_result: Option<rubato_audio::bms_loudness_analyzer::AnalysisResult>,
    pub analysis_checked: bool,
}

impl PlayerScoreState {
    pub fn new() -> Self {
        Self {
            playinfo: ReplayData::new(),
            replay_config: None,
            active_replay: None,
            db_score: None,
            rival_score: None,
            target_score: None,
            analysis_result: None,
            analysis_checked: false,
        }
    }
}

impl Default for PlayerScoreState {
    fn default() -> Self {
        Self::new()
    }
}

/// BMS Player main struct
pub struct BMSPlayer {
    model: BMSModel,
    lanerender: Option<LaneRenderer>,
    lane_property: Option<LaneProperty>,
    judge: JudgeManager,
    judge_notes: Vec<bms::model::judge_note::JudgeNote>,
    bga: Arc<Mutex<BGAProcessor>>,
    gauge: Option<GrooveGauge>,
    playtime: i64,
    /// Input state snapshot (keys, buttons, analog, controllers).
    input: PlayerInputState,
    keysound: KeySoundProcessor,
    assist: i32,
    playspeed: i32,
    state: PlayState,
    prevtime: i64,
    practice: PracticeConfiguration,
    starttimeoffset: i64,
    rhythm: Option<RhythmTimerProcessor>,
    startpressedtime: i64,
    adjusted_volume: f32,
    /// Score, replay, and analysis state.
    score: PlayerScoreState,
    /// Gauge log per gauge type
    gaugelog: Vec<Vec<f32>>,
    /// Skin for play screen
    play_skin: PlaySkin,
    /// MainState shared data
    main_state_data: MainStateData,
    /// Margin time in milliseconds (from resource)
    margin_time: i64,
    /// Pending side-effect requests produced during render/state transitions.
    pending: PendingActions,
    /// Fast-forward frequency option (from AudioConfig).
    /// Cached during initialization so set_play_speed can determine
    /// whether to apply pitch changes.
    fast_forward_freq_option: FrequencyType,
    /// BG note volume from AudioConfig.bgvolume.
    /// Used as fallback when adjusted_volume < 0.
    /// Set before create() by the caller.
    bg_volume: f32,
    /// System volume from AudioConfig.systemvolume (0.0-1.0).
    /// Cached for skin property display (integer ID 57, float ID 17).
    system_volume: f32,
    /// Key volume from AudioConfig.keyvolume (0.0-1.0).
    /// Cached for skin property display (integer ID 58, float ID 18).
    key_volume: f32,
    /// Play mode (PLAY, PRACTICE, AUTOPLAY, REPLAY).
    /// Set before create() by the caller. Determines input processor mode.
    play_mode: BMSPlayerMode,
    /// Course constraints (e.g., NO_SPEED). Set before create() by the caller.
    constraints: Vec<CourseDataConstraint>,
    /// Whether guide SE is enabled (from PlayerConfig.is_guide_se).
    /// Set before create() by the caller.
    is_guide_se: bool,
    /// Side effects produced by create() for the caller to apply.
    create_side_effects: Option<CreateSideEffects>,
    /// Player config reference (set before create() by the caller).
    /// Used for save_config, gauge_auto_shift, chart_preview, window_hold.
    player_config: PlayerConfig,
    /// Chart option override from PlayerResource (set before create()).
    chart_option: Option<ReplayData>,
    /// Skin name from header (set during skin loading for score recording).
    skin_name: Option<String>,
    /// Whether media loading has finished (set by the caller via resource.mediaLoadFinished()).
    media_load_finished: bool,
    /// Audio loading progress (0.0-1.0) from AudioDriver::get_progress().
    /// Updated each frame by MainController via update_loading_progress().
    audio_progress: f32,
    /// BGA loading progress (0.0-1.0) from BGAProcessor::progress().
    /// Updated each frame by MainController via update_loading_progress().
    bga_progress: f32,
    /// Whether BGA is enabled for the current song.
    /// Updated each frame by MainController via update_loading_progress().
    bga_enabled: bool,
    /// Whether we are in course mode (resource.getCourseBMSModels() != null).
    /// Set by the caller. Quick retry is disabled during courses.
    is_course_mode: bool,
    /// Input device type (for create_score_data). Set by the caller.
    device_type: rubato_input::bms_player_input_device::DeviceType,
    /// Whether frequency training is active (set when freq != 100 in practice mode).
    freq_on: bool,
    /// Whether IR score submission should be blocked (set when freq != 100 in practice mode).
    force_no_ir_send: bool,
    /// Initial course combo carried from a previous course stage (via PlayerResource).
    /// Set by the caller before create(); applied to JudgeManager in rebuild_judge_system().
    /// Java: judge.init() calls setCourseCombo(resource.getCombo()) when resource.getGauge() != null.
    initial_course_combo: i32,
    /// Initial course max combo carried from a previous course stage (via PlayerResource).
    initial_course_maxcombo: i32,
    /// Original BMS mode before chart-option conversion (e.g. 7key->9key).
    /// Set by the caller from PlayerResource.original_mode() before create().
    /// Used by SkinGauge to adjust parts count for border alignment.
    orgmode: Option<Mode>,
    /// Pre-computed lnmode override from chart data (SongData).
    /// When the chart explicitly defines LN types (has_any_long_note && !has_undefined_long_note),
    /// this overrides the config setting for image_index_value ID 308.
    /// Set by the caller before create() via `set_lnmode_override()`.
    lnmode_override: Option<i32>,
    /// Reverse index mapping JudgeNote index -> (timeline_index, lane) for syncing
    /// judge states back to model notes. Built during `rebuild_judge_system()`.
    judge_note_to_model: Vec<(usize, i32)>,
    /// Previous course stage gauge values for restoring gauge state across stages.
    /// Each inner Vec<f32> is a gauge log for one gauge type; the last element is the
    /// final gauge value from the previous stage.
    /// Java: resource.getGauge() -> FloatArray[] fed into GrooveGauge.create().
    previous_gauge_values: Option<Vec<Vec<f32>>>,
    /// Global config snapshot for skin property queries (e.g., BGA mode for image_index ID 72).
    /// Set by the caller before create() via `set_config()`.
    config: rubato_types::config::Config,
    /// Song metadata for skin string property queries (title, artist, etc.).
    /// Set by the caller before create() via `set_song_metadata()`.
    song_metadata: rubato_types::song_data::SongMetadata,
    /// Song data for boolean skin property queries (chart mode, LN, BGA, difficulty, etc.).
    /// Set by the caller before create() via `set_song_data()`.
    /// Java: SongDataBooleanProperty accesses state.resource.getSongdata().
    song_data: Option<rubato_types::song_data::SongData>,
    /// Player statistics (playcount, clear, judge counts, etc.) from PlayerData.
    /// Set by the caller before create() via `set_player_data()`.
    /// Java: IntegerPropertyFactory reads state.resource.getPlayerData() for IDs 30-37, 333.
    player_data: Option<rubato_types::player_data::PlayerData>,
    /// Cumulative playtime in seconds from PlayerData.
    /// Java: PlayerData.getPlaytime() -- total play time across all sessions.
    /// Set by the caller before create() via `set_cumulative_playtime()`.
    cumulative_playtime_seconds: i64,
    /// Skin offset snapshot from MainController.
    /// Populated by the caller via `set_offset_snapshot()` before create().
    /// Java: MainState inherits MainController which holds offset[].
    offset_snapshot: Vec<rubato_types::skin_offset::SkinOffset>,
    /// Replay key state for replay mode entry.
    /// Set by the caller via `set_replay_key_state()` before `prepare_pattern_pipeline()`.
    /// Used by `restore_replay_data` to determine replay pattern/option/HS mode.
    replay_key_state: ReplayKeyState,
}

mod accessors;
mod input_handling;
mod main_state_impl;
mod pattern;
mod render_skin;
mod scoring;
mod skin_context;
mod snapshot;

#[cfg(test)]
pub(crate) use main_state_impl::pad_gaugelog_with_zeros;

#[cfg(test)]
mod tests;
