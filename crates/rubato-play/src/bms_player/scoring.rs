use super::*;
use rubato_types::sync_utils::lock_or_recover;

impl BMSPlayer {
    /// Sync judge states from JudgeManager's internal note_states back to the
    /// BMSModel's Note objects.
    ///
    /// In Java, JudgeManager modifies Note objects in-place via shared references.
    /// In Rust, JudgeManager stores results in private `note_states: Vec<NoteJudgeState>`
    /// which are never written back to the model. This method bridges the gap by
    /// copying state and play_time from each JudgeNote's state into the corresponding
    /// Note on the model's TimeLine.
    ///
    /// Must be called after `judge.update()` so that `create_score_data()` and the
    /// result screen's timing distribution see correct values.
    pub(super) fn sync_judge_states_to_model(&mut self) {
        for (note_idx, &(tl_idx, lane)) in self.judge_note_to_model.iter().enumerate() {
            if tl_idx == usize::MAX {
                continue;
            }
            let state = self.judge.note_state(note_idx);
            let play_time = self.judge.note_play_time(note_idx);
            if state == 0 {
                continue;
            }
            if let Some(tl) = self.model.timelines.get_mut(tl_idx)
                && let Some(note) = tl.note_mut(lane)
            {
                note.set_state(state);
                note.set_micro_play_time(play_time);
            }
        }
    }

    /// Resolve a JudgeNote index to the corresponding model Note.
    ///
    /// Uses `judge_note_to_model` to map the JudgeNote index to (timeline_index, lane),
    /// then retrieves the Note from the model. Returns None if the index is out of bounds
    /// or the mapping is invalid.
    pub(super) fn resolve_judge_note(&self, note_idx: usize) -> Option<Note> {
        let &(tl_idx, lane) = self.judge_note_to_model.get(note_idx)?;
        if tl_idx == usize::MAX {
            return None;
        }
        let tl = self.model.timelines.get(tl_idx)?;
        tl.note(lane).cloned()
    }

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
            // Deviation from Java: Java uses STATE_PRACTICE_FINISHED for all modes
            // when stopping during Preload/Ready. Rust uses Aborted for Play mode
            // to enable quick retry (reload BMS without returning to song select).
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
        // Rust-only deviation: This check does NOT exist in Java BMSPlayer.stopPlay().
        // Java always proceeds to Finished or Failed regardless of judge counts.
        // Intentional improvement: when no notes were judged (e.g., autoplay was off
        // but the user didn't play), abort instead of showing an empty result screen.
        // Course mode is excluded because aborting mid-course would break the sequence.
        if self.state != PlayState::Finished
            && !self.is_course_mode
            && self.judge.judge_count(0)
                + self.judge.judge_count(1)
                + self.judge.judge_count(2)
                + self.judge.judge_count(3)
                == 0
        {
            if let Some(ref mut keyinput) = self.input.keyinput {
                keyinput.stop_judge();
            }
            self.keysound.stop_bg_play();
            if self.media_load_finished {
                self.pending.pending_stop_all_notes = true;
            }
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
            if self.media_load_finished {
                self.pending.pending_stop_all_notes = true;
            }
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
        // total_notes >= past_notes() in normal play (past_notes counts judged notes).
        // Subtraction is safe: all values are small i32 (max ~10k notes).
        score.minbp = score.judge_counts.ebd
            + score.judge_counts.lbd
            + score.judge_counts.epr
            + score.judge_counts.lpr
            + score.judge_counts.ems
            + score.judge_counts.lms
            + self.total_notes
            - self.judge.past_notes();

        // Timing statistics (Java BMSPlayer.createScoreData() lines 1053-1094)
        //
        // Java iterates ALL playable notes:
        //   - Judged (state 1-4): adds abs(time) to avgduration
        //   - Unjudged: adds 1,000,000 (1-second penalty) to avgduration
        //   - count++ for every note (both judged and unjudged)
        //   - avgjudge = avgduration / count
        //
        // The Rust-only avg and stddev computations use only judged notes.
        let mut avgduration: i64 = 0;
        let mut total_count: i64 = 0;
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
                        total_count += 1;
                        if (1..=4).contains(&state) {
                            play_times.push(time);
                            avgduration += time.saturating_abs();
                            average += time;
                        } else {
                            // Unjudged note: 1-second penalty (Java parity)
                            avgduration += 1_000_000;
                        }
                    }
                }
            }
        }
        score.timing_stats.total_duration = avgduration;
        score.timing_stats.total_avg = average;
        if total_count > 0 {
            // avgjudge uses total note count as denominator (Java parity)
            score.timing_stats.avgjudge = avgduration / total_count;
        }
        if !play_times.is_empty() {
            // avg uses only judged note count (Rust-only stat)
            score.timing_stats.avg = average / play_times.len() as i64;
        }

        let mut stddev_acc: i128 = 0;
        for &time in &play_times {
            let mean_offset = time as i128 - score.timing_stats.avg as i128;
            stddev_acc += mean_offset * mean_offset;
        }
        let mut stddev: i64 = 0;
        if !play_times.is_empty() {
            stddev = ((stddev_acc / play_times.len() as i128) as f64).sqrt() as i64;
        }
        score.timing_stats.stddev = stddev;

        // Java: score.setDeviceType(main.getInputProcessor().getDeviceType());
        score.play_option.device_type = Some(match device_type {
            rubato_input::bms_player_input_device::DeviceType::Keyboard => {
                rubato_types::bms_player_input_device::Type::KEYBOARD
            }
            rubato_input::bms_player_input_device::DeviceType::BmController => {
                rubato_types::bms_player_input_device::Type::BM_CONTROLLER
            }
            rubato_input::bms_player_input_device::DeviceType::Midi => {
                rubato_types::bms_player_input_device::Type::MIDI
            }
        });
        score.play_option.skin = self.skin_name.clone();

        Some(score)
    }

    /// Build replay data from the current play session's pattern info.
    /// Key input log is NOT included here (it lives on BMSPlayerInputProcessor);
    /// the caller (MainController) must copy it before writing to PlayerResource.
    ///
    /// Corresponds to Java: resource.getReplayData() population in BMSPlayer constructor
    /// and dispose/result paths.
    pub fn build_replay_data(&self) -> ReplayData {
        let mut rd = self.score.playinfo.clone();
        rd.sha256 = Some(self.model.sha256.clone());
        // Java BMSPlayer.java:846: replay.mode = config.getLnmode()
        // Stores the LN mode setting (0=LONGNOTE, 1=CN, 2=HCN), not the chart mode ID.
        rd.mode = self.player_config.play_settings.lnmode;
        rd.date = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Some(ref gauge) = self.gauge {
            rd.gauge = gauge.gauge_type();
        }
        if let Some(ref config) = self.score.replay_config {
            rd.config = Some(config.clone());
        }
        rd
    }

    /// Corresponds to Java BMSPlayer.update(int judge, long time)
    ///
    /// Note: gauge.update(judge) is NOT called here because it is already
    /// called in JudgeManager::update_micro(). Calling it here would be a
    /// double-update.
    pub fn update_judge(&mut self, judge: i32, time: i64) {
        if self.judge.combo() == 0 {
            // Java: main.update(judge, mtime / 1000) -- JudgeManager converts
            // microseconds to milliseconds before calling BMSPlayer.update().
            // BGAProcessor.time is in milliseconds, so misslayertime must match.
            lock_or_recover(&self.bga).set_misslayer_tme(time / 1000);
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

    pub fn playtime(&self) -> i64 {
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

        // 6. Push updated config back to MainController via outbox.
        // In Java, BMSPlayer writes directly to main.getPlayerConfig() (shared reference).
        // In Rust, we own a clone, so we must push changes back explicitly.
        self.pending.pending_play_config_update = Some((mode, pc.clone()));
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
