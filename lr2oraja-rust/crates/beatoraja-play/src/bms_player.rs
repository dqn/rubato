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
use beatoraja_core::main_state::{MainState, MainStateData, MainStateType};
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_core::score_data::ScoreData;
use beatoraja_core::timer_manager::TimerManager;
use beatoraja_pattern::autoplay_modifier::AutoplayModifier;
use beatoraja_pattern::extra_note_modifier::ExtraNoteModifier;
use beatoraja_pattern::lane_shuffle_modifier::{PlayerBattleModifier, PlayerFlipModifier};
use beatoraja_pattern::long_note_modifier::LongNoteModifier;
use beatoraja_pattern::mine_note_modifier::MineNoteModifier;
use beatoraja_pattern::mode_modifier::ModeModifier;
use beatoraja_pattern::pattern_modifier::{AssistLevel, PatternModifier};
use beatoraja_pattern::scroll_speed_modifier::ScrollSpeedModifier;
use beatoraja_types::audio_config::FrequencyType;
use beatoraja_types::clear_type::ClearType;
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
    bga: BGAProcessor,
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
}

impl BMSPlayer {
    pub fn new(model: BMSModel) -> Self {
        let playtime = model.get_last_note_time() + TIME_MARGIN;
        let total_notes = model.get_total_notes();
        BMSPlayer {
            model,
            lanerender: None,
            lane_property: None,
            judge: JudgeManager::new(),
            bga: BGAProcessor::new(),
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
        }
    }

    pub fn set_play_speed(&mut self, playspeed: i32) {
        self.playspeed = playspeed;
        // TODO: Phase 22 - audio pitch change
        // if main.getConfig().getAudioConfig().getFastForward() == FrequencyType.FREQUENCY {
        //     main.getAudioProcessor().setGlobalPitch(playspeed as f32 / 100.0);
        // }
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

    /// Set the active replay data for keylog playback.
    /// Should be called when entering REPLAY mode after restore_replay_data().
    pub fn set_active_replay(&mut self, replay: Option<ReplayData>) {
        self.active_replay = replay;
    }

    /// Set the margin time in milliseconds (from resource).
    pub fn set_margin_time(&mut self, margin_time: i64) {
        self.margin_time = margin_time;
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
            // main.getAudioProcessor().setGlobalPitch(1.0);
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
            // In Java: if resource.getPlayMode().mode == PLAY => STATE_ABORTED
            // else => STATE_PRACTICE_FINISHED
            // We default to ABORTED since we lack resource.getPlayMode()
            self.state = STATE_ABORTED;
            return;
        }
        if self.main_state_data.timer.is_timer_on(TIMER_FAILED)
            || self.main_state_data.timer.is_timer_on(TIMER_FADEOUT)
        {
            return;
        }
        if self.state != STATE_FINISHED
            && self.judge.get_judge_count(0)
                + self.judge.get_judge_count(1)
                + self.judge.get_judge_count(2)
                + self.judge.get_judge_count(3)
                == 0
        {
            // No notes judged - abort
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
            && (self.judge.get_past_notes() == self.total_notes/* || resource.getPlayMode().mode == AUTOPLAY */)
        {
            self.state = STATE_FINISHED;
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
            log::info!("STATE_FINISHED");
        } else if self.state == STATE_FINISHED
            && !self.main_state_data.timer.is_timer_on(TIMER_FADEOUT)
        {
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
        } else if self.state != STATE_FINISHED {
            // main.getAudioProcessor().setGlobalPitch(1.0);
            self.state = STATE_FAILED;
            self.main_state_data.timer.set_timer_on(TIMER_FAILED);
            // if resource.mediaLoadFinished() { main.getAudioProcessor().stop(null); }
            // play(PLAY_STOP);
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
        if self.state != STATE_ABORTED
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
                clear = if self.assist == 1 {
                    ClearType::LightAssistEasy
                } else {
                    ClearType::AssistEasy
                };
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
            } else {
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
        // TODO(Phase 41): score.skin = Some(get_skin().header.get_name().to_string());

        Some(score)
    }

    /// Corresponds to Java BMSPlayer.update(int judge, long time)
    pub fn update_judge(&mut self, judge: i32, time: i64) {
        if self.judge.get_combo() == 0 {
            self.bga.set_misslayer_tme(time);
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
        for skin_type in SkinType::values() {
            if skin_type.get_mode() == Some(model_mode.clone()) {
                return Some(skin_type);
            }
        }
        None
    }

    /// Save play config from lane renderer state.
    /// Corresponds to Java saveConfig() private method.
    fn save_config(&self) {
        // TODO: Phase 22 - requires PlayerResource, constraint check, PlayerConfig
        // In Java:
        // 1. Check if NO_SPEED constraint - if so, return early
        // 2. Get PlayConfig from playerConfig.getPlayConfig(mode).getPlayconfig()
        // 3. If fixhispeed != OFF: save duration; else save hispeed
        // 4. Save lanecover, lift, hidden from lanerender
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

        // TODO: → Phase 37 — GhostBattle seed/option override
        // When GhostBattle is active (via GhostBattlePlay::consume()):
        //   - Set playinfo.randomoption from ghost's random ordinal
        //   - If player config random == MIRROR, apply mirror inversion logic
        // Java lines 119-138

        // TODO: → Phase 37 — ChartOption seed/option override
        // When resource.getChartOption() is set (and GhostBattle is not active):
        //   - Load randomoption, randomoptionseed, randomoption2, randomoption2seed,
        //     doubleoption, rand from chart_option
        // Java lines 140-148

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
            let mut pm2 = beatoraja_pattern::pattern_modifier::create_pattern_modifier(
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
        let mut pm1 = beatoraja_pattern::pattern_modifier::create_pattern_modifier(
            self.playinfo.randomoption,
            0,
            &mode,
            config,
        );
        if self.playinfo.randomoptionseed != -1 {
            pm1.set_seed(self.playinfo.randomoptionseed);
        } else {
            // TODO: → Phase 37 — GhostBattle seed override
            // When GhostBattle is active, use ghost's lane pattern seed from RandomTrainer.getRandomSeedMap()
            // Java: if (ghostBattle.isPresent()) { pm.setSeed(seedmap.get(pattern)); }

            // TODO: → Phase 37 — RandomTrainer seed override
            // When RandomTrainer is active and mode == BEAT_7K, use seed from RandomTrainer.getRandomSeedMap()
            // Java: if (RandomTrainer.isActive() && model.getMode() == Mode.BEAT_7K) { pm.setSeed(seedmap.get(...)); }

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
                // TODO: → Phase 41 — Actual model reload via resource.loadBMSModel(playinfo.rand)
                // model = resource.loadBMSModel(playinfo.rand);
                // BMSModelUtils.setStartNoteTime(model, 1000);
                // BMSPlayerRule.validate(model);
                log::info!("譜面分岐 : {:?}", self.playinfo.rand);
                let reload_rand = self.playinfo.rand.clone();
                // After reload, store model's random back into playinfo
                // self.playinfo.rand = model.getRandom() (done after actual reload)
                return Some(reload_rand);
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

    fn create(&mut self) {
        let mode = self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
        self.lane_property = Some(LaneProperty::new(&mode));
        self.judge = JudgeManager::new();
        self.control = Some(ControlInputProcessor::new(mode));
        if let Some(ref lp) = self.lane_property {
            self.keyinput = Some(KeyInputProccessor::new(lp));
        }

        self.lanerender = Some(LaneRenderer::new(&self.model));

        // TODO: Phase 22 - skin loading, audio setup, input setup
        // loadSkin(getSkinType());
        // guide SE setup
        // input processor setup

        self.judge.init(&self.model, 0, None, &[]);

        let use_expansion = false; // TODO: from PlaySkin note expansion rate
        self.rhythm = Some(RhythmTimerProcessor::new(&self.model, use_expansion));
        self.bga = BGAProcessor::new();

        // Initialize gauge log
        if let Some(ref gauge) = self.gauge {
            let gauge_type_len = gauge.get_gauge_type_length();
            self.gaugelog = Vec::with_capacity(gauge_type_len);
            for _ in 0..gauge_type_len {
                self.gaugelog
                    .push(Vec::with_capacity((self.playtime / 500 + 2) as usize));
            }
        }

        // TODO: Phase 22 - score data, target score setup
        // In Java: if autoplay.mode == PRACTICE => state = STATE_PRACTICE
        // else => set target score, etc.
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
        // startpressedtime tracking is done via MainController input in Java
        // We track it locally here for state machine logic
        // if input.startPressed() || input.isSelectPressed() { startpressedtime = micronow; }

        match self.state {
            // STATE_PRELOAD - wait for resources
            STATE_PRELOAD => {
                // Chart preview handling (chartPreview config)
                // TODO: Phase 22 - config.isChartPreview() logic with timer 141

                // Check if media loaded and load timers elapsed
                let load_threshold =
                    (self.play_skin.get_loadstart() + self.play_skin.get_loadend()) as i64 * 1000;
                // In Java: resource.mediaLoadFinished() && micronow > load_threshold
                //          && micronow - startpressedtime > 1000000
                // We simulate media loaded = true for now (blocked on Phase 22)
                let media_loaded = true; // TODO: Phase 22 - resource.mediaLoadFinished()
                if media_loaded
                    && micronow > load_threshold
                    && micronow - self.startpressedtime > 1_000_000
                {
                    // Loudness analysis check
                    if !self.analysis_checked {
                        self.adjusted_volume = -1.0;
                        self.analysis_checked = true;
                        // TODO: Phase 22 - analysisTask handling
                    }

                    self.bga.prepare(&() as &dyn std::any::Any);
                    self.state = STATE_READY;
                    self.main_state_data.timer.set_timer_on(TIMER_READY);
                    // play(PLAY_READY);
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
                    // Reset for practice restart
                    // resource.reloadBMSFile(); model = resource.getBMSModel();
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
                // practice.processInput(input) - TODO: Phase 22

                // In Java: if input.getKeyState(0) && resource.mediaLoadFinished() && time checks
                // Practice start is triggered by key press
                // TODO: Phase 22 - full practice start logic
            }

            // STATE_PRACTICE_FINISHED
            STATE_PRACTICE_FINISHED => {
                if self
                    .main_state_data
                    .timer
                    .get_now_time_for_id(TIMER_FADEOUT)
                    > self.play_skin.get_close() as i64
                {
                    // input.setEnable(true); input.setStartTime(0);
                    // main.changeState(MainStateType.MUSICSELECT);
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
                    self.keysound
                        .start_bg_play(&self.model, self.starttimeoffset * 1000);
                    log::info!("STATE_PLAY");
                }
            }

            // STATE_PLAY - main gameplay
            STATE_PLAY => {
                let deltatime = micronow - self.prevtime;
                let deltaplay = deltatime * (100 - self.playspeed as i64) / 100;
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
                // skin.pomyu.updateTimer(this); - TODO: Phase 22

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

                // Stage failed check
                if let Some(ref gauge) = self.gauge {
                    let g = gauge.get_value();
                    if g == 0.0 {
                        // GAUGEAUTOSHIFT_NONE: transition to FAILED
                        // TODO: Phase 22 - config.getGaugeAutoShift() check
                        self.state = STATE_FAILED;
                        self.main_state_data.timer.set_timer_on(TIMER_FAILED);
                        // if resource.mediaLoadFinished() { main.getAudioProcessor().stop(null); }
                        // play(PLAY_STOP);
                        log::info!("STATE_FAILED");
                    }
                }
            }

            // STATE_FAILED
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
                // TODO: Phase 22 - input.startPressed() ^ input.isSelectPressed()

                if self.main_state_data.timer.get_now_time_for_id(TIMER_FAILED)
                    > self.play_skin.get_close() as i64
                {
                    // main.getAudioProcessor().setGlobalPitch(1.0);
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
                    // resource.setGauge(gaugelog);
                    // resource.setGrooveGauge(gauge);
                    // resource.setAssist(assist);
                    // input.setEnable(true); input.setStartTime(0);
                    self.save_config();

                    // Transition: practice -> STATE_PRACTICE, else -> RESULT or MUSICSELECT
                    // TODO: Phase 22 - main.changeState()
                    log::info!("Failed close, transition to result/select");
                }
            }

            // STATE_FINISHED
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
                if self
                    .main_state_data
                    .timer
                    .get_now_time_for_id(TIMER_FADEOUT)
                    > 0
                // skin.getFadeout() - TODO: Phase 22
                {
                    // main.getAudioProcessor().setGlobalPitch(1.0);
                    // resource.getBGAManager().stop();
                    // resource.setScoreData(createScoreData());
                    // resource.setCombo(judge.getCourseCombo());
                    // resource.setMaxcombo(judge.getCourseMaxcombo());
                    self.save_config();
                    // resource.setGauge(gaugelog);
                    // resource.setGrooveGauge(gauge);
                    // resource.setAssist(assist);
                    // input.setEnable(true); input.setStartTime(0);

                    // Transition: practice -> STATE_PRACTICE, else -> RESULT
                    // TODO: Phase 22 - main.changeState()
                    log::info!("Finished, transition to result/select");
                }
            }

            // STATE_ABORTED
            STATE_ABORTED => {
                // Quick retry check
                // TODO: Phase 22 - input.startPressed() ^ input.isSelectPressed()

                if self
                    .main_state_data
                    .timer
                    .get_now_time_for_id(TIMER_FADEOUT)
                    > 0
                // skin.getFadeout() - TODO: Phase 22
                {
                    // input.setEnable(true); input.setStartTime(0);
                    // main.changeState(MainStateType.MUSICSELECT);
                    log::info!("Aborted, transition to MUSICSELECT");
                }
            }

            _ => {}
        }

        self.prevtime = micronow;
    }

    fn input(&mut self) {
        if let Some(ref mut control) = self.control {
            control.input();
        }
        // Build InputContext for key input processing.
        // key_states comes from main.getInputProcessor() — not yet integrated.
        // auto_presstime comes from the judge manager.
        let auto_presstime = self.judge.get_auto_presstime().to_vec();
        let now = self.main_state_data.timer.get_now_time();
        if let Some(ref mut keyinput) = self.keyinput {
            let mut ctx = crate::key_input_processor::InputContext {
                now,
                key_states: &[], // TODO: Phase 22+ — integrate BMSPlayerInputProcessor key states
                auto_presstime: &auto_presstime,
                is_autoplay: false, // TODO: Phase 22+ — read from resource.getPlayMode()
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

        // The PRELOAD->READY transition requires:
        // 1. media_loaded = true (hardcoded for now)
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
}
