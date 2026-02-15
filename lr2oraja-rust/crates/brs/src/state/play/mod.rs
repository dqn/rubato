// Play state — the core gameplay state.
//
// Ported from Java `BMSPlayer.java` (1,219 lines).
// Orchestrates: input -> judge -> gauge -> key sound -> score -> skin sync.

mod control_input;
mod play_skin_state;
pub mod pomyu_chara;
pub mod practice;

use tracing::{info, warn};

use std::path::{Path, PathBuf};

use bms_audio::driver::AudioDriver;
use bms_audio::key_sound::KeySoundProcessor;
use bms_audio::kira_driver::KiraAudioDriver;
use bms_database::score_data_property::ScoreDataProperty;
use bms_input::input_processor::InputProcessor;
use bms_model::{BmsModel, JudgeRankType, LaneProperty, Note, PlayMode};
use bms_pattern::{
    AssistLevel, AutoplayModifier, ExtraNoteModifier, LaneCrossShuffle, LaneMirrorShuffle,
    LanePlayableRandomShuffle, LaneRandomShuffle, LaneRotateShuffle, LongNoteMode,
    LongNoteModifier, MineNoteMode, MineNoteModifier, ModeModifier, NoteShuffleModifier,
    PatternModifier, PlayerBattleShuffle, PlayerFlipShuffle, PracticeModifier, RandomType,
    RandomUnit, ScrollSpeedMode, ScrollSpeedModifier, SevenToNinePattern, SevenToNineType,
    get_random,
};
use bms_render::bga::bga_processor::BgaProcessor;
use bms_replay::key_input_log::KeyInputLog;
use bms_rule::gauge_property::GaugeType;
use bms_rule::judge_manager::{JudgeConfig, JudgeEvent, JudgeManager};
use bms_rule::{ClearType, GrooveGauge, JUDGE_BD, JUDGE_MS, JUDGE_PR, JudgeAlgorithm, PlayerRule};
use bms_skin::property_id::{
    TIMER_COMBO_1P, TIMER_COMBO_2P, TIMER_ENDOFNOTE_1P, TIMER_FADEOUT, TIMER_FAILED,
    TIMER_FULLCOMBO_1P, TIMER_GAUGE_MAX_1P, TIMER_JUDGE_1P, TIMER_JUDGE_2P, TIMER_MUSIC_END,
    TIMER_PLAY, TIMER_READY, TIMER_RHYTHM,
};
use bms_skin::property_mapper;

use bms_database::RivalDataAccessor;

use crate::app_state::AppStateType;
use crate::state::{GameStateHandler, StateContext};
use crate::target_property::{RivalScore, TargetContext, TargetProperty};
use play_skin_state::ScratchAngleState;

/// Extra time after last note before play is considered finished (5 seconds).
const FINISH_MARGIN_US: i64 = 5_000_000;

/// Gauge log recording interval (500ms).
const GAUGE_LOG_INTERVAL_US: i64 = 500_000;

/// Ready phase duration before play starts (milliseconds).
const READY_DURATION_MS: i64 = 1000;

/// Duration after finished/failed before transitioning (milliseconds).
const CLOSE_DURATION_MS: i64 = 500;

/// Sentinel for "not set" timestamps.
const NOT_SET: i64 = i64::MIN;

/// Gauge auto-shift modes (from Java PlayerConfig.gauge_auto_shift).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeAutoShift {
    /// No auto-shift; gauge death = Failed.
    None = 0,
    /// Continue playing even when gauge is dead.
    Continue = 1,
    /// Shift from survival gauges to groove gauge on death.
    SurvivalToGroove = 2,
    /// Shift to best clear gauge on death.
    BestClear = 3,
    /// Shift to gauge below current on death.
    SelectToUnder = 4,
}

impl GaugeAutoShift {
    fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Continue,
            2 => Self::SurvivalToGroove,
            3 => Self::BestClear,
            4 => Self::SelectToUnder,
            _ => Self::None,
        }
    }
}

/// Play phase state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayPhase {
    /// Loading resources (audio, skin). Transitions to Ready when done.
    Preload,
    /// Practice mode: showing practice settings UI. User adjusts settings.
    Practice,
    /// Practice mode: fadeout after Escape, transitioning to MusicSelect.
    PracticeFinished,
    /// Countdown before play starts. TIMER_READY is active.
    Ready,
    /// Active gameplay. TIMER_PLAY is active.
    Playing,
    /// All notes have been processed. Brief delay before Result.
    Finished,
    /// Gauge died. Brief delay before Result (or retry).
    Failed,
}

/// Play state — the core gameplay state.
///
/// Orchestrates: input -> judge -> gauge -> key sound -> score -> skin sync.
pub struct PlayState {
    phase: PlayPhase,

    // Chart data
    judge_notes: Vec<Note>,
    lane_property: LaneProperty,

    // Judge + gauge
    judge_manager: Option<JudgeManager>,
    gauge: Option<GrooveGauge>,
    gauge_auto_shift: GaugeAutoShift,
    bottom_gauge: GaugeType,

    // Timing
    playtime_us: i64,
    last_note_time_us: i64,
    last_gauge_log_time_us: i64,

    // Gauge log (per-gauge-type values recorded every 500ms)
    gauge_log: Vec<Vec<f32>>,

    // Replay
    replay_log: Vec<KeyInputLog>,
    replay_cursor: usize,
    is_autoplay: bool,
    is_replay: bool,

    // Input
    input_processor: Option<InputProcessor>,

    // Key state for manual/replay play
    key_states: Vec<bool>,
    key_changed_times: Vec<i64>,

    // Audio
    audio_driver: Option<Box<dyn AudioDriver + Send>>,
    key_sound_processor: Option<KeySoundProcessor>,

    // BGA
    bga_processor: Option<BgaProcessor>,

    // Control state
    #[allow(dead_code)]
    play_speed: i32,
    key_beam_stop: bool,
    assist: i32,
    #[allow(dead_code)]
    is_judge_started: bool,

    // BPM tracking
    min_bpm: f64,
    max_bpm: f64,
    main_bpm: f64,
    now_bpm: f64,

    // Score comparison
    score_data_property: ScoreDataProperty,

    // Scratch angle animation
    scratch_angle: ScratchAngleState,

    // Abort detection
    start_pressed: bool,
    select_pressed: bool,

    // Practice mode
    is_practice: bool,
    practice_config: Option<practice::PracticeConfiguration>,
}

impl PlayState {
    pub fn new() -> Self {
        Self {
            phase: PlayPhase::Preload,
            judge_notes: Vec::new(),
            lane_property: LaneProperty::new(PlayMode::Beat7K),
            judge_manager: None,
            gauge: None,
            gauge_auto_shift: GaugeAutoShift::None,
            bottom_gauge: GaugeType::Normal,
            playtime_us: 0,
            last_note_time_us: 0,
            last_gauge_log_time_us: 0,
            gauge_log: Vec::new(),
            replay_log: Vec::new(),
            replay_cursor: 0,
            is_autoplay: true,
            is_replay: false,
            key_states: Vec::new(),
            key_changed_times: Vec::new(),
            audio_driver: None,
            key_sound_processor: None,
            bga_processor: None,
            input_processor: None,
            play_speed: 100,
            key_beam_stop: false,
            assist: 0,
            is_judge_started: false,
            min_bpm: 0.0,
            max_bpm: 0.0,
            main_bpm: 0.0,
            now_bpm: 0.0,
            score_data_property: ScoreDataProperty::new(),
            scratch_angle: ScratchAngleState::new(0),
            start_pressed: false,
            select_pressed: false,
            is_practice: false,
            practice_config: None,
        }
    }

    /// Get the current play phase.
    #[allow(dead_code)]
    pub fn phase(&self) -> PlayPhase {
        self.phase
    }

    /// Get the gauge log (recorded every 500ms, each entry = per-gauge-type values).
    #[allow(dead_code)]
    pub fn gauge_log(&self) -> &[Vec<f32>] {
        &self.gauge_log
    }

    /// Set autoplay mode.
    #[allow(dead_code)]
    pub fn set_autoplay(&mut self, autoplay: bool) {
        self.is_autoplay = autoplay;
    }

    /// Get a reference to the BGA processor (for rendering).
    #[allow(dead_code)]
    pub fn bga_processor(&self) -> Option<&BgaProcessor> {
        self.bga_processor.as_ref()
    }

    /// Set replay log (enables replay mode).
    #[allow(dead_code)]
    pub fn set_replay_log(&mut self, log: Vec<KeyInputLog>) {
        self.replay_log = log;
        self.is_replay = true;
        self.is_autoplay = false;
    }

    /// Build the rival score array for target comparison.
    ///
    /// Iterates all loaded rivals, queries each rival's score for the given
    /// song (sha256 + mode), and collects them alongside the player's own
    /// score. The result is sorted by exscore descending, matching Java's
    /// `RivalTargetProperty.createScoreArray()`.
    fn build_rival_scores(ctx: &StateContext) -> Vec<RivalScore> {
        let db = match ctx.database {
            Some(db) => db,
            None => return Vec::new(),
        };
        let model = match &ctx.resource.bms_model {
            Some(m) => m,
            None => return Vec::new(),
        };

        let sha256 = &model.sha256;
        let mode = model.mode.mode_id();
        let mut scores = Vec::new();

        for rival in db.rival.rivals() {
            match RivalDataAccessor::get_rival_score(&rival.db_path, sha256, mode) {
                Ok(Some(sd)) => {
                    scores.push(RivalScore {
                        name: rival.info.name.clone(),
                        exscore: sd.exscore(),
                    });
                }
                Ok(None) => {} // Rival has no score for this song
                Err(e) => {
                    warn!(rival = rival.info.name, "Failed to load rival score: {e}");
                }
            }
        }

        // Add player's own score (name = "" to match Java convention)
        let oldscore = &ctx.resource.oldscore;
        scores.push(RivalScore {
            name: String::new(),
            exscore: oldscore.exscore(),
        });

        // Sort by exscore descending
        scores.sort_by(|a, b| b.exscore.cmp(&a.exscore));
        scores
    }

    /// Initialize judge and gauge from the loaded model.
    fn init_judge_and_gauge(&mut self, ctx: &mut StateContext) {
        let model = match &ctx.resource.bms_model {
            Some(m) => m,
            None => {
                info!("Play: no BMS model loaded, skipping to Result");
                *ctx.transition = Some(AppStateType::Result);
                return;
            }
        };

        // Clone model for pattern modification
        let mut model = model.clone();
        self.lane_property = LaneProperty::new(model.mode);

        // Initialize BGA processor from model (before pattern modifiers alter it)
        self.bga_processor = Some(BgaProcessor::new(&model));

        // Apply pre-shuffle modifiers (scroll, longnote, mine, extranote)
        // Java: applied before lane shuffle, config value > 0 means active
        // Java offsets config values by -1 (e.g., ScrollMode 1 -> enum index 0)
        self.assist += apply_pre_shuffle_modifiers(&mut model, ctx.player_config);

        // Apply 1P pattern shuffle
        let random_type = get_random(ctx.player_config.random as usize, model.mode);
        let seed: i64 = rand::random();
        self.assist += apply_pattern_modifier(
            &mut model,
            random_type,
            0,
            seed,
            ctx.player_config.hran_threshold_bpm,
        );

        // DP: Apply 2P pattern + doubleoption (flip)
        if model.mode.player_count() > 1 {
            let random_type_2p = get_random(ctx.player_config.random2 as usize, model.mode);
            self.assist += apply_pattern_modifier(
                &mut model,
                random_type_2p,
                1,
                seed,
                ctx.player_config.hran_threshold_bpm,
            );
            apply_double_option(&mut model, ctx.player_config.doubleoption);
        }

        // SP Battle mode: doubleoption >= 2 converts SP to DP with battle shuffle
        // and optionally autoplay scratch (doubleoption == 3).
        // Java: BMSPlayer lines 331-351
        if model.mode.player_count() == 1 && ctx.player_config.doubleoption >= 2 {
            self.assist +=
                apply_double_option_with_autoplay(&mut model, ctx.player_config.doubleoption);
            self.lane_property = LaneProperty::new(model.mode);
        }

        // Apply 7-to-9 mode modifier (after lane shuffle, matching Java order)
        if ctx.player_config.seven_to_nine_pattern >= 1 && model.mode == PlayMode::Beat7K {
            let pattern = SevenToNinePattern::from_id(ctx.player_config.seven_to_nine_pattern);
            let seven_type = SevenToNineType::from_id(ctx.player_config.seven_to_nine_type);
            let mut mode_mod = ModeModifier::new(PlayMode::Beat7K, PlayMode::PopN9K)
                .with_pattern(pattern)
                .with_type(seven_type)
                .with_hran_threshold_bpm(ctx.player_config.hran_threshold_bpm as f64);
            mode_mod.modify(&mut model);
            self.assist += assist_to_i32(mode_mod.assist_level());
            self.lane_property = LaneProperty::new(model.mode);
            info!(
                pattern = ctx.player_config.seven_to_nine_pattern,
                "Play: applied 7-to-9 mode modifier"
            );
        }

        let rule = PlayerRule::lr2();
        self.judge_notes = model.build_judge_notes();

        let total_notes = self.judge_notes.iter().filter(|n| n.is_playable()).count();
        let total = if model.total > 0.0 {
            model.total
        } else {
            PlayerRule::default_total(total_notes)
        };

        // Determine gauge type from player config
        let gauge_type = gauge_type_from_i32(ctx.player_config.gauge);
        self.gauge_auto_shift = GaugeAutoShift::from_i32(ctx.player_config.gauge_auto_shift);
        self.bottom_gauge = gauge_type_from_i32(ctx.player_config.bottom_shiftable_gauge);

        let judge_rank = rule
            .judge
            .window_rule
            .resolve_judge_rank(model.judge_rank, model.judge_rank_type);

        // Judge window rates
        let (jwr, sjwr) = if ctx.player_config.custom_judge {
            (
                [
                    ctx.player_config.key_judge_window_rate_perfect_great,
                    ctx.player_config.key_judge_window_rate_great,
                    ctx.player_config.key_judge_window_rate_good,
                ],
                [
                    ctx.player_config.scratch_judge_window_rate_perfect_great,
                    ctx.player_config.scratch_judge_window_rate_great,
                    ctx.player_config.scratch_judge_window_rate_good,
                ],
            )
        } else {
            ([100, 100, 100], [100, 100, 100])
        };

        let config = JudgeConfig {
            notes: &self.judge_notes,
            play_mode: model.mode,
            ln_type: model.ln_type,
            judge_rank,
            judge_window_rate: jwr,
            scratch_judge_window_rate: sjwr,
            algorithm: JudgeAlgorithm::Combo,
            autoplay: self.is_autoplay,
            judge_property: &rule.judge,
            lane_property: Some(&self.lane_property),
        };

        let mut jm = JudgeManager::new(&config);
        let mut gauge = GrooveGauge::new(&rule.gauge, gauge_type, total, total_notes);

        // Allocate key state arrays
        let phys_count = self.lane_property.physical_key_count();
        self.key_states = vec![false; phys_count];
        self.key_changed_times = vec![NOT_SET; phys_count];

        // Prime JudgeManager: set prev_time to -1 so notes at time_us=0 are not skipped.
        jm.update(
            -1,
            &self.judge_notes,
            &self.key_states,
            &self.key_changed_times,
            &mut gauge,
        );

        // Calculate playtime
        self.last_note_time_us = self
            .judge_notes
            .iter()
            .map(|n| n.time_us.max(n.end_time_us))
            .max()
            .unwrap_or(0);
        self.playtime_us = self.last_note_time_us + FINISH_MARGIN_US;

        self.judge_manager = Some(jm);
        self.gauge = Some(gauge);

        // Initialize scratch angle state
        self.scratch_angle = ScratchAngleState::new(self.lane_property.scratch_count());

        // Store total notes in resource
        ctx.resource.score_data.notes = total_notes as i32;

        // Initialize BPM tracking
        self.min_bpm = model.min_bpm();
        self.max_bpm = model.max_bpm();
        self.main_bpm = model.main_bpm();
        self.now_bpm = model.initial_bpm;

        // Initialize ScoreDataProperty for real-time score comparison
        let mut sdp = ScoreDataProperty::new();
        let oldscore = &ctx.resource.oldscore;
        let best_ghost = oldscore.decode_ghost();
        let target = TargetProperty::resolve(&ctx.player_config.targetid);
        let rival_scores = Self::build_rival_scores(ctx);
        let rival_scores_ref = if rival_scores.is_empty() && ctx.database.is_none() {
            None
        } else {
            Some(rival_scores.as_slice())
        };
        let target_ctx = TargetContext {
            total_notes: total_notes as i32,
            current_exscore: oldscore.exscore(),
            rival_scores: rival_scores_ref,
            ranking_data: ctx.resource.ranking_data.as_ref(),
        };
        let (target_exscore, _target_name) = target.compute_target(&target_ctx);
        sdp.set_target_score(
            oldscore.exscore(),
            best_ghost,
            target_exscore,
            None, // target ghost (static targets don't have ghost)
            total_notes as i32,
        );
        self.score_data_property = sdp;
    }

    /// Handle the Playing phase render logic (timer-driven state checks).
    fn render_playing(&mut self, ctx: &mut StateContext) {
        let ptime_us = ctx.timer.now_time_of(TIMER_PLAY) * 1000;

        // Update current BPM at this time position
        if let Some(model) = &ctx.resource.bms_model {
            self.now_bpm = model.bpm_at(ptime_us);
        }

        // Update ScoreDataProperty
        if let Some(jm) = &self.judge_manager {
            self.score_data_property.update(jm.score(), jm.past_notes());
        }

        // Update BGA timeline and movie frames
        if let Some(bga) = &mut self.bga_processor {
            bga.update(ptime_us);
            if let Some(images) = &mut ctx.bevy_images {
                bga.update_movie_frames(images);
            }

            // Sync BGA image handles to shared game state for skin rendering
            if let Some(shared) = &mut ctx.shared_state {
                shared.bga_image = bga.get_bga_image().cloned();
                shared.layer_image = bga.get_layer_image().cloned();
                shared.poor_image = bga.get_poor_image().cloned();
                shared.poor_active = bga.is_poor_active();
            }
        }

        // BGM autoplay via KeySoundProcessor
        if let (Some(ksp), Some(driver)) = (&mut self.key_sound_processor, &mut self.audio_driver) {
            ksp.update(ptime_us, driver.as_mut());
        }

        // Record gauge log every 500ms
        self.record_gauge_log(ptime_us);

        // Check gauge death
        if let Some(gauge) = &self.gauge
            && gauge.value() <= 0.0
            && !self.handle_gauge_death(ctx)
        {
            return; // Transitioned to Failed
        }

        // Update gauge-related timers
        if let Some(gauge) = &self.gauge {
            ctx.timer
                .switch_timer(TIMER_GAUGE_MAX_1P, gauge.active_gauge().is_max());
        }

        // Check end-of-notes
        ctx.timer
            .switch_timer(TIMER_ENDOFNOTE_1P, ptime_us > self.last_note_time_us);

        // Check fullcombo (no combo-breaking judgments)
        if let Some(jm) = &self.judge_manager {
            let score = jm.score();
            let judged = score.total_judge_count();
            if judged >= score.notes
                && score.judge_count(JUDGE_BD) == 0
                && score.judge_count(JUDGE_PR) == 0
                && score.judge_count(JUDGE_MS) == 0
            {
                ctx.timer.switch_timer(TIMER_FULLCOMBO_1P, true);
            }
        }

        // Check if play is finished
        if ptime_us >= self.playtime_us {
            self.phase = PlayPhase::Finished;
            ctx.timer.set_timer_on(TIMER_MUSIC_END);
            info!("Play: finished (all notes processed)");
        }
    }

    /// Handle gauge death. Returns true if play continues, false if transitioned to Failed.
    fn handle_gauge_death(&mut self, ctx: &mut StateContext) -> bool {
        match self.gauge_auto_shift {
            GaugeAutoShift::None => {
                self.phase = PlayPhase::Failed;
                ctx.timer.set_timer_on(TIMER_FAILED);
                self.key_beam_stop = true;
                info!("Play: gauge death -> Failed");
                false
            }
            GaugeAutoShift::Continue => true,
            GaugeAutoShift::SurvivalToGroove => {
                if let Some(gauge) = &mut self.gauge {
                    let active = gauge.active_type();
                    if active == GaugeType::Hard || active == GaugeType::ExHard {
                        gauge.set_active_type(GaugeType::Normal);
                        info!("Play: GAS survival->groove");
                    }
                }
                true
            }
            GaugeAutoShift::BestClear => {
                self.shift_to_best_clear_gauge();
                true
            }
            GaugeAutoShift::SelectToUnder => {
                self.shift_to_lower_gauge();
                true
            }
        }
    }

    /// Shift to the best gauge that's still alive.
    fn shift_to_best_clear_gauge(&mut self) {
        let gauge = match &mut self.gauge {
            Some(g) => g,
            None => return,
        };
        let bottom_idx = self.bottom_gauge as usize;
        let types = [
            GaugeType::ExHard,
            GaugeType::Hard,
            GaugeType::Normal,
            GaugeType::Easy,
            GaugeType::AssistEasy,
        ];
        for &gt in &types {
            if (gt as usize) < bottom_idx {
                continue;
            }
            if gauge.value_of(gt) > 0.0 {
                gauge.set_active_type(gt);
                info!("Play: GAS bestclear -> {:?}", gt);
                return;
            }
        }
    }

    /// Shift to one gauge type below current.
    fn shift_to_lower_gauge(&mut self) {
        let gauge = match &mut self.gauge {
            Some(g) => g,
            None => return,
        };
        let active = gauge.active_type();
        let lower = match active {
            GaugeType::ExHard => Some(GaugeType::Hard),
            GaugeType::Hard => Some(GaugeType::Normal),
            GaugeType::Normal => Some(GaugeType::Easy),
            GaugeType::Easy => Some(GaugeType::AssistEasy),
            _ => None,
        };
        if let Some(gt) = lower
            && (gt as usize) >= (self.bottom_gauge as usize)
        {
            gauge.set_active_type(gt);
            info!("Play: GAS select-to-under -> {:?}", gt);
        }
    }

    /// Record gauge values at 500ms intervals.
    fn record_gauge_log(&mut self, ptime_us: i64) {
        while self.last_gauge_log_time_us + GAUGE_LOG_INTERVAL_US <= ptime_us {
            self.last_gauge_log_time_us += GAUGE_LOG_INTERVAL_US;
            if let Some(gauge) = &self.gauge {
                let values: Vec<f32> = GaugeType::ALL
                    .iter()
                    .map(|&gt| gauge.value_of(gt))
                    .collect();
                self.gauge_log.push(values);
            }
        }
    }

    /// Inject replay events into key state up to the current time.
    fn inject_replay_events(&mut self, ptime_us: i64) {
        let phys_count = self.key_states.len();
        while self.replay_cursor < self.replay_log.len() {
            let event = &self.replay_log[self.replay_cursor];
            if event.get_time() > ptime_us {
                break;
            }
            let key = event.keycode as usize;
            if key < phys_count {
                self.key_states[key] = event.pressed;
                self.key_changed_times[key] = event.get_time();
            }
            self.replay_cursor += 1;
        }
    }

    /// Build score data and save to resource for Result state.
    fn build_score_data(&self, ctx: &mut StateContext) {
        if let Some(jm) = &self.judge_manager {
            let mut score = jm.score().clone();
            score.maxcombo = jm.max_combo();

            if let Some(gauge) = &self.gauge {
                score.clear = if gauge.is_qualified() {
                    ClearType::from_gauge_type(gauge.active_type() as usize)
                        .unwrap_or(ClearType::Normal)
                } else {
                    ClearType::Failed
                };
                score.gauge = gauge.active_type() as i32;
            }

            if let Some(model) = &ctx.resource.bms_model {
                score.sha256 = model.sha256.clone();
                score.mode = model.mode.mode_id();
            }

            score.minbp = score.judge_count(JUDGE_BD)
                + score.judge_count(JUDGE_PR)
                + score.judge_count(JUDGE_MS);
            score.assist = self.assist;

            ctx.resource.score_data = score;
        }

        ctx.resource.gauge_log = self.gauge_log.clone();
        ctx.resource.maxcombo = self.judge_manager.as_ref().map_or(0, |jm| jm.max_combo());
        ctx.resource.update_score = !self.is_autoplay && !self.is_replay;
    }

    /// Apply practice settings to the BMS model and reinitialize judge/gauge.
    ///
    /// Ported from Java BMSPlayer.java lines 684-722.
    /// Called when the user presses the play key in the practice menu.
    /// Unlike `init_judge_and_gauge`, this applies practice-specific modifiers
    /// (freq, time range, practice random) instead of the normal config modifiers.
    fn apply_practice_settings(&mut self, ctx: &mut StateContext) {
        let property = match &self.practice_config {
            Some(pc) => pc.property.clone(),
            None => return,
        };

        // Reload BMS model to get a fresh copy
        if let Err(e) = ctx.resource.reload_bms() {
            warn!("Play: practice apply_settings reload_bms failed: {e}");
            return;
        }

        // Clone the fresh model for modification
        let mut model = match &ctx.resource.bms_model {
            Some(m) => m.clone(),
            None => return,
        };

        // Apply frequency change
        if property.freq != 100 {
            let freq_ratio = property.freq as f64 / 100.0;
            model.change_frequency(freq_ratio);
        }

        // Override total and judge_rank from practice settings
        model.total = property.total;
        model.judge_rank = property.judgerank;
        model.judge_rank_type = JudgeRankType::BmsonJudgeRank;

        // Apply PracticeModifier: move notes outside the selected time range
        // Times are scaled by freq (Java: starttime * 100 / freq)
        let start_ms = (property.starttime as i64) * 100 / (property.freq as i64);
        let end_ms = (property.endtime as i64) * 100 / (property.freq as i64);
        let mut practice_mod = PracticeModifier::new(start_ms, end_ms);
        practice_mod.modify(&mut model);

        self.lane_property = LaneProperty::new(model.mode);

        // Apply pattern modifiers from practice settings (not player config)
        let seed: i64 = rand::random();
        self.assist = 0;
        let random_type = get_random(property.random as usize, model.mode);
        self.assist += apply_pattern_modifier(
            &mut model,
            random_type,
            0,
            seed,
            ctx.player_config.hran_threshold_bpm,
        );

        if model.mode.player_count() > 1 {
            let random_type_2p = get_random(property.random2 as usize, model.mode);
            self.assist += apply_pattern_modifier(
                &mut model,
                random_type_2p,
                1,
                seed,
                ctx.player_config.hran_threshold_bpm,
            );
            apply_double_option(&mut model, property.doubleop);
        }

        // Build judge notes and initialize gauge from modified model
        let rule = PlayerRule::lr2();
        self.judge_notes = model.build_judge_notes();

        let total_notes = self.judge_notes.iter().filter(|n| n.is_playable()).count();
        let total = if model.total > 0.0 {
            model.total
        } else {
            PlayerRule::default_total(total_notes)
        };

        let judge_rank = rule
            .judge
            .window_rule
            .resolve_judge_rank(model.judge_rank, model.judge_rank_type);

        let (jwr, sjwr) = if ctx.player_config.custom_judge {
            (
                [
                    ctx.player_config.key_judge_window_rate_perfect_great,
                    ctx.player_config.key_judge_window_rate_great,
                    ctx.player_config.key_judge_window_rate_good,
                ],
                [
                    ctx.player_config.scratch_judge_window_rate_perfect_great,
                    ctx.player_config.scratch_judge_window_rate_great,
                    ctx.player_config.scratch_judge_window_rate_good,
                ],
            )
        } else {
            ([100, 100, 100], [100, 100, 100])
        };

        let config = JudgeConfig {
            notes: &self.judge_notes,
            play_mode: model.mode,
            ln_type: model.ln_type,
            judge_rank,
            judge_window_rate: jwr,
            scratch_judge_window_rate: sjwr,
            algorithm: JudgeAlgorithm::Combo,
            autoplay: self.is_autoplay,
            judge_property: &rule.judge,
            lane_property: Some(&self.lane_property),
        };

        let mut jm = JudgeManager::new(&config);

        // Practice gauge: use practice settings instead of player config
        let gauge_type = gauge_type_from_i32(property.gaugetype);
        let mut gauge = GrooveGauge::new(&rule.gauge, gauge_type, total, total_notes);
        gauge.set_value(property.startgauge as f32);
        // Practice: no auto-shift
        self.gauge_auto_shift = GaugeAutoShift::Continue;

        // Allocate key state arrays
        let phys_count = self.lane_property.physical_key_count();
        self.key_states = vec![false; phys_count];
        self.key_changed_times = vec![NOT_SET; phys_count];

        // Prime JudgeManager
        jm.update(
            -1,
            &self.judge_notes,
            &self.key_states,
            &self.key_changed_times,
            &mut gauge,
        );

        // Calculate playtime
        self.last_note_time_us = self
            .judge_notes
            .iter()
            .map(|n| n.time_us.max(n.end_time_us))
            .max()
            .unwrap_or(0);
        self.playtime_us = self.last_note_time_us + FINISH_MARGIN_US;

        self.judge_manager = Some(jm);
        self.gauge = Some(gauge);

        // Initialize scratch angle state
        self.scratch_angle = ScratchAngleState::new(self.lane_property.scratch_count());

        ctx.resource.score_data.notes = total_notes as i32;

        // BPM tracking
        self.min_bpm = model.min_bpm();
        self.max_bpm = model.max_bpm();
        self.main_bpm = model.main_bpm();
        self.now_bpm = model.initial_bpm;

        // Practice mode: no target score comparison
        self.score_data_property = ScoreDataProperty::new();
        ctx.resource.update_score = false;

        // Reset gauge log for new practice loop
        self.gauge_log.clear();
        self.last_gauge_log_time_us = 0;
        self.key_beam_stop = false;
        self.is_judge_started = false;

        // Reinitialize audio for the modified model
        if let Some(driver) = &mut self.audio_driver {
            driver.stop_all();
        }
        let base_path = ctx.resource.bms_dir.as_deref().unwrap_or(Path::new("."));
        match KiraAudioDriver::new() {
            Ok(mut driver) => {
                if let Err(e) = driver.set_model(&model, base_path) {
                    warn!("Play: practice audio reload failed: {e}");
                }
                self.key_sound_processor = Some(KeySoundProcessor::new(&model, 1.0));
                self.audio_driver = Some(Box::new(driver));
            }
            Err(e) => {
                warn!("Play: practice audio driver creation failed: {e}");
            }
        }

        // Initialize BGA processor
        self.bga_processor = Some(BgaProcessor::new(&model));

        info!(
            freq = property.freq,
            start_ms, end_ms, "Play: practice settings applied"
        );
    }
}

impl Default for PlayState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateHandler for PlayState {
    fn create(&mut self, ctx: &mut StateContext) {
        info!("Play: create");
        self.phase = PlayPhase::Preload;
        self.gauge_log.clear();
        self.last_gauge_log_time_us = 0;
        self.replay_cursor = 0;
        self.key_beam_stop = false;
        self.is_judge_started = false;
        self.start_pressed = false;
        self.select_pressed = false;
        self.is_practice = ctx.resource.is_practice;

        // Practice mode: initialize PracticeConfiguration
        if self.is_practice
            && let Some(model) = &ctx.resource.bms_model
        {
            let config_dir = PathBuf::from(&ctx.config.playerpath);
            let pc = practice::PracticeConfiguration::new(model, config_dir);
            self.practice_config = Some(pc);
            info!("Play: practice mode enabled");
        }

        // Load best score from DB before play
        if let Some(db) = ctx.database
            && let Some(model) = &ctx.resource.bms_model
        {
            let sha256 = &model.sha256;
            let mode = model.mode.mode_id();
            match db.score_db.get_score_data(sha256, mode) {
                Ok(Some(old)) => ctx.resource.oldscore = old,
                Ok(None) => ctx.resource.oldscore = Default::default(),
                Err(e) => {
                    warn!("Play: failed to load old score: {e}");
                    ctx.resource.oldscore = Default::default();
                }
            }
        }

        if !self.is_practice {
            self.init_judge_and_gauge(ctx);
        }

        // Initialize InputProcessor for manual play (not autoplay/replay)
        if !self.is_autoplay && !self.is_replay {
            let mut ip = InputProcessor::new();
            let mode_id = ctx.resource.play_mode.mode_id();
            let mode_config = ctx.player_config.play_config(mode_id);
            ip.set_play_config(mode_config);
            self.input_processor = Some(ip);
        } else {
            self.input_processor = None;
        }
    }

    fn prepare(&mut self, ctx: &mut StateContext) {
        info!("Play: prepare");

        // Initialize audio driver and key sound processor
        if let Some(model) = &ctx.resource.bms_model {
            let base_path = ctx.resource.bms_dir.as_deref().unwrap_or(Path::new("."));
            match KiraAudioDriver::new() {
                Ok(mut driver) => {
                    if let Err(e) = driver.set_model(model, base_path) {
                        warn!("Play: failed to load audio: {e}");
                    }
                    self.key_sound_processor = Some(KeySoundProcessor::new(model, 1.0));
                    self.audio_driver = Some(Box::new(driver));
                }
                Err(e) => {
                    warn!("Play: failed to create audio driver: {e}");
                }
            }
        }

        // Preload BGA images and movie processors if Bevy assets are available
        if let (Some(bga), Some(model)) = (&mut self.bga_processor, &ctx.resource.bms_model)
            && let Some(images) = &mut ctx.bevy_images
        {
            bga.set_frameskip(ctx.config.frameskip);
            let base_path = ctx.resource.bms_dir.as_deref().unwrap_or(Path::new("."));
            bga.prepare(model, base_path, images);
        }

        if self.is_practice {
            self.phase = PlayPhase::Practice;
            info!("Play: prepare -> Practice settings");
        } else {
            self.phase = PlayPhase::Ready;
            ctx.timer.set_timer_on(TIMER_READY);
        }
    }

    fn render(&mut self, ctx: &mut StateContext) {
        match self.phase {
            PlayPhase::Preload => {
                // Should not reach here (prepare transitions to Ready)
            }
            PlayPhase::Practice => {
                // If TIMER_PLAY was on (returning from a play loop), reload the BMS model
                if ctx.timer.is_timer_on(TIMER_PLAY) {
                    if let Err(e) = ctx.resource.reload_bms() {
                        warn!("Play: practice reload_bms failed: {e}");
                    }
                    ctx.timer.set_timer_off(TIMER_PLAY);
                    ctx.timer.set_timer_off(TIMER_RHYTHM);
                    ctx.timer.set_timer_off(TIMER_MUSIC_END);
                    ctx.timer.set_timer_off(TIMER_FAILED);
                    // Stop audio from previous loop
                    if let Some(driver) = &mut self.audio_driver {
                        driver.stop_all();
                    }
                }
            }
            PlayPhase::PracticeFinished => {
                // Wait for fadeout to complete, then transition to MusicSelect
                if ctx.timer.now_time_of(TIMER_FADEOUT) > CLOSE_DURATION_MS {
                    ctx.resource.is_practice = false;
                    *ctx.transition = Some(AppStateType::MusicSelect);
                    info!("Play: PracticeFinished -> MusicSelect");
                }
            }
            PlayPhase::Ready => {
                if ctx.timer.now_time_of(TIMER_READY) > READY_DURATION_MS {
                    self.phase = PlayPhase::Playing;
                    ctx.timer.set_timer_on(TIMER_PLAY);
                    ctx.timer.set_timer_on(TIMER_RHYTHM);
                    info!("Play: Ready -> Playing");
                }
            }
            PlayPhase::Playing => {
                self.render_playing(ctx);
            }
            PlayPhase::Finished => {
                if self.is_practice {
                    // Practice loop: return to practice settings
                    self.phase = PlayPhase::Practice;
                    info!("Play: Finished -> Practice (loop)");
                } else if ctx.timer.now_time_of(TIMER_MUSIC_END) > CLOSE_DURATION_MS {
                    self.build_score_data(ctx);
                    *ctx.transition = Some(AppStateType::Result);
                    info!("Play: Finished -> Result");
                }
            }
            PlayPhase::Failed => {
                if self.is_practice {
                    // Practice loop: return to practice settings
                    self.phase = PlayPhase::Practice;
                    info!("Play: Failed -> Practice (loop)");
                } else if ctx.timer.now_time_of(TIMER_FAILED) > CLOSE_DURATION_MS {
                    self.build_score_data(ctx);
                    *ctx.transition = Some(AppStateType::Result);
                    info!("Play: Failed -> Result");
                }
            }
        }

        // Update scratch angle animation
        if let Some(jm) = &self.judge_manager {
            let ptime_ms = ctx.timer.now_time_of(TIMER_PLAY);
            self.scratch_angle.update(
                ptime_ms,
                &self.lane_property,
                &self.key_states,
                jm.auto_presstime(),
                self.is_autoplay,
            );
        }

        // Sync play state to shared game state for skin rendering
        if let Some(shared) = &mut ctx.shared_state
            && let (Some(jm), Some(gauge)) = (&self.judge_manager, &self.gauge)
        {
            let current_bpm = self.now_bpm as i32;
            play_skin_state::sync_play_state(shared, jm, gauge, current_bpm);
            play_skin_state::sync_play_options(
                shared,
                self.is_autoplay,
                gauge.active_type() as i32,
                true, // BGA is always on when bga_processor exists
            );

            // 23-2: Hispeed / Duration / Lanecover
            let mode_id = ctx.resource.play_mode.mode_id();
            let play_config = &ctx.player_config.play_config(mode_id).playconfig;
            play_skin_state::sync_play_hispeed_duration(
                shared,
                play_config,
                self.now_bpm,
                self.main_bpm,
                self.min_bpm,
                self.max_bpm,
            );

            // 23-3: Play time / Music progress
            let play_elapsed_us = ctx.timer.now_time_of(TIMER_PLAY) * 1000;
            let total_time_us = ctx
                .resource
                .bms_model
                .as_ref()
                .map(|m| m.total_time_us)
                .unwrap_or(0);
            play_skin_state::sync_play_time(shared, play_elapsed_us, total_time_us);

            // 23-4: Score comparison
            play_skin_state::sync_play_score_comparison(shared, &self.score_data_property, jm);

            // 23-5: Gauge range / Realtime rank / Extended options
            play_skin_state::sync_play_gauge_range(shared, gauge);
            play_skin_state::sync_play_realtime_rank(shared, &self.score_data_property);
            play_skin_state::sync_play_extended_options(
                shared,
                self.phase,
                self.is_replay,
                self.is_practice,
                play_config,
                self.start_pressed || self.select_pressed,
            );

            // 23-6: Offsets / Judge per key
            play_skin_state::sync_play_offsets(shared, play_config, &self.scratch_angle);
            play_skin_state::sync_play_judge_per_key(shared, jm, &self.lane_property);
            play_skin_state::sync_play_judge_indicators(shared, jm);
        }
    }

    fn input(&mut self, ctx: &mut StateContext) {
        // Practice phase input: process menu navigation and play trigger
        if self.phase == PlayPhase::Practice {
            if let (Some(pc), Some(input_state)) = (&mut self.practice_config, ctx.input_state) {
                // Check for Escape to abort practice
                if input_state
                    .pressed_keys
                    .contains(&bms_input::control_keys::ControlKeys::Escape)
                {
                    pc.save_property();
                    ctx.timer.set_timer_on(TIMER_FADEOUT);
                    self.phase = PlayPhase::PracticeFinished;
                    info!("Play: Practice -> PracticeFinished (escape)");
                    return;
                }

                if pc.process_input(input_state) {
                    // User pressed play key: apply settings and start playing
                    self.apply_practice_settings(ctx);
                    self.phase = PlayPhase::Ready;
                    ctx.timer.set_timer_on(TIMER_READY);
                    info!("Play: Practice -> Ready (play key pressed)");
                }
            }
            return;
        }

        if self.phase != PlayPhase::Playing || self.key_beam_stop {
            return;
        }

        let ptime_us = ctx.timer.now_time_of(TIMER_PLAY) * 1000;

        // Poll keyboard via InputProcessor (manual play mode)
        if let (Some(ip), Some(backend)) = (&mut self.input_processor, ctx.keyboard_backend) {
            ip.poll_keyboard(ptime_us, backend);
            // Copy key states from InputProcessor
            let phys_count = self.key_states.len();
            for i in 0..phys_count {
                self.key_states[i] = ip.get_key_state(i);
                self.key_changed_times[i] = ip.get_key_changed_time(i);
            }
        }

        // Inject replay events
        if self.is_replay {
            self.inject_replay_events(ptime_us);
        }

        // Update JudgeManager
        if let (Some(jm), Some(gauge)) = (&mut self.judge_manager, &mut self.gauge) {
            let events = jm.update(
                ptime_us,
                &self.judge_notes,
                &self.key_states,
                &self.key_changed_times,
                gauge,
            );

            // Process events inline (mine damage, audio)
            for event in &events {
                match event {
                    JudgeEvent::MineDamage { damage, .. } => {
                        gauge.add_value(-(*damage as f32));
                    }
                    JudgeEvent::KeySound { wav_id } => {
                        if let Some(driver) = &mut self.audio_driver {
                            let note = Note::keysound(*wav_id);
                            driver.play_note(&note, 1.0, 0);
                        }
                    }
                    JudgeEvent::Judge { lane, judge, .. } => {
                        // Trigger miss layer on BD/PR/MS judgments
                        if *judge >= JUDGE_BD
                            && let Some(bga) = &mut self.bga_processor
                        {
                            bga.set_miss_triggered(ptime_us);
                        }
                        // Per-player judge/combo timers
                        let player = self.lane_property.lane_player(*lane);
                        let judge_timer = if player == 0 {
                            TIMER_JUDGE_1P
                        } else {
                            TIMER_JUDGE_2P
                        };
                        let combo_timer = if player == 0 {
                            TIMER_COMBO_1P
                        } else {
                            TIMER_COMBO_2P
                        };
                        ctx.timer.set_timer_on(judge_timer);
                        ctx.timer.set_timer_on(combo_timer);
                    }
                    JudgeEvent::HcnGauge { .. } => {
                        // Already handled internally by JudgeManager
                    }
                }
            }

            // Track whether any notes have been judged (for key beam behavior)
            if !self.is_judge_started && jm.past_notes() > 0 {
                self.is_judge_started = true;
            }

            // Update key beam timers
            update_key_beam_timers(
                &self.lane_property,
                &self.key_states,
                jm.auto_presstime(),
                self.key_beam_stop,
                self.is_autoplay,
                self.is_judge_started,
                ctx.timer,
            );

            // Reset key changed times for next frame
            self.key_changed_times.fill(NOT_SET);

            // Reset InputProcessor's key changed times
            if let Some(ip) = &mut self.input_processor {
                ip.reset_all_key_changed_time();
            }

            // Update score in resource
            ctx.resource.score_data = jm.score().clone();
            ctx.resource.maxcombo = ctx.resource.maxcombo.max(jm.max_combo());
        }
    }

    fn shutdown(&mut self, ctx: &mut StateContext) {
        info!("Play: shutdown");
        if let Some(driver) = &mut self.audio_driver {
            driver.stop_all();
        }
        if let Some(bga) = &mut self.bga_processor {
            bga.dispose();
        }
        if self.is_practice {
            // Practice mode: save practice property, don't save score
            if let Some(pc) = &self.practice_config {
                pc.save_property();
            }
            ctx.resource.update_score = false;
            ctx.resource.is_practice = false;
        } else {
            self.build_score_data(ctx);
        }
    }
}

/// Update key beam timers based on key states and autoplay press times.
///
/// Ported from Java `KeyInputProccessor.input()` — toggles TIMER_KEYON/TIMER_KEYOFF
/// per lane for skin key beam animation.
fn update_key_beam_timers(
    lane_property: &LaneProperty,
    key_states: &[bool],
    auto_presstime: &[i64],
    key_beam_stop: bool,
    _is_autoplay: bool,
    _is_judge_started: bool,
    timer: &mut crate::timer_manager::TimerManager,
) {
    for lane in 0..lane_property.lane_count() {
        let offset = lane_property.lane_skin_offset(lane);
        let player = lane_property.lane_player(lane);
        let is_scratch = lane_property.scratch_index(lane).is_some();

        let mut pressed = false;
        if !key_beam_stop {
            for &key in lane_property.lane_to_keys(lane) {
                if key_states.get(key).copied().unwrap_or(false)
                    || auto_presstime.get(key).copied().unwrap_or(NOT_SET) != NOT_SET
                {
                    pressed = true;
                    break;
                }
            }
        }

        let timer_on = property_mapper::key_on_timer_id(player, offset);
        let timer_off = property_mapper::key_off_timer_id(player, offset);
        if timer_on < 0 || timer_off < 0 {
            continue;
        }

        if pressed {
            // Activate key-on timer. For scratch lanes, always re-trigger
            // (scratch can toggle direction rapidly).
            if !timer.is_timer_on(timer_on) || is_scratch {
                timer.set_timer_on(timer_on);
                timer.set_timer_off(timer_off);
            }
        } else if timer.is_timer_on(timer_on) {
            timer.set_timer_on(timer_off);
            timer.set_timer_off(timer_on);
        }
    }
}

/// Apply a pattern modifier to the model and return the assist level as i32.
fn apply_pattern_modifier(
    model: &mut BmsModel,
    rt: RandomType,
    player: usize,
    seed: i64,
    hran_bpm: i32,
) -> i32 {
    let cs = rt.is_scratch_lane_modify();
    let mut modifier: Box<dyn PatternModifier> = match rt.unit() {
        RandomUnit::None => return 0,
        RandomUnit::Lane => match rt {
            RandomType::Mirror | RandomType::MirrorEx => {
                Box::new(LaneMirrorShuffle::new(player, cs))
            }
            RandomType::Random | RandomType::RandomEx => {
                Box::new(LaneRandomShuffle::new(player, cs, seed))
            }
            RandomType::Rotate | RandomType::RotateEx => {
                Box::new(LaneRotateShuffle::new(player, cs, seed))
            }
            RandomType::Cross => Box::new(LaneCrossShuffle::new(player, cs)),
            RandomType::RandomPlayable => {
                Box::new(LanePlayableRandomShuffle::new(player, cs, seed))
            }
            _ => return 0,
        },
        RandomUnit::Note => Box::new(NoteShuffleModifier::new(rt, player, seed, hran_bpm)),
        RandomUnit::Player => return 0, // Handled by apply_double_option
    };
    let assist = match modifier.assist_level() {
        AssistLevel::None => 0,
        AssistLevel::LightAssist => 1,
        AssistLevel::Assist => 2,
    };
    modifier.modify(model);
    assist
}

/// Apply DP double option (flip/battle).
fn apply_double_option(model: &mut BmsModel, doubleoption: i32) {
    match doubleoption {
        1 => PlayerFlipShuffle::new().modify(model),
        2 => PlayerBattleShuffle::new().modify(model),
        _ => {}
    }
}

/// Apply DP double option with battle autoplay scratch.
///
/// When `doubleoption == 3`, applies Battle mode and then AutoplayModifier
/// for scratch lanes, matching Java `BMSPlayer` lines 331-351.
fn apply_double_option_with_autoplay(model: &mut BmsModel, doubleoption: i32) -> i32 {
    if doubleoption < 2 {
        return 0;
    }

    // Only applies to SP modes that can be converted to DP
    let can_battle = matches!(
        model.mode,
        PlayMode::Beat5K | PlayMode::Beat7K | PlayMode::Keyboard24K
    );
    if !can_battle {
        return 0;
    }

    // Convert SP -> DP mode
    match model.mode {
        PlayMode::Beat5K => model.mode = PlayMode::Beat10K,
        PlayMode::Beat7K => model.mode = PlayMode::Beat14K,
        PlayMode::Keyboard24K => model.mode = PlayMode::Keyboard24KDouble,
        _ => {}
    }

    // Apply battle shuffle
    PlayerBattleShuffle::new().modify(model);

    // doubleoption == 3: also autoplay scratch lanes
    if doubleoption == 3 {
        let scratch_keys = model.mode.scratch_keys().to_vec();
        let mut autoplay = AutoplayModifier::new(scratch_keys);
        autoplay.modify(model);
    }

    // Battle always counts as light assist
    1
}

/// Apply pre-shuffle modifiers (scroll, longnote, mine, extranote).
///
/// These are applied before the lane shuffle, matching Java `BMSPlayer` lines 303-329.
/// Config values > 0 mean active; Java subtracts 1 from the config value to get the
/// enum index.
fn apply_pre_shuffle_modifiers(model: &mut BmsModel, config: &bms_config::PlayerConfig) -> i32 {
    let mut assist = 0i32;

    // Scroll speed modifier (config.scroll_mode: 0=off, 1=remove, 2=add)
    if config.scroll_mode > 0 {
        let mode = match config.scroll_mode - 1 {
            0 => ScrollSpeedMode::Remove,
            _ => ScrollSpeedMode::Add,
        };
        let mut modifier = ScrollSpeedModifier::new(mode)
            .with_section(config.scroll_section as u32)
            .with_rate(config.scroll_rate);
        modifier.modify(model);
        assist += assist_to_i32(modifier.assist_level());
        info!(
            mode = config.scroll_mode,
            "Play: applied scroll speed modifier"
        );
    }

    // LongNote modifier (config.longnote_mode: 0=off, 1=remove, 2=add_ln, 3=add_cn, 4=add_hcn, 5=add_all)
    if config.longnote_mode > 0 {
        let mode = match config.longnote_mode - 1 {
            0 => LongNoteMode::Remove,
            1 => LongNoteMode::AddLn,
            2 => LongNoteMode::AddCn,
            3 => LongNoteMode::AddHcn,
            _ => LongNoteMode::AddAll,
        };
        let mut modifier = LongNoteModifier::new(mode, config.longnote_rate);
        modifier.modify(model);
        assist += assist_to_i32(modifier.assist_level());
        info!(
            mode = config.longnote_mode,
            "Play: applied longnote modifier"
        );
    }

    // Mine note modifier (config.mine_mode: 0=off, 1=remove, 2=add_random, 3=add_near, 4=add_blank)
    if config.mine_mode > 0 {
        let mode = match config.mine_mode - 1 {
            0 => MineNoteMode::Remove,
            1 => MineNoteMode::AddRandom,
            2 => MineNoteMode::AddNear,
            _ => MineNoteMode::AddBlank,
        };
        let mut modifier = MineNoteModifier::new(mode);
        modifier.modify(model);
        assist += assist_to_i32(modifier.assist_level());
        info!(mode = config.mine_mode, "Play: applied mine note modifier");
    }

    // Extra note modifier (config.extranote_depth > 0 activates it)
    if config.extranote_depth > 0 {
        let mut modifier =
            ExtraNoteModifier::new(config.extranote_depth as usize, config.extranote_scratch);
        modifier.modify(model);
        assist += assist_to_i32(modifier.assist_level());
        info!(
            depth = config.extranote_depth,
            "Play: applied extra note modifier"
        );
    }

    assist
}

/// Convert AssistLevel to i32 for assist accumulation.
fn assist_to_i32(level: AssistLevel) -> i32 {
    match level {
        AssistLevel::None => 0,
        AssistLevel::LightAssist => 1,
        AssistLevel::Assist => 2,
    }
}

/// Convert player config gauge value to GaugeType.
fn gauge_type_from_i32(v: i32) -> GaugeType {
    match v {
        0 => GaugeType::AssistEasy,
        1 => GaugeType::Easy,
        3 => GaugeType::Hard,
        4 => GaugeType::ExHard,
        5 => GaugeType::Hazard,
        6 => GaugeType::Class,
        7 => GaugeType::ExClass,
        8 => GaugeType::ExHardClass,
        _ => GaugeType::Normal,
    }
}

// --- Test helpers ---

#[cfg(test)]
impl PlayState {
    /// Set manual key states for testing (bypasses InputProcessor).
    #[allow(dead_code)]
    pub(crate) fn set_key_states(&mut self, states: Vec<bool>, times: Vec<i64>) {
        self.key_states = states;
        self.key_changed_times = times;
    }

    /// Get the current gauge value.
    #[allow(dead_code)]
    pub(crate) fn gauge_value(&self) -> f32 {
        self.gauge.as_ref().map_or(0.0, |g| g.value())
    }

    /// Get the current gauge type.
    pub(crate) fn gauge_type(&self) -> Option<GaugeType> {
        self.gauge.as_ref().map(|g| g.active_type())
    }

    /// Check if the gauge is qualified.
    pub(crate) fn gauge_qualified(&self) -> bool {
        self.gauge.as_ref().map_or(false, |g| g.is_qualified())
    }

    /// Get the score data from the judge manager.
    pub(crate) fn score(&self) -> Option<&bms_rule::ScoreData> {
        self.judge_manager.as_ref().map(|jm| jm.score())
    }

    /// Get the max combo from the judge manager.
    #[allow(dead_code)]
    pub(crate) fn max_combo(&self) -> i32 {
        self.judge_manager.as_ref().map_or(0, |jm| jm.max_combo())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_resource::PlayerResource;
    use crate::timer_manager::TimerManager;
    use bms_config::{Config, PlayerConfig};
    use bms_model::BmsDecoder;
    use std::path::Path;

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

    fn test_bms_dir() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test-bms")
            .leak()
    }

    fn load_test_model(filename: &str) -> bms_model::BmsModel {
        let path = test_bms_dir().join(filename);
        BmsDecoder::decode(&path).unwrap()
    }

    /// Run create+prepare on a PlayState and return its phase.
    fn init_play_state(
        state: &mut PlayState,
        timer: &mut TimerManager,
        resource: &mut PlayerResource,
        config: &Config,
        player_config: &mut PlayerConfig,
    ) {
        let mut transition = None;
        let mut ctx = make_ctx(timer, resource, config, player_config, &mut transition);
        state.create(&mut ctx);
        state.prepare(&mut ctx);
    }

    /// Advance to the Playing phase by stepping time past READY_DURATION_MS.
    fn advance_to_playing(
        state: &mut PlayState,
        timer: &mut TimerManager,
        resource: &mut PlayerResource,
        config: &Config,
        player_config: &mut PlayerConfig,
    ) {
        let mut transition = None;
        timer.set_now_micro_time(timer.now_micro_time() + (READY_DURATION_MS + 1) * 1000);
        let mut ctx = make_ctx(timer, resource, config, player_config, &mut transition);
        state.render(&mut ctx);
        assert_eq!(state.phase(), PlayPhase::Playing);
    }

    /// Run the game loop for a given number of microseconds from current time.
    fn run_game_loop(
        state: &mut PlayState,
        timer: &mut TimerManager,
        resource: &mut PlayerResource,
        config: &Config,
        player_config: &mut PlayerConfig,
        duration_us: i64,
        step_us: i64,
    ) -> Option<AppStateType> {
        let start = timer.now_micro_time();
        let end = start + duration_us;
        let mut transition = None;
        let mut t = start;
        while t <= end {
            timer.set_now_micro_time(t);
            transition = None;
            let mut ctx = make_ctx(timer, resource, config, player_config, &mut transition);
            state.render(&mut ctx);
            state.input(&mut ctx);
            if transition.is_some() {
                return transition;
            }
            t += step_us;
        }
        transition
    }

    // --- Phase transition tests ---

    #[test]
    fn create_sets_preload_phase() {
        let mut state = PlayState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(load_test_model("minimal_7k.bms"));
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
        // After create, prepare transitions to Ready
        state.prepare(&mut ctx);
        assert_eq!(state.phase(), PlayPhase::Ready);
        assert!(timer.is_timer_on(TIMER_READY));
    }

    #[test]
    fn ready_transitions_to_playing() {
        let mut state = PlayState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(load_test_model("minimal_7k.bms"));
        let config = Config::default();
        let mut player_config = PlayerConfig::default();

        init_play_state(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        assert_eq!(state.phase(), PlayPhase::Ready);

        // Before READY_DURATION_MS
        timer.set_now_micro_time(500_000);
        let mut transition = None;
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(state.phase(), PlayPhase::Ready);

        // After READY_DURATION_MS
        advance_to_playing(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        assert!(timer.is_timer_on(TIMER_PLAY));
        assert!(timer.is_timer_on(TIMER_RHYTHM));
    }

    #[test]
    fn playing_transitions_to_finished() {
        let mut state = PlayState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(load_test_model("minimal_7k.bms"));
        let config = Config::default();
        let mut player_config = PlayerConfig::default();

        init_play_state(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        advance_to_playing(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );

        // Advance past playtime
        let play_timer_base = timer.now_micro_time();
        let playtime_ms = state.playtime_us / 1000 + 1;
        timer.set_now_micro_time(play_timer_base + playtime_ms * 1000);
        let mut transition = None;
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(state.phase(), PlayPhase::Finished);
        assert!(timer.is_timer_on(TIMER_MUSIC_END));
    }

    #[test]
    fn finished_transitions_to_result() {
        let mut state = PlayState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(load_test_model("minimal_7k.bms"));
        let config = Config::default();
        let mut player_config = PlayerConfig::default();

        init_play_state(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        advance_to_playing(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );

        // Force Finished phase
        state.phase = PlayPhase::Finished;
        let finish_time = timer.now_micro_time() + 1000;
        timer.set_now_micro_time(finish_time);
        timer.set_timer_on(TIMER_MUSIC_END);

        // Advance past close duration
        timer.set_now_micro_time(finish_time + (CLOSE_DURATION_MS + 1) * 1000);
        let mut transition = None;
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(transition, Some(AppStateType::Result));
    }

    #[test]
    fn failed_transitions_to_result() {
        let mut state = PlayState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(load_test_model("minimal_7k.bms"));
        let config = Config::default();
        let mut player_config = PlayerConfig::default();

        init_play_state(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        advance_to_playing(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );

        // Force Failed phase
        state.phase = PlayPhase::Failed;
        let fail_time = timer.now_micro_time() + 1000;
        timer.set_now_micro_time(fail_time);
        timer.set_timer_on(TIMER_FAILED);

        // Advance past close duration
        timer.set_now_micro_time(fail_time + (CLOSE_DURATION_MS + 1) * 1000);
        let mut transition = None;
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(transition, Some(AppStateType::Result));
    }

    #[test]
    fn no_model_skips_to_result() {
        let mut state = PlayState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        // No bms_model set
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
        assert_eq!(transition, Some(AppStateType::Result));
    }

    // --- Autoplay tests ---

    #[test]
    fn autoplay_all_pgreat() {
        let mut state = PlayState::new();
        state.set_autoplay(true);

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(load_test_model("minimal_7k.bms"));
        let config = Config::default();
        let mut player_config = PlayerConfig::default();

        init_play_state(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        advance_to_playing(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );

        // Run game loop past all notes
        let end_time = state.playtime_us + 1_000_000;
        let result = run_game_loop(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            end_time,
            1_000, // 1ms steps
        );

        // Should have transitioned to Result via Finished
        assert!(
            result == Some(AppStateType::Result) || state.phase() == PlayPhase::Finished,
            "Expected Finished or Result transition, got phase={:?}, transition={:?}",
            state.phase(),
            result,
        );

        // Check all PGREAT
        let score = state.score().expect("score should exist");
        let pg = score.judge_count(bms_rule::JUDGE_PG);
        assert!(pg > 0, "PG count should be > 0, got {pg}");
        assert_eq!(score.judge_count(bms_rule::JUDGE_GR), 0);
        assert_eq!(score.judge_count(JUDGE_BD), 0);
        assert_eq!(score.judge_count(JUDGE_PR), 0);
        assert_eq!(score.judge_count(JUDGE_MS), 0);

        // Gauge should be qualified
        assert!(state.gauge_qualified(), "Gauge should be qualified");
    }

    // --- Gauge tests ---

    #[test]
    fn gauge_log_recorded_during_play() {
        let mut state = PlayState::new();
        state.set_autoplay(true);

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(load_test_model("minimal_7k.bms"));
        let config = Config::default();
        let mut player_config = PlayerConfig::default();

        init_play_state(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        advance_to_playing(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );

        // Run for 2 seconds
        run_game_loop(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            2_000_000,
            10_000,
        );

        // Should have at least 3 gauge log entries (at 0.5s, 1.0s, 1.5s)
        let log = state.gauge_log();
        assert!(
            log.len() >= 3,
            "Expected >= 3 gauge log entries, got {}",
            log.len()
        );
        // Each entry should have 9 values (one per GaugeType)
        for entry in log {
            assert_eq!(entry.len(), 9, "Each gauge log entry should have 9 values");
        }
    }

    #[test]
    fn gauge_auto_shift_continue_does_not_fail() {
        let mut state = PlayState::new();
        state.set_autoplay(false); // Manual play, no input -> all MISS

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(load_test_model("minimal_7k.bms"));
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        player_config.gauge = 3; // Hard gauge
        player_config.gauge_auto_shift = 1; // Continue

        init_play_state(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        advance_to_playing(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );

        // Run for full playtime — should not transition to Failed
        let end_time = state.playtime_us + 1_000_000;
        let result = run_game_loop(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            end_time,
            10_000,
        );

        // Should reach Finished or Result, never Failed
        assert_ne!(state.phase(), PlayPhase::Failed);
        assert!(
            result == Some(AppStateType::Result)
                || state.phase() == PlayPhase::Finished
                || state.phase() == PlayPhase::Playing,
        );
    }

    // --- ScoreData tests ---

    #[test]
    fn shutdown_saves_score_data() {
        let mut state = PlayState::new();
        state.set_autoplay(true);

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(load_test_model("minimal_7k.bms"));
        let config = Config::default();
        let mut player_config = PlayerConfig::default();

        init_play_state(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        advance_to_playing(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );

        // Run until done
        let end_time = state.playtime_us + 1_000_000;
        run_game_loop(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            end_time,
            1_000,
        );

        // Call shutdown
        let mut transition = None;
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.shutdown(&mut ctx);

        // Score should be populated
        let score = &resource.score_data;
        assert!(score.judge_count(bms_rule::JUDGE_PG) > 0);
        assert!(score.maxcombo > 0);
        // Autoplay: update_score should be false
        assert!(!resource.update_score);
    }

    // --- Replay tests ---

    #[test]
    fn replay_mode_processes_events() {
        let model = load_test_model("minimal_7k.bms");
        let judge_notes = model.build_judge_notes();
        let lp = LaneProperty::new(model.mode);

        // Create simple replay: press each note at its time
        let mut log = Vec::new();
        for note in &judge_notes {
            if !note.is_playable() || note.is_long_note() {
                continue;
            }
            let keys = lp.lane_to_keys(note.lane);
            let key = keys[0] as i32;
            log.push(KeyInputLog::new(note.time_us, key, true));
            log.push(KeyInputLog::new(note.time_us + 80_000, key, false));
        }

        let mut state = PlayState::new();
        state.set_autoplay(false);
        state.set_replay_log(log);

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(model);
        let config = Config::default();
        let mut player_config = PlayerConfig::default();

        init_play_state(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        advance_to_playing(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );

        // Run full game loop
        let end_time = state.playtime_us + 1_000_000;
        run_game_loop(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            end_time,
            1_000,
        );

        // All notes should be PGREAT (pressed at exact time)
        let score = state.score().expect("score should exist");
        let pg = score.judge_count(bms_rule::JUDGE_PG);
        assert!(pg > 0, "PG count should be > 0");
        assert_eq!(score.judge_count(JUDGE_MS), 0);
    }

    // --- GaugeAutoShift tests ---

    #[test]
    fn gauge_auto_shift_survival_to_groove() {
        let mut state = PlayState::new();
        state.set_autoplay(false);

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.bms_model = Some(load_test_model("minimal_7k.bms"));
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        player_config.gauge = 3; // Hard gauge
        player_config.gauge_auto_shift = 2; // SurvivalToGroove

        init_play_state(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );
        advance_to_playing(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
        );

        // Run enough to kill the hard gauge (no input = all MISS)
        let end_time = state.playtime_us + 1_000_000;
        run_game_loop(
            &mut state,
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            end_time,
            10_000,
        );

        // Should have shifted to Normal gauge
        assert_eq!(state.gauge_type(), Some(GaugeType::Normal));
        // Should NOT be in Failed phase
        assert_ne!(state.phase(), PlayPhase::Failed);
    }

    // --- Gauge type conversion tests ---

    #[test]
    fn gauge_type_from_i32_all_values() {
        assert_eq!(gauge_type_from_i32(0), GaugeType::AssistEasy);
        assert_eq!(gauge_type_from_i32(1), GaugeType::Easy);
        assert_eq!(gauge_type_from_i32(2), GaugeType::Normal);
        assert_eq!(gauge_type_from_i32(3), GaugeType::Hard);
        assert_eq!(gauge_type_from_i32(4), GaugeType::ExHard);
        assert_eq!(gauge_type_from_i32(5), GaugeType::Hazard);
        assert_eq!(gauge_type_from_i32(99), GaugeType::Normal);
    }

    #[test]
    fn gauge_auto_shift_from_i32_all_values() {
        assert_eq!(GaugeAutoShift::from_i32(0), GaugeAutoShift::None);
        assert_eq!(GaugeAutoShift::from_i32(1), GaugeAutoShift::Continue);
        assert_eq!(
            GaugeAutoShift::from_i32(2),
            GaugeAutoShift::SurvivalToGroove
        );
        assert_eq!(GaugeAutoShift::from_i32(3), GaugeAutoShift::BestClear);
        assert_eq!(GaugeAutoShift::from_i32(4), GaugeAutoShift::SelectToUnder);
        assert_eq!(GaugeAutoShift::from_i32(99), GaugeAutoShift::None);
    }

    // --- Key beam timer tests ---

    #[test]
    fn key_beam_press_activates_keyon_timer() {
        let lp = LaneProperty::new(PlayMode::Beat7K);
        let mut timer = TimerManager::new();
        timer.set_now_micro_time(1_000_000);

        // Press key for lane 0 (offset=1, player=0)
        let mut key_states = vec![false; lp.physical_key_count()];
        key_states[0] = true;
        let auto_pt = vec![NOT_SET; lp.physical_key_count()];

        update_key_beam_timers(&lp, &key_states, &auto_pt, false, false, false, &mut timer);

        // TIMER_KEYON_1P_KEY1 (offset=1) should be on
        assert!(timer.is_timer_on(bms_skin::property_id::TIMER_KEYON_1P_KEY1));
        assert!(!timer.is_timer_on(bms_skin::property_id::TIMER_KEYOFF_1P_KEY1));
    }

    #[test]
    fn key_beam_release_activates_keyoff_timer() {
        let lp = LaneProperty::new(PlayMode::Beat7K);
        let mut timer = TimerManager::new();
        timer.set_now_micro_time(1_000_000);

        // First press
        let mut key_states = vec![false; lp.physical_key_count()];
        key_states[0] = true;
        let auto_pt = vec![NOT_SET; lp.physical_key_count()];
        update_key_beam_timers(&lp, &key_states, &auto_pt, false, false, false, &mut timer);

        // Then release
        timer.set_now_micro_time(2_000_000);
        key_states[0] = false;
        update_key_beam_timers(&lp, &key_states, &auto_pt, false, false, false, &mut timer);

        // TIMER_KEYOFF_1P_KEY1 should be on, KEYON should be off
        assert!(timer.is_timer_on(bms_skin::property_id::TIMER_KEYOFF_1P_KEY1));
        assert!(!timer.is_timer_on(bms_skin::property_id::TIMER_KEYON_1P_KEY1));
    }

    #[test]
    fn key_beam_stop_prevents_timer_changes() {
        let lp = LaneProperty::new(PlayMode::Beat7K);
        let mut timer = TimerManager::new();
        timer.set_now_micro_time(1_000_000);

        let mut key_states = vec![false; lp.physical_key_count()];
        key_states[0] = true;
        let auto_pt = vec![NOT_SET; lp.physical_key_count()];

        // key_beam_stop = true → no timer activation
        update_key_beam_timers(&lp, &key_states, &auto_pt, true, false, false, &mut timer);

        assert!(!timer.is_timer_on(bms_skin::property_id::TIMER_KEYON_1P_KEY1));
    }

    #[test]
    fn key_beam_scratch_activates_offset_0() {
        let lp = LaneProperty::new(PlayMode::Beat7K);
        let mut timer = TimerManager::new();
        timer.set_now_micro_time(1_000_000);

        // Press scratch (key 7 maps to lane 7, offset=0)
        let mut key_states = vec![false; lp.physical_key_count()];
        key_states[7] = true;
        let auto_pt = vec![NOT_SET; lp.physical_key_count()];

        update_key_beam_timers(&lp, &key_states, &auto_pt, false, false, false, &mut timer);

        // TIMER_KEYON_1P_SCRATCH (offset=0) should be on
        assert!(timer.is_timer_on(bms_skin::property_id::TIMER_KEYON_1P_SCRATCH));
    }
}
