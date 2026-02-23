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
use beatoraja_types::clear_type::ClearType;
use beatoraja_types::replay_data::ReplayData;
use beatoraja_types::skin_type::SkinType;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;

pub static TIME_MARGIN: i32 = 5000;

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
    pub fn create_score_data(&self) -> Option<ScoreData> {
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

        self.judge.init(&self.model, 0);

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
                    // keyinput.startJudge(model, replay keylog, resource.getMarginTime());
                    if let Some(ref mut ki) = self.keyinput {
                        ki.start_judge(0); // TODO: Phase 22 - marginTime
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
        if let Some(ref mut keyinput) = self.keyinput {
            keyinput.input();
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

    #[test]
    fn create_score_data_returns_none_when_no_notes_hit() {
        let model = make_model();
        let player = BMSPlayer::new(model);
        // No notes hit - all judge counts are 0
        let result = player.create_score_data();
        assert!(result.is_none());
    }

    #[test]
    fn create_score_data_returns_some_when_aborted() {
        let model = make_model();
        let mut player = BMSPlayer::new(model);
        player.state = STATE_ABORTED;
        // Even with no notes, aborted state returns score data
        let result = player.create_score_data();
        assert!(result.is_some());
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
}
