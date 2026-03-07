use super::skin_context::{PlayMouseContext, PlayRenderContext};
use super::*;

impl MainState for BMSPlayer {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::Play)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.play.main_state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.play.main_state_data
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

    fn take_pending_reload_bms(&mut self) -> bool {
        std::mem::take(&mut self.pending.pending_reload_bms)
    }

    fn notify_media_load_finished(&mut self) {
        self.media_load_finished = true;
    }

    fn receive_reloaded_model(&mut self, model: bms_model::bms_model::BMSModel) {
        self.play.model = model;
    }

    fn take_bga_cache(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        // Return the Arc<Mutex<BGAProcessor>> for caching on PlayerResource.
        // The Arc is cloned so that BMSPlayer can still hold a reference
        // (though it will be dropped shortly after during state transition).
        Some(Box::new(Arc::clone(&self.play.bga)))
    }

    fn render_skin(&mut self, sprite: &mut rubato_render::sprite_batch::SpriteBatch) {
        let mut skin = match self.play.main_state_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.play.main_state_data.timer);

        {
            let mut ctx = PlayRenderContext {
                timer: &mut timer,
                judge: &self.play.judge,
                gauge: self.play.gauge.as_ref(),
                player_config: &self.player_config,
                option_info: &self.score.playinfo,
                play_config: &self
                    .player_config
                    .play_config_ref(
                        self.play
                            .model
                            .mode()
                            .cloned()
                            .unwrap_or(bms_model::mode::Mode::BEAT_7K),
                    )
                    .playconfig,
                target_score: self.score.target_score.as_ref(),
                playtime: self.play.playtime,
                total_notes: self.play.total_notes,
                play_mode: self.play_mode,
                state: self.state,
                media_load_finished: self.media_load_finished,
            };
            skin.update_custom_objects_timed(&mut ctx);
            skin.swap_sprite_batch(sprite);
            skin.draw_all_objects_timed(&mut ctx);
            skin.swap_sprite_batch(sprite);
        }

        self.play.main_state_data.timer = timer;
        self.play.main_state_data.skin = Some(skin);
    }

    fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.play.main_state_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.play.main_state_data.timer);

        {
            let mut ctx = PlayMouseContext {
                timer: &mut timer,
                player: self,
            };
            skin.mouse_pressed_at(&mut ctx, button, x, y);
        }

        self.play.main_state_data.timer = timer;
        self.play.main_state_data.skin = Some(skin);
    }

    fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.play.main_state_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.play.main_state_data.timer);

        {
            let mut ctx = PlayMouseContext {
                timer: &mut timer,
                player: self,
            };
            skin.mouse_dragged_at(&mut ctx, button, x, y);
        }

        self.play.main_state_data.timer = timer;
        self.play.main_state_data.skin = Some(skin);
    }

    fn create(&mut self) {
        let mode = self.play.model.mode().copied().unwrap_or(Mode::BEAT_7K);
        self.play.lane_property = Some(LaneProperty::new(&mode));
        self.play.judge = JudgeManager::new();
        self.input.control = Some(ControlInputProcessor::new(mode));
        if let Some(ref lp) = self.play.lane_property {
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

        self.play.lanerender = Some(LaneRenderer::new(&self.play.model));

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

        self.play.judge.init(&self.play.model, 0, None, &[]);

        // --- Note expansion rate from PlaySkin ---
        // Translated from: BMSPlayer.create() Java line 542-543
        // ```java
        // rhythm = new RhythmTimerProcessor(model,
        //     (getSkin() instanceof PlaySkin) ? ((PlaySkin) getSkin()).getNoteExpansionRate()[0] != 100
        //         || ((PlaySkin) getSkin()).getNoteExpansionRate()[1] != 100 : false);
        // ```
        let rates = self.play.play_skin.get_note_expansion_rate();
        let use_expansion = rates[0] != 100 || rates[1] != 100;
        self.rhythm = Some(RhythmTimerProcessor::new(&self.play.model, use_expansion));

        // Reuse existing BGAProcessor (injected via set_bga_processor from PlayerResource)
        // to preserve the texture cache between plays. Only update timelines for the new model.
        // Java: bga = resource.getBGAManager(); (BMSPlayer.java line 545)
        if let Ok(mut bga) = self.play.bga.lock() {
            bga.set_model_timelines(&self.play.model);
        }

        // Initialize gauge log
        if let Some(ref gauge) = self.play.gauge {
            let gauge_type_len = gauge.gauge_type_length();
            self.gaugelog = Vec::with_capacity(gauge_type_len);
            for _ in 0..gauge_type_len {
                self.gaugelog
                    .push(Vec::with_capacity((self.play.playtime / 500 + 2) as usize));
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

        let total_notes = self.play.model.total_notes();

        if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Practice {
            self.play.main_state_data.score.set_target_score_with_ghost(
                0,
                None,
                0,
                None,
                total_notes,
            );
            self.practice.create(&self.play.model);
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

            self.play.main_state_data.score.set_target_score_with_ghost(
                score.exscore(),
                score.decode_ghost(),
                target_exscore,
                target_ghost,
                total_notes,
            );
        }
    }

    fn render(&mut self) {
        let micronow = self.play.main_state_data.timer.now_micro_time();

        // Input start timer
        let input_time = self.play.play_skin.get_loadstart() as i64; // skin.getInput() in Java
        if micronow > input_time * 1000 {
            self.play
                .main_state_data
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
                    if self
                        .play
                        .main_state_data
                        .timer
                        .is_timer_on(TimerId::new(141))
                        && micronow > self.startpressedtime
                    {
                        self.play
                            .main_state_data
                            .timer
                            .set_timer_off(TimerId::new(141));
                        if let Some(ref mut lr) = self.play.lanerender {
                            lr.init(&self.play.model);
                        }
                    } else if !self
                        .play
                        .main_state_data
                        .timer
                        .is_timer_on(TimerId::new(141))
                        && micronow == self.startpressedtime
                    {
                        self.play.main_state_data.timer.set_micro_timer(
                            TimerId::new(141),
                            micronow - self.starttimeoffset * 1000,
                        );
                    }
                }

                // Check if media loaded and load timers elapsed
                let load_threshold = (self.play.play_skin.get_loadstart()
                    + self.play.play_skin.get_loadend())
                    as i64
                    * 1000;
                // Translated from: Java BMSPlayer.render() lines 607-608
                if self.media_load_finished
                    && micronow > load_threshold
                    && micronow - self.startpressedtime > 1_000_000
                {
                    // Chart preview cleanup on transition
                    if self.player_config.display_settings.chart_preview {
                        self.play
                            .main_state_data
                            .timer
                            .set_timer_off(TimerId::new(141));
                        if let Some(ref mut lr) = self.play.lanerender {
                            lr.init(&self.play.model);
                        }
                    }

                    // Loudness analysis check (Java BMSPlayer.render() lines 615-641)
                    if !self.score.analysis_checked {
                        self.audio.adjusted_volume = -1.0;
                        self.score.analysis_checked = true;
                        if let Some(result) = self.score.analysis_result.take() {
                            let config_key_volume = self.audio.bg_volume;
                            self.apply_loudness_analysis(&result, config_key_volume);
                        }
                    }

                    self.play
                        .bga
                        .lock()
                        .expect("bga lock poisoned")
                        .prepare(&() as &dyn std::any::Any);
                    self.state = PlayState::Ready;
                    self.play.main_state_data.timer.set_timer_on(TIMER_READY);
                    self.queue_sound(rubato_types::sound_type::SoundType::PlayReady);
                    log::info!("PlayState::Ready");
                }
                // PM character neutral timer
                if !self
                    .play
                    .main_state_data
                    .timer
                    .is_timer_on(TIMER_PM_CHARA_1P_NEUTRAL)
                    || !self
                        .play
                        .main_state_data
                        .timer
                        .is_timer_on(TIMER_PM_CHARA_2P_NEUTRAL)
                {
                    self.play
                        .main_state_data
                        .timer
                        .set_timer_on(TIMER_PM_CHARA_1P_NEUTRAL);
                    self.play
                        .main_state_data
                        .timer
                        .set_timer_on(TIMER_PM_CHARA_2P_NEUTRAL);
                }
            }

            // PlayState::Practice - practice mode config
            PlayState::Practice => {
                if self.play.main_state_data.timer.is_timer_on(TIMER_PLAY) {
                    // Reset for practice restart: reload BMS file to get a fresh model
                    // (modifiers mutate the model during play, so we need a clean copy).
                    // Java: resource.reloadBMSFile(); model = resource.getBMSModel();
                    // Rust: pending flag triggers MainController to reload resource and
                    // push fresh model back via receive_reloaded_model().
                    self.pending.pending_reload_bms = true;
                    if let Some(ref mut lr) = self.play.lanerender {
                        lr.init(&self.play.model);
                    }
                    if let Some(ref mut ki) = self.input.keyinput {
                        ki.key_beam_stop = false;
                    }
                    self.play.main_state_data.timer.set_timer_off(TIMER_PLAY);
                    self.play.main_state_data.timer.set_timer_off(TIMER_RHYTHM);
                    self.play.main_state_data.timer.set_timer_off(TIMER_FAILED);
                    self.play.main_state_data.timer.set_timer_off(TIMER_FADEOUT);
                    self.play
                        .main_state_data
                        .timer
                        .set_timer_off(TIMER_ENDOFNOTE_1P);

                    for raw in TIMER_PM_CHARA_1P_NEUTRAL.as_i32()..=TIMER_PM_CHARA_DANCE.as_i32() {
                        self.play
                            .main_state_data
                            .timer
                            .set_timer_off(TimerId::new(raw));
                    }
                }
                if !self
                    .play
                    .main_state_data
                    .timer
                    .is_timer_on(TIMER_PM_CHARA_1P_NEUTRAL)
                    || !self
                        .play
                        .main_state_data
                        .timer
                        .is_timer_on(TIMER_PM_CHARA_2P_NEUTRAL)
                {
                    self.play
                        .main_state_data
                        .timer
                        .set_timer_on(TIMER_PM_CHARA_1P_NEUTRAL);
                    self.play
                        .main_state_data
                        .timer
                        .set_timer_on(TIMER_PM_CHARA_2P_NEUTRAL);
                }
                if let Some(ref mut control) = self.input.control {
                    control.enable_control = false;
                    control.enable_cursor = false;
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
                let load_threshold = (self.play.play_skin.get_loadstart()
                    + self.play.play_skin.get_loadend())
                    as i64
                    * 1000;
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
                            &mut self.play.model,
                            property.freq as f32 / 100.0,
                        );
                        if self.audio.fast_forward_freq_option == FrequencyType::FREQUENCY {
                            self.pending.pending_global_pitch = Some(property.freq as f32 / 100.0);
                        }
                    }

                    self.play.model.total = property.total;

                    // Apply practice modifier (time range)
                    let mut pm = rubato_core::pattern::practice_modifier::PracticeModifier::new(
                        property.starttime as i64 * 100 / property.freq as i64,
                        property.endtime as i64 * 100 / property.freq as i64,
                    );
                    pm.modify(&mut self.play.model);

                    // DP options
                    if self.play.model.mode().map_or(1, |m| m.player()) == 2 {
                        if property.doubleop == 1 {
                            let mut flip =
                                rubato_core::pattern::lane_shuffle_modifier::PlayerFlipModifier::new();
                            flip.modify(&mut self.play.model);
                        }
                        let mut pm2 =
                            rubato_core::pattern::pattern_modifier::create_pattern_modifier(
                                property.random2,
                                1,
                                &self.play.model.mode().copied().unwrap_or(Mode::BEAT_7K),
                                &self.player_config,
                            );
                        pm2.modify(&mut self.play.model);
                    }

                    // 1P random option
                    let mut pm1 = rubato_core::pattern::pattern_modifier::create_pattern_modifier(
                        property.random,
                        0,
                        &self.play.model.mode().copied().unwrap_or(Mode::BEAT_7K),
                        &self.player_config,
                    );
                    pm1.modify(&mut self.play.model);

                    // Gauge, judgerank, lane init
                    self.play.gauge = self.practice.gauge(&self.play.model);
                    self.play.model.judgerank = property.judgerank;
                    if let Some(ref mut lr) = self.play.lanerender {
                        lr.init(&self.play.model);
                    }
                    self.play.play_skin.pomyu.init();

                    self.starttimeoffset = if property.starttime > 1000 {
                        (property.starttime as i64 - 1000) * 100 / property.freq as i64
                    } else {
                        0
                    };
                    self.play.playtime = ((property.endtime as i64 + 1000) * 100
                        / property.freq as i64) as i32
                        + TIME_MARGIN;

                    self.play
                        .bga
                        .lock()
                        .expect("bga lock poisoned")
                        .prepare(&() as &dyn std::any::Any);
                    self.state = PlayState::Ready;
                    self.play.main_state_data.timer.set_timer_on(TIMER_READY);
                    log::info!("Practice -> PlayState::Ready");
                }
            }

            // PlayState::PracticeFinished
            // Translated from: Java BMSPlayer.render() lines 726-731
            PlayState::PracticeFinished => {
                let skin_fadeout = self
                    .play
                    .main_state_data
                    .skin
                    .as_ref()
                    .map_or(0, |s| s.fadeout()) as i64;
                if self
                    .play
                    .main_state_data
                    .timer
                    .now_time_for_id(TIMER_FADEOUT)
                    > skin_fadeout
                {
                    // input.setEnable(true); input.setStartTime(0);
                    self.pending.pending_state_change = Some(MainStateType::MusicSelect);
                    log::info!("Practice finished, transition to MUSICSELECT");
                }
            }

            // PlayState::Ready - countdown before play
            PlayState::Ready => {
                if self.play.main_state_data.timer.now_time_for_id(TIMER_READY)
                    > self.play.play_skin.get_playstart() as i64
                {
                    if let Some(ref lr) = self.play.lanerender {
                        self.score.replay_config = Some(lr.play_config().clone());
                    }
                    self.state = PlayState::Play;
                    self.play
                        .main_state_data
                        .timer
                        .set_micro_timer(TIMER_PLAY, micronow - self.starttimeoffset * 1000);
                    self.play
                        .main_state_data
                        .timer
                        .set_micro_timer(TIMER_RHYTHM, micronow - self.starttimeoffset * 1000);

                    // input.setStartTime(micronow + timer.getStartMicroTime() - starttimeoffset * 1000);
                    // input.setKeyLogMarginTime(resource.getMarginTime());
                    // Java: keyinput.startJudge(model, replay != null ? replay.keylog : null, resource.getMarginTime())
                    if let Some(ref mut ki) = self.input.keyinput {
                        let timelines = &self.play.model.timelines;
                        let last_tl_micro = timelines.last().map_or(0, |tl| tl.micro_time());
                        let keylog = self
                            .score
                            .active_replay
                            .as_ref()
                            .map(|r| r.keylog.as_slice());
                        ki.start_judge(last_tl_micro, keylog, self.play.margin_time);
                    }
                    // Resolve initial BG volume: use adjusted_volume if >= 0,
                    // otherwise fall back to bg_volume from AudioConfig.
                    let initial_bg_vol = if self.audio.adjusted_volume >= 0.0 {
                        self.audio.adjusted_volume
                    } else {
                        self.audio.bg_volume
                    };
                    self.audio.keysound.start_bg_play(
                        &self.play.model,
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
                let current_play_timer = self.play.main_state_data.timer.micro_timer(TIMER_PLAY);
                self.play
                    .main_state_data
                    .timer
                    .set_micro_timer(TIMER_PLAY, current_play_timer + deltaplay);

                // Rhythm timer update
                let now_bpm = self
                    .play
                    .lanerender
                    .as_ref()
                    .map_or(120.0, |lr| lr.now_bpm());
                if let Some(ref mut rhythm) = self.rhythm {
                    let play_timer_micro = self
                        .play
                        .main_state_data
                        .timer
                        .now_micro_time_for_id(TIMER_PLAY);
                    let (rhythm_timer, rhythm_on) = rhythm.update(
                        self.play.main_state_data.timer.now_time(),
                        micronow,
                        deltatime,
                        now_bpm,
                        self.playspeed,
                        freq,
                        play_timer_micro,
                    );
                    if rhythm_on {
                        self.play
                            .main_state_data
                            .timer
                            .set_micro_timer(TIMER_RHYTHM, rhythm_timer);
                    }
                }

                // Update BG autoplay thread: play time and volume.
                // Translated from: Java AutoplayThread.run() reads player.timer.getNowMicroTime(TIMER_PLAY)
                // and player.getAdjustedVolume() / config.getAudioConfig().getBgvolume().
                {
                    let play_micro = self
                        .play
                        .main_state_data
                        .timer
                        .now_micro_time_for_id(TIMER_PLAY);
                    self.audio.keysound.update_play_time(play_micro);
                    let vol = if self.audio.adjusted_volume >= 0.0 {
                        self.audio.adjusted_volume
                    } else {
                        self.audio.bg_volume
                    };
                    self.audio.keysound.update_volume(vol);
                }

                let ptime = self.play.main_state_data.timer.now_time_for_id(TIMER_PLAY);
                // Gauge log
                if let Some(ref gauge) = self.play.gauge {
                    for (i, log) in self.gaugelog.iter_mut().enumerate() {
                        if log.len() as i64 <= ptime / 500 {
                            let val = gauge.value_by_type(i as i32);
                            log.push(val);
                        }
                    }
                    self.play
                        .main_state_data
                        .timer
                        .switch_timer(TIMER_GAUGE_MAX_1P, gauge.gauge().is_max());
                }

                // pomyu timer update
                // Translated from: Java BMSPlayer.render() line 766
                let past_notes = self.play.judge.past_notes();
                let gauge_is_max = self.play.gauge.as_ref().is_some_and(|g| g.gauge().is_max());
                self.play.play_skin.pomyu.update_timer(
                    &mut self.play.main_state_data.timer,
                    past_notes,
                    gauge_is_max,
                );

                // Check play time elapsed
                if (self.play.playtime as i64) < ptime {
                    self.state = PlayState::Finished;
                    self.play
                        .main_state_data
                        .timer
                        .set_timer_on(TIMER_MUSIC_END);
                    for raw in TIMER_PM_CHARA_1P_NEUTRAL.as_i32()..=TIMER_PM_CHARA_2P_BAD.as_i32() {
                        self.play
                            .main_state_data
                            .timer
                            .set_timer_off(TimerId::new(raw));
                    }
                    self.play
                        .main_state_data
                        .timer
                        .set_timer_off(TIMER_PM_CHARA_DANCE);
                    log::info!("PlayState::Finished");
                } else if (self.play.playtime - TIME_MARGIN) as i64 <= ptime {
                    self.play
                        .main_state_data
                        .timer
                        .switch_timer(TIMER_ENDOFNOTE_1P, true);
                }

                // Stage failed check with gauge auto shift
                // Translated from: Java BMSPlayer.render() lines 782-815
                if let Some(ref mut gauge) = self.play.gauge {
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
                                self.play.main_state_data.timer.set_timer_on(TIMER_FAILED);
                                // if resource.mediaLoadFinished() { main.getAudioProcessor().stop(null); }
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
                self.audio.keysound.stop_bg_play();

                // Quick retry check (START xor SELECT)
                // Translated from: Java BMSPlayer.render() lines 823-838
                if (self.input.input_start_pressed ^ self.input.input_select_pressed)
                    && !self.is_course_mode
                    && self.play_mode.mode == rubato_core::bms_player_mode::Mode::Play
                {
                    self.pending.pending_global_pitch = Some(1.0);
                    self.save_config();
                    self.pending.pending_reload_bms = true;
                    self.pending.pending_state_change = Some(MainStateType::Play);
                } else if self
                    .play
                    .main_state_data
                    .timer
                    .now_time_for_id(TIMER_FAILED)
                    > self.play.play_skin.get_close() as i64
                {
                    self.pending.pending_global_pitch = Some(1.0);
                    // if resource.mediaLoadFinished() { resource.getBGAManager().stop(); }

                    // Fill remaining gauge log with 0
                    if self.play.main_state_data.timer.is_timer_on(TIMER_PLAY) {
                        let failed_time = self.play.main_state_data.timer.timer(TIMER_FAILED);
                        let play_time = self.play.main_state_data.timer.timer(TIMER_PLAY);
                        let mut l = failed_time - play_time;
                        while l < self.play.playtime as i64 + 500 {
                            for glog in self.gaugelog.iter_mut() {
                                glog.push(0.0);
                            }
                            l += 500;
                        }
                    }
                    let score = if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Play
                        || self.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay
                    {
                        self.create_score_data(self.device_type)
                    } else {
                        None
                    };
                    self.pending.pending_score_handoff =
                        Some(rubato_types::score_handoff::ScoreHandoff {
                            score_data: score,
                            combo: self.play.judge.course_combo(),
                            maxcombo: self.play.judge.course_maxcombo(),
                            gauge: self.gaugelog.clone(),
                            groove_gauge: self.play.gauge.clone(),
                            assist: self.assist,
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
                self.audio.keysound.stop_bg_play();

                if self
                    .play
                    .main_state_data
                    .timer
                    .now_time_for_id(TIMER_MUSIC_END)
                    > self.play.play_skin.get_finish_margin() as i64
                {
                    self.play
                        .main_state_data
                        .timer
                        .switch_timer(TIMER_FADEOUT, true);
                }
                // skin.getFadeout() from the loaded skin
                let skin_fadeout = self
                    .play
                    .main_state_data
                    .skin
                    .as_ref()
                    .map_or(0, |s| s.fadeout()) as i64;
                if self
                    .play
                    .main_state_data
                    .timer
                    .now_time_for_id(TIMER_FADEOUT)
                    > skin_fadeout
                {
                    self.pending.pending_global_pitch = Some(1.0);
                    // resource.getBGAManager().stop();
                    let score = if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Play
                        || self.play_mode.mode == rubato_core::bms_player_mode::Mode::Replay
                    {
                        self.create_score_data(self.device_type)
                    } else {
                        None
                    };
                    self.save_config();
                    self.pending.pending_score_handoff =
                        Some(rubato_types::score_handoff::ScoreHandoff {
                            score_data: score,
                            combo: self.play.judge.course_combo(),
                            maxcombo: self.play.judge.course_maxcombo(),
                            gauge: self.gaugelog.clone(),
                            groove_gauge: self.play.gauge.clone(),
                            assist: self.assist,
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
                if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Play
                    && (self.input.input_start_pressed ^ self.input.input_select_pressed)
                    && !self.is_course_mode
                {
                    self.pending.pending_global_pitch = Some(1.0);
                    self.save_config();
                    self.pending.pending_reload_bms = true;
                    self.pending.pending_state_change = Some(MainStateType::Play);
                }

                // skin.getFadeout() from the loaded skin
                let skin_fadeout = self
                    .play
                    .main_state_data
                    .skin
                    .as_ref()
                    .map_or(0, |s| s.fadeout()) as i64;
                if self
                    .play
                    .main_state_data
                    .timer
                    .now_time_for_id(TIMER_FADEOUT)
                    > skin_fadeout
                {
                    // input.setEnable(true); input.setStartTime(0);
                    self.pending.pending_state_change = Some(MainStateType::MusicSelect);
                    log::info!("Aborted, transition to MUSICSELECT");
                }
            }
        }

        self.prevtime = micronow;

        // Copy recent judge data to timer for SkinTimingVisualizer/SkinHitErrorVisualizer
        self.play.main_state_data.timer.set_recent_judges(
            self.play.judge.recent_judges_index(),
            self.play.judge.recent_judges(),
        );
    }

    fn input(&mut self) {
        // Compute values before taking mutable borrows
        let is_note_end = self.is_note_end();
        let is_timer_play_on = self.play.main_state_data.timer.is_timer_on(TIMER_PLAY);
        let now_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        // Process control input (START+SELECT, lane cover, hispeed, etc.)
        if let (Some(mut control), Some(lanerender)) =
            (self.input.control.take(), self.play.lanerender.as_mut())
        {
            let pending_analog_resets = &mut self.input.pending_analog_resets;
            let input_analog_recent_ms = &mut self.input.input_analog_recent_ms;
            let input_analog_diff_ticks = &mut self.input.input_analog_diff_ticks;
            let mut analog_diff_and_reset = |key: usize, ms_tolerance: i32| -> i32 {
                if key >= input_analog_recent_ms.len() || key >= input_analog_diff_ticks.len() {
                    return 0;
                }
                let d_ticks = if input_analog_recent_ms[key] <= ms_tolerance as i64 {
                    0.max(input_analog_diff_ticks[key])
                } else {
                    0
                };
                input_analog_recent_ms[key] = i64::MAX;
                input_analog_diff_ticks[key] = 0;
                if !pending_analog_resets.contains(&key) {
                    pending_analog_resets.push(key);
                }
                d_ticks
            };
            let mut ctx = crate::control_input_processor::ControlInputContext {
                lanerender,
                start_pressed: self.input.input_start_pressed,
                select_pressed: self.input.input_select_pressed,
                control_key_up: self.input.control_key_up,
                control_key_down: self.input.control_key_down,
                control_key_escape_pressed: self.input.control_key_escape_pressed,
                control_key_num1: self.input.control_key_num1,
                control_key_num2: self.input.control_key_num2,
                control_key_num3: self.input.control_key_num3,
                control_key_num4: self.input.control_key_num4,
                key_states: &self.input.input_key_states,
                scroll: self.input.input_scroll,
                is_analog: &self.input.input_is_analog,
                analog_diff_and_reset: &mut analog_diff_and_reset,
                is_timer_play_on,
                is_note_end,
                window_hold: self.player_config.select_settings.is_window_hold,
                autoplay_mode: self.play_mode.mode,
                now_millis,
            };

            let result = control.input(&mut ctx);

            // Apply result actions
            if let Some(speed) = result.play_speed {
                self.set_play_speed(speed);
            }
            if result.clear_start {
                self.input.input_start_pressed = false;
            }
            if result.clear_select {
                self.input.input_select_pressed = false;
            }
            if result.reset_scroll {
                self.input.input_scroll = 0;
            }
            if result.stop_play {
                // Restore control before stopping (stop_play may need it)
                self.input.control = Some(control);
                self.stop_play();
            } else {
                self.input.control = Some(control);
            }
        }

        // Build InputContext for key input processing.
        let auto_presstime = self.play.judge.auto_presstime().to_vec();
        let now = self.play.main_state_data.timer.now_time();
        let is_autoplay = self.play_mode.mode == rubato_core::bms_player_mode::Mode::Autoplay;
        if let Some(ref mut keyinput) = self.input.keyinput {
            let mut ctx = crate::key_input_processor::InputContext {
                now,
                key_states: &self.input.input_key_states,
                auto_presstime: &auto_presstime,
                is_autoplay,
                timer: &mut self.play.main_state_data.timer,
            };
            keyinput.input(&mut ctx);
        }
    }

    fn sync_input_from(&mut self, input: &BMSPlayerInputProcessor) {
        self.input.input_start_pressed = input.start_pressed();
        self.input.input_select_pressed = input.is_select_pressed();
        self.input.input_key_states.clear();
        self.input
            .input_key_states
            .extend((0..KEYSTATE_SIZE as i32).map(|i| input.key_state(i)));
        self.input.control_key_up = input.control_key_state(ControlKeys::Up);
        self.input.control_key_down = input.control_key_state(ControlKeys::Down);
        self.input.control_key_left = input.control_key_state(ControlKeys::Left);
        self.input.control_key_right = input.control_key_state(ControlKeys::Right);
        self.input.control_key_escape_pressed = input.control_key_state(ControlKeys::Escape);
        self.input.control_key_num1 = input.control_key_state(ControlKeys::Num1);
        self.input.control_key_num2 = input.control_key_state(ControlKeys::Num2);
        self.input.control_key_num3 = input.control_key_state(ControlKeys::Num3);
        self.input.control_key_num4 = input.control_key_state(ControlKeys::Num4);
        self.input.input_scroll = input.get_scroll();
        self.input.input_is_analog.clear();
        self.input
            .input_is_analog
            .extend((0..KEYSTATE_SIZE).map(|i| input.is_analog_input(i)));
        self.input.input_analog_diff_ticks.clear();
        self.input
            .input_analog_diff_ticks
            .extend((0..KEYSTATE_SIZE).map(|i| input.analog_diff(i)));
        self.input.input_analog_recent_ms.clear();
        self.input
            .input_analog_recent_ms
            .extend((0..KEYSTATE_SIZE).map(|i| input.time_since_last_analog_reset(i)));
        self.input.pending_analog_resets.clear();
        self.device_type = input.device_type();
    }

    fn sync_input_back_to(&mut self, input: &mut BMSPlayerInputProcessor) {
        if !self.input.input_start_pressed {
            input.start_changed(false);
        }
        if !self.input.input_select_pressed {
            input.select_pressed = false;
        }
        if self.input.input_scroll == 0 {
            input.reset_scroll();
        }
        for key in self.input.pending_analog_resets.drain(..) {
            input.reset_analog_input(key);
        }
    }

    fn sync_audio(&mut self, audio: &mut dyn rubato_audio::audio_driver::AudioDriver) {
        for cmd in self.drain_pending_bg_notes() {
            audio.play_note(&cmd.note, cmd.volume, 0);
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
        self.play.main_state_data.skin = None;
        self.play.main_state_data.stage = None;

        if let Some(ref mut lr) = self.play.lanerender {
            lr.dispose();
        }
        self.practice.dispose();
        log::info!("Play state resources disposed");
    }
}
