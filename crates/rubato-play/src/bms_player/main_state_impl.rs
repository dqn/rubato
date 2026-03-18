use super::*;
use rubato_types::sync_utils::lock_or_recover;

fn judge_timer_id(player: usize) -> rubato_types::timer_id::TimerId {
    match player {
        0 => rubato_types::timer_id::TimerId::new(46),
        1 => rubato_types::timer_id::TimerId::new(47),
        2 => rubato_types::timer_id::TimerId::new(247),
        _ => rubato_types::timer_id::TimerId::UNDEFINED,
    }
}

fn combo_timer_id(player: usize) -> rubato_types::timer_id::TimerId {
    match player {
        0 => rubato_types::timer_id::TimerId::new(446),
        1 => rubato_types::timer_id::TimerId::new(447),
        2 => rubato_types::timer_id::TimerId::new(448),
        _ => rubato_types::timer_id::TimerId::UNDEFINED,
    }
}

fn bomb_timer_id(player: i32, key: i32) -> rubato_types::timer_id::TimerId {
    if player < 2 {
        if key < 10 {
            return rubato_types::timer_id::TimerId::new(50 + key + player * 10);
        }
        if key < 100 {
            return rubato_types::timer_id::TimerId::new(1010 + key - 10 + player * 100);
        }
    }
    rubato_types::timer_id::TimerId::UNDEFINED
}

/// Maximum number of gauge log entries to pad on stage failure.
/// 100_000 entries at 500ms intervals covers ~14 hours, far exceeding any
/// realistic song length while preventing unbounded allocation from corrupted
/// playtime values.
const GAUGELOG_PAD_MAX_ENTRIES: i64 = 100_000;

/// Fill remaining gauge log entries with 0.0 from `start_ms` up to
/// `playtime + 500` (in milliseconds), capped at `GAUGELOG_PAD_MAX_ENTRIES`
/// to guard against corrupted playtime.
pub(crate) fn pad_gaugelog_with_zeros(gaugelog: &mut [Vec<f32>], start_ms: i64, playtime: i32) {
    let mut l = start_ms;
    let mut entries_added = 0_i64;
    while l < playtime as i64 + 500 && entries_added < GAUGELOG_PAD_MAX_ENTRIES {
        for glog in gaugelog.iter_mut() {
            glog.push(0.0);
        }
        l += 500;
        entries_added += 1;
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
        self.pending.pending_state_change.take()
    }

    fn take_pending_global_pitch(&mut self) -> Option<f32> {
        self.pending.pending_global_pitch.take()
    }

    fn drain_pending_sounds(&mut self) -> Vec<(rubato_types::sound_type::SoundType, bool)> {
        std::mem::take(&mut self.pending.pending_sounds)
    }

    fn take_score_handoff(&mut self) -> Option<rubato_types::score_handoff::ScoreHandoff> {
        self.pending.pending_score_handoff.take()
    }

    fn take_state_create_effects(&mut self) -> Option<rubato_core::main_state::StateCreateEffects> {
        let effects = self.create_side_effects.take()?;
        Some(rubato_core::main_state::StateCreateEffects {
            play_config_mode: match effects.input_mode_action {
                InputModeAction::SetPlayConfig(mode) => Some(mode),
                _ => None,
            },
            disable_input: matches!(effects.input_mode_action, InputModeAction::DisableInput),
            guide_se: effects.is_guide_se,
        })
    }

    fn take_pending_reload_bms(&mut self) -> bool {
        std::mem::take(&mut self.pending.pending_reload_bms)
    }

    fn take_pending_play_config_update(
        &mut self,
    ) -> Option<(bms_model::mode::Mode, rubato_types::play_config::PlayConfig)> {
        self.pending.pending_play_config_update.take()
    }

    fn receive_updated_play_config(
        &mut self,
        mode: bms_model::mode::Mode,
        play_config: rubato_types::play_config::PlayConfig,
    ) {
        if let Some(ref mut lr) = self.lanerender {
            lr.apply_play_config(&play_config);
            lr.init(&self.model);
        }
        self.player_config.play_config(mode).playconfig = play_config;
    }

    fn notify_media_load_finished(&mut self) {
        self.media_load_finished = true;
    }

    fn receive_reloaded_model(&mut self, model: bms_model::bms_model::BMSModel) {
        self.model = model;
    }

    fn bms_model(&self) -> Option<&bms_model::bms_model::BMSModel> {
        Some(&self.model)
    }

    fn take_bga_cache(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        // Return the Arc<Mutex<BGAProcessor>> for caching on PlayerResource.
        // The Arc is cloned so that BMSPlayer can still hold a reference
        // (though it will be dropped shortly after during state transition).
        Some(Box::new(Arc::clone(&self.bga)))
    }

    fn render_skin(&mut self, sprite: &mut rubato_render::sprite_batch::SpriteBatch) {
        self.render_skin_impl(sprite);
    }

    fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        self.handle_skin_mouse_pressed_impl(button, x, y);
    }

    fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        self.handle_skin_mouse_dragged_impl(button, x, y);
    }

    fn create(&mut self) {
        let mode = self.model.mode().copied().unwrap_or(Mode::BEAT_7K);
        self.lane_property = Some(LaneProperty::new(&mode));
        self.input.control = Some(ControlInputProcessor::new(mode));
        if let Some(ref lp) = self.lane_property {
            self.input.keyinput = Some(KeyInputProccessor::new(lp));
        }

        // --- loadSkin(getSkinType()) ---
        // Translated from: BMSPlayer.create() Java line 510
        // In Java: loadSkin(getSkinType());
        // This delegates to MainState.loadSkin() which calls SkinLoader.load().
        // The actual skin loading requires SkinLoader integration; we call the
        // trait method which logs a warning if not yet wired. The skin type is
        // captured in CreateSideEffects for the caller to use.
        let skin_type = self.skin_type();
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
            rubato_core::bms_player_mode::Mode::Play
            | rubato_core::bms_player_mode::Mode::Practice => InputModeAction::SetPlayConfig(mode),
            rubato_core::bms_player_mode::Mode::Autoplay
            | rubato_core::bms_player_mode::Mode::Replay => InputModeAction::DisableInput,
        };

        // Store side effects for the caller
        self.create_side_effects = Some(CreateSideEffects {
            is_guide_se: self.is_guide_se,
            input_mode_action,
            skin_type,
        });

        let mut lr = LaneRenderer::new(&self.model);
        let play_config = &self.player_config.play_config_ref(mode).playconfig;
        lr.apply_play_config(play_config);
        lr.init(&self.model);
        self.lanerender = Some(lr);

        // --- NO_SPEED constraint ---
        // Translated from: BMSPlayer.create() Java lines 533-538
        // ```java
        // for (CourseData.CourseDataConstraint i : resource.getConstraint()) {
        //     if (i == NO_SPEED) { control.setEnableControl(false); break; }
        // }
        // ```
        if self.constraints.contains(&CourseDataConstraint::NoSpeed)
            && let Some(ref mut control) = self.input.control
        {
            control.enable_control = false;
        }

        // --- Judge initialization ---
        // Translated from: Java BMSPlayer.create() judge.init() call.
        // Uses from_config() which properly initializes lane_states, note_states,
        // keyassign, sckey, auto_presstime, and all lane iteration state.
        self.rebuild_judge_system(&mode);

        // --- Gauge initialization ---
        // Translated from: BMSPlayer.create() Java line ~540
        // gauge = GrooveGauge.create(model, gauge_type, resource)
        // For practice mode, gauge is initialized later in the practice loop (line 581).
        if self.play_mode.mode != rubato_core::bms_player_mode::Mode::Practice {
            // Java: gauge = GrooveGauge.create(model, replay != null ? replay.gauge : config.getGauge(), resource);
            // In replay mode, use the gauge type from the replay data.
            let gauge_type = if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay {
                self.score
                    .active_replay
                    .as_ref()
                    .map(|r| r.gauge)
                    .unwrap_or(self.player_config.play_settings.gauge)
            } else {
                self.player_config.play_settings.gauge
            };
            let grade = if self.is_course_mode { 1 } else { 0 };

            // Java: GrooveGauge.create(model, type, resource) extracts gauge property
            // from course constraints when in course mode. Map constraint variants to
            // GaugeProperty for courses that specify gauge tables.
            let gauge_property = if self.is_course_mode {
                self.constraints.iter().find_map(|c| match c {
                    CourseDataConstraint::Gauge5Keys => {
                        Some(crate::gauge_property::GaugeProperty::FiveKeys)
                    }
                    CourseDataConstraint::Gauge7Keys => {
                        Some(crate::gauge_property::GaugeProperty::SevenKeys)
                    }
                    CourseDataConstraint::Gauge9Keys => {
                        Some(crate::gauge_property::GaugeProperty::Pms)
                    }
                    CourseDataConstraint::Gauge24Keys => {
                        Some(crate::gauge_property::GaugeProperty::Keyboard)
                    }
                    CourseDataConstraint::GaugeLr2 => {
                        Some(crate::gauge_property::GaugeProperty::Lr2)
                    }
                    _ => None,
                })
            } else {
                None
            };

            self.gauge = crate::groove_gauge::create_groove_gauge(
                &self.model,
                gauge_type,
                grade,
                gauge_property,
            );

            // Java: GrooveGauge.create(model, type, resource) restores gauge values
            // from the previous course stage. After creating the gauge, read the last
            // value from each gauge type's log and restore it.
            if let Some(ref previous_values) = self.previous_gauge_values
                && let Some(ref mut gauge) = self.gauge
            {
                for (i, log) in previous_values.iter().enumerate() {
                    if let Some(&last_val) = log.last() {
                        gauge.set_value_by_type(i as i32, last_val);
                    }
                }
            }
        }

        // --- Note expansion rate from PlaySkin ---
        // Translated from: BMSPlayer.create() Java line 542-543
        // ```java
        // rhythm = new RhythmTimerProcessor(model,
        //     (getSkin() instanceof PlaySkin) ? ((PlaySkin) getSkin()).getNoteExpansionRate()[0] != 100
        //         || ((PlaySkin) getSkin()).getNoteExpansionRate()[1] != 100 : false);
        // ```
        let rates = &self.play_skin.note_expansion_rate;
        let use_expansion = rates[0] != 100 || rates[1] != 100;
        self.rhythm = Some(RhythmTimerProcessor::new(&self.model, use_expansion));

        // Reuse existing BGAProcessor (injected via set_bga_processor from PlayerResource)
        // to preserve the texture cache between plays. Only update timelines for the new model.
        // Java: bga = resource.getBGAManager(); (BMSPlayer.java line 545)
        if let Ok(mut bga) = self.bga.lock() {
            bga.set_model_timelines(&self.model);
            bga.get_misslayer_duration =
                self.player_config.display_settings.misslayer_duration as i64;

            // Load BGA images and movies from model.bgamap.
            // Java: BMSResource dispatches image/movie loading after setModel().
            let base_dir = self
                .model
                .path()
                .and_then(|p| std::path::Path::new(&p).parent().map(|d| d.to_path_buf()));
            if let Some(ref dir) = base_dir {
                bga.set_movie_count(self.model.bgamap.len());
                for (id, entry) in self.model.bgamap.iter().enumerate() {
                    if entry.is_empty() {
                        continue;
                    }
                    let path = dir.join(entry);
                    if !path.exists() {
                        continue;
                    }
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if crate::bga::bg_image_processor::PIC_EXTENSION
                        .iter()
                        .any(|&e| e == ext)
                    {
                        bga.put_image(id, &path);
                    } else if crate::bga::bga_processor::MOV_EXTENSION
                        .iter()
                        .any(|&e| e == ext)
                    {
                        let mut mp = crate::bga::ffmpeg_processor::FFmpegProcessor::new(1);
                        mp.create(&path.to_string_lossy());
                        bga.set_movie(id, Box::new(mp));
                    }
                }
            }
        }

        // Initialize gauge log
        if let Some(ref gauge) = self.gauge {
            let gauge_type_len = gauge.gauge_type_length();
            self.gaugelog = Vec::with_capacity(gauge_type_len);
            for _ in 0..gauge_type_len {
                self.gaugelog
                    .push(Vec::with_capacity(self.playtime.max(0) as usize / 500 + 2));
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
        //     state = PlayState::Practice;
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
        let score = self.score.db_score.clone().unwrap_or_default();
        log::info!("Score data loaded from score database");

        let total_notes = self.model.total_notes();

        if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Practice {
            self.main_state_data
                .score
                .set_target_score_with_ghost(0, None, 0, None, total_notes);
            self.practice.create(&self.model);
            self.state = PlayState::Practice;
        } else {
            // Determine the effective target score:
            // - If rival score is absent or in course mode, use the pre-computed target_score
            //   (caller should have computed via TargetProperty::from_id().target())
            // - Otherwise, use the rival score as the target
            let effective_target = if self.score.rival_score.is_none() || self.is_course_mode {
                self.score.target_score.clone()
            } else {
                self.score.rival_score.clone()
            };

            let (target_exscore, target_ghost) = match effective_target {
                Some(ref t) => (t.exscore(), t.decode_ghost()),
                None => (0, None),
            };

            self.main_state_data.score.set_target_score_with_ghost(
                score.exscore(),
                score.decode_ghost(),
                target_exscore,
                target_ghost,
                total_notes,
            );
        }
    }

    fn render(&mut self) {
        let micronow = self.main_state_data.timer.now_micro_time();

        // Input start timer
        let input_time = self.main_state_data.skin.as_ref().map_or(0, |s| s.input()) as i64; // skin.getInput() in Java
        if micronow > input_time * 1000 {
            self.main_state_data
                .timer
                .switch_timer(TIMER_STARTINPUT, true);
        }
        // startpressedtime tracking: update when START or SELECT is pressed
        // Translated from: Java BMSPlayer.render() line 590
        if self.input.input_start_pressed || self.input.input_select_pressed {
            self.startpressedtime = micronow;
        }

        match self.state {
            // PlayState::Preload - wait for resources
            PlayState::Preload => {
                // Chart preview handling
                // Translated from: Java BMSPlayer.render() lines 598-604
                if self.player_config.display_settings.chart_preview {
                    if self.main_state_data.timer.is_timer_on(TimerId::new(141))
                        && micronow > self.startpressedtime
                    {
                        self.main_state_data.timer.set_timer_off(TimerId::new(141));
                        if let Some(ref mut lr) = self.lanerender {
                            lr.init(&self.model);
                        }
                    } else if !self.main_state_data.timer.is_timer_on(TimerId::new(141))
                        && micronow == self.startpressedtime
                    {
                        self.main_state_data.timer.set_micro_timer(
                            TimerId::new(141),
                            micronow - self.starttimeoffset * 1000,
                        );
                    }
                }

                // Check if media loaded and load timers elapsed
                let load_threshold =
                    (self.play_skin.loadstart + self.play_skin.loadend) as i64 * 1000;
                // Translated from: Java BMSPlayer.render() lines 607-608
                if self.media_load_finished
                    && micronow > load_threshold
                    && micronow - self.startpressedtime > 1_000_000
                {
                    // Chart preview cleanup on transition
                    if self.player_config.display_settings.chart_preview {
                        self.main_state_data.timer.set_timer_off(TimerId::new(141));
                        if let Some(ref mut lr) = self.lanerender {
                            lr.init(&self.model);
                        }
                    }

                    // Loudness analysis check (Java BMSPlayer.render() lines 615-641)
                    if !self.score.analysis_checked {
                        self.adjusted_volume = -1.0;
                        self.score.analysis_checked = true;
                        if let Some(result) = self.score.analysis_result.take() {
                            let config_key_volume = self.bg_volume;
                            self.apply_loudness_analysis(&result, config_key_volume);
                        }
                    }

                    lock_or_recover(&self.bga).prepare(&() as &dyn std::any::Any);
                    self.state = PlayState::Ready;
                    self.main_state_data.timer.set_timer_on(TIMER_READY);
                    self.queue_sound(rubato_types::sound_type::SoundType::PlayReady);
                    log::info!("PlayState::Ready");
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

            // PlayState::Practice - practice mode config
            PlayState::Practice => {
                if self.main_state_data.timer.is_timer_on(TIMER_PLAY) {
                    // Reset for practice restart: reload BMS file to get a fresh model
                    // (modifiers mutate the model during play, so we need a clean copy).
                    // Java: resource.reloadBMSFile(); model = resource.getBMSModel();
                    // Rust: pending flag triggers MainController to reload resource and
                    // push fresh model back via receive_reloaded_model().
                    self.pending.pending_reload_bms = true;
                    if let Some(ref mut lr) = self.lanerender {
                        lr.init(&self.model);
                    }
                    if let Some(ref mut ki) = self.input.keyinput {
                        ki.key_beam_stop = false;
                    }
                    self.main_state_data.timer.set_timer_off(TIMER_PLAY);
                    self.main_state_data.timer.set_timer_off(TIMER_RHYTHM);
                    self.main_state_data.timer.set_timer_off(TIMER_FAILED);
                    self.main_state_data.timer.set_timer_off(TIMER_FADEOUT);
                    self.main_state_data.timer.set_timer_off(TIMER_ENDOFNOTE_1P);

                    for raw in TIMER_PM_CHARA_1P_NEUTRAL.as_i32()..=TIMER_PM_CHARA_DANCE.as_i32() {
                        self.main_state_data.timer.set_timer_off(TimerId::new(raw));
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
                if let Some(ref mut control) = self.input.control {
                    control.enable_control = false;
                    control.enable_cursor = false;
                }
                // Process practice input navigation (UP/DOWN/LEFT/RIGHT)
                // Translated from: Java BMSPlayer.render() line 680
                // Java uses System.currentTimeMillis() here for key-repeat debounce.
                // We use the monotonic game timer instead to avoid mixing clock domains
                // (wall-clock vs game-timer). The presscount logic only needs a
                // monotonically increasing millisecond value for rate-limiting.
                let now_millis = self.main_state_data.timer.now_time();
                // Control key states are read from input_key_states.
                // In the Java version, these come from BMSPlayerInputProcessor control keys.
                // For now we pass the input_start/select state as a proxy for key0 check.
                self.practice.process_input(
                    self.input.control_key_up,
                    self.input.control_key_down,
                    self.input.control_key_left,
                    self.input.control_key_right,
                    now_millis,
                );

                // Practice start logic: press key0 while media is loaded and timers elapsed
                // Translated from: Java BMSPlayer.render() lines 682-723
                let key0_pressed = self
                    .input
                    .input_key_states
                    .first()
                    .copied()
                    .unwrap_or(false);
                let load_threshold =
                    (self.play_skin.loadstart + self.play_skin.loadend) as i64 * 1000;
                if key0_pressed
                    && self.media_load_finished
                    && micronow > load_threshold
                    && micronow - self.startpressedtime > 1_000_000
                {
                    // Apply practice configuration and start play
                    if let Some(ref mut control) = self.input.control {
                        control.enable_control = true;
                        control.enable_cursor = true;
                    }

                    let property = self.practice.practice_property().clone();

                    // Apply frequency if != 100
                    if property.freq != 100 {
                        bms_model_utils::change_frequency(
                            &mut self.model,
                            property.freq as f32 / 100.0,
                        );
                        if self.fast_forward_freq_option == FrequencyType::FREQUENCY {
                            self.pending.pending_global_pitch = Some(property.freq as f32 / 100.0);
                        }
                        // Block IR submission and mark frequency training active
                        // (matches Java FreqTrainerResult semantics)
                        self.freq_on = true;
                        self.force_no_ir_send = true;
                    }

                    self.model.total = property.total;

                    // Apply practice modifier (time range)
                    let freq = property.freq.max(1) as i64;
                    let mut pm = rubato_core::pattern::practice_modifier::PracticeModifier::new(
                        property.starttime as i64 * 100 / freq,
                        property.endtime as i64 * 100 / freq,
                    );
                    pm.modify(&mut self.model);

                    // DP options
                    if self.model.mode().map_or(1, |m| m.player()) == 2 {
                        if property.doubleop == 1 {
                            let mut flip =
                                rubato_core::pattern::lane_shuffle_modifier::PlayerFlipModifier::new();
                            flip.modify(&mut self.model);
                        }
                        let mut pm2 =
                            rubato_core::pattern::pattern_modifier::create_pattern_modifier(
                                property.random2,
                                1,
                                &self.model.mode().copied().unwrap_or(Mode::BEAT_7K),
                                &self.player_config,
                            );
                        pm2.modify(&mut self.model);
                    }

                    // 1P random option
                    let mut pm1 = rubato_core::pattern::pattern_modifier::create_pattern_modifier(
                        property.random,
                        0,
                        &self.model.mode().copied().unwrap_or(Mode::BEAT_7K),
                        &self.player_config,
                    );
                    pm1.modify(&mut self.model);

                    // Gauge, judgerank, judge, lane init
                    self.gauge = self.practice.gauge(&self.model);
                    // Reinitialize gaugelog for the new gauge (practice restart).
                    // Without this, stale entries from the previous practice run remain.
                    if let Some(ref gauge) = self.gauge {
                        self.gaugelog =
                            (0..gauge.gauge_type_length()).map(|_| Vec::new()).collect();
                    }
                    self.model.judgerank = property.judgerank;
                    let mode = self.model.mode().copied().unwrap_or(Mode::BEAT_7K);
                    self.rebuild_judge_system(&mode);
                    if let Some(ref mut lr) = self.lanerender {
                        lr.init(&self.model);
                    }
                    self.play_skin.pomyu.init();

                    self.starttimeoffset = if property.starttime > 1000 {
                        (property.starttime as i64 - 1000) * 100 / freq
                    } else {
                        0
                    };
                    self.playtime =
                        ((property.endtime as i64 + 1000) * 100 / freq) as i32 + TIME_MARGIN;

                    lock_or_recover(&self.bga).prepare(&() as &dyn std::any::Any);
                    self.state = PlayState::Ready;
                    self.main_state_data.timer.set_timer_on(TIMER_READY);
                    log::info!("Practice -> PlayState::Ready");
                }
            }

            // PlayState::PracticeFinished
            // Translated from: Java BMSPlayer.render() lines 726-731
            PlayState::PracticeFinished => {
                let skin_fadeout = self
                    .main_state_data
                    .skin
                    .as_ref()
                    .map_or(0, |s| s.fadeout()) as i64;
                if self.main_state_data.timer.now_time_for_id(TIMER_FADEOUT) > skin_fadeout {
                    // input.setEnable(true); input.setStartTime(0);
                    self.pending.pending_global_pitch = Some(1.0);
                    self.pending.pending_state_change = Some(MainStateType::MusicSelect);
                    log::info!("Practice finished, transition to MUSICSELECT");
                }
            }

            // PlayState::Ready - countdown before play
            PlayState::Ready => {
                if self.main_state_data.timer.now_time_for_id(TIMER_READY)
                    > self.play_skin.playstart as i64
                {
                    if let Some(ref lr) = self.lanerender {
                        self.score.replay_config = Some(lr.play_config().clone());
                    }
                    self.state = PlayState::Play;
                    self.main_state_data
                        .timer
                        .set_micro_timer(TIMER_PLAY, micronow - self.starttimeoffset * 1000);
                    self.main_state_data
                        .timer
                        .set_micro_timer(TIMER_RHYTHM, micronow - self.starttimeoffset * 1000);

                    // Java: input.setStartTime(micronow + timer.getStartMicroTime() - starttimeoffset * 1000);
                    // Java: input.setKeyLogMarginTime(resource.getMarginTime());
                    let start_micro_time = self.main_state_data.timer.start_micro_time();
                    self.input.pending_input_start_time =
                        Some(micronow + start_micro_time - self.starttimeoffset * 1000);
                    self.input.pending_key_log_margin_time = Some(self.margin_time);
                    // Java: keyinput.startJudge(model, replay != null ? replay.keylog : null, resource.getMarginTime())
                    if let Some(ref mut ki) = self.input.keyinput {
                        let timelines = &self.model.timelines;
                        let last_tl_micro = timelines.last().map_or(0, |tl| tl.micro_time());
                        let keylog = self
                            .score
                            .active_replay
                            .as_ref()
                            .map(|r| r.keylog.as_slice());
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
                    log::info!("PlayState::Play");
                }
            }

            // PlayState::Play - main gameplay
            PlayState::Play => {
                let deltatime = micronow - self.prevtime;
                let deltaplay = deltatime.saturating_mul(100 - self.playspeed as i64) / 100;
                let freq = self.practice.practice_property().freq;
                let current_play_timer = self.main_state_data.timer.micro_timer(TIMER_PLAY);
                self.main_state_data
                    .timer
                    .set_micro_timer(TIMER_PLAY, current_play_timer + deltaplay);

                // Rhythm timer update
                let now_bpm = self.lanerender.as_ref().map_or(120.0, |lr| lr.now_bpm());
                if let Some(ref mut rhythm) = self.rhythm {
                    let play_timer_micro =
                        self.main_state_data.timer.now_micro_time_for_id(TIMER_PLAY);
                    let (rhythm_timer, rhythm_on) =
                        rhythm.update(&crate::rhythm_timer_processor::RhythmUpdateParams {
                            now: self.main_state_data.timer.now_time(),
                            micronow,
                            deltatime,
                            nowbpm: now_bpm,
                            play_speed: self.playspeed,
                            freq,
                            play_timer_micro,
                        });
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
                    let play_micro = self.main_state_data.timer.now_micro_time_for_id(TIMER_PLAY);
                    self.keysound.update_play_time(play_micro);
                    let vol = if self.adjusted_volume >= 0.0 {
                        self.adjusted_volume
                    } else {
                        self.bg_volume
                    };
                    self.keysound.update_volume(vol);
                }

                // Judge update: evaluate key presses against notes
                // Translated from: Java BMSPlayer.render() judge.update() call
                {
                    let play_micro = self.main_state_data.timer.now_micro_time_for_id(TIMER_PLAY);
                    if let Some(ref mut gauge) = self.gauge {
                        self.judge.update(
                            play_micro,
                            &self.judge_notes,
                            &self.input.input_key_states,
                            &self.input.input_key_changed_times,
                            gauge,
                        );
                    }
                    // Trigger key beam timers for newly judged lanes.
                    // In Java, JudgeManager calls keyinput.inputKeyOn(lane) directly;
                    // in Rust, we drain the event queue after update().
                    let judged = self.judge.drain_judged_lanes();
                    if !judged.is_empty()
                        && let Some(ref mut keyinput) = self.input.keyinput
                    {
                        for lane in judged {
                            keyinput.input_key_on(lane, &mut self.main_state_data.timer);
                        }
                    }
                    // Trigger judge/combo/bomb timers. Java does this inside
                    // JudgeManager.update2(); Rust queues it so the main thread
                    // owns all timer mutations.
                    let visual_events = self.judge.drain_judged_visual_events();
                    for event in visual_events {
                        if event.judge <= self.play_skin.judgetimer {
                            let bomb_timer =
                                bomb_timer_id(event.player as i32, event.offset as i32);
                            if bomb_timer != rubato_types::timer_id::TimerId::UNDEFINED {
                                self.main_state_data.timer.set_timer_on(bomb_timer);
                            }
                        }

                        let judge_timer = judge_timer_id(event.player);
                        if judge_timer != rubato_types::timer_id::TimerId::UNDEFINED {
                            self.main_state_data.timer.set_timer_on(judge_timer);
                        }

                        if self.play_skin.judgeregion >= 3 {
                            for player in 0..3 {
                                if player != event.player {
                                    let combo_timer = combo_timer_id(player);
                                    if combo_timer != rubato_types::timer_id::TimerId::UNDEFINED {
                                        self.main_state_data.timer.set_timer_off(combo_timer);
                                    }
                                }
                            }
                        }

                        let combo_timer = combo_timer_id(event.player);
                        if combo_timer != rubato_types::timer_id::TimerId::UNDEFINED {
                            self.main_state_data.timer.set_timer_on(combo_timer);
                        }
                    }
                    // Trigger per-judge side effects (BGA miss layer, score timers,
                    // pomyu, fullcombo timer). Corresponds to Java BMSPlayer.update(judge, time).
                    let events = self.judge.drain_judged_events();
                    for (judge, mtime) in events {
                        self.update_judge(judge, mtime);
                    }
                    // Sync judge states back to model notes so that create_score_data()
                    // and the result screen can read correct state/play_time values.
                    // In Java, JudgeManager modifies Note objects in-place via shared
                    // references; in Rust we must explicitly copy the results back.
                    self.sync_judge_states_to_model();

                    // Drain keysound events and resolve JudgeNote indices to model
                    // Notes for audio playback. Corresponds to Java
                    // keysound.play(note, keyvolume, 0) / keysound.setVolume(note, vol)
                    // calls in JudgeManager.update().
                    let key_volume = self.key_volume;
                    let play_indices = self.judge.drain_keysound_play_indices();
                    for note_idx in play_indices {
                        if let Some(note) = self.resolve_judge_note(note_idx) {
                            self.pending.pending_keysound_plays.push((note, key_volume));
                        }
                    }
                    let vol_indices = self.judge.drain_keysound_volume_set_indices();
                    for (note_idx, volume) in vol_indices {
                        if let Some(note) = self.resolve_judge_note(note_idx) {
                            // NaN sentinel means "use key_volume from config"
                            let resolved_vol = if volume.is_nan() { key_volume } else { volume };
                            self.pending
                                .pending_keysound_volume_sets
                                .push((note, resolved_vol));
                        }
                    }
                }

                let ptime = self.main_state_data.timer.now_time_for_id(TIMER_PLAY);
                // Gauge log
                if let Some(ref gauge) = self.gauge {
                    for (i, log) in self.gaugelog.iter_mut().enumerate() {
                        if log.len() as i64 <= ptime / 500 {
                            let val = gauge.value_by_type(i as i32);
                            log.push(val);
                        }
                    }
                    self.main_state_data
                        .timer
                        .switch_timer(TIMER_GAUGE_MAX_1P, gauge.gauge().is_max());
                }

                // pomyu timer update
                // Translated from: Java BMSPlayer.render() line 766
                let past_notes = self.judge.past_notes();
                let gauge_is_max = self.gauge.as_ref().is_some_and(|g| g.gauge().is_max());
                self.play_skin.pomyu.update_timer(
                    &mut self.main_state_data.timer,
                    past_notes,
                    gauge_is_max,
                );

                // Check play time elapsed
                if (self.playtime as i64) < ptime {
                    self.state = PlayState::Finished;
                    self.main_state_data.timer.set_timer_on(TIMER_MUSIC_END);
                    for raw in TIMER_PM_CHARA_1P_NEUTRAL.as_i32()..=TIMER_PM_CHARA_2P_BAD.as_i32() {
                        self.main_state_data.timer.set_timer_off(TimerId::new(raw));
                    }
                    self.main_state_data
                        .timer
                        .set_timer_off(TIMER_PM_CHARA_DANCE);
                    log::info!("PlayState::Finished");
                } else if (self.playtime - TIME_MARGIN) as i64 <= ptime {
                    self.main_state_data
                        .timer
                        .switch_timer(TIMER_ENDOFNOTE_1P, true);
                }

                // Stage failed check with gauge auto shift
                // Translated from: Java BMSPlayer.render() lines 782-815
                if let Some(ref mut gauge) = self.gauge {
                    let gas = self.player_config.play_settings.gauge_auto_shift;
                    use rubato_types::groove_gauge::{CLASS, EXHARDCLASS, HAZARD, NORMAL};
                    use rubato_types::player_config::{
                        GAUGEAUTOSHIFT_BESTCLEAR, GAUGEAUTOSHIFT_CONTINUE, GAUGEAUTOSHIFT_NONE,
                        GAUGEAUTOSHIFT_SELECT_TO_UNDER, GAUGEAUTOSHIFT_SURVIVAL_TO_GROOVE,
                    };

                    if gas == GAUGEAUTOSHIFT_BESTCLEAR || gas == GAUGEAUTOSHIFT_SELECT_TO_UNDER {
                        // Auto-shift to best qualifying gauge
                        let len = if gas == GAUGEAUTOSHIFT_BESTCLEAR {
                            if gauge.gauge_type() >= CLASS {
                                EXHARDCLASS + 1
                            } else {
                                HAZARD + 1
                            }
                        } else {
                            // SELECT_TO_UNDER
                            if gauge.is_course_gauge() {
                                (self
                                    .player_config
                                    .play_settings
                                    .gauge
                                    .clamp(NORMAL, EXHARDCLASS)
                                    + CLASS
                                    - NORMAL)
                                    .min(EXHARDCLASS)
                                    + 1
                            } else {
                                self.player_config.play_settings.gauge.min(HAZARD) + 1
                            }
                        };
                        let start_type = if gauge.is_course_gauge() {
                            CLASS
                        } else if gauge.gauge_type()
                            < self.player_config.play_settings.bottom_shiftable_gauge
                        {
                            gauge.gauge_type()
                        } else {
                            self.player_config.play_settings.bottom_shiftable_gauge
                        };
                        let mut best_type = start_type;
                        for i in start_type..len {
                            if gauge.value_by_type(i) > 0.0 && gauge.gauge_by_type(i).is_qualified()
                            {
                                best_type = i;
                            }
                        }
                        gauge.set_type(best_type);
                    } else if gauge.value() == 0.0 {
                        match gas {
                            GAUGEAUTOSHIFT_NONE => {
                                // FAILED transition
                                self.state = PlayState::Failed;
                                self.main_state_data.timer.set_timer_on(TIMER_FAILED);
                                if self.media_load_finished {
                                    self.pending.pending_stop_all_notes = true;
                                }
                                self.queue_sound(rubato_types::sound_type::SoundType::PlayStop);
                                log::info!("PlayState::Failed");
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

            // PlayState::Failed
            // Translated from: Java BMSPlayer.render() lines 818-869
            PlayState::Failed => {
                if let Some(ref mut control) = self.input.control {
                    control.enable_control = false;
                    control.enable_cursor = false;
                }
                if let Some(ref mut ki) = self.input.keyinput {
                    ki.stop_judge();
                }
                self.keysound.stop_bg_play();

                // Quick retry check (START xor SELECT)
                // Translated from: Java BMSPlayer.render() lines 823-838
                // Guard: skip if a state transition is already queued to avoid
                // calling save_config() on every frame while keys are held.
                if (self.input.input_start_pressed ^ self.input.input_select_pressed)
                    && !self.is_course_mode
                    && self.play_mode.mode == rubato_core::bms_player_mode::Mode::Play
                    && self.pending.pending_state_change.is_none()
                {
                    self.pending.pending_global_pitch = Some(1.0);
                    self.save_config();
                    self.pending.pending_reload_bms = true;
                    self.pending.pending_state_change = Some(MainStateType::Play);
                } else if self.main_state_data.timer.now_time_for_id(TIMER_FAILED)
                    > self.play_skin.close as i64
                {
                    self.pending.pending_global_pitch = Some(1.0);
                    // if resource.mediaLoadFinished() { resource.getBGAManager().stop(); }

                    // Fill remaining gauge log with 0
                    if self.main_state_data.timer.is_timer_on(TIMER_PLAY) {
                        let failed_time = self.main_state_data.timer.timer(TIMER_FAILED);
                        let play_time = self.main_state_data.timer.timer(TIMER_PLAY);
                        let start_ms = (failed_time - play_time).max(0);
                        pad_gaugelog_with_zeros(&mut self.gaugelog, start_ms, self.playtime);
                    }
                    // Ensure model notes have judge states before computing score data.
                    self.sync_judge_states_to_model();
                    let score = if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Play
                        || self.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay
                    {
                        self.create_score_data(self.device_type)
                    } else {
                        None
                    };
                    let replay = self.build_replay_data();
                    self.pending.pending_score_handoff =
                        Some(rubato_types::score_handoff::ScoreHandoff {
                            score_data: score,
                            combo: self.judge.course_combo(),
                            maxcombo: self.judge.course_maxcombo(),
                            gauge: self.gaugelog.clone(),
                            groove_gauge: self.gauge.clone(),
                            assist: self.assist,
                            freq_on: self.freq_on,
                            force_no_ir_send: self.force_no_ir_send,
                            replay_data: Some(replay),
                            updated_model: Some(self.model.clone()),
                        });
                    // input.setEnable(true); input.setStartTime(0);
                    self.save_config();

                    // Transition: practice -> PlayState::Practice, else -> RESULT or MUSICSELECT
                    if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Practice {
                        self.state = PlayState::Practice;
                    } else if self
                        .pending
                        .pending_score_handoff
                        .as_ref()
                        .is_some_and(|h| h.score_data.is_some())
                    {
                        self.pending.pending_state_change = Some(MainStateType::Result);
                    } else {
                        self.pending.pending_state_change = Some(MainStateType::MusicSelect);
                    }
                    log::info!("Failed close, transition to result/select");
                }
            }

            // PlayState::Finished
            // Translated from: Java BMSPlayer.render() lines 872-911
            PlayState::Finished => {
                if let Some(ref mut control) = self.input.control {
                    control.enable_control = false;
                    control.enable_cursor = false;
                }
                if let Some(ref mut ki) = self.input.keyinput {
                    ki.stop_judge();
                }
                self.keysound.stop_bg_play();

                if self.main_state_data.timer.now_time_for_id(TIMER_MUSIC_END)
                    > self.play_skin.finish_margin as i64
                {
                    self.main_state_data.timer.switch_timer(TIMER_FADEOUT, true);
                }
                // skin.getFadeout() from the loaded skin
                let skin_fadeout = self
                    .main_state_data
                    .skin
                    .as_ref()
                    .map_or(0, |s| s.fadeout()) as i64;
                if self.main_state_data.timer.now_time_for_id(TIMER_FADEOUT) > skin_fadeout {
                    self.pending.pending_global_pitch = Some(1.0);
                    // resource.getBGAManager().stop();
                    // Ensure model notes have judge states before computing score data.
                    self.sync_judge_states_to_model();
                    let score = if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Play
                        || self.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay
                    {
                        self.create_score_data(self.device_type)
                    } else {
                        None
                    };
                    self.save_config();
                    let replay = self.build_replay_data();
                    self.pending.pending_score_handoff =
                        Some(rubato_types::score_handoff::ScoreHandoff {
                            score_data: score,
                            combo: self.judge.course_combo(),
                            maxcombo: self.judge.course_maxcombo(),
                            gauge: self.gaugelog.clone(),
                            groove_gauge: self.gauge.clone(),
                            assist: self.assist,
                            freq_on: self.freq_on,
                            force_no_ir_send: self.force_no_ir_send,
                            replay_data: Some(replay),
                            updated_model: Some(self.model.clone()),
                        });
                    // input.setEnable(true); input.setStartTime(0);

                    // Transition: practice -> PlayState::Practice, else -> RESULT
                    if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Practice {
                        self.state = PlayState::Practice;
                    } else {
                        self.pending.pending_state_change = Some(MainStateType::Result);
                    }
                    log::info!("Finished, transition to result/select");
                }
            }

            // PlayState::Aborted
            // Translated from: Java BMSPlayer.render() lines 914-936
            PlayState::Aborted => {
                // Quick retry check (START xor SELECT in PLAY mode, not course)
                // Guard: skip if a state transition is already queued to avoid
                // calling save_config() on every frame while keys are held.
                if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Play
                    && (self.input.input_start_pressed ^ self.input.input_select_pressed)
                    && !self.is_course_mode
                    && self.pending.pending_state_change.is_none()
                {
                    self.pending.pending_global_pitch = Some(1.0);
                    self.save_config();
                    self.pending.pending_reload_bms = true;
                    self.pending.pending_state_change = Some(MainStateType::Play);
                } else if self.main_state_data.timer.now_time_for_id(TIMER_FADEOUT)
                    > self
                        .main_state_data
                        .skin
                        .as_ref()
                        .map_or(0, |s| s.fadeout()) as i64
                {
                    // skin.getFadeout() from the loaded skin
                    // input.setEnable(true); input.setStartTime(0);
                    self.pending.pending_state_change = Some(MainStateType::MusicSelect);
                    log::info!("Aborted, transition to MUSICSELECT");
                }
            }
        }

        self.prevtime = micronow;

        // Copy recent judge data to timer for SkinTimingVisualizer/SkinHitErrorVisualizer
        self.main_state_data
            .timer
            .set_recent_judges(self.judge.recent_judges_index(), self.judge.recent_judges());
    }

    fn input(&mut self) {
        self.input_impl();
    }

    fn sync_input_from(&mut self, input: &BMSPlayerInputProcessor) {
        self.sync_input_from_impl(input);
    }

    fn sync_input_back_to(&mut self, input: &mut BMSPlayerInputProcessor) {
        self.sync_input_back_to_impl(input);
    }

    fn sync_audio(&mut self, audio: &mut dyn rubato_audio::audio_driver::AudioDriver) {
        self.sync_audio_impl(audio);
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
        if let Some(ref mut skin) = self.main_state_data.skin {
            skin.dispose_skin();
        }
        self.main_state_data.skin = None;

        if let Some(ref mut lr) = self.lanerender {
            lr.dispose();
        }
        self.practice.dispose();
        log::info!("Play state resources disposed");
    }
}

impl BMSPlayer {
    /// Rebuild judge_notes and JudgeManager from the current model state.
    ///
    /// Used during initial create() and practice mode restarts so that the judge
    /// system always references the current (possibly re-modified) model data.
    pub(super) fn rebuild_judge_system(&mut self, mode: &Mode) {
        self.judge_notes = bms_model::judge_note::build_judge_notes(&self.model);
        let rule = BMSPlayerRule::for_mode(mode);

        // Compute judge window rates from player config, applying course constraints
        let mut key_judge_window_rate = if self.player_config.judge_settings.custom_judge {
            [
                self.player_config
                    .judge_settings
                    .key_judge_window_rate_perfect_great,
                self.player_config
                    .judge_settings
                    .key_judge_window_rate_great,
                self.player_config.judge_settings.key_judge_window_rate_good,
            ]
        } else {
            [100, 100, 100]
        };
        let mut scratch_judge_window_rate = if self.player_config.judge_settings.custom_judge {
            [
                self.player_config
                    .judge_settings
                    .scratch_judge_window_rate_perfect_great,
                self.player_config
                    .judge_settings
                    .scratch_judge_window_rate_great,
                self.player_config
                    .judge_settings
                    .scratch_judge_window_rate_good,
            ]
        } else {
            [100, 100, 100]
        };

        // Apply course constraints to judge window rates
        for con in &self.constraints {
            match con {
                CourseDataConstraint::NoGreat => {
                    key_judge_window_rate[1] = 0;
                    key_judge_window_rate[2] = 0;
                    scratch_judge_window_rate[1] = 0;
                    scratch_judge_window_rate[2] = 0;
                }
                CourseDataConstraint::NoGood => {
                    key_judge_window_rate[2] = 0;
                    scratch_judge_window_rate[2] = 0;
                }
                _ => {}
            }
        }

        let autoplay = matches!(
            self.play_mode.mode,
            rubato_core::bms_player_mode::Mode::Autoplay
        );
        let is_play_or_practice = matches!(
            self.play_mode.mode,
            rubato_core::bms_player_mode::Mode::Play | rubato_core::bms_player_mode::Mode::Practice
        );
        let judgeregion = self.play_skin.judgeregion.max(1);

        // Read judge algorithm from the mode-specific play config
        let algorithm = self
            .player_config
            .play_config_ref(*mode)
            .playconfig
            .judgetype()
            .parse::<JudgeAlgorithm>()
            .unwrap_or(JudgeAlgorithm::Combo);

        let judge_config = JudgeConfig {
            notes: &self.judge_notes,
            mode,
            ln_type: self.model.lntype(),
            judge_rank: self.model.judgerank,
            judge_window_rate: key_judge_window_rate,
            scratch_judge_window_rate,
            algorithm,
            autoplay,
            judge_property: &rule.judge,
            lane_property: self.lane_property.as_ref(),
            auto_adjust_enabled: self
                .player_config
                .judge_settings
                .notes_display_timing_auto_adjust,
            is_play_or_practice,
            judgeregion,
        };
        self.judge = JudgeManager::from_config(&judge_config);

        // Build reverse index: JudgeNote index -> (timeline_index, lane)
        // Used by sync_judge_states_to_model() to write judge results back to model notes.
        self.judge_note_to_model = self
            .judge_notes
            .iter()
            .map(|jn| {
                let lane = jn.lane as i32;
                // Binary search for a timeline with matching micro_time.
                // Timelines are sorted by time (ascending), but multiple timelines
                // can share the same micro_time (e.g., barline + note at same beat).
                // binary_search_by_key returns an arbitrary match among duplicates,
                // so we must scan forward and backward to find the one that actually
                // contains a note on the target lane.
                let tl_idx = match self
                    .model
                    .timelines
                    .binary_search_by_key(&jn.time_us, |tl| tl.micro_time())
                {
                    Ok(idx) => Self::find_timeline_with_note_on_lane(
                        &self.model.timelines,
                        idx,
                        jn.time_us,
                        lane,
                    ),
                    Err(_) => usize::MAX,
                };
                (tl_idx, lane)
            })
            .collect();

        // Carry course combo from previous stage.
        // Translated from: JudgeManager.init() Java lines 211-214
        // ```java
        // FloatArray[] f = resource.getGauge();
        // if (f != null) {
        //     setCourseCombo(resource.getCombo());
        //     setCourseMaxcombo(resource.getMaxcombo());
        // }
        // ```
        // The initial_course_combo fields are only non-zero when the caller
        // detected a non-null gauge on PlayerResource (i.e., not the first course stage).
        if self.initial_course_combo != 0 || self.initial_course_maxcombo != 0 {
            self.judge.set_course_combo(self.initial_course_combo);
            self.judge.set_course_maxcombo(self.initial_course_maxcombo);
        }
    }

    /// Given a `binary_search_by_key` hit at `idx`, scan forward and backward
    /// among timelines sharing the same `target_time` to find one that has a
    /// note on the given `lane`. Falls back to `idx` if no matching timeline
    /// is found (best-effort, preserves previous behavior).
    fn find_timeline_with_note_on_lane(
        timelines: &[bms_model::time_line::TimeLine],
        idx: usize,
        target_time: i64,
        lane: i32,
    ) -> usize {
        // Check the initial hit first (common case: no duplicates).
        if timelines[idx].note(lane).is_some() {
            return idx;
        }
        // Scan backward while micro_time matches.
        let mut i = idx;
        while i > 0 {
            i -= 1;
            if timelines[i].micro_time() != target_time {
                break;
            }
            if timelines[i].note(lane).is_some() {
                return i;
            }
        }
        // Scan forward while micro_time matches.
        let mut i = idx + 1;
        while i < timelines.len() {
            if timelines[i].micro_time() != target_time {
                break;
            }
            if timelines[i].note(lane).is_some() {
                return i;
            }
            i += 1;
        }
        // No timeline at this time has a note on the target lane.
        // Fall back to the original index (best-effort, matches previous behavior).
        idx
    }
}
