use super::*;

impl BMSPlayer {
    pub fn new(model: BMSModel) -> Self {
        Self::new_with_resource_gen(model, 1)
    }

    /// Create a BMSPlayer with the given song_resource_gen for BGAProcessor cache sizing.
    /// Java: BGAProcessor(256, Math.max(config.getSongResourceGen(), 1))
    pub fn new_with_resource_gen(model: BMSModel, song_resource_gen: i32) -> Self {
        let playtime = model.last_note_time() + TIME_MARGIN;
        let total_notes = model.total_notes();
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
            input: PlayerInputState::new(),
            keysound: KeySoundProcessor::new(),
            assist: 0,
            playspeed: 100,
            state: PlayState::Preload,
            prevtime: 0,
            practice: PracticeConfiguration::new(),
            starttimeoffset: 0,
            rhythm: None,
            startpressedtime: 0,
            adjusted_volume: -1.0,
            score: PlayerScoreState::new(),
            gaugelog: Vec::new(),
            play_skin: PlaySkin::new(),
            main_state_data: MainStateData::new(TimerManager::new()),
            total_notes,
            margin_time: 0,
            pending: PendingActions::new(),
            fast_forward_freq_option: FrequencyType::UNPROCESSED,
            bg_volume: 0.5,
            play_mode: BMSPlayerMode::PLAY,
            constraints: Vec::new(),
            is_guide_se: false,
            create_side_effects: None,
            player_config: PlayerConfig::default(),
            chart_option: None,
            skin_name: None,
            media_load_finished: false,
            is_course_mode: false,
            device_type: rubato_input::bms_player_input_device::DeviceType::Keyboard,
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
    pub fn bga_processor_arc(&self) -> Arc<Mutex<BGAProcessor>> {
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
        result: Option<rubato_audio::bms_loudness_analyzer::AnalysisResult>,
    ) {
        self.score.analysis_result = result;
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
    pub fn play_mode(&self) -> &BMSPlayerMode {
        &self.play_mode
    }

    /// Set course constraints before calling create().
    ///
    /// When NO_SPEED is present, control input (speed changes) will be disabled.
    pub fn set_constraints(&mut self, constraints: Vec<CourseDataConstraint>) {
        self.constraints = constraints;
    }

    /// Get course constraints.
    pub fn constraints(&self) -> &[CourseDataConstraint] {
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
    pub fn player_config(&self) -> &PlayerConfig {
        &self.player_config
    }

    /// Take the pending state change (if any). Returns None if no transition is pending.
    /// The caller should apply this via main.changeState().
    pub fn take_pending_state_change(&mut self) -> Option<MainStateType> {
        self.pending.pending_state_change.take()
    }

    /// Set whether we are in course mode.
    pub fn set_course_mode(&mut self, is_course: bool) {
        self.is_course_mode = is_course;
    }

    /// Queue a system sound to be played by MainController.
    pub(super) fn queue_sound(&mut self, sound: rubato_types::sound_type::SoundType) {
        self.pending.pending_sounds.push((sound, false));
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
            self.pending.pending_global_pitch = Some(playspeed as f32 / 100.0);
        }
    }

    pub fn play_speed(&self) -> i32 {
        self.playspeed
    }

    pub fn keyinput(&mut self) -> Option<&mut KeyInputProccessor> {
        self.input.keyinput.as_mut()
    }

    /// Get a reference to the player input state sub-struct.
    pub fn input_state(&self) -> &PlayerInputState {
        &self.input
    }

    /// Get a mutable reference to the player input state sub-struct.
    pub fn input_state_mut(&mut self) -> &mut PlayerInputState {
        &mut self.input
    }

    pub fn state(&self) -> PlayState {
        self.state
    }

    pub fn adjusted_volume(&self) -> f32 {
        self.adjusted_volume
    }

    /// Drain pending BG note commands from the autoplay thread.
    ///
    /// The caller should call `AudioDriver::play_note(note, volume, 0)` for each
    /// returned command. This should be called each frame from the main render loop.
    pub fn drain_pending_bg_notes(&self) -> Vec<crate::key_sound_processor::BgNoteCommand> {
        self.keysound.drain_pending_bg_notes()
    }

    pub fn lanerender(&self) -> Option<&LaneRenderer> {
        self.lanerender.as_ref()
    }

    pub fn lanerender_mut(&mut self) -> Option<&mut LaneRenderer> {
        self.lanerender.as_mut()
    }

    pub fn lane_property(&self) -> Option<&LaneProperty> {
        self.lane_property.as_ref()
    }

    pub fn judge_manager(&self) -> &JudgeManager {
        &self.judge
    }

    pub fn judge_manager_mut(&mut self) -> &mut JudgeManager {
        &mut self.judge
    }

    pub fn gauge(&self) -> Option<&GrooveGauge> {
        self.gauge.as_ref()
    }

    pub fn gauge_mut(&mut self) -> Option<&mut GrooveGauge> {
        self.gauge.as_mut()
    }

    /// Get a shared reference to the BGA processor.
    /// Used by the skin system to connect the SkinBgaObject for BGA rendering.
    pub fn bga_processor(&self) -> &Arc<Mutex<BGAProcessor>> {
        &self.bga
    }

    /// Set the active replay data for keylog playback.
    /// Should be called when entering REPLAY mode after restore_replay_data().
    pub fn set_active_replay(&mut self, replay: Option<ReplayData>) {
        self.score.active_replay = replay;
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
        self.score.db_score = score;
    }

    /// Set the rival score data from PlayerResource.
    ///
    /// The caller should read this from `PlayerResourceAccess::rival_score_data()`.
    /// When rival score is available and not in course mode, it will be used as the
    /// target score in `create()`.
    ///
    /// Java: `resource.getRivalScoreData()`
    pub fn set_rival_score(&mut self, score: Option<ScoreData>) {
        self.score.rival_score = score;
    }

    /// Set the target score data computed from TargetProperty.
    ///
    /// The caller should compute this via
    /// `TargetProperty::from_id(config.targetid).target(main)`
    /// when rival score is None or when in course mode.
    /// If rival score is set and not in course mode, this field is ignored
    /// (rival score is used as the target instead).
    ///
    /// Java: `TargetProperty.getTargetProperty(config.getTargetid()).getTarget(main)`
    pub fn set_target_score(&mut self, score: Option<ScoreData>) {
        self.score.target_score = score;
    }

    /// Take the pending global pitch value, if any.
    /// After calling this, the pending value is cleared (consumed).
    /// The caller should apply the returned pitch to the audio driver.
    pub fn take_pending_global_pitch(&mut self) -> Option<f32> {
        self.pending.pending_global_pitch.take()
    }

    /// Apply loudness analysis result to compute the adjusted volume.
    ///
    /// Translated from: BMSPlayer.render() PlayState::Preload loudness check (Java lines 614-641)
    ///
    /// When called, sets `adjusted_volume` based on the analysis result.
    /// Returns the adjusted volume (or -1.0 if analysis failed).
    pub fn apply_loudness_analysis(
        &mut self,
        analysis_result: &rubato_audio::bms_loudness_analyzer::AnalysisResult,
        config_key_volume: f32,
    ) -> f32 {
        self.score.analysis_checked = true;
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
        self.score.analysis_checked
    }

    pub fn practice_configuration(&self) -> &PracticeConfiguration {
        &self.practice
    }

    pub fn practice_configuration_mut(&mut self) -> &mut PracticeConfiguration {
        &mut self.practice
    }
}
