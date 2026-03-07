use super::*;

impl BMSPlayer {
    /// Corresponds to Java BMSPlayer.stopPlay()
    pub fn stop_play(&mut self) {
        // if main.hasObsListener() { main.getObsListener().triggerPlayEnded(); }
        if self.state == PlayState::Practice {
            self.practice.save_property();
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
            self.state = PlayState::PracticeFinished;
            return;
        }
        if self.state == PlayState::Preload || self.state == PlayState::Ready {
            self.pending.pending_global_pitch = Some(1.0);
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
            if self.play_mode.mode == rubato_core::bms_player_mode::Mode::Play {
                self.state = PlayState::Aborted;
            } else {
                self.state = PlayState::PracticeFinished;
            }
            return;
        }
        if self.main_state_data.timer.is_timer_on(TIMER_FAILED)
            || self.main_state_data.timer.is_timer_on(TIMER_FADEOUT)
        {
            return;
        }
        if self.state != PlayState::Finished
            && !self.is_course_mode
            && self.judge.judge_count(0)
                + self.judge.judge_count(1)
                + self.judge.judge_count(2)
                + self.judge.judge_count(3)
                == 0
        {
            // No notes judged and not in course mode - abort
            if let Some(ref mut keyinput) = self.input.keyinput {
                keyinput.stop_judge();
            }
            self.keysound.stop_bg_play();
            // if resource.mediaLoadFinished() { main.getAudioProcessor().stop(null); }
            self.state = PlayState::Aborted;
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
            return;
        }
        if self.state != PlayState::Finished
            && (self.judge.past_notes() == self.total_notes
                || self.play_mode.mode == rubato_core::bms_player_mode::Mode::Autoplay)
        {
            self.state = PlayState::Finished;
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
            log::info!("PlayState::Finished");
        } else if self.state == PlayState::Finished
            && !self.main_state_data.timer.is_timer_on(TIMER_FADEOUT)
        {
            self.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
        } else if self.state != PlayState::Finished {
            self.pending.pending_global_pitch = Some(1.0);
            self.state = PlayState::Failed;
            self.main_state_data.timer.set_timer_on(TIMER_FAILED);
            // if resource.mediaLoadFinished() { main.getAudioProcessor().stop(null); }
            self.queue_sound(rubato_types::sound_type::SoundType::PlayStop);
            log::info!("PlayState::Failed");
        }
    }

    /// Corresponds to Java BMSPlayer.createScoreData()
    ///
    /// `device_type` comes from `MainController.input_processor().get_device_type()`.
    pub fn create_score_data(
        &self,
        device_type: rubato_input::bms_player_input_device::DeviceType,
    ) -> Option<ScoreData> {
        let mut score = self.judge.score_data().clone();

        // If not in course mode and not aborted, check if any notes were hit
        if !self.is_course_mode
            && self.state != PlayState::Aborted
            && (score.judge_counts.epg
                + score.judge_counts.lpg
                + score.judge_counts.egr
                + score.judge_counts.lgr
                + score.judge_counts.egd
                + score.judge_counts.lgd
                + score.judge_counts.ebd
                + score.judge_counts.lbd
                == 0)
        {
            return None;
        }

        let mut clear = ClearType::Failed;
        if self.state != PlayState::Failed
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
            } else if self.judge.past_notes() == self.judge.combo() {
                if self.judge.judge_count(2) == 0 {
                    if self.judge.judge_count(1) == 0 {
                        clear = ClearType::Max;
                    } else {
                        clear = ClearType::Perfect;
                    }
                } else {
                    clear = ClearType::FullCombo;
                }
            } else if !self.is_course_mode {
                clear = gauge.clear_type();
            }
        }
        score.clear = clear.id();
        if let Some(ref gauge) = self.gauge {
            score.play_option.gauge = if gauge.is_type_changed() {
                -1
            } else {
                gauge.gauge_type()
            };
        }
        score.play_option.option = self.encode_option_for_score();
        score.play_option.seed = self.encode_seed_for_score();
        let ghost: Vec<i32> = self.judge.ghost().to_vec();
        score.encode_ghost(Some(&ghost));

        score.passnotes = self.judge.past_notes();
        score.minbp = score.judge_counts.ebd
            + score.judge_counts.lbd
            + score.judge_counts.epr
            + score.judge_counts.lpr
            + score.judge_counts.ems
            + score.judge_counts.lms
            + self.total_notes
            - self.judge.past_notes();

        // Timing statistics (Java BMSPlayer.createScoreData() lines 1053-1094)
        let mut avgduration: i64 = 0;
        let mut average: i64 = 0;
        let mut play_times: Vec<i64> = Vec::new();
        let lanes = self.model.mode().map(|m| m.key()).unwrap_or(0);
        for tl in &self.model.timelines {
            for i in 0..lanes {
                if let Some(note) = tl.note(i) {
                    let include = match note {
                        Note::Normal(_) => true,
                        Note::Long { end, note_type, .. } => {
                            let is_ln_end = ((self.model.lntype() == LNTYPE_LONGNOTE
                                && *note_type == TYPE_UNDEFINED)
                                || *note_type == TYPE_LONGNOTE)
                                && *end;
                            !is_ln_end
                        }
                        _ => false,
                    };
                    if include {
                        let state = note.state();
                        let time = note.micro_play_time();
                        if (1..=4).contains(&state) {
                            play_times.push(time);
                            avgduration += time.abs();
                            average += time;
                        }
                    }
                }
            }
        }
        score.timing_stats.total_duration = avgduration;
        score.timing_stats.total_avg = average;
        if !play_times.is_empty() {
            score.timing_stats.avgjudge = avgduration / play_times.len() as i64;
            score.timing_stats.avg = average / play_times.len() as i64;
        }

        let mut stddev: i64 = 0;
        for &time in &play_times {
            let mean_offset = time - score.timing_stats.avg;
            stddev += mean_offset * mean_offset;
        }
        if !play_times.is_empty() {
            stddev = ((stddev / play_times.len() as i64) as f64).sqrt() as i64;
        }
        score.timing_stats.stddev = stddev;

        // Java: score.setDeviceType(main.getInputProcessor().getDeviceType());
        score.play_option.device_type = Some(match device_type {
            rubato_input::bms_player_input_device::DeviceType::Keyboard => {
                rubato_types::stubs::bms_player_input_device::Type::KEYBOARD
            }
            rubato_input::bms_player_input_device::DeviceType::BmController => {
                rubato_types::stubs::bms_player_input_device::Type::BM_CONTROLLER
            }
            rubato_input::bms_player_input_device::DeviceType::Midi => {
                rubato_types::stubs::bms_player_input_device::Type::MIDI
            }
        });
        score.play_option.skin = self.skin_name.clone();

        Some(score)
    }

    /// Corresponds to Java BMSPlayer.update(int judge, long time)
    pub fn update_judge(&mut self, judge: i32, time: i64) {
        if self.judge.combo() == 0 {
            self.bga
                .lock()
                .expect("bga lock poisoned")
                .set_misslayer_tme(time);
        }
        if let Some(ref mut gauge) = self.gauge {
            gauge.update(judge);
        }

        // Full combo check
        let is_fullcombo = self.judge.past_notes() == self.total_notes
            && self.judge.past_notes() == self.judge.combo();
        self.main_state_data
            .timer
            .switch_timer(TIMER_FULLCOMBO_1P, is_fullcombo);

        // Update score data property
        let score_clone = self.judge.score_data().clone();
        let past_notes = self.judge.past_notes();
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
            self.judge.score_data().exscore() >= self.main_state_data.score.best_score(),
        );
        self.main_state_data.timer.switch_timer(
            TIMER_SCORE_TARGET,
            self.judge.score_data().exscore() >= self.main_state_data.score.rival_score(),
        );

        self.play_skin.pomyu.pm_chara_judge = judge + 1;
    }

    pub fn is_note_end(&self) -> bool {
        self.judge.past_notes() == self.total_notes
    }

    pub fn past_notes(&self) -> i32 {
        self.judge.past_notes()
    }

    pub fn playtime(&self) -> i32 {
        self.playtime
    }

    pub fn mode(&self) -> Mode {
        self.model.mode().copied().unwrap_or(Mode::BEAT_7K)
    }

    /// Get skin type matching the current model mode.
    /// Corresponds to Java getSkinType() which iterates SkinType.values().
    pub fn skin_type(&self) -> Option<SkinType> {
        let model_mode = self.model.mode().copied().unwrap_or(Mode::BEAT_7K);
        SkinType::values()
            .into_iter()
            .find(|&skin_type| skin_type.mode() == Some(model_mode))
    }

    /// Save play config from lane renderer state.
    ///
    /// Corresponds to Java saveConfig() private method.
    /// Persists hispeed/duration, lanecover, lift, hidden from the lane renderer
    /// back into the PlayerConfig's PlayConfig for the current mode.
    pub(super) fn save_config(&mut self) {
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
        let duration = lr.duration();
        let hispeed = lr.hispeed();
        let lanecover = lr.lanecover();
        let lift = lr.lift_region();
        let hidden = lr.hidden_cover();

        // 3. Get PlayConfig from playerConfig.getPlayConfig(mode).getPlayconfig()
        let mode = self.model.mode().copied().unwrap_or(Mode::BEAT_7K);
        let pc = &mut self.player_config.play_config(mode).playconfig;

        // 4. If fixhispeed != OFF: save duration; else save hispeed
        if pc.fixhispeed != rubato_types::play_config::FIX_HISPEED_OFF {
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
        self.score.playinfo.randomoption = config.play_settings.random;
        self.score.playinfo.randomoption2 = config.play_settings.random2;
        self.score.playinfo.doubleoption = config.play_settings.doubleoption;
    }

    /// Get option information (replay data with random options).
    /// Corresponds to Java getOptionInformation() returning playinfo.
    pub fn option_information(&self) -> &ReplayData {
        &self.score.playinfo
    }

    /// Encode the random seed for ScoreData storage.
    ///
    /// For SP (player=1): returns `playinfo.randomoptionseed`.
    /// For DP (player=2): returns `randomoption2seed * 65536 * 256 + randomoptionseed`.
    ///
    /// Corresponds to Java BMSPlayer line 1029:
    /// `score.setSeed((model.getMode().player == 2 ? playinfo.randomoption2seed * 65536 * 256 : 0) + playinfo.randomoptionseed)`
    pub fn encode_seed_for_score(&self) -> i64 {
        let player_count = self.model.mode().map_or(1, |m| m.player());
        if player_count == 2 {
            self.score.playinfo.randomoption2seed * 65536 * 256
                + self.score.playinfo.randomoptionseed
        } else {
            self.score.playinfo.randomoptionseed
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
        let player_count = self.model.mode().map_or(1, |m| m.player());
        if player_count == 2 {
            self.score.playinfo.randomoption
                + self.score.playinfo.randomoption2 * 10
                + self.score.playinfo.doubleoption * 100
        } else {
            self.score.playinfo.randomoption
        }
    }
}
