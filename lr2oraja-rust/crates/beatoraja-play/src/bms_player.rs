use crate::bga::bga_processor::BGAProcessor;
use crate::control_input_processor::ControlInputProcessor;
use crate::groove_gauge::GrooveGauge;
use crate::judge_manager::JudgeManager;
use crate::key_input_processor::KeyInputProccessor;
use crate::key_sound_processor::KeySoundProcessor;
use crate::lane_property::LaneProperty;
use crate::lane_renderer::LaneRenderer;
use crate::practice_configuration::PracticeConfiguration;
use crate::rhythm_timer_processor::RhythmTimerProcessor;
use beatoraja_core::score_data::ScoreData;
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
}

impl BMSPlayer {
    pub fn new(model: BMSModel) -> Self {
        let playtime = model.get_last_note_time() + TIME_MARGIN;
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
        }
    }

    pub fn create(&mut self) {
        let mode = self.model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
        self.lane_property = Some(LaneProperty::new(&mode));
        self.judge = JudgeManager::new();
        self.control = Some(ControlInputProcessor::new(mode));
        if let Some(ref lp) = self.lane_property {
            self.keyinput = Some(KeyInputProccessor::new(lp));
        }

        self.lanerender = Some(LaneRenderer::new(&self.model));

        // TODO: Phase 7+ dependency - requires skin loading, audio setup, input setup
        self.judge.init(&self.model, 0);

        let use_expansion = false; // TODO: from PlaySkin note expansion rate
        self.rhythm = Some(RhythmTimerProcessor::new(&self.model, use_expansion));
        self.bga = BGAProcessor::new();
    }

    pub fn render(&mut self) {
        // TODO: Phase 7+ dependency - main render/state machine loop (400+ lines in Java)
        // Handles state transitions:
        // STATE_PRELOAD -> STATE_READY (when media loaded)
        // STATE_PRACTICE -> STATE_READY (when play pressed)
        // STATE_READY -> STATE_PLAY (after playstart margin)
        // STATE_PLAY -> STATE_FINISHED / STATE_FAILED
        // STATE_FINISHED -> result screen
        // STATE_FAILED -> result screen / retry
        // STATE_ABORTED -> retry / music select
    }

    pub fn set_play_speed(&mut self, playspeed: i32) {
        self.playspeed = playspeed;
        // TODO: Phase 7+ dependency - audio pitch change
    }

    pub fn get_play_speed(&self) -> i32 {
        self.playspeed
    }

    pub fn input(&mut self) {
        if let Some(ref mut control) = self.control {
            control.input();
        }
        if let Some(ref mut keyinput) = self.keyinput {
            keyinput.input();
        }
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

    pub fn stop_play(&mut self) {
        // TODO: Phase 7+ dependency - full stop play logic (50+ lines in Java)
        // Handles different stop scenarios based on current state
        match self.state {
            STATE_PRACTICE => {
                self.practice.save_property();
                self.state = STATE_PRACTICE_FINISHED;
            }
            STATE_PRELOAD | STATE_READY => {
                self.state = STATE_PRACTICE_FINISHED;
            }
            _ => {
                if self.state != STATE_FINISHED {
                    self.state = STATE_FAILED;
                }
            }
        }
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut lr) = self.lanerender {
            lr.dispose();
        }
        self.practice.dispose();
    }

    pub fn create_score_data(&self) -> Option<ScoreData> {
        // TODO: Phase 7+ dependency - full score data creation (100+ lines in Java)
        // Creates ScoreData with clear type, option, seed, ghost, replay data, etc.
        let score = self.judge.get_score_data().clone();
        Some(score)
    }

    pub fn update(&mut self, judge: i32, time: i64) {
        if self.judge.get_combo() == 0 {
            self.bga.set_misslayer_tme(time);
        }
        if let Some(ref mut gauge) = self.gauge {
            gauge.update(judge);
        }
        // TODO: Phase 7+ dependency - timer updates, score property updates
    }

    pub fn is_note_end(&self) -> bool {
        // TODO: compare with songdata notes
        false
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
        // TODO: Phase 7+ dependency - requires PlayerResource, constraint check, PlayerConfig
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

    pub fn get_now_quarter_note_time(&self) -> i64 {
        self.rhythm
            .as_ref()
            .map_or(0, |r| r.get_now_quarter_note_time())
    }
}
