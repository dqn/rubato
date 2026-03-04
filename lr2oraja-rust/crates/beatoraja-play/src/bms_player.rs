use std::sync::{Arc, Mutex};

use crate::bga::bga_processor::BGAProcessor;
use crate::control_input_processor::ControlInputProcessor;
use crate::groove_gauge::GrooveGauge;
use crate::judge_manager::JudgeManager;
use crate::key_input_processor::KeyInputProccessor;
use crate::key_sound_processor::KeySoundProcessor;
use crate::lane_property::LaneProperty;
use crate::lane_renderer::LaneRenderer;
use crate::play_skin::PlaySkin;
use crate::practice_configuration::PracticeConfiguration;
use crate::rhythm_timer_processor::RhythmTimerProcessor;
use beatoraja_core::bms_player_mode::BMSPlayerMode;
use beatoraja_core::main_state::{MainState, MainStateData, MainStateType};
use beatoraja_core::pattern::autoplay_modifier::AutoplayModifier;
use beatoraja_core::pattern::extra_note_modifier::ExtraNoteModifier;
use beatoraja_core::pattern::lane_shuffle_modifier::{PlayerBattleModifier, PlayerFlipModifier};
use beatoraja_core::pattern::long_note_modifier::LongNoteModifier;
use beatoraja_core::pattern::mine_note_modifier::MineNoteModifier;
use beatoraja_core::pattern::mode_modifier::ModeModifier;
use beatoraja_core::pattern::pattern_modifier::{AssistLevel, PatternModifier};
use beatoraja_core::pattern::scroll_speed_modifier::ScrollSpeedModifier;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_core::score_data::ScoreData;
use beatoraja_core::timer_manager::TimerManager;
use beatoraja_types::audio_config::FrequencyType;
use beatoraja_types::clear_type::ClearType;
use beatoraja_types::course_data::CourseDataConstraint;
use beatoraja_types::play_config::PlayConfig;
use beatoraja_types::replay_data::ReplayData;
use beatoraja_types::skin_type::SkinType;
use bms_model::bms_model::{BMSModel, LNTYPE_LONGNOTE};
use bms_model::bms_model_utils;
use bms_model::mode::Mode;
use bms_model::note::{Note, TYPE_LONGNOTE, TYPE_UNDEFINED};

pub static TIME_MARGIN: i32 = 5000;

/// Key state flags for replay mode.
/// Corresponds to Java `main.getInputProcessor().getKeyState(N)` checks.
#[derive(Clone, Debug, Default)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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

pub const STATE_PRELOAD: i32 = 0;
pub const STATE_PRACTICE: i32 = 1;
pub const STATE_PRACTICE_FINISHED: i32 = 2;
pub const STATE_READY: i32 = 3;
pub const STATE_PLAY: i32 = 4;
pub const STATE_FAILED: i32 = 5;
pub const STATE_FINISHED: i32 = 6;
pub const STATE_ABORTED: i32 = 7;

// SkinProperty timer constants used in BMSPlayer
const TIMER_STARTINPUT: i32 = 1;
const TIMER_FADEOUT: i32 = 2;
const TIMER_FAILED: i32 = 3;
const TIMER_READY: i32 = 40;
const TIMER_PLAY: i32 = 41;
const TIMER_GAUGE_MAX_1P: i32 = 44;
const TIMER_FULLCOMBO_1P: i32 = 48;
const TIMER_RHYTHM: i32 = 140;
const TIMER_ENDOFNOTE_1P: i32 = 143;
const TIMER_SCORE_A: i32 = 348;
const TIMER_SCORE_AA: i32 = 349;
const TIMER_SCORE_AAA: i32 = 350;
const TIMER_SCORE_BEST: i32 = 351;
const TIMER_SCORE_TARGET: i32 = 352;
const TIMER_PM_CHARA_1P_NEUTRAL: i32 = 900;
const TIMER_PM_CHARA_2P_NEUTRAL: i32 = 905;
const TIMER_PM_CHARA_2P_BAD: i32 = 907;
const TIMER_MUSIC_END: i32 = 908;
const TIMER_PM_CHARA_DANCE: i32 = 909;

/// BMS Player main struct
pub struct BMSPlayer {
    model: BMSModel,
    lanerender: Option<LaneRenderer>,
    lane_property: Option<LaneProperty>,
    judge: JudgeManager,
    bga: Arc<Mutex<BGAProcessor>>,
    gauge: Option<GrooveGauge>,
    playtime: i32,
    keyinput: Option<KeyInputProccessor>,
    control: Option<ControlInputProcessor>,
    keysound: KeySoundProcessor,
    assist: i32,
    playspeed: i32,
    state: i32,
    prevtime: i64,
    practice: PracticeConfiguration,
    starttimeoffset: i64,
    rhythm: Option<RhythmTimerProcessor>,
    startpressedtime: i64,
    adjusted_volume: f32,
    analysis_checked: bool,
    playinfo: ReplayData,
    replay_config: Option<beatoraja_types::play_config::PlayConfig>,
    /// Gauge log per gauge type
    gaugelog: Vec<Vec<f32>>,
    /// Skin for play screen
    play_skin: PlaySkin,
    /// MainState shared data
    main_state_data: MainStateData,
    /// Total notes in song (from songdata)
    total_notes: i32,
    /// Active replay data for keylog playback (set when in REPLAY mode)
    active_replay: Option<ReplayData>,
    /// Margin time in milliseconds (from resource)
    margin_time: i64,
    /// Pending global pitch to apply to the audio driver.
    /// Set by BMSPlayer during state transitions; consumed by the caller.
    /// None means no change requested.
    pending_global_pitch: Option<f32>,
    /// Fast-forward frequency option (from AudioConfig).
    /// Cached during initialization so set_play_speed can determine
    /// whether to apply pitch changes.
    fast_forward_freq_option: FrequencyType,
    /// BG note volume from AudioConfig.bgvolume.
    /// Used as fallback when adjusted_volume < 0.
    /// Set before create() by the caller.
    bg_volume: f32,
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
    /// Analysis result from BMSLoudnessAnalyzer (set by caller via async task).
    analysis_result: Option<beatoraja_audio::bms_loudness_analyzer::AnalysisResult>,
    /// Whether media loading has finished (set by the caller via resource.mediaLoadFinished()).
    media_load_finished: bool,
    /// Input state: START button pressed (from BMSPlayerInputProcessor).
    /// Updated each frame by the caller before calling render().
    input_start_pressed: bool,
    /// Input state: SELECT button pressed (from BMSPlayerInputProcessor).
    /// Updated each frame by the caller before calling render().
    input_select_pressed: bool,
    /// Input state: key states array (from BMSPlayerInputProcessor).
    /// Updated each frame by the caller before calling render().
    input_key_states: Vec<bool>,
    /// Control key states for practice mode navigation (from InputProcessorAccess).
    /// [up, down, left, right] = [Num8, Num2, Num4, Num6]
    /// Updated each frame by the caller before calling render().
    control_key_up: bool,
    control_key_down: bool,
    control_key_left: bool,
    control_key_right: bool,
    /// Pending state change to request from MainController.
    /// Set during render() when a state transition is needed.
    /// The caller should consume this via `take_pending_state_change()`.
    pending_state_change: Option<MainStateType>,
    /// Whether we are in course mode (resource.getCourseBMSModels() != null).
    /// Set by the caller. Quick retry is disabled during courses.
    is_course_mode: bool,
    /// Pending system sound requests. Consumed by MainController via drain_pending_sounds().
    pending_sounds: Vec<(beatoraja_types::sound_type::SoundType, bool)>,
    /// Pending score handoff for Result state. Consumed by MainController.
    pending_score_handoff: Option<beatoraja_types::score_handoff::ScoreHandoff>,
    /// Pending BMS file reload request (for quick retry).
    pending_reload_bms: bool,
    /// Input device type (for create_score_data). Set by the caller.
    device_type: beatoraja_input::bms_player_input_device::DeviceType,
    /// Player's own score data loaded from the score DB.
    /// Set by the caller before create(). Used to initialize ScoreDataProperty.
    /// Java: main.getPlayDataAccessor().readScoreData(model, config.getLnmode())
    db_score: Option<ScoreData>,
    /// Rival score data from PlayerResource.
    /// Set by the caller before create(). Used for target score computation.
    /// Java: resource.getRivalScoreData()
    rival_score: Option<ScoreData>,
    /// Target score data (computed from TargetProperty or rival score).
    /// Set by the caller before create(). The caller is responsible for computing
    /// target via TargetProperty::get_target_property(config.targetid).get_target(main)
    /// when rival_score is None or course mode is active.
    /// Java: TargetProperty.getTargetProperty(config.getTargetid()).getTarget(main)
    target_score: Option<ScoreData>,
}

impl BMSPlayer {
    pub fn new(model: BMSModel) -> Self {
        Self::new_with_resource_gen(model, 1)
    }

    /// Create a BMSPlayer with the given song_resource_gen for BGAProcessor cache sizing.
    /// Java: BGAProcessor(256, Math.max(config.getSongResourceGen(), 1))
    pub fn new_with_resource_gen(model: BMSModel, song_resource_gen: i32) -> Self {
        let playtime = model.get_last_note_time() + TIME_MARGIN;
        let total_notes = model.get_total_notes();
        BMSPlayer {
            model,
            lanerender: None,
            lane_property: None,
            judge: JudgeManager::new(),
            bga: Arc::new(Mutex::new(BGAProcessor::new_with_resource_gen(
                song_resource_gen,
            ))),
            gauge: None,
            playtime,
            keyinput: None,
            control: None,
            keysound: KeySoundProcessor::new(),
            assist: 0,
            playspeed: 100,
            state: STATE_PRELOAD,
            prevtime: 0,
            practice: PracticeConfiguration::new(),
            starttimeoffset: 0,
            rhythm: None,
            startpressedtime: 0,
            adjusted_volume: -1.0,
            analysis_checked: false,
            playinfo: ReplayData::new(),
            replay_config: None,
            gaugelog: Vec::new(),
            play_skin: PlaySkin::new(),
            main_state_data: MainStateData::new(TimerManager::new()),
            total_notes,
            active_replay: None,
            margin_time: 0,
            pending_global_pitch: None,
            fast_forward_freq_option: FrequencyType::UNPROCESSED,
            bg_volume: 0.5,
            play_mode: BMSPlayerMode::PLAY,
            constraints: Vec::new(),
            is_guide_se: false,
            create_side_effects: None,
            player_config: PlayerConfig::default(),
            chart_option: None,
            skin_name: None,
            analysis_result: None,
            media_load_finished: false,
            input_start_pressed: false,
            input_select_pressed: false,
            input_key_states: Vec::new(),
            control_key_up: false,
            control_key_down: false,
            control_key_left: false,
            control_key_right: false,
            pending_state_change: None,
            is_course_mode: false,
            pending_sounds: Vec::new(),
            pending_score_handoff: None,
            pending_reload_bms: false,
            device_type: beatoraja_input::bms_player_input_device::DeviceType::Keyboard,
            db_score: None,
            rival_score: None,
            target_score: None,
        }
    }

    /// Set the BGA processor from PlayerResource for texture cache reuse between plays.
    ///
    /// In Java, `BMSPlayer.create()` calls `bga = resource.getBGAManager()` to reuse the
    /// same BGAProcessor instance (and its texture cache) across plays. Without this,
    /// a fresh BGAProcessor is created every time in `create()`, discarding cached textures.
    ///
    /// The caller (LauncherStateFactory) should extract the processor from PlayerResource
    /// via `get_bga_any()`, downcast to `Arc<Mutex<BGAProcessor>>`, and inject it here.
    /// After `create()`, the processor is stored back via `set_bga_any()`.
    ///
    /// Java: BMSPlayer.java line 545 — `bga = resource.getBGAManager();`
    pub fn set_bga_processor(&mut self, bga: Arc<Mutex<BGAProcessor>>) {
        self.bga = bga;
    }

    /// Get the BGA processor for storing back to PlayerResource after create().
    /// Returns the Arc so the caller can store it for reuse in subsequent plays.
    pub fn get_bga_processor_arc(&self) -> Arc<Mutex<BGAProcessor>> {
        Arc::clone(&self.bga)
    }

    /// Set the chart option override (from PlayerResource) before calling create().
    pub fn set_chart_option(&mut self, chart_option: Option<ReplayData>) {
        self.chart_option = chart_option;
    }

    /// Set the skin name (from skin header) for score recording.
    pub fn set_skin_name(&mut self, name: Option<String>) {
        self.skin_name = name;
    }

    /// Set the loudness analysis result (from async task on PlayerResource).
    pub fn set_analysis_result(
        &mut self,
        result: Option<beatoraja_audio::bms_loudness_analyzer::AnalysisResult>,
    ) {
        self.analysis_result = result;
    }

    /// Set the play mode before calling create().
    ///
    /// Determines how the input processor will be configured:
    /// - PLAY/PRACTICE: input.set_play_config(mode)
    /// - AUTOPLAY/REPLAY: input.set_enable(false)
    pub fn set_play_mode(&mut self, play_mode: BMSPlayerMode) {
        self.play_mode = play_mode;
    }

    /// Get the current play mode.
    pub fn get_play_mode(&self) -> &BMSPlayerMode {
        &self.play_mode
    }

    /// Set course constraints before calling create().
    ///
    /// When NO_SPEED is present, control input (speed changes) will be disabled.
    pub fn set_constraints(&mut self, constraints: Vec<CourseDataConstraint>) {
        self.constraints = constraints;
    }

    /// Get course constraints.
    pub fn get_constraints(&self) -> &[CourseDataConstraint] {
        &self.constraints
    }

    /// Set whether guide SE is enabled before calling create().
    ///
    /// This comes from PlayerConfig.is_guide_se.
    pub fn set_guide_se(&mut self, enabled: bool) {
        self.is_guide_se = enabled;
    }

    /// Set the player config. Used for save_config, gauge_auto_shift, chart_preview, etc.
    pub fn set_player_config(&mut self, config: PlayerConfig) {
        self.player_config = config;
    }

    /// Get the player config reference.
    pub fn get_player_config(&self) -> &PlayerConfig {
        &self.player_config
    }

    /// Set whether media loading has finished.
    /// Called by the caller when resource.mediaLoadFinished() becomes true.
    pub fn set_media_load_finished(&mut self, finished: bool) {
        self.media_load_finished = finished;
    }

    /// Update input state from BMSPlayerInputProcessor each frame.
    pub fn set_input_state(
        &mut self,
        start_pressed: bool,
        select_pressed: bool,
        key_states: &[bool],
    ) {
        self.input_start_pressed = start_pressed;
        self.input_select_pressed = select_pressed;
        self.input_key_states.clear();
        self.input_key_states.extend_from_slice(key_states);
    }

    /// Update control key states for practice mode navigation.
    /// [up, down, left, right] maps to numpad [Num8, Num2, Num4, Num6].
    pub fn set_control_key_state(&mut self, up: bool, down: bool, left: bool, right: bool) {
        self.control_key_up = up;
        self.control_key_down = down;
        self.control_key_left = left;
        self.control_key_right = right;
    }

    /// Take the pending state change (if any). Returns None if no transition is pending.
    /// The caller should apply this via main.changeState().
    pub fn take_pending_state_change(&mut self) -> Option<MainStateType> {
        self.pending_state_change.take()
    }

    /// Set whether we are in course mode.
    pub fn set_course_mode(&mut self, is_course: bool) {
        self.is_course_mode = is_course;
    }

    /// Set the input device type (for create_score_data).
    pub fn set_device_type(
        &mut self,
        device_type: beatoraja_input::bms_player_input_device::DeviceType,
    ) {
        self.device_type = device_type;
    }

    /// Queue a system sound to be played by MainController.
    fn queue_sound(&mut self, sound: beatoraja_types::sound_type::SoundType) {
        self.pending_sounds.push((sound, false));
    }

    /// Take the side effects produced by create().
    ///
    /// Returns None if create() has not been called or side effects have already been taken.
    /// The caller should apply these to the audio processor and input processor.
    pub fn take_create_side_effects(&mut self) -> Option<CreateSideEffects> {
        self.create_side_effects.take()
    }

    /// Set the fast-forward frequency option for pitch control.
    /// Should be called during initialization from AudioConfig.
    pub fn set_fast_forward_freq_option(&mut self, freq_option: FrequencyType) {
        self.fast_forward_freq_option = freq_option;
    }

    /// Set the BG note volume from AudioConfig.bgvolume.
    /// Should be called during initialization.
    pub fn set_bg_volume(&mut self, volume: f32) {
        self.bg_volume = volume;
    }

    /// Set play speed and optionally request global pitch change.
    ///
    /// Translated from: BMSPlayer.setPlaySpeed(int) + audio pitch logic (Java line 946)
    ///
    /// When `fast_forward_freq_option` is `FREQUENCY`, sets a pending global pitch for
    /// the audio driver. The caller should check `take_pending_global_pitch()` after calling this.
    pub fn set_play_speed(&mut self, playspeed: i32) {
        self.playspeed = playspeed;
        // In Java: if (config.getAudioConfig().getFastForward() == FrequencyType.FREQUENCY)
        //     main.getAudioProcessor().setGlobalPitch(playspeed / 100f);
        if self.fast_forward_freq_option == FrequencyType::FREQUENCY {
            self.pending_global_pitch = Some(playspeed as f32 / 100.0);
        }
    }

    pub fn get_play_speed(&self) -> i32 {
        self.playspeed
    }

    pub fn get_keyinput(&mut self) -> Option<&mut KeyInputProccessor> {
        self.keyinput.as_mut()
    }

    pub fn get_state(&self) -> i32 {
        self.state
    }

    pub fn get_adjusted_volume(&self) -> f32 {
        self.adjusted_volume
    }

    /// Drain pending BG note commands from the autoplay thread.
    ///
    /// The caller should call `AudioDriver::play_note(note, volume, 0)` for each
    /// returned command. This should be called each frame from the main render loop.
    pub fn drain_pending_bg_notes(&self) -> Vec<crate::key_sound_processor::BgNoteCommand> {
        self.keysound.drain_pending_bg_notes()
    }

    pub fn get_lanerender(&self) -> Option<&LaneRenderer> {
        self.lanerender.as_ref()
    }

    pub fn get_lanerender_mut(&mut self) -> Option<&mut LaneRenderer> {
        self.lanerender.as_mut()
    }

    pub fn get_lane_property(&self) -> Option<&LaneProperty> {
        self.lane_property.as_ref()
    }

    pub fn get_judge_manager(&self) -> &JudgeManager {
        &self.judge
    }

    pub fn get_judge_manager_mut(&mut self) -> &mut JudgeManager {
        &mut self.judge
    }

    pub fn get_gauge(&self) -> Option<&GrooveGauge> {
        self.gauge.as_ref()
    }

    pub fn get_gauge_mut(&mut self) -> Option<&mut GrooveGauge> {
        self.gauge.as_mut()
    }

    /// Get a shared reference to the BGA processor.
    /// Used by the skin system to connect the SkinBgaObject for BGA rendering.
    pub fn get_bga_processor(&self) -> &Arc<Mutex<BGAProcessor>> {
        &self.bga
    }

    /// Set the active replay data for keylog playback.
    /// Should be called when entering REPLAY mode after restore_replay_data().
    pub fn set_active_replay(&mut self, replay: Option<ReplayData>) {
        self.active_replay = replay;
    }

    /// Set the margin time in milliseconds (from resource).
    pub fn set_margin_time(&mut self, margin_time: i64) {
        self.margin_time = margin_time;
    }

    /// Set the player's own score data loaded from the score database.
    ///
    /// The caller should read this via `MainControllerAccess::read_score_data_by_hash()`
    /// using the model's SHA256 hash, has-undefined-LN flag, and lnmode from PlayerConfig.
    /// This is used in `create()` to initialize `ScoreDataProperty` with the player's
    /// best score and ghost data.
    ///
    /// Java: `main.getPlayDataAccessor().readScoreData(model, config.getLnmode())`
    pub fn set_db_score(&mut self, score: Option<ScoreData>) {
        self.db_score = score;
    }

    /// Set the rival score data from PlayerResource.
    ///
    /// The caller should read this from `PlayerResourceAccess::get_rival_score_data()`.
    /// When rival score is available and not in course mode, it will be used as the
    /// target score in `create()`.
    ///
    /// Java: `resource.getRivalScoreData()`
    pub fn set_rival_score(&mut self, score: Option<ScoreData>) {
        self.rival_score = score;
    }

    /// Set the target score data computed from TargetProperty.
    ///
    /// The caller should compute this via
    /// `TargetProperty::get_target_property(config.targetid).get_target(main)`
    /// when rival score is None or when in course mode.
    /// If rival score is set and not in course mode, this field is ignored
    /// (rival score is used as the target instead).
    ///
    /// Java: `TargetProperty.getTargetProperty(config.getTargetid()).getTarget(main)`
    pub fn set_target_score(&mut self, score: Option<ScoreData>) {
        self.target_score = score;
    }

    /// Take the pending global pitch value, if any.
    /// After calling this, the pending value is cleared (consumed).
    /// The caller should apply the returned pitch to the audio driver.
    pub fn take_pending_global_pitch(&mut self) -> Option<f32> {
        self.pending_global_pitch.take()
    }

    /// Apply loudness analysis result to compute the adjusted volume.
    ///
    /// Translated from: BMSPlayer.render() STATE_PRELOAD loudness check (Java lines 614-641)
    ///
    /// When called, sets `adjusted_volume` based on the analysis result.
    /// Returns the adjusted volume (or -1.0 if analysis failed).
    pub fn apply_loudness_analysis(
        &mut self,
        analysis_result: &beatoraja_audio::bms_loudness_analyzer::AnalysisResult,
        config_key_volume: f32,
    ) -> f32 {
        self.analysis_checked = true;
        if analysis_result.success {
            self.adjusted_volume = analysis_result.calculate_adjusted_volume(config_key_volume);
            log::info!(
                "Volume set to {} ({} LUFS)",
                self.adjusted_volume,
                analysis_result.loudness_lufs
            );
        } else {
            self.adjusted_volume = -1.0;
            if let Some(ref msg) = analysis_result.error_message {
                log::warn!("Loudness analysis failed: {}", msg);
            }
        }
        self.adjusted_volume
    }

    /// Check if loudness analysis has been applied.
    pub fn is_analysis_checked(&self) -> bool {
        self.analysis_checked
    }

    pub fn get_practice_configuration(&self) -> &PracticeConfiguration {
        &self.practice
    }

    pub fn get_practice_configuration_mut(&mut self) -> &mut PracticeConfiguration {
        &mut self.practice
    }

    /// Corresponds to Java BMSPlayer.stopPlay()
    pub fn stop_play(&mut self) {
        // if main.hasObsListener() { main.getObsListener().triggerPlayEnded(); }
        if self.state == STATE_PRACTICE {
            self.practice.save_property();
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
            self.state = STATE_PRACTICE_FINISHED;
            return;
        }
        if self.state == STATE_PRELOAD || self.state == STATE_READY {
            self.pending_global_pitch = Some(1.0);
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
            if self.play_mode.mode == beatoraja_core::bms_player_mode::Mode::Play {
                self.state = STATE_ABORTED;
            } else {
                self.state = STATE_PRACTICE_FINISHED;
            }
            return;
        }
        if self.main_state_data.timer.is_timer_on(TIMER_FAILED)
            || self.main_state_data.timer.is_timer_on(TIMER_FADEOUT)
        {
            return;
        }
        if self.state != STATE_FINISHED
            && !self.is_course_mode
            && self.judge.get_judge_count(0)
                + self.judge.get_judge_count(1)
                + self.judge.get_judge_count(2)
                + self.judge.get_judge_count(3)
                == 0
        {
            // No notes judged and not in course mode - abort
            if let Some(ref mut keyinput) = self.keyinput {
                keyinput.stop_judge();
            }
            self.keysound.stop_bg_play();
            // if resource.mediaLoadFinished() { main.getAudioProcessor().stop(null); }
            self.state = STATE_ABORTED;
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
            return;
        }
        if self.state != STATE_FINISHED
            && (self.judge.get_past_notes() == self.total_notes
                || self.play_mode.mode == beatoraja_core::bms_player_mode::Mode::Autoplay)
        {
            self.state = STATE_FINISHED;
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
            log::info!("STATE_FINISHED");
        } else if self.state == STATE_FINISHED
            && !self.main_state_data.timer.is_timer_on(TIMER_FADEOUT)
        {
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
        } else if self.state != STATE_FINISHED {
            self.pending_global_pitch = Some(1.0);
            self.state = STATE_FAILED;
            self.main_state_data.timer.set_timer_on(TIMER_FAILED);
            // if resource.mediaLoadFinished() { main.getAudioProcessor().stop(null); }
            self.queue_sound(beatoraja_types::sound_type::SoundType::PlayStop);
            log::info!("STATE_FAILED");
        }
    }

    /// Corresponds to Java BMSPlayer.createScoreData()
    ///
    /// `device_type` comes from `MainController.get_input_processor().get_device_type()`.
    pub fn create_score_data(
        &self,
        device_type: beatoraja_input::bms_player_input_device::DeviceType,
    ) -> Option<ScoreData> {
        let mut score = self.judge.get_score_data().clone();

        // If not in course mode and not aborted, check if any notes were hit
        if !self.is_course_mode
            && self.state != STATE_ABORTED
            && (score.epg
                + score.lpg
                + score.egr
                + score.lgr
                + score.egd
                + score.lgd
                + score.ebd
                + score.lbd
                == 0)
        {
            return None;
        }

        let mut clear = ClearType::Failed;
        if self.state != STATE_FAILED
            && let Some(ref gauge) = self.gauge
            && gauge.is_qualified()
        {
            if self.assist > 0 {
                if !self.is_course_mode {
                    clear = if self.assist == 1 {
                        ClearType::LightAssistEasy
                    } else {
                        ClearType::AssistEasy
                    };
                }
            } else if self.judge.get_past_notes() == self.judge.get_combo() {
                if self.judge.get_judge_count(2) == 0 {
                    if self.judge.get_judge_count(1) == 0 {
                        clear = ClearType::Max;
                    } else {
                        clear = ClearType::Perfect;
                    }
                } else {
                    clear = ClearType::FullCombo;
                }
            } else if !self.is_course_mode {
                clear = gauge.get_clear_type();
            }
        }
        score.clear = clear.id();
        if let Some(ref gauge) = self.gauge {
            score.gauge = if gauge.is_type_changed() {
                -1
            } else {
                gauge.get_type()
            };
        }
        score.option = self.encode_option_for_score();
        score.seed = self.encode_seed_for_score();
        let ghost: Vec<i32> = self.judge.get_ghost().to_vec();
        score.encode_ghost(Some(&ghost));

        score.passnotes = self.judge.get_past_notes();
        score.minbp = score.ebd
            + score.lbd
            + score.epr
            + score.lpr
            + score.ems
            + score.lms
            + self.total_notes
            - self.judge.get_past_notes();

        // Timing statistics (Java BMSPlayer.createScoreData() lines 1053-1094)
        let mut avgduration: i64 = 0;
        let mut average: i64 = 0;
        let mut play_times: Vec<i64> = Vec::new();
        let lanes = self.model.get_mode().map(|m| m.key()).unwrap_or(0);
        for tl in self.model.get_all_time_lines() {
            for i in 0..lanes {
                if let Some(note) = tl.get_note(i) {
                    let include = match note {
                        Note::Normal(_) => true,
                        Note::Long { end, note_type, .. } => {
                            let is_ln_end = ((self.model.get_lntype() == LNTYPE_LONGNOTE
                                && *note_type == TYPE_UNDEFINED)
                                || *note_type == TYPE_LONGNOTE)
                                && *end;
                            !is_ln_end
                        }
                        _ => false,
                    };
                    if include {
                        let state = note.get_state();
                        let time = note.get_micro_play_time();
                        if (1..=4).contains(&state) {
                            play_times.push(time);
                            avgduration += time.abs();
                            average += time;
                        }
                    }
                }
            }
        }
        score.total_duration = avgduration;
        score.total_avg = average;
        if !play_times.is_empty() {
            score.avgjudge = avgduration / play_times.len() as i64;
            score.avg = average / play_times.len() as i64;
        }

        let mut stddev: i64 = 0;
        for &time in &play_times {
            let mean_offset = time - score.avg;
            stddev += mean_offset * mean_offset;
        }
        if !play_times.is_empty() {
            stddev = ((stddev / play_times.len() as i64) as f64).sqrt() as i64;
        }
        score.stddev = stddev;

        // Java: score.setDeviceType(main.getInputProcessor().getDeviceType());
        score.device_type = Some(match device_type {
            beatoraja_input::bms_player_input_device::DeviceType::Keyboard => {
                beatoraja_types::stubs::bms_player_input_device::Type::KEYBOARD
            }
            beatoraja_input::bms_player_input_device::DeviceType::BmController => {
                beatoraja_types::stubs::bms_player_input_device::Type::BM_CONTROLLER
            }
            beatoraja_input::bms_player_input_device::DeviceType::Midi => {
                beatoraja_types::stubs::bms_player_input_device::Type::MIDI
            }
        });
        score.skin = self.skin_name.clone();

        Some(score)
    }

    /// Corresponds to Java BMSPlayer.update(int judge, long time)
    pub fn update_judge(&mut self, judge: i32, time: i64) {
        if self.judge.get_combo() == 0 {
            self.bga.lock().unwrap().set_misslayer_tme(time);
        }
        if let Some(ref mut gauge) = self.gauge {
            gauge.update(judge);
        }

        // Full combo check
        let is_fullcombo = self.judge.get_past_notes() == self.total_notes
            && self.judge.get_past_notes() == self.judge.get_combo();
        self.main_state_data
            .timer
            .switch_timer(TIMER_FULLCOMBO_1P, is_fullcombo);

        // Update score data property
        let score_clone = self.judge.get_score_data().clone();
        let past_notes = self.judge.get_past_notes();
        self.main_state_data
            .score
            .update_score_with_notes(Some(&score_clone), past_notes);

        self.main_state_data
            .timer
            .switch_timer(TIMER_SCORE_A, self.main_state_data.score.qualify_rank(18));
        self.main_state_data
            .timer
            .switch_timer(TIMER_SCORE_AA, self.main_state_data.score.qualify_rank(21));
        self.main_state_data
            .timer
            .switch_timer(TIMER_SCORE_AAA, self.main_state_data.score.qualify_rank(24));
        self.main_state_data.timer.switch_timer(
            TIMER_SCORE_BEST,
            self.judge.get_score_data().get_exscore()
                >= self.main_state_data.score.get_best_score(),
        );
        self.main_state_data.timer.switch_timer(
            TIMER_SCORE_TARGET,
            self.judge.get_score_data().get_exscore()
                >= self.main_state_data.score.get_rival_score(),
        );

        self.play_skin.pomyu.pm_chara_judge = judge + 1;
    }

    pub fn is_note_end(&self) -> bool {
        self.judge.get_past_notes() == self.total_notes
    }

    pub fn get_past_notes(&self) -> i32 {
        self.judge.get_past_notes()
    }

    pub fn get_playtime(&self) -> i32 {
        self.playtime
    }

    pub fn get_mode(&self) -> Mode {
        self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K)
    }

    /// Get skin type matching the current model mode.
    /// Corresponds to Java getSkinType() which iterates SkinType.values().
    pub fn get_skin_type(&self) -> Option<SkinType> {
        let model_mode = self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
        SkinType::values()
            .into_iter()
            .find(|&skin_type| skin_type.get_mode() == Some(model_mode.clone()))
    }

    /// Save play config from lane renderer state.
    ///
    /// Corresponds to Java saveConfig() private method.
    /// Persists hispeed/duration, lanecover, lift, hidden from the lane renderer
    /// back into the PlayerConfig's PlayConfig for the current mode.
    fn save_config(&mut self) {
        // 1. Check if NO_SPEED constraint - if so, return early
        for c in &self.constraints {
            if *c == CourseDataConstraint::NoSpeed {
                return;
            }
        }

        // 2. Read lane renderer state
        let lr = match self.lanerender {
            Some(ref lr) => lr,
            None => return,
        };
        let duration = lr.get_duration();
        let hispeed = lr.get_hispeed();
        let lanecover = lr.get_lanecover();
        let lift = lr.get_lift_region();
        let hidden = lr.get_hidden_cover();

        // 3. Get PlayConfig from playerConfig.getPlayConfig(mode).getPlayconfig()
        let mode = self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
        let pc = self
            .player_config
            .get_play_config(mode)
            .get_playconfig_mut();

        // 4. If fixhispeed != OFF: save duration; else save hispeed
        if pc.fixhispeed != beatoraja_types::play_config::FIX_HISPEED_OFF {
            pc.duration = duration;
        } else {
            pc.hispeed = hispeed;
        }

        // 5. Save lanecover, lift, hidden
        pc.lanecover = lanecover;
        pc.lift = lift;
        pc.hidden = hidden;
    }

    /// Initialize playinfo from PlayerConfig.
    ///
    /// Corresponds to Java BMSPlayer constructor lines 110-112:
    /// ```java
    /// playinfo.randomoption = config.getRandom();
    /// playinfo.randomoption2 = config.getRandom2();
    /// playinfo.doubleoption = config.getDoubleoption();
    /// ```
    ///
    /// This should be called before `restore_replay_data` (which may override
    /// these values from replay) and before `build_pattern_modifiers` (which
    /// uses the final values).
    pub fn init_playinfo_from_config(&mut self, config: &PlayerConfig) {
        self.playinfo.randomoption = config.random;
        self.playinfo.randomoption2 = config.random2;
        self.playinfo.doubleoption = config.doubleoption;
    }

    /// Get option information (replay data with random options).
    /// Corresponds to Java getOptionInformation() returning playinfo.
    pub fn get_option_information(&self) -> &ReplayData {
        &self.playinfo
    }

    /// Encode the random seed for ScoreData storage.
    ///
    /// For SP (player=1): returns `playinfo.randomoptionseed`.
    /// For DP (player=2): returns `randomoption2seed * 65536 * 256 + randomoptionseed`.
    ///
    /// Corresponds to Java BMSPlayer line 1029:
    /// `score.setSeed((model.getMode().player == 2 ? playinfo.randomoption2seed * 65536 * 256 : 0) + playinfo.randomoptionseed)`
    pub fn encode_seed_for_score(&self) -> i64 {
        let player_count = self.model.get_mode().map_or(1, |m| m.player());
        if player_count == 2 {
            self.playinfo.randomoption2seed * 65536 * 256 + self.playinfo.randomoptionseed
        } else {
            self.playinfo.randomoptionseed
        }
    }

    /// Encode the random option for ScoreData storage.
    ///
    /// For SP (player=1): returns `playinfo.randomoption`.
    /// For DP (player=2): returns `randomoption + randomoption2 * 10 + doubleoption * 100`.
    ///
    /// Corresponds to Java BMSPlayer line 1027-1028:
    /// `score.setOption(playinfo.randomoption + (model.getMode().player == 2
    ///     ? (playinfo.randomoption2 * 10 + playinfo.doubleoption * 100) : 0))`
    pub fn encode_option_for_score(&self) -> i32 {
        let player_count = self.model.get_mode().map_or(1, |m| m.player());
        if player_count == 2 {
            self.playinfo.randomoption
                + self.playinfo.randomoption2 * 10
                + self.playinfo.doubleoption * 100
        } else {
            self.playinfo.randomoption
        }
    }

    /// Build and apply the pattern modifier chain.
    ///
    /// Corresponds to the pattern modifier section of the Java BMSPlayer constructor
    /// (lines ~303-447). This method:
    /// 1. Applies pre-option modifiers (scroll, LN, mine, extra)
    /// 2. Handles DP battle mode (doubleoption >= 2): converts SP to DP, adds PlayerBattleModifier
    /// 3. Handles DP flip (doubleoption == 1): adds PlayerFlipModifier
    /// 4. Applies 2P random option (DP only)
    /// 5. Applies 1P random option
    /// 6. Handles 7to9 mode
    /// 7. Manages seeds (save/restore from playinfo)
    /// 8. Accumulates assist level
    ///
    /// Returns `true` if score submission is valid (no assist/special options).
    pub fn build_pattern_modifiers(&mut self, config: &PlayerConfig) -> bool {
        let mut score = true;

        // GhostBattle seed/option override (Java lines 119-138)
        let mut ghost_battle = crate::ghost_battle_play::consume();
        if let Some(ref mut gb) = ghost_battle {
            self.playinfo.randomoption = gb.random;
            // Mirror inversion: if player config is MIRROR, flip ghost's option
            const IDENTITY: i32 = 0; // Random::Identity ordinal
            const MIRROR: i32 = 1; // Random::Mirror ordinal
            const RANDOM: i32 = 2; // Random::Random ordinal
            if config.random == MIRROR {
                match gb.random {
                    IDENTITY => self.playinfo.randomoption = MIRROR,
                    MIRROR => self.playinfo.randomoption = IDENTITY,
                    RANDOM => {
                        // Reverse the decimal digit representation of the lane pattern
                        let reversed: i32 = gb
                            .lanes
                            .to_string()
                            .chars()
                            .rev()
                            .collect::<String>()
                            .parse()
                            .unwrap_or(gb.lanes);
                        gb.lanes = reversed;
                    }
                    _ => {}
                }
            }
        } else if let Some(chart_option) = self.chart_option.take() {
            // ChartOption override (Java lines 140-148)
            self.playinfo.randomoption = chart_option.randomoption;
            self.playinfo.randomoptionseed = chart_option.randomoptionseed;
            self.playinfo.randomoption2 = chart_option.randomoption2;
            self.playinfo.randomoption2seed = chart_option.randomoption2seed;
            self.playinfo.doubleoption = chart_option.doubleoption;
            self.playinfo.rand = chart_option.rand;
        }

        // -- Phase 1: Pre-option modifiers (scroll, LN, mine, extra) --
        let mut pre_mods: Vec<Box<dyn PatternModifier>> = Vec::new();

        if config.scroll_mode > 0 {
            pre_mods.push(Box::new(ScrollSpeedModifier::with_params(
                config.scroll_mode - 1,
                config.scroll_section,
                config.scroll_rate,
            )));
        }
        if config.longnote_mode > 0 {
            pre_mods.push(Box::new(LongNoteModifier::with_params(
                config.longnote_mode - 1,
                config.longnote_rate,
            )));
        }
        if config.mine_mode > 0 {
            pre_mods.push(Box::new(MineNoteModifier::with_mode(config.mine_mode - 1)));
        }
        if config.extranote_depth > 0 {
            pre_mods.push(Box::new(ExtraNoteModifier::new(
                config.extranote_type,
                config.extranote_depth,
                config.extranote_scratch,
            )));
        }

        // Apply pre-option modifiers and accumulate assist level
        for m in pre_mods.iter_mut() {
            m.modify(&mut self.model);
            let assist_level = m.get_assist_level();
            if assist_level != AssistLevel::None {
                self.assist = self.assist.max(if assist_level == AssistLevel::Assist {
                    2
                } else {
                    1
                });
                score = false;
            }
        }

        // -- Phase 2: DP battle mode handling (doubleoption >= 2) --
        if self.playinfo.doubleoption >= 2 {
            let mode = self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
            if mode == Mode::BEAT_5K || mode == Mode::BEAT_7K || mode == Mode::KEYBOARD_24K {
                // Convert SP mode to DP mode
                let new_mode = match mode {
                    Mode::BEAT_5K => Mode::BEAT_10K,
                    Mode::BEAT_7K => Mode::BEAT_14K,
                    Mode::KEYBOARD_24K => Mode::KEYBOARD_24K_DOUBLE,
                    _ => unreachable!(),
                };
                self.model.set_mode(new_mode);

                // Apply PlayerBattleModifier
                let mut battle_mod = PlayerBattleModifier::new();
                battle_mod.modify(&mut self.model);

                // If doubleoption == 3, also add AutoplayModifier for scratch keys
                if self.playinfo.doubleoption == 3 {
                    let dp_mode = self.model.get_mode().cloned().unwrap_or(Mode::BEAT_14K);
                    let scratch_keys = dp_mode.scratch_key().to_vec();
                    let mut autoplay_mod = AutoplayModifier::new(scratch_keys);
                    autoplay_mod.modify(&mut self.model);
                }

                self.assist = self.assist.max(1);
                score = false;
                log::info!("Pattern option: BATTLE (L-ASSIST)");
            } else {
                // Not SP mode, so BATTLE is not applied
                self.playinfo.doubleoption = 0;
            }
        }

        // -- Phase 3: Random option modifiers --
        // This section corresponds to Java lines 384-447
        let mode = self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
        let player_count = mode.player();
        let mut pattern_array: Vec<Option<Vec<i32>>> = vec![None; player_count as usize];

        let mut random_mods: Vec<Box<dyn PatternModifier>> = Vec::new();

        // DP option modifiers
        if player_count == 2 {
            if self.playinfo.doubleoption == 1 {
                random_mods.push(Box::new(PlayerFlipModifier::new()));
            }
            log::info!("Pattern option (DP): {}", self.playinfo.doubleoption);

            // 2P random option
            let mut pm2 = beatoraja_core::pattern::pattern_modifier::create_pattern_modifier(
                self.playinfo.randomoption2,
                1,
                &mode,
                config,
            );
            if self.playinfo.randomoption2seed != -1 {
                pm2.set_seed(self.playinfo.randomoption2seed);
            } else {
                self.playinfo.randomoption2seed = pm2.get_seed();
            }
            random_mods.push(pm2);
            log::info!(
                "Pattern option (2P): {}, Seed: {}",
                self.playinfo.randomoption2,
                self.playinfo.randomoption2seed
            );
        }

        // 1P random option
        let mut pm1 = beatoraja_core::pattern::pattern_modifier::create_pattern_modifier(
            self.playinfo.randomoption,
            0,
            &mode,
            config,
        );
        if self.playinfo.randomoptionseed != -1 {
            pm1.set_seed(self.playinfo.randomoptionseed);
        } else {
            // GhostBattle/RandomTrainer seed override requires RandomTrainer::getRandomSeedMap()
            // which lives in beatoraja-modmenu (circular dep). The seed map would need to be
            // passed in as an external dependency when GhostBattle or RandomTrainer is active.
            self.playinfo.randomoptionseed = pm1.get_seed();
        }
        random_mods.push(pm1);
        log::info!(
            "Pattern option (1P): {}, Seed: {}",
            self.playinfo.randomoption,
            self.playinfo.randomoptionseed
        );

        // 7to9 mode
        if config.seven_to_nine_pattern >= 1 && mode == Mode::BEAT_7K {
            let mode_mod = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config.clone());
            random_mods.push(Box::new(mode_mod));
        }

        // Apply all random modifiers
        for m in random_mods.iter_mut() {
            m.modify(&mut self.model);

            let assist_level = m.get_assist_level();
            if assist_level != AssistLevel::None {
                log::info!("Assist pattern option selected");
                self.assist = self.assist.max(if assist_level == AssistLevel::Assist {
                    2
                } else {
                    1
                });
                score = false;
            }

            // Collect lane shuffle patterns for display
            if m.is_lane_shuffle_to_display() {
                let current_mode = self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
                let player_idx = m.get_player() as usize;
                if player_idx < pattern_array.len()
                    && let Some(pattern) = m.get_lane_shuffle_random_pattern(&current_mode)
                {
                    pattern_array[player_idx] = Some(pattern);
                }
            }
        }

        // Store lane shuffle pattern in playinfo
        // Convert Vec<Option<Vec<i32>>> to Option<Vec<Vec<i32>>>
        let has_any_pattern = pattern_array.iter().any(|p| p.is_some());
        if has_any_pattern {
            let patterns: Vec<Vec<i32>> = pattern_array
                .into_iter()
                .map(|p| p.unwrap_or_default())
                .collect();
            self.playinfo.lane_shuffle_pattern = Some(patterns);
        }

        score
    }

    pub fn get_now_quarter_note_time(&self) -> i64 {
        self.rhythm
            .as_ref()
            .map_or(0, |r| r.get_now_quarter_note_time())
    }

    pub fn get_play_skin(&self) -> &PlaySkin {
        &self.play_skin
    }

    pub fn get_play_skin_mut(&mut self) -> &mut PlaySkin {
        &mut self.play_skin
    }

    pub fn get_gaugelog(&self) -> &[Vec<f32>] {
        &self.gaugelog
    }

    /// Restore replay data into playinfo based on key state.
    ///
    /// Corresponds to Java BMSPlayer constructor lines 150-214.
    ///
    /// When in REPLAY mode for a single song:
    /// - If `replay` is `None`: cannot load replay, switch to PLAY mode.
    /// - Key1 held (pattern_key): Copy all pattern options + seeds + rand from replay to playinfo.
    ///   Then switch to PLAY mode (replay pattern mode).
    /// - Key2 held (option_key): Copy pattern options (no seeds, no rand) from replay to playinfo.
    ///   Then switch to PLAY mode (replay option mode).
    /// - Key4 held (hs_key): Save replay's PlayConfig for HS restoration.
    ///   Then switch to PLAY mode.
    /// - If any of the above keys were held, `replay` is discarded and mode becomes PLAY.
    /// - If none of the above keys were held, the replay is kept for keylog playback.
    ///
    /// Returns `ReplayRestoreResult` with whether to stay in replay mode, the replay data,
    /// and any HS config to apply.
    pub fn restore_replay_data(
        &mut self,
        replay: Option<ReplayData>,
        key_state: &ReplayKeyState,
    ) -> ReplayRestoreResult {
        match replay {
            None => {
                // No replay data available -> fall back to PLAY mode
                log::info!("リプレイデータを読み込めなかったため、通常プレイモードに移行");
                ReplayRestoreResult {
                    stay_replay: false,
                    replay: None,
                    hs_replay_config: None,
                }
            }
            Some(replay_data) => {
                let mut is_replay_pattern_play = false;
                let mut hs_config: Option<PlayConfig> = None;

                if key_state.pattern_key {
                    // Replay pattern mode: copy options + seeds + rand
                    log::info!("リプレイ再現モード : 譜面");
                    self.playinfo.randomoption = replay_data.randomoption;
                    self.playinfo.randomoptionseed = replay_data.randomoptionseed;
                    self.playinfo.randomoption2 = replay_data.randomoption2;
                    self.playinfo.randomoption2seed = replay_data.randomoption2seed;
                    self.playinfo.doubleoption = replay_data.doubleoption;
                    self.playinfo.rand = replay_data.rand.clone();
                    is_replay_pattern_play = true;
                } else if key_state.option_key {
                    // Replay option mode: copy options only (no seeds, no rand)
                    log::info!("リプレイ再現モード : オプション");
                    self.playinfo.randomoption = replay_data.randomoption;
                    self.playinfo.randomoption2 = replay_data.randomoption2;
                    self.playinfo.doubleoption = replay_data.doubleoption;
                    is_replay_pattern_play = true;
                }

                if key_state.hs_key {
                    // Replay HS option mode: save replay config
                    log::info!("リプレイ再現モード : ハイスピード");
                    hs_config = replay_data.config.clone();
                    is_replay_pattern_play = true;
                }

                if is_replay_pattern_play {
                    // Switch to PLAY mode, discard replay
                    ReplayRestoreResult {
                        stay_replay: false,
                        replay: None,
                        hs_replay_config: hs_config,
                    }
                } else {
                    // Normal replay mode: keep replay for keylog playback
                    ReplayRestoreResult {
                        stay_replay: true,
                        replay: Some(replay_data),
                        hs_replay_config: None,
                    }
                }
            }
        }
    }

    /// Select the gauge type to use.
    ///
    /// Corresponds to Java BMSPlayer constructor lines 456-466.
    ///
    /// In REPLAY mode with a replay, uses the replay's gauge type.
    /// Additionally, key3/key5 can shift the gauge type upward:
    ///   shift = (key5 ? 1 : 0) + (key3 ? 2 : 0)
    ///   If replay.gauge is not HAZARD or EXHARDCLASS, increment gauge by shift.
    /// In PLAY mode, uses the config gauge type.
    pub fn select_gauge_type(
        replay: Option<&ReplayData>,
        config_gauge: i32,
        key_state: &ReplayKeyState,
    ) -> i32 {
        match replay {
            Some(replay_data) => {
                let mut gauge = replay_data.gauge;
                let shift = (if key_state.gauge_shift_key5 { 1 } else { 0 })
                    + (if key_state.gauge_shift_key3 { 2 } else { 0 });
                for _ in 0..shift {
                    if gauge != beatoraja_types::groove_gauge::HAZARD
                        && gauge != beatoraja_types::groove_gauge::EXHARDCLASS
                    {
                        gauge += 1;
                    }
                }
                gauge
            }
            None => config_gauge,
        }
    }

    /// Handle RANDOM syntax (branch chart loading) for replay/play mode.
    ///
    /// Corresponds to Java BMSPlayer constructor lines 225-242.
    ///
    /// If the model has RANDOM branches:
    /// - In REPLAY mode: use replay.rand
    /// - If resource has a saved seed (randomoptionseed != -1): use resource's rand
    /// - If rand is set and non-empty: reload BMS model with that rand
    ///   (actual model reload deferred → Phase 41)
    /// - Store final model.getRandom() into playinfo.rand
    ///
    /// Returns the rand values to use for model reload (if any), or None.
    pub fn handle_random_syntax(
        &mut self,
        is_replay_mode: bool,
        replay: Option<&ReplayData>,
        resource_replay_seed: i64,
        resource_rand: &[i32],
    ) -> Option<Vec<i32>> {
        let model_random = self.model.get_random().map(|r| r.to_vec());
        if let Some(ref random) = model_random
            && !random.is_empty()
        {
            if is_replay_mode {
                if let Some(replay_data) = replay {
                    self.playinfo.rand = replay_data.rand.clone();
                }
            } else if resource_replay_seed != -1 {
                // This path is hit on MusicResult / QuickRetry
                self.playinfo.rand = resource_rand.to_vec();
            }

            if !self.playinfo.rand.is_empty() {
                // Return rand to the caller for model reload via PlayerResource.
                // Caller should: resource.load_bms_model(rand), then update self.model
                // and self.playinfo.rand = model.get_random().
                log::info!("譜面分岐 : {:?}", self.playinfo.rand);
                return Some(self.playinfo.rand.clone());
            }

            // No rand override, store model's random into playinfo
            self.playinfo.rand = random.clone();
            log::info!("譜面分岐 : {:?}", self.playinfo.rand);
        }
        None
    }

    /// Calculate non-modifier assist flags (BPM guide, custom judge, constant speed).
    ///
    /// Corresponds to Java BMSPlayer constructor lines 269-301.
    /// This method checks assist conditions that are NOT from pattern modifiers:
    /// 1. BPM guide with variable BPM → LightAssist (assist=1)
    /// 2. Custom judge with any window rate > 100 → Assist (assist=2)
    /// 3. Constant speed enabled → Assist (assist=2)
    ///
    /// Accumulates with any existing assist level (e.g., from `build_pattern_modifiers`).
    /// Returns `true` if score submission is still valid (no assist triggered here).
    pub fn calculate_non_modifier_assist(&mut self, config: &PlayerConfig) -> bool {
        let mut score = true;

        // BPM Guide check (Java lines 269-272)
        // BPM変化がなければBPMガイドなし
        if config.bpmguide && (self.model.get_min_bpm() < self.model.get_max_bpm()) {
            self.assist = self.assist.max(1);
            score = false;
        }

        // Custom Judge check (Java lines 275-280)
        if config.custom_judge
            && (config.key_judge_window_rate_perfect_great > 100
                || config.key_judge_window_rate_great > 100
                || config.key_judge_window_rate_good > 100
                || config.scratch_judge_window_rate_perfect_great > 100
                || config.scratch_judge_window_rate_great > 100
                || config.scratch_judge_window_rate_good > 100)
        {
            self.assist = self.assist.max(2);
            score = false;
        }

        // Constant speed check (Java lines 297-301)
        // Constant considered as assist in Endless Dream
        // This is a community discussion result, see https://github.com/seraxis/lr2oraja-endlessdream/issues/42
        let mode = self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
        if config
            .get_play_config_ref(mode)
            .get_playconfig()
            .enable_constant
        {
            self.assist = self.assist.max(2);
            score = false;
        }

        score
    }

    /// Apply frequency trainer speed modification.
    ///
    /// Corresponds to Java BMSPlayer constructor lines 246-267.
    ///
    /// When freq trainer is enabled in PLAY mode (non-course):
    /// 1. Adjusts playtime based on frequency ratio
    /// 2. Scales chart timing via `BMSModelUtils::change_frequency`
    /// 3. Returns result with freq state and optional global pitch
    ///
    /// Returns `None` if freq trainer should not be applied (freq == 100,
    /// not play mode, or course mode).
    pub fn apply_freq_trainer(
        &mut self,
        freq: i32,
        is_play_mode: bool,
        is_course: bool,
        freq_option: &FrequencyType,
    ) -> Option<FreqTrainerResult> {
        if freq == 100 || freq == 0 || !is_play_mode || is_course {
            return None;
        }

        // Adjust playtime: (lastNoteTime + 1000) * 100 / freq + TIME_MARGIN
        self.playtime = (self.model.get_last_note_time() + 1000) * 100 / freq + TIME_MARGIN;

        // Scale chart timing
        bms_model_utils::change_frequency(&mut self.model, freq as f32 / 100.0);

        // Determine global pitch
        let global_pitch = match freq_option {
            FrequencyType::FREQUENCY => Some(freq as f32 / 100.0),
            _ => None,
        };

        // Format freq string (matches Java FreqTrainerMenu.getFreqString())
        let rate = freq as f32 / 100.0;
        let freq_string = format!("[{:.02}x]", rate);

        Some(FreqTrainerResult {
            freq_on: true,
            freq_string,
            force_no_ir_send: true,
            global_pitch,
        })
    }

    /// Get the ClearType override for the current assist level.
    ///
    /// Corresponds to Java BMSPlayer assist → ClearType mapping:
    /// - assist == 0 → None (no override)
    /// - assist == 1 → LightAssistEasy
    /// - assist >= 2 → NoPlay
    pub fn get_clear_type_for_assist(&self) -> Option<ClearType> {
        if self.assist == 0 {
            None
        } else if self.assist == 1 {
            Some(ClearType::LightAssistEasy)
        } else {
            Some(ClearType::NoPlay)
        }
    }

    /// Build guide SE configuration for the audio driver.
    ///
    /// Translated from: BMSPlayer.create() guide SE setup (Java lines 512-524)
    ///
    /// Returns a list of (judge_index, Option<path>) pairs.
    /// When `is_guide_se` is true, each entry contains the resolved path from
    /// `SystemSoundManager::get_sound_paths()` for the corresponding guide SE type.
    /// When false, all entries contain None (clearing the additional key sounds).
    ///
    /// The caller should apply each entry to the audio driver:
    ///   `audio.set_additional_key_sound(judge, true, path);`
    ///   `audio.set_additional_key_sound(judge, false, path);`
    pub fn build_guide_se_config(
        is_guide_se: bool,
        sound_manager: &beatoraja_core::system_sound_manager::SystemSoundManager,
    ) -> Vec<(i32, Option<String>)> {
        use beatoraja_core::system_sound_manager::SoundType;

        let guide_se_types = [
            SoundType::GuidesePg,
            SoundType::GuideseGr,
            SoundType::GuideseGd,
            SoundType::GuideseBd,
            SoundType::GuidesePr,
            SoundType::GuideseMs,
        ];

        let mut config = Vec::with_capacity(6);
        for (i, sound_type) in guide_se_types.iter().enumerate() {
            if is_guide_se {
                let paths = sound_manager.get_sound_paths(sound_type);
                let path = paths.first().map(|p| p.to_string_lossy().to_string());
                config.push((i as i32, path));
            } else {
                config.push((i as i32, None));
            }
        }
        config
    }

    /// Get mutable reference to playinfo for testing.
    #[cfg(test)]
    pub fn playinfo_mut(&mut self) -> &mut ReplayData {
        &mut self.playinfo
    }
}

impl MainState for BMSPlayer {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::Play)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.main_state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.main_state_data
    }

    fn take_pending_state_change(&mut self) -> Option<MainStateType> {
        self.pending_state_change.take()
    }

    fn take_pending_global_pitch(&mut self) -> Option<f32> {
        self.pending_global_pitch.take()
    }

    fn drain_pending_sounds(&mut self) -> Vec<(beatoraja_types::sound_type::SoundType, bool)> {
        std::mem::take(&mut self.pending_sounds)
    }

    fn take_score_handoff(&mut self) -> Option<beatoraja_types::score_handoff::ScoreHandoff> {
        self.pending_score_handoff.take()
    }

    fn take_pending_reload_bms(&mut self) -> bool {
        std::mem::take(&mut self.pending_reload_bms)
    }

    fn receive_reloaded_model(&mut self, model: bms_model::bms_model::BMSModel) {
        self.model = model;
    }

    fn take_bga_cache(&mut self) -> Option<Box<dyn std::any::Any>> {
        // Return the Arc<Mutex<BGAProcessor>> for caching on PlayerResource.
        // The Arc is cloned so that BMSPlayer can still hold a reference
        // (though it will be dropped shortly after during state transition).
        Some(Box::new(Arc::clone(&self.bga)))
    }

    fn create(&mut self) {
        let mode = self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
        self.lane_property = Some(LaneProperty::new(&mode));
        self.judge = JudgeManager::new();
        self.control = Some(ControlInputProcessor::new(mode.clone()));
        if let Some(ref lp) = self.lane_property {
            self.keyinput = Some(KeyInputProccessor::new(lp));
        }

        // --- loadSkin(getSkinType()) ---
        // Translated from: BMSPlayer.create() Java line 510
        // In Java: loadSkin(getSkinType());
        // This delegates to MainState.loadSkin() which calls SkinLoader.load().
        // The actual skin loading requires SkinLoader integration; we call the
        // trait method which logs a warning if not yet wired. The skin type is
        // captured in CreateSideEffects for the caller to use.
        let skin_type = self.get_skin_type();
        if let Some(st) = skin_type {
            self.load_skin(st.id());
        }

        // --- Guide SE setup ---
        // Translated from: BMSPlayer.create() Java lines 512-524
        // The guide SE flag is passed through to CreateSideEffects. The caller
        // should resolve paths using build_guide_se_config(is_guide_se, sound_manager)
        // and apply them to the audio driver.

        // --- Input processor mode setup ---
        // Translated from: BMSPlayer.create() Java lines 526-531
        // ```java
        // if (autoplay.mode == PLAY || autoplay.mode == PRACTICE) {
        //     input.setPlayConfig(config.getPlayConfig(model.getMode()));
        // } else if (autoplay.mode == AUTOPLAY || autoplay.mode == REPLAY) {
        //     input.setEnable(false);
        // }
        // ```
        let input_mode_action = match self.play_mode.mode {
            beatoraja_core::bms_player_mode::Mode::Play
            | beatoraja_core::bms_player_mode::Mode::Practice => {
                InputModeAction::SetPlayConfig(mode)
            }
            beatoraja_core::bms_player_mode::Mode::Autoplay
            | beatoraja_core::bms_player_mode::Mode::Replay => InputModeAction::DisableInput,
        };

        // Store side effects for the caller
        self.create_side_effects = Some(CreateSideEffects {
            is_guide_se: self.is_guide_se,
            input_mode_action,
            skin_type,
        });

        self.lanerender = Some(LaneRenderer::new(&self.model));

        // --- NO_SPEED constraint ---
        // Translated from: BMSPlayer.create() Java lines 533-538
        // ```java
        // for (CourseData.CourseDataConstraint i : resource.getConstraint()) {
        //     if (i == NO_SPEED) { control.setEnableControl(false); break; }
        // }
        // ```
        if self.constraints.contains(&CourseDataConstraint::NoSpeed)
            && let Some(ref mut control) = self.control
        {
            control.set_enable_control(false);
        }

        self.judge.init(&self.model, 0, None, &[]);

        // --- Note expansion rate from PlaySkin ---
        // Translated from: BMSPlayer.create() Java line 542-543
        // ```java
        // rhythm = new RhythmTimerProcessor(model,
        //     (getSkin() instanceof PlaySkin) ? ((PlaySkin) getSkin()).getNoteExpansionRate()[0] != 100
        //         || ((PlaySkin) getSkin()).getNoteExpansionRate()[1] != 100 : false);
        // ```
        let rates = self.play_skin.get_note_expansion_rate();
        let use_expansion = rates[0] != 100 || rates[1] != 100;
        self.rhythm = Some(RhythmTimerProcessor::new(&self.model, use_expansion));

        // Reuse existing BGAProcessor (injected via set_bga_processor from PlayerResource)
        // to preserve the texture cache between plays. Only update timelines for the new model.
        // Java: bga = resource.getBGAManager(); (BMSPlayer.java line 545)
        if let Ok(mut bga) = self.bga.lock() {
            bga.set_model_timelines(&self.model);
        }

        // Initialize gauge log
        if let Some(ref gauge) = self.gauge {
            let gauge_type_len = gauge.get_gauge_type_length();
            self.gaugelog = Vec::with_capacity(gauge_type_len);
            for _ in 0..gauge_type_len {
                self.gaugelog
                    .push(Vec::with_capacity((self.playtime / 500 + 2) as usize));
            }
        }

        // --- Score DB load + target/rival score wiring ---
        // Translated from: BMSPlayer.create() Java lines 547-571
        //
        // ```java
        // ScoreData score = main.getPlayDataAccessor().readScoreData(model, config.getLnmode());
        // if (score == null) { score = new ScoreData(); }
        //
        // if (autoplay.mode == PRACTICE) {
        //     getScoreDataProperty().setTargetScore(0, null, 0, null, model.getTotalNotes());
        //     practice.create(model, main.getConfig());
        //     state = STATE_PRACTICE;
        // } else {
        //     if (resource.getRivalScoreData() == null || resource.getCourseBMSModels() != null) {
        //         ScoreData targetScore = TargetProperty.getTargetProperty(config.getTargetid()).getTarget(main);
        //         resource.setTargetScoreData(targetScore);
        //     } else {
        //         resource.setTargetScoreData(resource.getRivalScoreData());
        //     }
        //     ScoreData target = resource.getTargetScoreData();
        //     getScoreDataProperty().setTargetScore(
        //         score.getExscore(), score.decodeGhost(),
        //         target != null ? target.getExscore() : 0,
        //         target != null ? target.decodeGhost() : null,
        //         model.getTotalNotes());
        // }
        // ```
        //
        // The caller must pre-load db_score, rival_score, and target_score via
        // set_db_score(), set_rival_score(), and set_target_score() before create().
        let score = self.db_score.clone().unwrap_or_default();
        log::info!("Score data loaded from score database");

        let total_notes = self.model.get_total_notes();

        if self.play_mode.mode == beatoraja_core::bms_player_mode::Mode::Practice {
            self.main_state_data
                .score
                .set_target_score_with_ghost(0, None, 0, None, total_notes);
            self.practice.create(&self.model);
            self.state = STATE_PRACTICE;
        } else {
            // Determine the effective target score:
            // - If rival score is absent or in course mode, use the pre-computed target_score
            //   (caller should have computed via TargetProperty::get_target_property().get_target())
            // - Otherwise, use the rival score as the target
            let effective_target = if self.rival_score.is_none() || self.is_course_mode {
                self.target_score.clone()
            } else {
                self.rival_score.clone()
            };

            let (target_exscore, target_ghost) = match effective_target {
                Some(ref t) => (t.get_exscore(), t.decode_ghost()),
                None => (0, None),
            };

            self.main_state_data.score.set_target_score_with_ghost(
                score.get_exscore(),
                score.decode_ghost(),
                target_exscore,
                target_ghost,
                total_notes,
            );
        }
    }

    fn render(&mut self) {
        let micronow = self.main_state_data.timer.get_now_micro_time();

        // Input start timer
        let input_time = self.play_skin.get_loadstart() as i64; // skin.getInput() in Java
        if micronow > input_time * 1000 {
            self.main_state_data
                .timer
                .switch_timer(TIMER_STARTINPUT, true);
        }
        // startpressedtime tracking: update when START or SELECT is pressed
        // Translated from: Java BMSPlayer.render() line 590
        if self.input_start_pressed || self.input_select_pressed {
            self.startpressedtime = micronow;
        }

        match self.state {
            // STATE_PRELOAD - wait for resources
            STATE_PRELOAD => {
                // Chart preview handling
                // Translated from: Java BMSPlayer.render() lines 598-604
                if self.player_config.chart_preview {
                    if self.main_state_data.timer.is_timer_on(141)
                        && micronow > self.startpressedtime
                    {
                        self.main_state_data.timer.set_timer_off(141);
                        if let Some(ref mut lr) = self.lanerender {
                            lr.init(&self.model);
                        }
                    } else if !self.main_state_data.timer.is_timer_on(141)
                        && micronow == self.startpressedtime
                    {
                        self.main_state_data
                            .timer
                            .set_micro_timer(141, micronow - self.starttimeoffset * 1000);
                    }
                }

                // Check if media loaded and load timers elapsed
                let load_threshold =
                    (self.play_skin.get_loadstart() + self.play_skin.get_loadend()) as i64 * 1000;
                // Translated from: Java BMSPlayer.render() lines 607-608
                if self.media_load_finished
                    && micronow > load_threshold
                    && micronow - self.startpressedtime > 1_000_000
                {
                    // Chart preview cleanup on transition
                    if self.player_config.chart_preview {
                        self.main_state_data.timer.set_timer_off(141);
                        if let Some(ref mut lr) = self.lanerender {
                            lr.init(&self.model);
                        }
                    }

                    // Loudness analysis check (Java BMSPlayer.render() lines 615-641)
                    if !self.analysis_checked {
                        self.adjusted_volume = -1.0;
                        self.analysis_checked = true;
                        if let Some(result) = self.analysis_result.take() {
                            let config_key_volume = self.bg_volume;
                            self.apply_loudness_analysis(&result, config_key_volume);
                        }
                    }

                    self.bga.lock().unwrap().prepare(&() as &dyn std::any::Any);
                    self.state = STATE_READY;
                    self.main_state_data.timer.set_timer_on(TIMER_READY);
                    self.queue_sound(beatoraja_types::sound_type::SoundType::PlayReady);
                    log::info!("STATE_READY");
                }
                // PM character neutral timer
                if !self
                    .main_state_data
                    .timer
                    .is_timer_on(TIMER_PM_CHARA_1P_NEUTRAL)
                    || !self
                        .main_state_data
                        .timer
                        .is_timer_on(TIMER_PM_CHARA_2P_NEUTRAL)
                {
                    self.main_state_data
                        .timer
                        .set_timer_on(TIMER_PM_CHARA_1P_NEUTRAL);
                    self.main_state_data
                        .timer
                        .set_timer_on(TIMER_PM_CHARA_2P_NEUTRAL);
                }
            }

            // STATE_PRACTICE - practice mode config
            STATE_PRACTICE => {
                if self.main_state_data.timer.is_timer_on(TIMER_PLAY) {
                    // Reset for practice restart: reload BMS file to get a fresh model
                    // (modifiers mutate the model during play, so we need a clean copy).
                    // Java: resource.reloadBMSFile(); model = resource.getBMSModel();
                    // Rust: pending flag triggers MainController to reload resource and
                    // push fresh model back via receive_reloaded_model().
                    self.pending_reload_bms = true;
                    if let Some(ref mut lr) = self.lanerender {
                        lr.init(&self.model);
                    }
                    if let Some(ref mut ki) = self.keyinput {
                        ki.set_key_beam_stop(false);
                    }
                    self.main_state_data.timer.set_timer_off(TIMER_PLAY);
                    self.main_state_data.timer.set_timer_off(TIMER_RHYTHM);
                    self.main_state_data.timer.set_timer_off(TIMER_FAILED);
                    self.main_state_data.timer.set_timer_off(TIMER_FADEOUT);
                    self.main_state_data.timer.set_timer_off(TIMER_ENDOFNOTE_1P);

                    for i in TIMER_PM_CHARA_1P_NEUTRAL..=TIMER_PM_CHARA_DANCE {
                        self.main_state_data.timer.set_timer_off(i);
                    }
                }
                if !self
                    .main_state_data
                    .timer
                    .is_timer_on(TIMER_PM_CHARA_1P_NEUTRAL)
                    || !self
                        .main_state_data
                        .timer
                        .is_timer_on(TIMER_PM_CHARA_2P_NEUTRAL)
                {
                    self.main_state_data
                        .timer
                        .set_timer_on(TIMER_PM_CHARA_1P_NEUTRAL);
                    self.main_state_data
                        .timer
                        .set_timer_on(TIMER_PM_CHARA_2P_NEUTRAL);
                }
                if let Some(ref mut control) = self.control {
                    control.set_enable_control(false);
                    control.set_enable_cursor(false);
                }
                // Process practice input navigation (UP/DOWN/LEFT/RIGHT)
                // Translated from: Java BMSPlayer.render() line 680
                let now_millis = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                // Control key states are read from input_key_states.
                // In the Java version, these come from BMSPlayerInputProcessor control keys.
                // For now we pass the input_start/select state as a proxy for key0 check.
                self.practice.process_input(
                    self.control_key_up,
                    self.control_key_down,
                    self.control_key_left,
                    self.control_key_right,
                    now_millis,
                );

                // Practice start logic: press key0 while media is loaded and timers elapsed
                // Translated from: Java BMSPlayer.render() lines 682-723
                let key0_pressed = self.input_key_states.first().copied().unwrap_or(false);
                let load_threshold =
                    (self.play_skin.get_loadstart() + self.play_skin.get_loadend()) as i64 * 1000;
                if key0_pressed
                    && self.media_load_finished
                    && micronow > load_threshold
                    && micronow - self.startpressedtime > 1_000_000
                {
                    // Apply practice configuration and start play
                    if let Some(ref mut control) = self.control {
                        control.set_enable_control(true);
                        control.set_enable_cursor(true);
                    }

                    let property = self.practice.get_practice_property().clone();

                    // Apply frequency if != 100
                    if property.freq != 100 {
                        bms_model_utils::change_frequency(
                            &mut self.model,
                            property.freq as f32 / 100.0,
                        );
                        if self.fast_forward_freq_option == FrequencyType::FREQUENCY {
                            self.pending_global_pitch = Some(property.freq as f32 / 100.0);
                        }
                    }

                    self.model.set_total(property.total);

                    // Apply practice modifier (time range)
                    let mut pm = beatoraja_core::pattern::practice_modifier::PracticeModifier::new(
                        property.starttime as i64 * 100 / property.freq as i64,
                        property.endtime as i64 * 100 / property.freq as i64,
                    );
                    pm.modify(&mut self.model);

                    // DP options
                    if self.model.get_mode().map_or(1, |m| m.player()) == 2 {
                        if property.doubleop == 1 {
                            let mut flip =
                                beatoraja_core::pattern::lane_shuffle_modifier::PlayerFlipModifier::new();
                            flip.modify(&mut self.model);
                        }
                        let mut pm2 =
                            beatoraja_core::pattern::pattern_modifier::create_pattern_modifier(
                                property.random2,
                                1,
                                &self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K),
                                &self.player_config,
                            );
                        pm2.modify(&mut self.model);
                    }

                    // 1P random option
                    let mut pm1 =
                        beatoraja_core::pattern::pattern_modifier::create_pattern_modifier(
                            property.random,
                            0,
                            &self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K),
                            &self.player_config,
                        );
                    pm1.modify(&mut self.model);

                    // Gauge, judgerank, lane init
                    self.gauge = self.practice.get_gauge(&self.model);
                    self.model.set_judgerank(property.judgerank);
                    if let Some(ref mut lr) = self.lanerender {
                        lr.init(&self.model);
                    }
                    self.play_skin.pomyu.init();

                    self.starttimeoffset = if property.starttime > 1000 {
                        (property.starttime as i64 - 1000) * 100 / property.freq as i64
                    } else {
                        0
                    };
                    self.playtime = ((property.endtime as i64 + 1000) * 100 / property.freq as i64)
                        as i32
                        + TIME_MARGIN;

                    self.bga.lock().unwrap().prepare(&() as &dyn std::any::Any);
                    self.state = STATE_READY;
                    self.main_state_data.timer.set_timer_on(TIMER_READY);
                    log::info!("Practice -> STATE_READY");
                }
            }

            // STATE_PRACTICE_FINISHED
            // Translated from: Java BMSPlayer.render() lines 726-731
            STATE_PRACTICE_FINISHED => {
                let skin_fadeout = self
                    .main_state_data
                    .skin
                    .as_ref()
                    .map_or(0, |s| s.get_fadeout()) as i64;
                if self
                    .main_state_data
                    .timer
                    .get_now_time_for_id(TIMER_FADEOUT)
                    > skin_fadeout
                {
                    // input.setEnable(true); input.setStartTime(0);
                    self.pending_state_change = Some(MainStateType::MusicSelect);
                    log::info!("Practice finished, transition to MUSICSELECT");
                }
            }

            // STATE_READY - countdown before play
            STATE_READY => {
                if self.main_state_data.timer.get_now_time_for_id(TIMER_READY)
                    > self.play_skin.get_playstart() as i64
                {
                    if let Some(ref lr) = self.lanerender {
                        self.replay_config = Some(lr.get_play_config().clone());
                    }
                    self.state = STATE_PLAY;
                    self.main_state_data
                        .timer
                        .set_micro_timer(TIMER_PLAY, micronow - self.starttimeoffset * 1000);
                    self.main_state_data
                        .timer
                        .set_micro_timer(TIMER_RHYTHM, micronow - self.starttimeoffset * 1000);

                    // input.setStartTime(micronow + timer.getStartMicroTime() - starttimeoffset * 1000);
                    // input.setKeyLogMarginTime(resource.getMarginTime());
                    // Java: keyinput.startJudge(model, replay != null ? replay.keylog : null, resource.getMarginTime())
                    if let Some(ref mut ki) = self.keyinput {
                        let timelines = self.model.get_all_time_lines();
                        let last_tl_micro = timelines.last().map_or(0, |tl| tl.get_micro_time());
                        let keylog = self.active_replay.as_ref().map(|r| r.keylog.as_slice());
                        ki.start_judge(last_tl_micro, keylog, self.margin_time);
                    }
                    // Resolve initial BG volume: use adjusted_volume if >= 0,
                    // otherwise fall back to bg_volume from AudioConfig.
                    let initial_bg_vol = if self.adjusted_volume >= 0.0 {
                        self.adjusted_volume
                    } else {
                        self.bg_volume
                    };
                    self.keysound.start_bg_play(
                        &self.model,
                        self.starttimeoffset * 1000,
                        initial_bg_vol,
                    );
                    log::info!("STATE_PLAY");
                }
            }

            // STATE_PLAY - main gameplay
            STATE_PLAY => {
                let deltatime = micronow - self.prevtime;
                let deltaplay = deltatime.saturating_mul(100 - self.playspeed as i64) / 100;
                let freq = self.practice.get_practice_property().freq;
                let current_play_timer = self.main_state_data.timer.get_micro_timer(TIMER_PLAY);
                self.main_state_data
                    .timer
                    .set_micro_timer(TIMER_PLAY, current_play_timer + deltaplay);

                // Rhythm timer update
                let now_bpm = self
                    .lanerender
                    .as_ref()
                    .map_or(120.0, |lr| lr.get_now_bpm());
                if let Some(ref mut rhythm) = self.rhythm {
                    let play_timer_micro = self
                        .main_state_data
                        .timer
                        .get_now_micro_time_for_id(TIMER_PLAY);
                    let (rhythm_timer, rhythm_on) = rhythm.update(
                        self.main_state_data.timer.get_now_time(),
                        micronow,
                        deltatime,
                        now_bpm,
                        self.playspeed,
                        freq,
                        play_timer_micro,
                    );
                    if rhythm_on {
                        self.main_state_data
                            .timer
                            .set_micro_timer(TIMER_RHYTHM, rhythm_timer);
                    }
                }

                // Update BG autoplay thread: play time and volume.
                // Translated from: Java AutoplayThread.run() reads player.timer.getNowMicroTime(TIMER_PLAY)
                // and player.getAdjustedVolume() / config.getAudioConfig().getBgvolume().
                {
                    let play_micro = self
                        .main_state_data
                        .timer
                        .get_now_micro_time_for_id(TIMER_PLAY);
                    self.keysound.update_play_time(play_micro);
                    let vol = if self.adjusted_volume >= 0.0 {
                        self.adjusted_volume
                    } else {
                        self.bg_volume
                    };
                    self.keysound.update_volume(vol);
                }

                let ptime = self.main_state_data.timer.get_now_time_for_id(TIMER_PLAY);
                // Gauge log
                if let Some(ref gauge) = self.gauge {
                    for i in 0..self.gaugelog.len() {
                        if self.gaugelog[i].len() as i64 <= ptime / 500 {
                            let val = gauge.get_value_by_type(i as i32);
                            self.gaugelog[i].push(val);
                        }
                    }
                    self.main_state_data
                        .timer
                        .switch_timer(TIMER_GAUGE_MAX_1P, gauge.get_gauge().is_max());
                }

                // pomyu timer update
                // Translated from: Java BMSPlayer.render() line 766
                let past_notes = self.judge.get_past_notes();
                let gauge_is_max = self.gauge.as_ref().is_some_and(|g| g.get_gauge().is_max());
                self.play_skin.pomyu.update_timer(
                    &mut self.main_state_data.timer,
                    past_notes,
                    gauge_is_max,
                );

                // Check play time elapsed
                if (self.playtime as i64) < ptime {
                    self.state = STATE_FINISHED;
                    self.main_state_data.timer.set_timer_on(TIMER_MUSIC_END);
                    for i in TIMER_PM_CHARA_1P_NEUTRAL..=TIMER_PM_CHARA_2P_BAD {
                        self.main_state_data.timer.set_timer_off(i);
                    }
                    self.main_state_data
                        .timer
                        .set_timer_off(TIMER_PM_CHARA_DANCE);
                    log::info!("STATE_FINISHED");
                } else if (self.playtime - TIME_MARGIN) as i64 <= ptime {
                    self.main_state_data
                        .timer
                        .switch_timer(TIMER_ENDOFNOTE_1P, true);
                }

                // Stage failed check with gauge auto shift
                // Translated from: Java BMSPlayer.render() lines 782-815
                if let Some(ref mut gauge) = self.gauge {
                    let gas = self.player_config.gauge_auto_shift;
                    use beatoraja_types::groove_gauge::{CLASS, EXHARDCLASS, HAZARD, NORMAL};
                    use beatoraja_types::player_config::{
                        GAUGEAUTOSHIFT_BESTCLEAR, GAUGEAUTOSHIFT_CONTINUE, GAUGEAUTOSHIFT_NONE,
                        GAUGEAUTOSHIFT_SELECT_TO_UNDER, GAUGEAUTOSHIFT_SURVIVAL_TO_GROOVE,
                    };

                    if gas == GAUGEAUTOSHIFT_BESTCLEAR || gas == GAUGEAUTOSHIFT_SELECT_TO_UNDER {
                        // Auto-shift to best qualifying gauge
                        let len = if gas == GAUGEAUTOSHIFT_BESTCLEAR {
                            if gauge.get_type() >= CLASS {
                                EXHARDCLASS + 1
                            } else {
                                HAZARD + 1
                            }
                        } else {
                            // SELECT_TO_UNDER
                            if gauge.is_course_gauge() {
                                (self.player_config.gauge.clamp(NORMAL, EXHARDCLASS) + CLASS
                                    - NORMAL)
                                    .min(EXHARDCLASS)
                                    + 1
                            } else {
                                self.player_config.gauge.min(HAZARD) + 1
                            }
                        };
                        let start_type = if gauge.is_course_gauge() {
                            CLASS
                        } else if gauge.get_type() < self.player_config.bottom_shiftable_gauge {
                            gauge.get_type()
                        } else {
                            self.player_config.bottom_shiftable_gauge
                        };
                        let mut best_type = start_type;
                        for i in start_type..len {
                            if gauge.get_value_by_type(i) > 0.0
                                && gauge.get_gauge_by_type(i).is_qualified()
                            {
                                best_type = i;
                            }
                        }
                        gauge.set_type(best_type);
                    } else if gauge.get_value() == 0.0 {
                        match gas {
                            GAUGEAUTOSHIFT_NONE => {
                                // FAILED transition
                                self.state = STATE_FAILED;
                                self.main_state_data.timer.set_timer_on(TIMER_FAILED);
                                // if resource.mediaLoadFinished() { main.getAudioProcessor().stop(null); }
                                self.queue_sound(beatoraja_types::sound_type::SoundType::PlayStop);
                                log::info!("STATE_FAILED");
                            }
                            GAUGEAUTOSHIFT_CONTINUE => {
                                // Continue playing with 0 gauge
                            }
                            GAUGEAUTOSHIFT_SURVIVAL_TO_GROOVE => {
                                if !gauge.is_course_gauge() {
                                    gauge.set_type(NORMAL);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // STATE_FAILED
            // Translated from: Java BMSPlayer.render() lines 818-869
            STATE_FAILED => {
                if let Some(ref mut control) = self.control {
                    control.set_enable_control(false);
                    control.set_enable_cursor(false);
                }
                if let Some(ref mut ki) = self.keyinput {
                    ki.stop_judge();
                }
                self.keysound.stop_bg_play();

                // Quick retry check (START xor SELECT)
                // Translated from: Java BMSPlayer.render() lines 823-838
                if (self.input_start_pressed ^ self.input_select_pressed)
                    && !self.is_course_mode
                    && self.play_mode.mode == beatoraja_core::bms_player_mode::Mode::Play
                {
                    self.pending_global_pitch = Some(1.0);
                    self.save_config();
                    self.pending_reload_bms = true;
                    self.pending_state_change = Some(MainStateType::Play);
                } else if self.main_state_data.timer.get_now_time_for_id(TIMER_FAILED)
                    > self.play_skin.get_close() as i64
                {
                    self.pending_global_pitch = Some(1.0);
                    // if resource.mediaLoadFinished() { resource.getBGAManager().stop(); }

                    // Fill remaining gauge log with 0
                    if self.main_state_data.timer.is_timer_on(TIMER_PLAY) {
                        let failed_time = self.main_state_data.timer.get_timer(TIMER_FAILED);
                        let play_time = self.main_state_data.timer.get_timer(TIMER_PLAY);
                        let mut l = failed_time - play_time;
                        while l < self.playtime as i64 + 500 {
                            for glog in self.gaugelog.iter_mut() {
                                glog.push(0.0);
                            }
                            l += 500;
                        }
                    }
                    let score = if self.play_mode.mode
                        == beatoraja_core::bms_player_mode::Mode::Play
                        || self.play_mode.mode == beatoraja_core::bms_player_mode::Mode::Replay
                    {
                        self.create_score_data(self.device_type)
                    } else {
                        None
                    };
                    self.pending_score_handoff =
                        Some(beatoraja_types::score_handoff::ScoreHandoff {
                            score_data: score,
                            combo: self.judge.get_course_combo(),
                            maxcombo: self.judge.get_course_maxcombo(),
                            gauge: self.gaugelog.clone(),
                            groove_gauge: self.gauge.clone(),
                            assist: self.assist,
                        });
                    // input.setEnable(true); input.setStartTime(0);
                    self.save_config();

                    // Transition: practice -> STATE_PRACTICE, else -> RESULT or MUSICSELECT
                    if self.play_mode.mode == beatoraja_core::bms_player_mode::Mode::Practice {
                        self.state = STATE_PRACTICE;
                    } else if self
                        .pending_score_handoff
                        .as_ref()
                        .is_some_and(|h| h.score_data.is_some())
                    {
                        self.pending_state_change = Some(MainStateType::Result);
                    } else {
                        self.pending_state_change = Some(MainStateType::MusicSelect);
                    }
                    log::info!("Failed close, transition to result/select");
                }
            }

            // STATE_FINISHED
            // Translated from: Java BMSPlayer.render() lines 872-911
            STATE_FINISHED => {
                if let Some(ref mut control) = self.control {
                    control.set_enable_control(false);
                    control.set_enable_cursor(false);
                }
                if let Some(ref mut ki) = self.keyinput {
                    ki.stop_judge();
                }
                self.keysound.stop_bg_play();

                if self
                    .main_state_data
                    .timer
                    .get_now_time_for_id(TIMER_MUSIC_END)
                    > self.play_skin.get_finish_margin() as i64
                {
                    self.main_state_data.timer.switch_timer(TIMER_FADEOUT, true);
                }
                // skin.getFadeout() from the loaded skin
                let skin_fadeout = self
                    .main_state_data
                    .skin
                    .as_ref()
                    .map_or(0, |s| s.get_fadeout()) as i64;
                if self
                    .main_state_data
                    .timer
                    .get_now_time_for_id(TIMER_FADEOUT)
                    > skin_fadeout
                {
                    self.pending_global_pitch = Some(1.0);
                    // resource.getBGAManager().stop();
                    let score = if self.play_mode.mode
                        == beatoraja_core::bms_player_mode::Mode::Play
                        || self.play_mode.mode == beatoraja_core::bms_player_mode::Mode::Replay
                    {
                        self.create_score_data(self.device_type)
                    } else {
                        None
                    };
                    self.save_config();
                    self.pending_score_handoff =
                        Some(beatoraja_types::score_handoff::ScoreHandoff {
                            score_data: score,
                            combo: self.judge.get_course_combo(),
                            maxcombo: self.judge.get_course_maxcombo(),
                            gauge: self.gaugelog.clone(),
                            groove_gauge: self.gauge.clone(),
                            assist: self.assist,
                        });
                    // input.setEnable(true); input.setStartTime(0);

                    // Transition: practice -> STATE_PRACTICE, else -> RESULT
                    if self.play_mode.mode == beatoraja_core::bms_player_mode::Mode::Practice {
                        self.state = STATE_PRACTICE;
                    } else {
                        self.pending_state_change = Some(MainStateType::Result);
                    }
                    log::info!("Finished, transition to result/select");
                }
            }

            // STATE_ABORTED
            // Translated from: Java BMSPlayer.render() lines 914-936
            STATE_ABORTED => {
                // Quick retry check (START xor SELECT in PLAY mode, not course)
                if self.play_mode.mode == beatoraja_core::bms_player_mode::Mode::Play
                    && (self.input_start_pressed ^ self.input_select_pressed)
                    && !self.is_course_mode
                {
                    self.pending_global_pitch = Some(1.0);
                    self.save_config();
                    self.pending_reload_bms = true;
                    self.pending_state_change = Some(MainStateType::Play);
                }

                // skin.getFadeout() from the loaded skin
                let skin_fadeout = self
                    .main_state_data
                    .skin
                    .as_ref()
                    .map_or(0, |s| s.get_fadeout()) as i64;
                if self
                    .main_state_data
                    .timer
                    .get_now_time_for_id(TIMER_FADEOUT)
                    > skin_fadeout
                {
                    // input.setEnable(true); input.setStartTime(0);
                    self.pending_state_change = Some(MainStateType::MusicSelect);
                    log::info!("Aborted, transition to MUSICSELECT");
                }
            }

            _ => {}
        }

        self.prevtime = micronow;

        // Copy recent judge data to timer for SkinTimingVisualizer/SkinHitErrorVisualizer
        self.main_state_data.timer.set_recent_judges(
            self.judge.get_recent_judges_index(),
            self.judge.get_recent_judges(),
        );
    }

    fn input(&mut self) {
        // Compute values before taking mutable borrows
        let is_note_end = self.is_note_end();
        let is_timer_play_on = self.main_state_data.timer.is_timer_on(TIMER_PLAY);
        let now_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        // Process control input (START+SELECT, lane cover, hispeed, etc.)
        if let (Some(mut control), Some(lanerender)) =
            (self.control.take(), self.lanerender.as_mut())
        {
            // Wire BMSPlayerInputProcessor state into context
            let mut noop_analog = |_key: usize, _ms: i32| -> i32 { 0 };
            let mut ctx = crate::control_input_processor::ControlInputContext {
                lanerender,
                start_pressed: self.input_start_pressed,
                select_pressed: self.input_select_pressed,
                control_key_up: false,
                control_key_down: false,
                control_key_escape_pressed: false,
                control_key_num1: false,
                control_key_num2: false,
                control_key_num3: false,
                control_key_num4: false,
                key_states: &self.input_key_states,
                scroll: 0,
                is_analog: &[],
                analog_diff_and_reset: &mut noop_analog,
                is_timer_play_on,
                is_note_end,
                window_hold: self.player_config.is_window_hold,
                autoplay_mode: self.play_mode.mode,
                now_millis,
            };

            let result = control.input(&mut ctx);

            // Apply result actions
            if let Some(speed) = result.play_speed {
                self.set_play_speed(speed);
            }
            if result.stop_play {
                // Restore control before stopping (stop_play may need it)
                self.control = Some(control);
                self.stop_play();
            } else {
                self.control = Some(control);
            }
        }

        // Build InputContext for key input processing.
        let auto_presstime = self.judge.get_auto_presstime().to_vec();
        let now = self.main_state_data.timer.get_now_time();
        let is_autoplay = self.play_mode.mode == beatoraja_core::bms_player_mode::Mode::Autoplay;
        if let Some(ref mut keyinput) = self.keyinput {
            let mut ctx = crate::key_input_processor::InputContext {
                now,
                key_states: &self.input_key_states,
                auto_presstime: &auto_presstime,
                is_autoplay,
                timer: &mut self.main_state_data.timer,
            };
            keyinput.input(&mut ctx);
        }
    }

    fn pause(&mut self) {
        // In Java, pause/resume are inherited from MainState (default empty)
        // but timer management may be needed
    }

    fn resume(&mut self) {
        // In Java, pause/resume are inherited from MainState (default empty)
    }

    fn dispose(&mut self) {
        // Call default MainState dispose
        self.main_state_data.skin = None;
        self.main_state_data.stage = None;

        if let Some(ref mut lr) = self.lanerender {
            lr.dispose();
        }
        self.practice.dispose();
        log::info!("Play state resources disposed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_input::bms_player_input_device::DeviceType;
    use bms_model::bms_model::BMSModel;
    use bms_model::mode::Mode;

    fn make_model() -> BMSModel {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        model
    }

    fn make_model_with_time(last_note_time: i32) -> BMSModel {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        // Add a timeline at the given time to set last_note_time
        let mut timelines = Vec::new();
        let tl = bms_model::time_line::TimeLine::new(130.0, last_note_time as i64 * 1000, 8);
        timelines.push(tl);
        model.set_all_time_line(timelines);
        model
    }

    // --- Constructor tests ---

    #[test]
    fn new_creates_default_state() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        assert_eq!(player.get_state(), STATE_PRELOAD);
        assert_eq!(player.get_play_speed(), 100);
        assert_eq!(player.get_adjusted_volume(), -1.0);
        assert!(!player.analysis_checked);
    }

    #[test]
    fn new_sets_playtime_from_model() {
        let model = make_model();
        let expected_playtime = model.get_last_note_time() + TIME_MARGIN;
        let player = BMSPlayer::new(model);
        assert_eq!(player.get_playtime(), expected_playtime);
    }

    // --- MainState trait tests ---

    #[test]
    fn state_type_returns_play() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        assert_eq!(player.state_type(), Some(MainStateType::Play));
    }

    #[test]
    fn main_state_data_accessible() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        let data = player.main_state_data();
        // Timer should be initialized
        assert!(!data.timer.is_timer_on(TIMER_PLAY));
    }

    // --- State machine transition tests ---

    #[test]
    fn state_preload_transitions_to_ready() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.play_skin.set_loadstart(0);
        player.play_skin.set_loadend(0);
        player.media_load_finished = true;

        // The PRELOAD->READY transition requires:
        // 1. media_load_finished = true
        // 2. micronow > (loadstart + loadend) * 1000 = 0
        // 3. micronow - startpressedtime > 1_000_000
        //
        // To satisfy (2) and (3), we need micronow > 1_000_000.
        // Since TimerManager uses Instant::now(), micronow is near 0 in tests.
        // We force this by setting TIMER_PLAY to a known value and using set_micro_timer
        // to manipulate the effective "now" time. However, the simplest approach is
        // to directly manipulate the state and verify the transition logic.
        player.state = STATE_PRELOAD;
        player.startpressedtime = -2_000_000;

        // Set the timer's starttime far in the past by calling update repeatedly
        // won't help since elapsed is near-zero. Instead, use set_micro_timer
        // on a timer we read from to simulate "time has passed".
        // Actually, the simplest fix: set startpressedtime so the delta is satisfied
        // even with micronow near 0. micronow(~0) - startpressedtime(-2M) = 2M > 1M. Good.
        // But micronow(~0) > load_threshold(0) requires micronow > 0, which may be 0.
        // So let's update the timer to get a small positive value.
        std::thread::sleep(std::time::Duration::from_millis(2));
        player.main_state_data.timer.update();

        player.render();
        assert_eq!(player.get_state(), STATE_READY);
    }

    #[test]
    fn state_ready_transitions_to_play() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_READY;
        player.play_skin.set_playstart(0); // Instant transition
        player.main_state_data.timer.set_timer_on(TIMER_READY);
        player.lanerender = Some(LaneRenderer::new(&player.model));

        // Update timer and render
        player.main_state_data.timer.update();
        // TIMER_READY now_time should be > 0 (= playstart)
        // But get_now_time_for_id checks micronow - timer value, which is 0 since we just set it
        // We need some time to pass. Since playstart=0, any positive time works.
        // The condition is: timer.getNowTime(TIMER_READY) > skin.getPlaystart()
        // getNowTime(TIMER_READY) = (nowmicrotime - timer[TIMER_READY]) / 1000
        // Since we just set it, this is ~0. We need > 0.
        // Let's manually set the timer to past to simulate time passing.
        let now = player.main_state_data.timer.get_now_micro_time();
        player
            .main_state_data
            .timer
            .set_micro_timer(TIMER_READY, now - 2000); // 2ms ago

        player.render();
        assert_eq!(player.get_state(), STATE_PLAY);
    }

    #[test]
    fn state_play_transitions_to_finished_when_playtime_exceeded() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PLAY;
        player.playtime = 0; // Instant finish

        // Set TIMER_PLAY to far past so ptime is large
        player.main_state_data.timer.update();
        let now = player.main_state_data.timer.get_now_micro_time();
        player
            .main_state_data
            .timer
            .set_micro_timer(TIMER_PLAY, now - 2_000_000); // 2 seconds ago
        player.prevtime = now - 1000; // Small delta

        player.render();
        assert_eq!(player.get_state(), STATE_FINISHED);
    }

    #[test]
    fn state_play_transitions_to_failed_on_zero_gauge() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PLAY;
        player.playtime = 999_999; // Long playtime so we don't finish

        // Create a gauge at 0 value
        let gauge = crate::groove_gauge::create_groove_gauge(
            &player.model,
            beatoraja_types::groove_gauge::HARD,
            0,
            None,
        )
        .unwrap();
        player.gauge = Some(gauge);
        // Set gauge to 0
        player.gauge.as_mut().unwrap().set_value(0.0);

        // Setup timers
        player.main_state_data.timer.update();
        let now = player.main_state_data.timer.get_now_micro_time();
        player
            .main_state_data
            .timer
            .set_micro_timer(TIMER_PLAY, now - 1000);
        player.prevtime = now - 500;

        player.render();
        assert_eq!(player.get_state(), STATE_FAILED);
    }

    // --- stop_play tests ---

    #[test]
    fn stop_play_from_practice_goes_to_practice_finished() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PRACTICE;
        player.stop_play();
        assert_eq!(player.get_state(), STATE_PRACTICE_FINISHED);
        assert!(player.main_state_data.timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn stop_play_from_preload_goes_to_aborted() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PRELOAD;
        player.stop_play();
        assert_eq!(player.get_state(), STATE_ABORTED);
        assert!(player.main_state_data.timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn stop_play_from_ready_goes_to_aborted() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_READY;
        player.stop_play();
        assert_eq!(player.get_state(), STATE_ABORTED);
    }

    #[test]
    fn stop_play_from_play_with_no_notes_goes_to_aborted() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PLAY;
        // Judge has no notes hit (all counts = 0), and keyinput needs to exist
        player.keyinput = Some(KeyInputProccessor::new(&LaneProperty::new(&Mode::BEAT_7K)));
        player.stop_play();
        assert_eq!(player.get_state(), STATE_ABORTED);
    }

    #[test]
    fn stop_play_ignores_if_already_failed_timer() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PLAY;
        player.main_state_data.timer.set_timer_on(TIMER_FAILED);
        let prev_state = player.state;
        player.stop_play();
        // State should not change because TIMER_FAILED is already on
        assert_eq!(player.get_state(), prev_state);
    }

    // --- create_score_data tests ---

    /// Helper: create a model with notes that have specific state/playtime values.
    /// `notes_spec` is a vec of (state, micro_play_time) tuples for Normal notes.
    fn make_model_with_timed_notes(notes_spec: &[(i32, i64)]) -> BMSModel {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);

        let mut timelines = Vec::new();
        for (i, &(state, playtime)) in notes_spec.iter().enumerate() {
            let mut tl = bms_model::time_line::TimeLine::new(i as f64, (i as i64) * 1_000_000, 8);
            let mut note = bms_model::note::Note::new_normal(1);
            note.set_state(state);
            note.set_micro_play_time(playtime);
            tl.set_note(0, Some(note));
            timelines.push(tl);
        }
        model.set_all_time_line(timelines);
        model
    }

    #[test]
    fn create_score_data_timing_stats_with_hit_notes() {
        // Three notes with state 1-4 and known play times:
        //   note0: state=1, playtime=1000  (|1000| = 1000)
        //   note1: state=2, playtime=-2000 (|-2000| = 2000)
        //   note2: state=3, playtime=3000  (|3000| = 3000)
        let model = make_model_with_timed_notes(&[(1, 1000), (2, -2000), (3, 3000)]);
        let mut player = BMSPlayer::new(model);
        // Use ABORTED state to bypass the zero-notes-hit check
        player.state = STATE_ABORTED;

        let score = player.create_score_data(DeviceType::Keyboard).unwrap();

        // total_duration = |1000| + |-2000| + |3000| = 6000
        assert_eq!(score.total_duration, 6000);
        // total_avg = 1000 + (-2000) + 3000 = 2000
        assert_eq!(score.total_avg, 2000);
        // avgjudge = total_duration / count = 6000 / 3 = 2000
        assert_eq!(score.avgjudge, 2000);
        // avg = total_avg / count = 2000 / 3 = 666
        assert_eq!(score.avg, 666);
        // stddev = sqrt(((1000 - 666)^2 + (-2000 - 666)^2 + (3000 - 666)^2) / 3)
        //        = sqrt((111556 + 7111696 + 5449956) / 3)
        //        = sqrt(12673208 / 3)
        //        = sqrt(4224402)
        //        = 2055 (as i64 from f64::sqrt truncation)
        let mean = 666_i64;
        let var = ((1000 - mean).pow(2) + (-2000 - mean).pow(2) + (3000 - mean).pow(2)) / 3;
        let expected_stddev = (var as f64).sqrt() as i64;
        assert_eq!(score.stddev, expected_stddev);
    }

    #[test]
    fn create_score_data_timing_stats_no_judged_notes() {
        // Notes with state=0 (not judged) should not contribute to timing stats.
        // Fields should stay at their initial values.
        let model = make_model_with_timed_notes(&[(0, 5000), (0, -3000)]);
        let mut player = BMSPlayer::new(model);
        player.state = STATE_ABORTED;

        let score = player.create_score_data(DeviceType::Keyboard).unwrap();

        // No notes matched state 1-4:
        // avgjudge and avg stay at initial i64::MAX (conditional set not entered)
        assert_eq!(score.avgjudge, i64::MAX);
        assert_eq!(score.avg, i64::MAX);
        // total_duration, total_avg, and stddev are unconditionally set to 0
        assert_eq!(score.total_duration, 0);
        assert_eq!(score.total_avg, 0);
        assert_eq!(score.stddev, 0);
    }

    #[test]
    fn create_score_data_timing_stats_filters_ln_end_notes() {
        // LN end notes of longnote type should be excluded from timing stats.
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        // Default lntype is LNTYPE_LONGNOTE (0)

        let mut tl = bms_model::time_line::TimeLine::new(0.0, 0, 8);

        // Normal note: state=1, playtime=1000 → included
        let mut normal = bms_model::note::Note::new_normal(1);
        normal.set_state(1);
        normal.set_micro_play_time(1000);
        tl.set_note(0, Some(normal));

        // LN end note with TYPE_UNDEFINED (default) + lntype=LNTYPE_LONGNOTE → excluded
        let mut ln_end = bms_model::note::Note::new_long(1);
        ln_end.set_end(true);
        ln_end.set_state(1);
        ln_end.set_micro_play_time(5000);
        tl.set_note(1, Some(ln_end));

        // LN start note (not end): state=2, playtime=2000 → included
        let mut ln_start = bms_model::note::Note::new_long(1);
        ln_start.set_state(2);
        ln_start.set_micro_play_time(2000);
        tl.set_note(2, Some(ln_start));

        model.set_all_time_line(vec![tl]);

        let mut player = BMSPlayer::new(model);
        player.state = STATE_ABORTED;

        let score = player.create_score_data(DeviceType::Keyboard).unwrap();

        // Only normal(1000) and ln_start(2000) should be included
        assert_eq!(score.total_duration, 3000); // |1000| + |2000|
        assert_eq!(score.total_avg, 3000); // 1000 + 2000
        assert_eq!(score.avgjudge, 1500); // 3000 / 2
        assert_eq!(score.avg, 1500); // 3000 / 2
    }

    #[test]
    fn create_score_data_returns_none_when_no_notes_hit() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        // No notes hit - all judge counts are 0
        let result = player.create_score_data(DeviceType::Keyboard);
        assert!(result.is_none());
    }

    #[test]
    fn create_score_data_returns_some_when_aborted() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_ABORTED;
        // Even with no notes, aborted state returns score data
        let result = player.create_score_data(DeviceType::Keyboard);
        assert!(result.is_some());
    }

    // --- create_score_data device_type tests ---

    #[test]
    fn create_score_data_sets_device_type_keyboard() {
        use beatoraja_types::stubs::bms_player_input_device;

        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_ABORTED;

        let score = player.create_score_data(DeviceType::Keyboard).unwrap();
        assert_eq!(
            score.device_type,
            Some(bms_player_input_device::Type::KEYBOARD)
        );
    }

    #[test]
    fn create_score_data_sets_device_type_bm_controller() {
        use beatoraja_types::stubs::bms_player_input_device;

        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_ABORTED;

        let score = player.create_score_data(DeviceType::BmController).unwrap();
        assert_eq!(
            score.device_type,
            Some(bms_player_input_device::Type::BM_CONTROLLER)
        );
    }

    #[test]
    fn create_score_data_sets_device_type_midi() {
        use beatoraja_types::stubs::bms_player_input_device;

        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_ABORTED;

        let score = player.create_score_data(DeviceType::Midi).unwrap();
        assert_eq!(score.device_type, Some(bms_player_input_device::Type::MIDI));
    }

    // --- update_judge tests ---

    #[test]
    fn update_judge_updates_pomyu_chara_judge() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.gauge = Some(
            crate::groove_gauge::create_groove_gauge(
                &player.model,
                beatoraja_types::groove_gauge::NORMAL,
                0,
                None,
            )
            .unwrap(),
        );
        player.update_judge(0, 1_000_000); // PGREAT
        assert_eq!(player.play_skin.pomyu.pm_chara_judge, 1);

        player.update_judge(2, 2_000_000); // GOOD
        assert_eq!(player.play_skin.pomyu.pm_chara_judge, 3);
    }

    // --- set_play_speed tests ---

    #[test]
    fn set_play_speed_updates_value() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_play_speed(50);
        assert_eq!(player.get_play_speed(), 50);
    }

    // --- Getter tests ---

    #[test]
    fn get_mode_returns_model_mode() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        assert_eq!(player.get_mode(), Mode::BEAT_7K);
    }

    #[test]
    fn get_skin_type_returns_matching_type() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        let skin_type = player.get_skin_type();
        assert!(skin_type.is_some());
    }

    #[test]
    fn get_option_information_returns_playinfo() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        let info = player.get_option_information();
        assert_eq!(info.randomoption, 0);
    }

    #[test]
    fn is_note_end_false_initially() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        // With no notes, total_notes = 0 and past_notes = 0, so it should be true
        assert!(player.is_note_end());
    }

    #[test]
    fn get_now_quarter_note_time_zero_without_rhythm() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        assert_eq!(player.get_now_quarter_note_time(), 0);
    }

    // --- State machine lifecycle integration test ---

    #[test]
    fn lifecycle_preload_ready_play_finished() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.media_load_finished = true;

        // Start at PRELOAD
        assert_eq!(player.get_state(), STATE_PRELOAD);

        // Force transition to READY
        player.startpressedtime = -2_000_000;
        player.play_skin.set_loadstart(0);
        player.play_skin.set_loadend(0);
        std::thread::sleep(std::time::Duration::from_millis(2));
        player.main_state_data.timer.update();
        player.render();
        assert_eq!(player.get_state(), STATE_READY);

        // Force transition to PLAY
        player.play_skin.set_playstart(0);
        player.lanerender = Some(LaneRenderer::new(&player.model));
        let now = player.main_state_data.timer.get_now_micro_time();
        player
            .main_state_data
            .timer
            .set_micro_timer(TIMER_READY, now - 2000);
        player.render();
        assert_eq!(player.get_state(), STATE_PLAY);

        // Force transition to FINISHED
        player.playtime = 0; // Instant finish
        let now = player.main_state_data.timer.get_now_micro_time();
        player
            .main_state_data
            .timer
            .set_micro_timer(TIMER_PLAY, now - 2_000_000);
        player.prevtime = now - 1000;
        player.render();
        assert_eq!(player.get_state(), STATE_FINISHED);
    }

    // --- dispose test ---

    #[test]
    fn dispose_clears_skin_and_stage() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.dispose();
        assert!(player.main_state_data.skin.is_none());
        assert!(player.main_state_data.stage.is_none());
    }

    // --- build_pattern_modifiers tests ---

    fn make_default_config() -> beatoraja_core::player_config::PlayerConfig {
        beatoraja_core::player_config::PlayerConfig::default()
    }

    #[test]
    fn build_pattern_modifiers_default_config_no_assist() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let config = make_default_config();
        let score = player.build_pattern_modifiers(&config);
        assert!(score, "Default config should allow score submission");
        assert_eq!(player.assist, 0, "Default config should not set assist");
    }

    #[test]
    fn build_pattern_modifiers_scroll_mode() {
        // ScrollSpeedModifier requires at least one timeline
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        let tl = bms_model::time_line::TimeLine::new(130.0, 0, 8);
        model.set_all_time_line(vec![tl]);

        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.scroll_mode = 1; // Enable scroll speed modifier (Remove mode)
        player.build_pattern_modifiers(&config);
        // ScrollSpeedModifier in Remove mode sets LightAssist if BPM changes exist;
        // with a single-BPM model it sets None. Either way, the modifier was applied.
        // The key thing is it doesn't crash and processes correctly.
    }

    #[test]
    fn build_pattern_modifiers_longnote_mode() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.longnote_mode = 1; // Enable LN modifier (Remove mode)
        player.build_pattern_modifiers(&config);
        // LongNoteModifier in Remove mode sets Assist if LNs exist.
        // With empty model, no LNs, so assist stays None.
    }

    #[test]
    fn build_pattern_modifiers_mine_mode() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.mine_mode = 1; // Enable mine modifier (Remove mode)
        player.build_pattern_modifiers(&config);
        // MineNoteModifier in Remove mode sets LightAssist if mine notes exist.
    }

    #[test]
    fn build_pattern_modifiers_extranote() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.extranote_depth = 1; // Enable extra note modifier
        player.build_pattern_modifiers(&config);
    }

    #[test]
    fn build_pattern_modifiers_dp_battle_converts_sp_to_dp() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);

        let mut config = make_default_config();
        config.doubleoption = 2;
        player.playinfo.doubleoption = 2;

        let score = player.build_pattern_modifiers(&config);
        // SP BEAT_7K should be converted to BEAT_14K
        assert_eq!(player.get_mode(), Mode::BEAT_14K);
        // assist should be at least 1 (LightAssist)
        assert!(player.assist >= 1);
        // score should be false
        assert!(!score);
    }

    #[test]
    fn build_pattern_modifiers_dp_battle_with_autoplay_scratch() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);

        let mut config = make_default_config();
        config.doubleoption = 3; // Battle + L-ASSIST (autoplay scratch)
        player.playinfo.doubleoption = 3;

        player.build_pattern_modifiers(&config);
        // SP BEAT_7K should be converted to BEAT_14K
        assert_eq!(player.get_mode(), Mode::BEAT_14K);
        assert!(player.assist >= 1);
    }

    #[test]
    fn build_pattern_modifiers_dp_battle_non_sp_resets_doubleoption() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K); // Already DP
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);

        let mut config = make_default_config();
        config.doubleoption = 2;
        player.playinfo.doubleoption = 2;

        player.build_pattern_modifiers(&config);
        // Not SP mode, so BATTLE is not applied
        assert_eq!(player.get_mode(), Mode::BEAT_14K);
        assert_eq!(player.playinfo.doubleoption, 0);
    }

    #[test]
    fn build_pattern_modifiers_dp_flip() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K); // DP mode
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);

        let mut config = make_default_config();
        config.doubleoption = 1;
        player.playinfo.doubleoption = 1;

        player.build_pattern_modifiers(&config);
        // PlayerFlipModifier should be applied, mode stays BEAT_14K
        assert_eq!(player.get_mode(), Mode::BEAT_14K);
    }

    #[test]
    fn build_pattern_modifiers_random_option_seed_saved() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let config = make_default_config();

        player.build_pattern_modifiers(&config);
        // After applying modifiers, the 1P random seed should be saved in playinfo
        // Even with Identity (random=0), the seed is initialized
        assert_ne!(player.playinfo.randomoptionseed, -1);
    }

    #[test]
    fn build_pattern_modifiers_random_option_seed_restored() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let config = make_default_config();

        // Pre-set a seed (as if restoring from replay)
        player.playinfo.randomoptionseed = 12345;

        player.build_pattern_modifiers(&config);
        // The seed should be preserved (not overwritten)
        assert_eq!(player.playinfo.randomoptionseed, 12345);
    }

    #[test]
    fn build_pattern_modifiers_dp_random2_seed_saved() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K); // DP mode
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);
        let config = make_default_config();

        player.build_pattern_modifiers(&config);
        // In DP mode, the 2P random seed should also be saved
        assert_ne!(player.playinfo.randomoption2seed, -1);
    }

    #[test]
    fn build_pattern_modifiers_7to9() {
        let model = make_model(); // BEAT_7K
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.seven_to_nine_pattern = 1; // Enable 7to9

        player.build_pattern_modifiers(&config);
        // Mode should be changed from BEAT_7K to POPN_9K
        assert_eq!(player.get_mode(), Mode::POPN_9K);
        assert!(player.assist >= 1, "7to9 should set at least light assist");
    }

    #[test]
    fn build_pattern_modifiers_assist_accumulates_light() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        // Add timelines with a mine note to trigger assist
        let mut tl = bms_model::time_line::TimeLine::new(130.0, 0, 8);
        tl.set_note(0, Some(bms_model::note::Note::new_mine(-1, 10.0)));
        model.set_all_time_line(vec![tl]);

        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.mine_mode = 1; // Remove mines -> LightAssist

        let score = player.build_pattern_modifiers(&config);
        assert_eq!(
            player.assist, 1,
            "Mine removal should set assist to 1 (LightAssist)"
        );
        assert!(!score, "LightAssist should mark score as invalid");
    }

    #[test]
    fn build_pattern_modifiers_5k_battle() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_5K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);

        let mut config = make_default_config();
        config.doubleoption = 2;
        player.playinfo.doubleoption = 2;

        player.build_pattern_modifiers(&config);
        // BEAT_5K should be converted to BEAT_10K
        assert_eq!(player.get_mode(), Mode::BEAT_10K);
    }

    // --- encode_seed_for_score tests ---

    #[test]
    fn encode_seed_for_score_sp_returns_1p_seed() {
        let model = make_model(); // BEAT_7K (player=1)
        let mut player = BMSPlayer::new(model);
        player.playinfo.randomoptionseed = 12345;
        assert_eq!(player.encode_seed_for_score(), 12345);
    }

    #[test]
    fn encode_seed_for_score_dp_returns_combined() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K); // DP (player=2)
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);
        player.playinfo.randomoptionseed = 100;
        player.playinfo.randomoption2seed = 3;
        // Combined: 3 * 65536 * 256 + 100 = 3 * 16777216 + 100 = 50331748
        assert_eq!(player.encode_seed_for_score(), 50_331_748);
    }

    #[test]
    fn encode_seed_for_score_dp_zero_seeds() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);
        player.playinfo.randomoptionseed = 0;
        player.playinfo.randomoption2seed = 0;
        assert_eq!(player.encode_seed_for_score(), 0);
    }

    // --- encode_option_for_score tests ---

    #[test]
    fn encode_option_for_score_sp_returns_randomoption() {
        let model = make_model(); // BEAT_7K (player=1)
        let mut player = BMSPlayer::new(model);
        player.playinfo.randomoption = 5;
        assert_eq!(player.encode_option_for_score(), 5);
    }

    #[test]
    fn encode_option_for_score_dp_returns_combined() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K); // DP (player=2)
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);
        player.playinfo.randomoption = 2;
        player.playinfo.randomoption2 = 3;
        player.playinfo.doubleoption = 1;
        // Combined: 2 + 3 * 10 + 1 * 100 = 132
        assert_eq!(player.encode_option_for_score(), 132);
    }

    #[test]
    fn encode_option_for_score_dp_no_doubleoption() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);
        player.playinfo.randomoption = 1;
        player.playinfo.randomoption2 = 4;
        player.playinfo.doubleoption = 0;
        // Combined: 1 + 4 * 10 + 0 * 100 = 41
        assert_eq!(player.encode_option_for_score(), 41);
    }

    // --- seed round-trip test ---

    #[test]
    fn seed_round_trip_preserved_after_build_modifiers() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let config = make_default_config();

        // First build: generates a new seed
        player.build_pattern_modifiers(&config);
        let saved_seed = player.playinfo.randomoptionseed;
        assert_ne!(saved_seed, -1, "Seed should be initialized");

        // Second build with the same player: seed should be preserved
        // (since randomoptionseed is no longer -1, the restore path is used)
        let model2 = make_model();
        let mut player2 = BMSPlayer::new(model2);
        player2.playinfo.randomoptionseed = saved_seed;
        player2.build_pattern_modifiers(&config);
        assert_eq!(
            player2.playinfo.randomoptionseed, saved_seed,
            "Seed should be preserved on rebuild"
        );
    }

    #[test]
    fn build_pattern_modifiers_lane_shuffle_pattern_saved() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K); // DP mode
        model.set_judgerank(100);
        let tl = bms_model::time_line::TimeLine::new(130.0, 0, 16);
        model.set_all_time_line(vec![tl]);
        let mut player = BMSPlayer::new(model);

        let mut config = make_default_config();
        // Random (id=2) creates LaneRandomShuffleModifier with show_shuffle_pattern=true
        config.random = 2;
        player.playinfo.randomoption = 2;

        player.build_pattern_modifiers(&config);
        // lane_shuffle_pattern should be initialized with player count
        let lsp = player.playinfo.lane_shuffle_pattern.as_ref();
        assert!(
            lsp.is_some(),
            "lane_shuffle_pattern should be set for DP mode with Random option"
        );
        assert_eq!(
            lsp.unwrap().len(),
            2,
            "DP mode should have 2 player patterns"
        );
    }

    // --- restore_replay_data tests (Phase 34c) ---

    fn make_replay_data() -> ReplayData {
        let mut rd = ReplayData::new();
        rd.randomoption = 3;
        rd.randomoptionseed = 99999;
        rd.randomoption2 = 2;
        rd.randomoption2seed = 88888;
        rd.doubleoption = 1;
        rd.rand = vec![2, 5, 1];
        rd.gauge = beatoraja_types::groove_gauge::HARD;
        rd.config = Some(beatoraja_types::play_config::PlayConfig {
            hispeed: 5.0,
            duration: 300,
            ..beatoraja_types::play_config::PlayConfig::default()
        });
        rd
    }

    #[test]
    fn restore_replay_data_none_returns_no_stay() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let key_state = ReplayKeyState::default();

        let result = player.restore_replay_data(None, &key_state);
        assert!(!result.stay_replay);
        assert!(result.replay.is_none());
        assert!(result.hs_replay_config.is_none());
        // playinfo should be unchanged
        assert_eq!(player.playinfo.randomoption, 0);
        assert_eq!(player.playinfo.randomoptionseed, -1);
    }

    #[test]
    fn restore_replay_data_pattern_key_copies_all_fields() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let replay = make_replay_data();

        let key_state = ReplayKeyState {
            pattern_key: true,
            ..Default::default()
        };

        let result = player.restore_replay_data(Some(replay), &key_state);
        // Should switch to PLAY mode
        assert!(!result.stay_replay);
        assert!(result.replay.is_none());

        // All fields should be copied
        assert_eq!(player.playinfo.randomoption, 3);
        assert_eq!(player.playinfo.randomoptionseed, 99999);
        assert_eq!(player.playinfo.randomoption2, 2);
        assert_eq!(player.playinfo.randomoption2seed, 88888);
        assert_eq!(player.playinfo.doubleoption, 1);
        assert_eq!(player.playinfo.rand, vec![2, 5, 1]);
    }

    #[test]
    fn restore_replay_data_option_key_copies_options_not_seeds() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let replay = make_replay_data();

        let key_state = ReplayKeyState {
            option_key: true,
            ..Default::default()
        };

        let result = player.restore_replay_data(Some(replay), &key_state);
        // Should switch to PLAY mode
        assert!(!result.stay_replay);
        assert!(result.replay.is_none());

        // Options should be copied
        assert_eq!(player.playinfo.randomoption, 3);
        assert_eq!(player.playinfo.randomoption2, 2);
        assert_eq!(player.playinfo.doubleoption, 1);

        // Seeds should NOT be copied (remain at default -1)
        assert_eq!(player.playinfo.randomoptionseed, -1);
        assert_eq!(player.playinfo.randomoption2seed, -1);

        // Rand should NOT be copied
        assert!(player.playinfo.rand.is_empty());
    }

    #[test]
    fn restore_replay_data_hs_key_saves_config() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let replay = make_replay_data();

        let key_state = ReplayKeyState {
            hs_key: true,
            ..Default::default()
        };

        let result = player.restore_replay_data(Some(replay), &key_state);
        // Should switch to PLAY mode
        assert!(!result.stay_replay);
        assert!(result.replay.is_none());

        // HS config should be returned
        let hs_config = result.hs_replay_config.unwrap();
        assert_eq!(hs_config.hispeed, 5.0);
        assert_eq!(hs_config.duration, 300);
    }

    #[test]
    fn restore_replay_data_pattern_and_hs_keys_together() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let replay = make_replay_data();

        let key_state = ReplayKeyState {
            pattern_key: true,
            hs_key: true,
            ..Default::default()
        };

        let result = player.restore_replay_data(Some(replay), &key_state);
        assert!(!result.stay_replay);
        assert!(result.replay.is_none());

        // Pattern fields should be copied
        assert_eq!(player.playinfo.randomoption, 3);
        assert_eq!(player.playinfo.randomoptionseed, 99999);

        // HS config should also be returned
        assert!(result.hs_replay_config.is_some());
    }

    #[test]
    fn restore_replay_data_no_keys_stays_replay() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let replay = make_replay_data();

        let key_state = ReplayKeyState::default();

        let result = player.restore_replay_data(Some(replay.clone()), &key_state);
        // Should stay in REPLAY mode
        assert!(result.stay_replay);
        assert!(result.replay.is_some());
        assert!(result.hs_replay_config.is_none());

        // playinfo should be unchanged
        assert_eq!(player.playinfo.randomoption, 0);
        assert_eq!(player.playinfo.randomoptionseed, -1);
    }

    // --- select_gauge_type tests (Phase 34c) ---

    #[test]
    fn select_gauge_type_no_replay_uses_config() {
        let key_state = ReplayKeyState::default();
        let result =
            BMSPlayer::select_gauge_type(None, beatoraja_types::groove_gauge::NORMAL, &key_state);
        assert_eq!(result, beatoraja_types::groove_gauge::NORMAL);
    }

    #[test]
    fn select_gauge_type_replay_uses_replay_gauge() {
        let mut replay = make_replay_data();
        replay.gauge = beatoraja_types::groove_gauge::HARD;
        let key_state = ReplayKeyState::default();
        let result = BMSPlayer::select_gauge_type(
            Some(&replay),
            beatoraja_types::groove_gauge::NORMAL,
            &key_state,
        );
        assert_eq!(result, beatoraja_types::groove_gauge::HARD);
    }

    #[test]
    fn select_gauge_type_replay_with_key5_shifts_by_1() {
        let mut replay = make_replay_data();
        replay.gauge = beatoraja_types::groove_gauge::NORMAL; // 2
        let key_state = ReplayKeyState {
            gauge_shift_key5: true,
            ..Default::default()
        };
        let result = BMSPlayer::select_gauge_type(
            Some(&replay),
            beatoraja_types::groove_gauge::NORMAL,
            &key_state,
        );
        assert_eq!(result, beatoraja_types::groove_gauge::HARD); // 2 + 1 = 3
    }

    #[test]
    fn select_gauge_type_replay_with_key3_shifts_by_2() {
        let mut replay = make_replay_data();
        replay.gauge = beatoraja_types::groove_gauge::NORMAL; // 2
        let key_state = ReplayKeyState {
            gauge_shift_key3: true,
            ..Default::default()
        };
        let result = BMSPlayer::select_gauge_type(
            Some(&replay),
            beatoraja_types::groove_gauge::NORMAL,
            &key_state,
        );
        assert_eq!(result, beatoraja_types::groove_gauge::EXHARD); // 2 + 2 = 4
    }

    #[test]
    fn select_gauge_type_replay_with_both_keys_shifts_by_3() {
        let mut replay = make_replay_data();
        replay.gauge = beatoraja_types::groove_gauge::NORMAL; // 2
        let key_state = ReplayKeyState {
            gauge_shift_key3: true,
            gauge_shift_key5: true,
            ..Default::default()
        };
        let result = BMSPlayer::select_gauge_type(
            Some(&replay),
            beatoraja_types::groove_gauge::NORMAL,
            &key_state,
        );
        assert_eq!(result, beatoraja_types::groove_gauge::HAZARD); // 2 + 3 = 5
    }

    #[test]
    fn select_gauge_type_replay_hazard_no_shift() {
        let mut replay = make_replay_data();
        replay.gauge = beatoraja_types::groove_gauge::HAZARD; // 5
        let key_state = ReplayKeyState {
            gauge_shift_key5: true,
            ..Default::default()
        };
        let result = BMSPlayer::select_gauge_type(
            Some(&replay),
            beatoraja_types::groove_gauge::NORMAL,
            &key_state,
        );
        // HAZARD cannot be shifted further
        assert_eq!(result, beatoraja_types::groove_gauge::HAZARD);
    }

    // --- handle_random_syntax tests (Phase 34c) ---

    #[test]
    fn handle_random_syntax_no_random_in_model() {
        let model = make_model(); // No random branches set
        let mut player = BMSPlayer::new(model);
        let result = player.handle_random_syntax(false, None, -1, &[]);
        assert!(result.is_none());
        assert!(player.playinfo.rand.is_empty());
    }

    #[test]
    fn handle_random_syntax_replay_mode_uses_replay_rand() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        model.set_chart_information(bms_model::chart_information::ChartInformation::new(
            None,
            0,
            Some(vec![1, 3, 2]),
        )); // Model has random branches
        let mut player = BMSPlayer::new(model);

        let mut replay = make_replay_data();
        replay.rand = vec![2, 1, 3];

        let result = player.handle_random_syntax(true, Some(&replay), -1, &[]);
        // Should return Some with the replay's rand for model reload
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![2, 1, 3]);
        assert_eq!(player.playinfo.rand, vec![2, 1, 3]);
    }

    #[test]
    fn handle_random_syntax_resource_seed_set_uses_resource_rand() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        model.set_chart_information(bms_model::chart_information::ChartInformation::new(
            None,
            0,
            Some(vec![1, 3, 2]),
        ));
        let mut player = BMSPlayer::new(model);

        let resource_rand = vec![3, 2, 1];

        let result = player.handle_random_syntax(false, None, 42, &resource_rand);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![3, 2, 1]);
        assert_eq!(player.playinfo.rand, vec![3, 2, 1]);
    }

    #[test]
    fn handle_random_syntax_normal_play_stores_model_random() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        model.set_chart_information(bms_model::chart_information::ChartInformation::new(
            None,
            0,
            Some(vec![4, 5, 6]),
        ));
        let mut player = BMSPlayer::new(model);

        let result = player.handle_random_syntax(false, None, -1, &[]);
        // No reload needed (no rand override), but model's random should be stored
        assert!(result.is_none());
        assert_eq!(player.playinfo.rand, vec![4, 5, 6]);
    }

    #[test]
    fn handle_random_syntax_replay_empty_rand_stores_model_random() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        model.set_chart_information(bms_model::chart_information::ChartInformation::new(
            None,
            0,
            Some(vec![1, 2]),
        ));
        let mut player = BMSPlayer::new(model);

        let mut replay = make_replay_data();
        replay.rand = vec![]; // Empty rand in replay

        let result = player.handle_random_syntax(true, Some(&replay), -1, &[]);
        // Empty rand means no reload, store model's random
        assert!(result.is_none());
        assert_eq!(player.playinfo.rand, vec![1, 2]);
    }

    // --- calculate_non_modifier_assist tests (Phase 34d) ---

    /// Helper: create a model with uniform BPM (min == max).
    fn make_model_uniform_bpm() -> BMSModel {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_bpm(150.0);
        model.set_judgerank(100);
        // Single timeline at the same BPM → min == max
        let mut tl = bms_model::time_line::TimeLine::new(0.0, 0, 8);
        tl.set_bpm(150.0);
        model.set_all_time_line(vec![tl]);
        model
    }

    /// Helper: create a model with variable BPM (min < max).
    fn make_model_variable_bpm() -> BMSModel {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_bpm(120.0);
        model.set_judgerank(100);
        // Two timelines with different BPMs → min != max
        let mut tl1 = bms_model::time_line::TimeLine::new(0.0, 0, 8);
        tl1.set_bpm(120.0);
        let mut tl2 = bms_model::time_line::TimeLine::new(1.0, 1_000_000, 8);
        tl2.set_bpm(180.0);
        model.set_all_time_line(vec![tl1, tl2]);
        model
    }

    #[test]
    fn non_modifier_assist_bpmguide_uniform_bpm_no_assist() {
        let model = make_model_uniform_bpm();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.bpmguide = true; // BPM guide enabled

        let score = player.calculate_non_modifier_assist(&config);
        // Uniform BPM: min == max → BPM guide has no effect
        assert_eq!(player.assist, 0);
        assert!(score, "Score should remain valid with uniform BPM");
    }

    #[test]
    fn non_modifier_assist_bpmguide_variable_bpm_sets_light_assist() {
        let model = make_model_variable_bpm();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.bpmguide = true; // BPM guide enabled

        let score = player.calculate_non_modifier_assist(&config);
        // Variable BPM: min < max → assist = max(0, 1) = 1
        assert_eq!(player.assist, 1);
        assert!(
            !score,
            "Score should be invalid with BPM guide on variable BPM"
        );
    }

    #[test]
    fn non_modifier_assist_bpmguide_disabled_no_assist() {
        let model = make_model_variable_bpm();
        let mut player = BMSPlayer::new(model);
        let config = make_default_config(); // bpmguide defaults to false

        let score = player.calculate_non_modifier_assist(&config);
        assert_eq!(player.assist, 0);
        assert!(score);
    }

    #[test]
    fn non_modifier_assist_custom_judge_all_rates_lte_100_no_assist() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.custom_judge = true;
        // Set all rates to <= 100
        config.key_judge_window_rate_perfect_great = 100;
        config.key_judge_window_rate_great = 100;
        config.key_judge_window_rate_good = 100;
        config.scratch_judge_window_rate_perfect_great = 100;
        config.scratch_judge_window_rate_great = 100;
        config.scratch_judge_window_rate_good = 100;

        let score = player.calculate_non_modifier_assist(&config);
        assert_eq!(player.assist, 0);
        assert!(score);
    }

    #[test]
    fn non_modifier_assist_custom_judge_one_rate_over_100_sets_assist() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.custom_judge = true;
        // Only one rate > 100
        config.key_judge_window_rate_perfect_great = 101;
        config.key_judge_window_rate_great = 50;
        config.key_judge_window_rate_good = 50;
        config.scratch_judge_window_rate_perfect_great = 50;
        config.scratch_judge_window_rate_great = 50;
        config.scratch_judge_window_rate_good = 50;

        let score = player.calculate_non_modifier_assist(&config);
        assert_eq!(player.assist, 2);
        assert!(
            !score,
            "Score should be invalid with custom judge rate > 100"
        );
    }

    #[test]
    fn non_modifier_assist_custom_judge_scratch_rate_over_100_sets_assist() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.custom_judge = true;
        config.key_judge_window_rate_perfect_great = 50;
        config.key_judge_window_rate_great = 50;
        config.key_judge_window_rate_good = 50;
        config.scratch_judge_window_rate_perfect_great = 50;
        config.scratch_judge_window_rate_great = 50;
        config.scratch_judge_window_rate_good = 200; // Only scratch good > 100

        let score = player.calculate_non_modifier_assist(&config);
        assert_eq!(player.assist, 2);
        assert!(!score);
    }

    #[test]
    fn non_modifier_assist_custom_judge_disabled_no_assist() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.custom_judge = false; // Disabled
        // Even with high rates, custom judge is off
        config.key_judge_window_rate_perfect_great = 400;

        let score = player.calculate_non_modifier_assist(&config);
        assert_eq!(player.assist, 0);
        assert!(score);
    }

    #[test]
    fn non_modifier_assist_constant_speed_enabled_sets_assist() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.mode7.playconfig.enable_constant = true; // Enable constant speed for BEAT_7K

        let score = player.calculate_non_modifier_assist(&config);
        assert_eq!(player.assist, 2);
        assert!(!score, "Score should be invalid with constant speed");
    }

    #[test]
    fn non_modifier_assist_constant_speed_disabled_no_assist() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let config = make_default_config(); // enable_constant defaults to false

        let score = player.calculate_non_modifier_assist(&config);
        assert_eq!(player.assist, 0);
        assert!(score);
    }

    #[test]
    fn non_modifier_assist_accumulates_bpmguide_and_constant() {
        // BPM guide → assist=1, constant → assist=max(1,2)=2
        let model = make_model_variable_bpm();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.bpmguide = true;
        config.mode7.playconfig.enable_constant = true;

        let score = player.calculate_non_modifier_assist(&config);
        assert_eq!(
            player.assist, 2,
            "Assist should accumulate to max (BPM guide=1, constant=2)"
        );
        assert!(!score);
    }

    #[test]
    fn non_modifier_assist_preserves_existing_assist() {
        // If assist was already set to 1 by pattern modifiers, non-modifier check
        // should keep the max
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.assist = 1; // Pre-set by pattern modifiers

        let mut config = make_default_config();
        config.mode7.playconfig.enable_constant = true; // Would set assist=2

        let score = player.calculate_non_modifier_assist(&config);
        assert_eq!(player.assist, 2, "Assist should be max(1, 2) = 2");
        assert!(!score);
    }

    // --- get_clear_type_for_assist tests (Phase 34d) ---

    #[test]
    fn clear_type_for_assist_0_returns_none() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        // assist defaults to 0
        assert!(player.get_clear_type_for_assist().is_none());
    }

    #[test]
    fn clear_type_for_assist_1_returns_light_assist_easy() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.assist = 1;
        assert_eq!(
            player.get_clear_type_for_assist(),
            Some(ClearType::LightAssistEasy)
        );
    }

    #[test]
    fn clear_type_for_assist_2_returns_noplay() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.assist = 2;
        assert_eq!(player.get_clear_type_for_assist(), Some(ClearType::NoPlay));
    }

    #[test]
    fn clear_type_for_assist_3_returns_noplay() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.assist = 3; // Any value >= 2 should be NoPlay
        assert_eq!(player.get_clear_type_for_assist(), Some(ClearType::NoPlay));
    }

    // --- init_playinfo_from_config tests (Phase 34e) ---

    #[test]
    fn init_playinfo_from_config_copies_random_options() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.random = 3;
        config.random2 = 5;
        config.doubleoption = 2;

        player.init_playinfo_from_config(&config);

        assert_eq!(player.playinfo.randomoption, 3);
        assert_eq!(player.playinfo.randomoption2, 5);
        assert_eq!(player.playinfo.doubleoption, 2);
    }

    #[test]
    fn init_playinfo_from_config_default_config_zeros() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let config = make_default_config();

        player.init_playinfo_from_config(&config);

        assert_eq!(player.playinfo.randomoption, 0);
        assert_eq!(player.playinfo.randomoption2, 0);
        assert_eq!(player.playinfo.doubleoption, 0);
    }

    #[test]
    fn init_playinfo_from_config_does_not_touch_seeds() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        let mut config = make_default_config();
        config.random = 2;

        player.init_playinfo_from_config(&config);

        // Seeds should remain at their default (-1 from ReplayData::new())
        assert_eq!(player.playinfo.randomoptionseed, -1);
        assert_eq!(player.playinfo.randomoption2seed, -1);
    }

    // --- End-to-end DP flow tests (Phase 34e) ---

    #[test]
    fn e2e_dp_flow_config_init_build_encode() {
        // End-to-end test: config → init → build → encode
        // DP mode (BEAT_14K) with FLIP (doubleoption=1), random=2, random2=3
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);

        let mut config = make_default_config();
        config.random = 2;
        config.random2 = 3;
        config.doubleoption = 1;

        // Step 1: init from config
        player.init_playinfo_from_config(&config);
        assert_eq!(player.playinfo.randomoption, 2);
        assert_eq!(player.playinfo.randomoption2, 3);
        assert_eq!(player.playinfo.doubleoption, 1);

        // Step 2: build pattern modifiers
        player.build_pattern_modifiers(&config);

        // Step 3: encode option
        // Expected: randomoption + randomoption2 * 10 + doubleoption * 100
        // = 2 + 3 * 10 + 1 * 100 = 132
        assert_eq!(player.encode_option_for_score(), 132);
    }

    #[test]
    fn e2e_dp_flow_replay_overrides_config() {
        // Config sets random=2, random2=3, doubleoption=1
        // Replay pattern key overrides to random=5, random2=7, doubleoption=0
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);

        let mut config = make_default_config();
        config.random = 2;
        config.random2 = 3;
        config.doubleoption = 1;

        // Step 1: init from config
        player.init_playinfo_from_config(&config);

        // Step 2: replay overrides
        let mut replay = ReplayData::new();
        replay.randomoption = 5;
        replay.randomoptionseed = 42;
        replay.randomoption2 = 7;
        replay.randomoption2seed = 84;
        replay.doubleoption = 0;
        replay.rand = vec![1, 2];

        let key_state = ReplayKeyState {
            pattern_key: true,
            ..Default::default()
        };
        player.restore_replay_data(Some(replay), &key_state);

        // After replay override, playinfo should reflect replay values
        assert_eq!(player.playinfo.randomoption, 5);
        assert_eq!(player.playinfo.randomoption2, 7);
        assert_eq!(player.playinfo.doubleoption, 0);

        // Step 3: build pattern modifiers (uses overridden values)
        player.build_pattern_modifiers(&config);

        // Step 4: encode option
        // = 5 + 7 * 10 + 0 * 100 = 75
        assert_eq!(player.encode_option_for_score(), 75);
    }

    #[test]
    fn e2e_sp_mode_ignores_2p_options() {
        // SP mode (BEAT_7K) end-to-end: 2P options should be ignored in encoding
        let model = make_model(); // BEAT_7K
        let mut player = BMSPlayer::new(model);

        let mut config = make_default_config();
        config.random = 3;
        config.random2 = 5; // Should be irrelevant in SP
        config.doubleoption = 1; // Should be irrelevant in SP

        // Step 1: init from config
        player.init_playinfo_from_config(&config);
        // All values are copied to playinfo
        assert_eq!(player.playinfo.randomoption, 3);
        assert_eq!(player.playinfo.randomoption2, 5);
        assert_eq!(player.playinfo.doubleoption, 1);

        // Step 2: build pattern modifiers
        player.build_pattern_modifiers(&config);

        // Step 3: encode option — SP mode only uses randomoption
        // player_count == 1, so result is just randomoption
        assert_eq!(player.encode_option_for_score(), 3);
    }

    #[test]
    fn e2e_dp_battle_mode_config_init_build_encode() {
        // DP battle mode: SP BEAT_7K with doubleoption=2 → converts to BEAT_14K
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);

        let mut config = make_default_config();
        config.random = 1;
        config.random2 = 4;
        config.doubleoption = 2;

        // Step 1: init from config
        player.init_playinfo_from_config(&config);

        // Step 2: build pattern modifiers (converts SP to DP)
        let score = player.build_pattern_modifiers(&config);
        assert!(!score, "Battle mode should invalidate score");
        assert_eq!(player.get_mode(), Mode::BEAT_14K);

        // Step 3: encode option — now in DP mode (player=2)
        // = 1 + 4 * 10 + 2 * 100 = 241
        assert_eq!(player.encode_option_for_score(), 241);
    }

    // --- apply_freq_trainer tests (Phase 34f) ---

    #[test]
    fn freq_trainer_freq_100_returns_none() {
        let model = make_model_with_time(10000);
        let mut player = BMSPlayer::new(model);
        let result = player.apply_freq_trainer(100, true, false, &FrequencyType::FREQUENCY);
        assert!(result.is_none(), "freq=100 should return None (no change)");
    }

    #[test]
    fn freq_trainer_freq_0_returns_none() {
        let model = make_model_with_time(10000);
        let mut player = BMSPlayer::new(model);
        let result = player.apply_freq_trainer(0, true, false, &FrequencyType::FREQUENCY);
        assert!(result.is_none(), "freq=0 should return None (no change)");
    }

    #[test]
    fn freq_trainer_not_play_mode_returns_none() {
        let model = make_model_with_time(10000);
        let mut player = BMSPlayer::new(model);
        let result = player.apply_freq_trainer(150, false, false, &FrequencyType::FREQUENCY);
        assert!(
            result.is_none(),
            "Not play mode should return None (no change)"
        );
    }

    #[test]
    fn freq_trainer_course_mode_returns_none() {
        let model = make_model_with_time(10000);
        let mut player = BMSPlayer::new(model);
        let result = player.apply_freq_trainer(150, true, true, &FrequencyType::FREQUENCY);
        assert!(
            result.is_none(),
            "Course mode should return None (no change)"
        );
    }

    #[test]
    fn freq_trainer_freq_150_adjusts_playtime() {
        let model = make_model_with_time(10000);
        let mut player = BMSPlayer::new(model);
        let last_note_time = player.model.get_last_note_time();

        let result = player.apply_freq_trainer(150, true, false, &FrequencyType::FREQUENCY);
        assert!(result.is_some());

        // Expected: (lastNoteTime + 1000) * 100 / 150 + TIME_MARGIN
        let expected_playtime = (last_note_time + 1000) * 100 / 150 + TIME_MARGIN;
        assert_eq!(player.get_playtime(), expected_playtime);
    }

    #[test]
    fn freq_trainer_freq_50_adjusts_playtime() {
        let model = make_model_with_time(10000);
        let mut player = BMSPlayer::new(model);
        let last_note_time = player.model.get_last_note_time();

        let result = player.apply_freq_trainer(50, true, false, &FrequencyType::FREQUENCY);
        assert!(result.is_some());

        // Expected: (lastNoteTime + 1000) * 100 / 50 + TIME_MARGIN
        let expected_playtime = (last_note_time + 1000) * 100 / 50 + TIME_MARGIN;
        assert_eq!(player.get_playtime(), expected_playtime);
    }

    #[test]
    fn freq_trainer_freq_string_format() {
        let model = make_model_with_time(10000);
        let mut player = BMSPlayer::new(model);

        let result = player.apply_freq_trainer(150, true, false, &FrequencyType::FREQUENCY);
        let result = result.unwrap();
        assert_eq!(result.freq_string, "[1.50x]");

        // Test with freq=50
        let model2 = make_model_with_time(10000);
        let mut player2 = BMSPlayer::new(model2);
        let result2 = player2.apply_freq_trainer(50, true, false, &FrequencyType::FREQUENCY);
        let result2 = result2.unwrap();
        assert_eq!(result2.freq_string, "[0.50x]");

        // Test with freq=200
        let model3 = make_model_with_time(10000);
        let mut player3 = BMSPlayer::new(model3);
        let result3 = player3.apply_freq_trainer(200, true, false, &FrequencyType::FREQUENCY);
        let result3 = result3.unwrap();
        assert_eq!(result3.freq_string, "[2.00x]");
    }

    #[test]
    fn freq_trainer_global_pitch_set_when_frequency_type() {
        let model = make_model_with_time(10000);
        let mut player = BMSPlayer::new(model);

        let result = player.apply_freq_trainer(150, true, false, &FrequencyType::FREQUENCY);
        let result = result.unwrap();
        assert_eq!(result.global_pitch, Some(1.5));
    }

    #[test]
    fn freq_trainer_global_pitch_none_when_unprocessed() {
        let model = make_model_with_time(10000);
        let mut player = BMSPlayer::new(model);

        let result = player.apply_freq_trainer(150, true, false, &FrequencyType::UNPROCESSED);
        let result = result.unwrap();
        assert!(
            result.global_pitch.is_none(),
            "UNPROCESSED should not set global pitch"
        );
    }

    #[test]
    fn freq_trainer_result_fields_correct() {
        let model = make_model_with_time(10000);
        let mut player = BMSPlayer::new(model);

        let result = player.apply_freq_trainer(150, true, false, &FrequencyType::FREQUENCY);
        let result = result.unwrap();
        assert!(result.freq_on);
        assert!(result.force_no_ir_send);
    }

    #[test]
    fn freq_trainer_scales_chart_timing() {
        // Verify that change_frequency is called on the model
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        model.set_bpm(120.0);
        let mut tl = bms_model::time_line::TimeLine::new(0.0, 0, 8);
        tl.set_bpm(120.0);
        let mut tl2 = bms_model::time_line::TimeLine::new(1.0, 1_000_000, 8);
        tl2.set_bpm(120.0);
        tl2.set_note(0, Some(bms_model::note::Note::new_normal(1)));
        model.set_all_time_line(vec![tl, tl2]);
        let original_bpm = model.get_bpm();

        let mut player = BMSPlayer::new(model);
        player.apply_freq_trainer(150, true, false, &FrequencyType::FREQUENCY);

        // BPM should be scaled by 1.5
        let expected_bpm = original_bpm * 1.5;
        let actual_bpm = player.model.get_bpm();
        assert!(
            (actual_bpm - expected_bpm).abs() < 0.001,
            "BPM should be scaled: expected {}, got {}",
            expected_bpm,
            actual_bpm
        );
    }

    // --- Global pitch control tests ---

    #[test]
    fn set_play_speed_sets_pending_pitch_when_frequency_type() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_fast_forward_freq_option(FrequencyType::FREQUENCY);
        player.set_play_speed(150);
        assert_eq!(player.take_pending_global_pitch(), Some(1.5));
    }

    #[test]
    fn set_play_speed_no_pending_pitch_when_unprocessed() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_fast_forward_freq_option(FrequencyType::UNPROCESSED);
        player.set_play_speed(150);
        assert_eq!(player.take_pending_global_pitch(), None);
    }

    #[test]
    fn take_pending_global_pitch_clears_after_read() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_fast_forward_freq_option(FrequencyType::FREQUENCY);
        player.set_play_speed(200);
        assert_eq!(player.take_pending_global_pitch(), Some(2.0));
        // Second call should be None (consumed)
        assert_eq!(player.take_pending_global_pitch(), None);
    }

    #[test]
    fn stop_play_preload_sets_pending_pitch_to_one() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PRELOAD;
        player.stop_play();
        assert_eq!(player.take_pending_global_pitch(), Some(1.0));
    }

    #[test]
    fn stop_play_ready_sets_pending_pitch_to_one() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_READY;
        player.stop_play();
        assert_eq!(player.take_pending_global_pitch(), Some(1.0));
    }

    #[test]
    fn stop_play_failed_state_sets_pending_pitch_to_one() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PLAY;
        // Ensure no notes judged and no prior timer
        player.stop_play();
        // This goes to ABORTED (no notes judged), no pitch reset here
        assert_eq!(player.state, STATE_ABORTED);
        // No pending pitch for ABORTED path (matches Java - only resets on failed path)
        assert_eq!(player.take_pending_global_pitch(), None);
    }

    #[test]
    fn stop_play_failed_path_sets_pending_pitch_to_one() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PLAY;

        // Simulate some notes judged (not finished but notes exist)
        // Force the judge counts so we enter the failed branch
        player.judge.get_score_data_mut().epg = 5; // 5 early PGreats
        player.total_notes = 100; // not all past
        player.stop_play();
        assert_eq!(player.state, STATE_FAILED);
        assert_eq!(player.take_pending_global_pitch(), Some(1.0));
    }

    // --- Loudness analysis tests ---

    #[test]
    fn apply_loudness_analysis_success() {
        use beatoraja_audio::bms_loudness_analyzer::AnalysisResult;

        let model = make_model();
        let mut player = BMSPlayer::new(model);
        assert!(!player.is_analysis_checked());

        let result = AnalysisResult::new_success(-14.0);
        let vol = player.apply_loudness_analysis(&result, 1.0);
        assert!(player.is_analysis_checked());
        assert!(vol > 0.0 && vol <= 1.0);
        assert!((player.get_adjusted_volume() - vol).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_loudness_analysis_failure() {
        use beatoraja_audio::bms_loudness_analyzer::AnalysisResult;

        let model = make_model();
        let mut player = BMSPlayer::new(model);

        let result = AnalysisResult::new_error("test error".to_string());
        let vol = player.apply_loudness_analysis(&result, 1.0);
        assert!(player.is_analysis_checked());
        assert!((vol - (-1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_loudness_analysis_preserves_base_volume_on_failure() {
        use beatoraja_audio::bms_loudness_analyzer::AnalysisResult;

        let model = make_model();
        let mut player = BMSPlayer::new(model);

        let result = AnalysisResult::new_error("err".to_string());
        player.apply_loudness_analysis(&result, 0.8);
        // adjusted_volume should be -1.0 on failure
        assert!((player.get_adjusted_volume() - (-1.0)).abs() < f32::EPSILON);
    }

    // --- Guide SE config tests ---

    #[test]
    fn build_guide_se_config_disabled_returns_all_none() {
        let sm = beatoraja_core::system_sound_manager::SystemSoundManager::new(None, None);
        let config = BMSPlayer::build_guide_se_config(false, &sm);
        assert_eq!(config.len(), 6);
        for (i, (judge, path)) in config.iter().enumerate() {
            assert_eq!(*judge, i as i32);
            assert!(path.is_none(), "judge {} should have None path", i);
        }
    }

    #[test]
    fn build_guide_se_config_enabled_returns_six_entries() {
        // Without actual sound files, paths will be None (no files found)
        let sm = beatoraja_core::system_sound_manager::SystemSoundManager::new(None, None);
        let config = BMSPlayer::build_guide_se_config(true, &sm);
        assert_eq!(config.len(), 6);
        // All entries should exist (though paths may be None since no actual sound files)
        for (i, (judge, _path)) in config.iter().enumerate() {
            assert_eq!(*judge, i as i32);
        }
    }

    // --- Fast forward freq option tests ---

    #[test]
    fn set_fast_forward_freq_option_stored() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_fast_forward_freq_option(FrequencyType::FREQUENCY);
        player.set_play_speed(75);
        assert_eq!(player.take_pending_global_pitch(), Some(0.75));
    }

    // --- Phase 43a: create() side effects tests ---

    #[test]
    fn create_produces_side_effects() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.create();
        let effects = player.take_create_side_effects();
        assert!(effects.is_some(), "create() should produce side effects");
    }

    #[test]
    fn create_side_effects_consumed_after_take() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.create();
        let _ = player.take_create_side_effects();
        assert!(
            player.take_create_side_effects().is_none(),
            "second take should return None"
        );
    }

    #[test]
    fn create_side_effects_skin_type_matches_model() {
        let model = make_model(); // BEAT_7K
        let mut player = BMSPlayer::new(model);
        player.create();
        let effects = player.take_create_side_effects().unwrap();
        assert_eq!(effects.skin_type, Some(SkinType::Play7Keys));
    }

    #[test]
    fn create_side_effects_skin_type_5k() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_5K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);
        player.create();
        let effects = player.take_create_side_effects().unwrap();
        assert_eq!(effects.skin_type, Some(SkinType::Play5Keys));
    }

    #[test]
    fn create_side_effects_skin_type_14k() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_14K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);
        player.create();
        let effects = player.take_create_side_effects().unwrap();
        assert_eq!(effects.skin_type, Some(SkinType::Play14Keys));
    }

    #[test]
    fn create_side_effects_input_mode_play() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_play_mode(BMSPlayerMode::PLAY);
        player.create();
        let effects = player.take_create_side_effects().unwrap();
        assert_eq!(
            effects.input_mode_action,
            InputModeAction::SetPlayConfig(Mode::BEAT_7K)
        );
    }

    #[test]
    fn create_side_effects_input_mode_practice() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_play_mode(BMSPlayerMode::PRACTICE);
        player.create();
        let effects = player.take_create_side_effects().unwrap();
        assert_eq!(
            effects.input_mode_action,
            InputModeAction::SetPlayConfig(Mode::BEAT_7K)
        );
    }

    #[test]
    fn create_side_effects_input_mode_autoplay() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_play_mode(BMSPlayerMode::AUTOPLAY);
        player.create();
        let effects = player.take_create_side_effects().unwrap();
        assert_eq!(effects.input_mode_action, InputModeAction::DisableInput);
    }

    #[test]
    fn create_side_effects_input_mode_replay() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_play_mode(BMSPlayerMode::REPLAY_1);
        player.create();
        let effects = player.take_create_side_effects().unwrap();
        assert_eq!(effects.input_mode_action, InputModeAction::DisableInput);
    }

    #[test]
    fn create_side_effects_guide_se_disabled() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_guide_se(false);
        player.create();
        let effects = player.take_create_side_effects().unwrap();
        assert!(!effects.is_guide_se);
    }

    #[test]
    fn create_side_effects_guide_se_enabled() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_guide_se(true);
        player.create();
        let effects = player.take_create_side_effects().unwrap();
        assert!(effects.is_guide_se);
    }

    #[test]
    fn create_no_speed_disables_control() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_constraints(vec![CourseDataConstraint::NoSpeed]);
        player.create();
        // Verify control is disabled by checking its enable_control field
        let control = player.control.as_ref().unwrap();
        assert!(!control.is_enable_control());
    }

    #[test]
    fn create_without_no_speed_keeps_control_enabled() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_constraints(vec![CourseDataConstraint::Class]);
        player.create();
        let control = player.control.as_ref().unwrap();
        assert!(control.is_enable_control());
    }

    #[test]
    fn create_empty_constraints_keeps_control_enabled() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_constraints(vec![]);
        player.create();
        let control = player.control.as_ref().unwrap();
        assert!(control.is_enable_control());
    }

    #[test]
    fn create_practice_mode_sets_state_practice() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_play_mode(BMSPlayerMode::PRACTICE);
        player.create();
        assert_eq!(player.get_state(), STATE_PRACTICE);
    }

    #[test]
    fn create_play_mode_keeps_state_preload() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_play_mode(BMSPlayerMode::PLAY);
        player.create();
        assert_eq!(player.get_state(), STATE_PRELOAD);
    }

    #[test]
    fn create_note_expansion_rate_default_no_expansion() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        // Default PlaySkin has [100, 100] — no expansion
        player.create();
        // Rhythm processor should be created (existence is enough to verify create ran)
        assert!(player.rhythm.is_some());
    }

    #[test]
    fn create_note_expansion_rate_custom_triggers_expansion() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        // Set custom expansion rate before create
        player.play_skin.set_note_expansion_rate([120, 100]);
        player.create();
        assert!(player.rhythm.is_some());
    }

    #[test]
    fn set_play_mode_and_get() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_play_mode(BMSPlayerMode::AUTOPLAY);
        assert_eq!(
            player.get_play_mode().mode,
            beatoraja_core::bms_player_mode::Mode::Autoplay
        );
    }

    #[test]
    fn set_constraints_and_get() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_constraints(vec![
            CourseDataConstraint::NoSpeed,
            CourseDataConstraint::Class,
        ]);
        assert_eq!(player.get_constraints().len(), 2);
        assert!(
            player
                .get_constraints()
                .contains(&CourseDataConstraint::NoSpeed)
        );
        assert!(
            player
                .get_constraints()
                .contains(&CourseDataConstraint::Class)
        );
    }

    #[test]
    fn default_play_mode_is_play() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        assert_eq!(
            player.get_play_mode().mode,
            beatoraja_core::bms_player_mode::Mode::Play
        );
    }

    #[test]
    fn default_constraints_empty() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        assert!(player.get_constraints().is_empty());
    }

    #[test]
    fn default_guide_se_disabled() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        assert!(!player.is_guide_se);
    }

    #[test]
    fn create_side_effects_none_before_create() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        assert!(player.take_create_side_effects().is_none());
    }

    #[test]
    fn create_input_mode_5k_model_with_play_mode() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_5K);
        model.set_judgerank(100);
        let mut player = BMSPlayer::new(model);
        player.set_play_mode(BMSPlayerMode::PLAY);
        player.create();
        let effects = player.take_create_side_effects().unwrap();
        assert_eq!(
            effects.input_mode_action,
            InputModeAction::SetPlayConfig(Mode::BEAT_5K)
        );
    }

    #[test]
    fn create_no_speed_among_multiple_constraints() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.set_constraints(vec![
            CourseDataConstraint::Class,
            CourseDataConstraint::NoSpeed,
            CourseDataConstraint::Mirror,
        ]);
        player.create();
        let control = player.control.as_ref().unwrap();
        assert!(!control.is_enable_control());
    }

    // --- save_config tests ---

    #[test]
    fn save_config_skips_when_no_speed_constraint() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.lanerender = Some(LaneRenderer::new(&player.model));
        player.set_constraints(vec![CourseDataConstraint::NoSpeed]);

        // Set a known state on the lane renderer
        let pc_before = player
            .player_config
            .get_play_config_ref(Mode::BEAT_7K)
            .get_playconfig()
            .clone();

        player.save_config();

        // Config should not have changed
        let pc_after = player
            .player_config
            .get_play_config_ref(Mode::BEAT_7K)
            .get_playconfig();
        assert_eq!(pc_before.hispeed, pc_after.hispeed);
        assert_eq!(pc_before.lanecover, pc_after.lanecover);
    }

    #[test]
    fn save_config_saves_lane_renderer_state() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.lanerender = Some(LaneRenderer::new(&player.model));

        // Default fixhispeed is FIX_HISPEED_MAINBPM (not OFF), so duration should be saved
        player.save_config();

        let pc = player
            .player_config
            .get_play_config_ref(Mode::BEAT_7K)
            .get_playconfig();
        // Duration should be set from lane renderer (default duration)
        let lr_duration = player.lanerender.as_ref().unwrap().get_duration();
        assert_eq!(pc.duration, lr_duration);
    }

    #[test]
    fn save_config_saves_hispeed_when_fixhispeed_off() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.lanerender = Some(LaneRenderer::new(&player.model));

        // Set fixhispeed to OFF
        player
            .player_config
            .get_play_config(Mode::BEAT_7K)
            .get_playconfig_mut()
            .fixhispeed = beatoraja_types::play_config::FIX_HISPEED_OFF;

        player.save_config();

        let pc = player
            .player_config
            .get_play_config_ref(Mode::BEAT_7K)
            .get_playconfig();
        let lr_hispeed = player.lanerender.as_ref().unwrap().get_hispeed();
        assert_eq!(pc.hispeed, lr_hispeed);
    }

    // --- media_load_finished tests ---

    #[test]
    fn preload_does_not_transition_when_media_not_loaded() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.play_skin.set_loadstart(0);
        player.play_skin.set_loadend(0);
        player.media_load_finished = false; // Media not loaded
        player.startpressedtime = -2_000_000;

        std::thread::sleep(std::time::Duration::from_millis(2));
        player.main_state_data.timer.update();
        player.render();

        // Should stay in PRELOAD because media not loaded
        assert_eq!(player.get_state(), STATE_PRELOAD);
    }

    // --- input state wiring tests ---

    #[test]
    fn set_input_state_updates_fields() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);

        player.set_input_state(true, false, &[true, false, true]);
        assert!(player.input_start_pressed);
        assert!(!player.input_select_pressed);
        assert_eq!(player.input_key_states, vec![true, false, true]);

        player.set_input_state(false, true, &[false]);
        assert!(!player.input_start_pressed);
        assert!(player.input_select_pressed);
        assert_eq!(player.input_key_states, vec![false]);
    }

    // --- startpressedtime tracking tests ---

    #[test]
    fn startpressedtime_updates_when_start_pressed() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.input_start_pressed = true;
        player.startpressedtime = -999;

        std::thread::sleep(std::time::Duration::from_millis(1));
        player.main_state_data.timer.update();
        player.render();

        // startpressedtime should have been updated to micronow
        assert!(player.startpressedtime > -999);
    }

    // --- gauge auto shift tests ---

    #[test]
    fn gauge_autoshift_continue_does_not_fail() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PLAY;
        player.playtime = 999_999;
        player.player_config.gauge_auto_shift =
            beatoraja_types::player_config::GAUGEAUTOSHIFT_CONTINUE;

        let gauge = crate::groove_gauge::create_groove_gauge(
            &player.model,
            beatoraja_types::groove_gauge::HARD,
            0,
            None,
        )
        .unwrap();
        player.gauge = Some(gauge);
        player.gauge.as_mut().unwrap().set_value(0.0);

        player.main_state_data.timer.update();
        let now = player.main_state_data.timer.get_now_micro_time();
        player
            .main_state_data
            .timer
            .set_micro_timer(TIMER_PLAY, now - 1000);
        player.prevtime = now - 500;

        player.render();

        // Should NOT transition to FAILED with CONTINUE mode
        assert_eq!(player.get_state(), STATE_PLAY);
    }

    #[test]
    fn gauge_autoshift_survival_to_groove_shifts_type() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PLAY;
        player.playtime = 999_999;
        player.player_config.gauge_auto_shift =
            beatoraja_types::player_config::GAUGEAUTOSHIFT_SURVIVAL_TO_GROOVE;

        let gauge = crate::groove_gauge::create_groove_gauge(
            &player.model,
            beatoraja_types::groove_gauge::HARD,
            0,
            None,
        )
        .unwrap();
        player.gauge = Some(gauge);
        player.gauge.as_mut().unwrap().set_value(0.0);

        player.main_state_data.timer.update();
        let now = player.main_state_data.timer.get_now_micro_time();
        player
            .main_state_data
            .timer
            .set_micro_timer(TIMER_PLAY, now - 1000);
        player.prevtime = now - 500;

        player.render();

        // Should shift to NORMAL gauge type, not FAILED
        assert_eq!(player.get_state(), STATE_PLAY);
        assert_eq!(
            player.gauge.as_ref().unwrap().get_type(),
            beatoraja_types::groove_gauge::NORMAL
        );
    }

    // --- quick retry tests ---

    #[test]
    fn quick_retry_in_failed_state_with_start_xor_select() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_FAILED;
        player.lanerender = Some(LaneRenderer::new(&player.model));
        player.keyinput = Some(KeyInputProccessor::new(&LaneProperty::new(&Mode::BEAT_7K)));
        player.set_play_mode(BMSPlayerMode::PLAY);
        player.is_course_mode = false;

        // START pressed, SELECT not pressed (XOR = true)
        player.input_start_pressed = true;
        player.input_select_pressed = false;

        player.main_state_data.timer.update();
        player.render();

        // Should request transition to PLAY (quick retry)
        let state_change = player.take_pending_state_change();
        assert_eq!(state_change, Some(MainStateType::Play));
    }

    #[test]
    fn no_quick_retry_in_course_mode() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_FAILED;
        player.lanerender = Some(LaneRenderer::new(&player.model));
        player.keyinput = Some(KeyInputProccessor::new(&LaneProperty::new(&Mode::BEAT_7K)));
        player.set_play_mode(BMSPlayerMode::PLAY);
        player.is_course_mode = true;

        player.input_start_pressed = true;
        player.input_select_pressed = false;

        player.main_state_data.timer.update();
        player.render();

        // Quick retry should NOT trigger in course mode
        // (only TIMER_FAILED timeout transition should happen)
        let state_change = player.take_pending_state_change();
        assert_ne!(state_change, Some(MainStateType::Play));
    }

    #[test]
    fn aborted_quick_retry_with_start_xor_select() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_ABORTED;
        player.lanerender = Some(LaneRenderer::new(&player.model));
        player.set_play_mode(BMSPlayerMode::PLAY);
        player.is_course_mode = false;

        // SELECT pressed, START not pressed (XOR = true)
        player.input_start_pressed = false;
        player.input_select_pressed = true;

        player.main_state_data.timer.update();
        player.render();

        // Should request transition to PLAY
        let state_change = player.take_pending_state_change();
        assert_eq!(state_change, Some(MainStateType::Play));
    }

    // --- state transition tests ---

    #[test]
    fn failed_transitions_to_practice_in_practice_mode() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_FAILED;
        player.lanerender = Some(LaneRenderer::new(&player.model));
        player.keyinput = Some(KeyInputProccessor::new(&LaneProperty::new(&Mode::BEAT_7K)));
        player.set_play_mode(BMSPlayerMode::PRACTICE);

        // Set TIMER_FAILED so close time is exceeded
        player.main_state_data.timer.set_timer_on(TIMER_FAILED);
        player.main_state_data.timer.update();
        let now = player.main_state_data.timer.get_now_micro_time();
        player
            .main_state_data
            .timer
            .set_micro_timer(TIMER_FAILED, now - 10_000_000);
        player.play_skin.set_close(0);

        player.render();

        // In practice mode, should return to STATE_PRACTICE
        assert_eq!(player.get_state(), STATE_PRACTICE);
    }

    #[test]
    fn pending_state_change_consumed_once() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.pending_state_change = Some(MainStateType::Result);

        let first = player.take_pending_state_change();
        assert_eq!(first, Some(MainStateType::Result));

        let second = player.take_pending_state_change();
        assert_eq!(second, None);
    }

    // --- chart preview tests ---

    #[test]
    fn chart_preview_sets_timer_141_when_enabled() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_PRELOAD;
        player.player_config.chart_preview = true;
        player.startpressedtime = 0;

        // When micronow == startpressedtime and timer 141 is off, timer 141 should be set
        player.main_state_data.timer.update();
        let micronow = player.main_state_data.timer.get_now_micro_time();
        player.startpressedtime = micronow;

        player.render();

        // Timer 141 should have been set
        assert!(player.main_state_data.timer.is_timer_on(141));
    }

    // --- player config wiring tests ---

    #[test]
    fn set_player_config_persists() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);

        let mut config = PlayerConfig::default();
        config.chart_preview = false;
        config.is_window_hold = true;
        config.gauge_auto_shift = 3;

        player.set_player_config(config);

        assert!(!player.get_player_config().chart_preview);
        assert!(player.get_player_config().is_window_hold);
        assert_eq!(player.get_player_config().gauge_auto_shift, 3);
    }

    // --- course mode tests ---

    #[test]
    fn set_course_mode_persists() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);

        player.set_course_mode(true);
        assert!(player.is_course_mode);

        player.set_course_mode(false);
        assert!(!player.is_course_mode);
    }
}
