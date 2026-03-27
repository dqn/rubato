use super::*;
use rubato_types::property_snapshot::PropertySnapshot;
use rubato_types::skin_action_queue::SkinActionQueue;

impl BMSPlayer {
    /// Build a PropertySnapshot capturing all raw data needed for skin rendering.
    ///
    /// This replaces the PlayRenderContext adapter pattern. Every property ID that
    /// PlayRenderContext's integer_value/float_value/boolean_value/string_value/
    /// image_index_value methods handle must be captured here.
    pub(super) fn build_snapshot(&self, timer: &TimerManager) -> PropertySnapshot {
        let mut s = PropertySnapshot::new();

        // ================================================================
        // Timing
        // ================================================================
        s.now_time = timer.now_time();
        s.now_micro_time = timer.now_micro_time();
        s.boot_time_millis = timer.boot_time_millis();
        for (i, &val) in timer.timer_values().iter().enumerate() {
            if val != i64::MIN {
                s.timers.insert(TimerId::new(i as i32), val);
            }
        }
        s.recent_judges = timer.recent_judges().to_vec();
        s.recent_judges_index = timer.recent_judges_index();

        // ================================================================
        // State identity
        // ================================================================
        s.state_type = Some(rubato_types::main_state_type::MainStateType::Play);

        // ================================================================
        // Config
        // ================================================================
        s.config = Some(Box::new(self.config.clone()));
        s.player_config = Some(Box::new(self.player_config.clone()));

        // ================================================================
        // Play config
        // ================================================================
        let mode = self
            .model
            .mode()
            .cloned()
            .unwrap_or(bms::model::mode::Mode::BEAT_7K);
        s.play_config = Some(Box::new(
            self.player_config.play_config_ref(mode).playconfig.clone(),
        ));

        // ================================================================
        // Song / score data
        // ================================================================
        s.song_data = self.song_data.as_ref().map(|d| Box::new(d.clone()));
        s.score_data = self.score.db_score.as_ref().map(|d| Box::new(d.clone()));
        s.target_score_data = self
            .score
            .target_score
            .as_ref()
            .map(|d| Box::new(d.clone()));
        s.replay_option_data = Some(Box::new(self.score.playinfo.clone()));
        s.score_data_property = self.main_state_data.score.clone();

        // ================================================================
        // Player data
        // ================================================================
        s.player_data = self.player_data;

        // ================================================================
        // Offsets
        // ================================================================
        s.offsets = self.main_state_data.offsets.clone();

        // ================================================================
        // Media / practice
        // ================================================================
        s.is_media_load_finished = self.media_load_finished;
        s.is_practice_mode = self.play_mode.mode == crate::core::bms_player_mode::Mode::Practice;

        // ================================================================
        // Gauge data
        // ================================================================
        s.gauge_value = self.gauge.as_ref().map_or(0.0, |g| g.value());
        s.gauge_type = self.gauge.as_ref().map_or(0, |g| g.gauge_type());
        s.is_gauge_max = self.gauge.as_ref().is_some_and(|g| g.gauge().is_max());
        s.gauge_element_borders = match self.gauge.as_ref() {
            Some(g) => (0..g.gauge_type_length())
                .map(|i| {
                    let prop = g.gauge_by_type(i as i32).property();
                    (prop.border, prop.max)
                })
                .collect(),
            None => Vec::new(),
        };
        s.gauge_border_max = self.gauge.as_ref().map(|g| {
            let prop = g.gauge_by_type(g.gauge_type()).property();
            (prop.border, prop.max)
        });
        s.gauge_min = self
            .gauge
            .as_ref()
            .map_or(0.0, |g| g.gauge_by_type(g.gauge_type()).property().min);

        // ================================================================
        // Mode changed
        // ================================================================
        s.is_mode_changed = self.orgmode.is_some_and(|org| {
            self.model
                .mode()
                .copied()
                .unwrap_or(bms::model::mode::Mode::BEAT_7K)
                != org
        });

        // ================================================================
        // Judge data (now_judges, now_combos, judge_counts)
        // ================================================================
        // Populate for players 0, 1, 2 (matching NowJudgeDrawCondition coverage)
        s.now_judges = (0..3).map(|p| self.judge.now_judge(p)).collect();
        s.now_combos = (0..3).map(|p| self.judge.now_combo(p)).collect();
        for judge in 0..=5 {
            s.judge_counts
                .insert((judge, true), self.judge.judge_count_fast(judge, true));
            s.judge_counts
                .insert((judge, false), self.judge.judge_count_fast(judge, false));
        }

        // ================================================================
        // Lane shuffle patterns
        // ================================================================
        s.lane_shuffle_patterns = self.score.playinfo.lane_shuffle_pattern.clone();

        // ================================================================
        // Judge area
        // ================================================================
        s.judge_area = {
            let rule = BMSPlayerRule::for_mode(&mode);
            let mut jwr = if self.player_config.judge_settings.custom_judge {
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
            for con in &self.constraints {
                use crate::core::course_data::CourseDataConstraint;
                match con {
                    CourseDataConstraint::NoGreat => {
                        jwr[1] = 0;
                        jwr[2] = 0;
                    }
                    CourseDataConstraint::NoGood => {
                        jwr[2] = 0;
                    }
                    _ => {}
                }
            }
            Some(rule.judge.note_judge(self.model.judgerank, &jwr))
        };

        // ================================================================
        // Live lane renderer values
        // ================================================================
        let lr_ref = self.lanerender.as_ref();
        let live_hispeed = lr_ref.map_or(0.0, |lr| lr.hispeed());
        let live_lanecover = lr_ref.map_or(0.0, |lr| lr.lanecover());
        let live_lift = lr_ref.map_or(0.0, |lr| lr.lift_region());
        let live_hidden = lr_ref.map_or(0.0, |lr| lr.hidden_cover());
        let now_bpm = lr_ref.map_or(0.0, |lr| lr.now_bpm());
        let min_bpm = lr_ref.map_or(0.0, |lr| lr.min_bpm());
        let max_bpm = lr_ref.map_or(0.0, |lr| lr.max_bpm());
        let main_bpm = lr_ref.map_or(0.0, |lr| lr.main_bpm());
        let current_duration = lr_ref.map_or(0, |lr| lr.current_duration());

        // ================================================================
        // Integer properties
        // ================================================================
        // Hi-speed (LR2 format: hispeed * 100, e.g. 3.5 -> 350)
        s.integers.insert(10, (live_hispeed * 100.0) as i32);
        // Hi-speed integer part (NUMBER_HISPEED: 310)
        s.integers.insert(310, live_hispeed as i32);
        // Hi-speed fractional part (e.g. 3.52 -> 52)
        s.integers
            .insert(311, ((live_hispeed * 100.0) as i32) % 100);
        // Lanecover (0-1000 scale from live LaneRenderer)
        s.integers.insert(14, (live_lanecover * 1000.0) as i32);
        // Lift (0-1000 scale from live LaneRenderer)
        s.integers.insert(314, (live_lift * 1000.0) as i32);
        // Hidden (0-1000 scale from live LaneRenderer)
        s.integers.insert(315, (live_hidden * 1000.0) as i32);
        // Total notes
        s.integers.insert(350, self.total_notes());
        // Cumulative playtime (hours/minutes/seconds from PlayerData, in seconds)
        // Add elapsed play time so the display ticks up during gameplay.
        {
            let elapsed_secs = timer.now_time_for_id(TIMER_PLAY) / 1000;
            let total = self.cumulative_playtime_seconds + elapsed_secs;
            s.integers.insert(17, (total / 3600) as i32);
            s.integers.insert(18, ((total / 60) % 60) as i32);
            s.integers.insert(19, (total % 60) as i32);
        }
        // Volume (0-100 scale)
        s.integers.insert(57, (self.system_volume * 100.0) as i32);
        s.integers.insert(58, (self.key_volume * 100.0) as i32);
        s.integers.insert(59, (self.bg_volume * 100.0) as i32);
        // BPM
        s.integers.insert(90, max_bpm as i32);
        s.integers.insert(91, min_bpm as i32);
        s.integers.insert(92, main_bpm as i32);
        s.integers.insert(160, now_bpm as i32);
        // Elapsed playtime from TIMER_PLAY (Java: timer.getNowTime(TIMER_PLAY))
        s.integers
            .insert(161, (timer.now_time_for_id(TIMER_PLAY) / 60000) as i32);
        s.integers.insert(
            162,
            ((timer.now_time_for_id(TIMER_PLAY) / 1000) % 60) as i32,
        );
        // Remaining playtime (Java: max(playtime - elapsed + 1000, 0))
        {
            let elapsed = timer.now_time_for_id(TIMER_PLAY);
            let remaining = (self.playtime - elapsed + 1000).max(0);
            s.integers.insert(163, (remaining / 60000) as i32);
            s.integers.insert(164, ((remaining / 1000) % 60) as i32);
        }
        // Scroll duration from LaneRenderer (Java: getCurrentDuration())
        s.integers.insert(312, current_duration);
        // Lanecover2: (1 - lift) * lanecover * 1000
        s.integers
            .insert(316, ((1.0 - live_lift) * live_lanecover * 1000.0) as i32);
        // Chart length (minutes/seconds)
        s.integers
            .insert(1163, ((self.playtime.max(0) / 60000) % 60) as i32);
        s.integers
            .insert(1164, ((self.playtime.max(0) / 1000) % 60) as i32);
        // Scroll duration variants (IDs 1312-1327)
        for id in 1312..=1327 {
            let offset = id - 1312;
            let green = offset % 2 == 1;
            let cover = offset % 4 < 2;
            let bpm_mode = offset / 4;
            let bpm = match bpm_mode {
                0 => now_bpm,
                1 => main_bpm,
                2 => min_bpm,
                3 => max_bpm,
                _ => 0.0,
            };
            let val = if bpm == 0.0 || live_hispeed == 0.0 {
                0
            } else {
                (240000.0 / bpm / live_hispeed as f64
                    * if cover {
                        1.0 - live_lanecover as f64
                    } else {
                        1.0
                    }
                    * if green { 0.6 } else { 1.0 })
                .round() as i32
            };
            s.integers.insert(id, val);
        }
        // Loading progress (0-100, gradual)
        s.integers.insert(165, {
            if self.media_load_finished {
                100
            } else {
                let progress = if self.bga_enabled {
                    (self.audio_progress + self.bga_progress) / 2.0
                } else {
                    self.audio_progress
                };
                (progress * 100.0) as i32
            }
        });
        // Player statistics (IDs 30-37, 333)
        s.integers.insert(
            30,
            self.player_data
                .as_ref()
                .map_or(0, |pd| pd.playcount as i32),
        );
        s.integers.insert(
            31,
            self.player_data.as_ref().map_or(0, |pd| pd.clear as i32),
        );
        s.integers.insert(
            32,
            self.player_data
                .as_ref()
                .map_or(0, |pd| (pd.playcount - pd.clear) as i32),
        );
        s.integers.insert(
            33,
            self.player_data
                .as_ref()
                .map_or(0, |pd| pd.judge_count(0) as i32),
        );
        s.integers.insert(
            34,
            self.player_data
                .as_ref()
                .map_or(0, |pd| pd.judge_count(1) as i32),
        );
        s.integers.insert(
            35,
            self.player_data
                .as_ref()
                .map_or(0, |pd| pd.judge_count(2) as i32),
        );
        s.integers.insert(
            36,
            self.player_data
                .as_ref()
                .map_or(0, |pd| pd.judge_count(3) as i32),
        );
        s.integers.insert(
            37,
            self.player_data
                .as_ref()
                .map_or(0, |pd| pd.judge_count(4) as i32),
        );
        s.integers.insert(
            333,
            self.player_data.as_ref().map_or(0, |pd| {
                let total: i64 = (0..=3).map(|judge| pd.judge_count(judge)).sum();
                total.min(i32::MAX as i64) as i32
            }),
        );

        // ================================================================
        // Float properties
        // ================================================================
        // Volume (0.0-1.0)
        s.floats.insert(17, self.system_volume);
        s.floats.insert(18, self.key_volume);
        s.floats.insert(19, self.bg_volume);
        // Loading progress (0.0-1.0, gradual)
        s.floats.insert(165, {
            if self.media_load_finished {
                1.0
            } else if self.bga_enabled {
                (self.audio_progress + self.bga_progress) / 2.0
            } else {
                self.audio_progress
            }
        });
        // Gauge value (0.0-100.0)
        s.floats
            .insert(1107, self.gauge.as_ref().map_or(0.0, |g| g.value()));
        // Hi-speed (from live LaneRenderer, not saved play config)
        s.floats.insert(310, live_hispeed);

        // ================================================================
        // Boolean properties
        // ================================================================
        // OPTION_AUTOPLAYOFF (32)
        s.booleans.insert(
            32,
            self.play_mode.mode != crate::core::bms_player_mode::Mode::Autoplay
                && self.play_mode.mode != crate::core::bms_player_mode::Mode::Replay,
        );
        // OPTION_AUTOPLAYON (33)
        s.booleans.insert(
            33,
            self.play_mode.mode == crate::core::bms_player_mode::Mode::Autoplay
                || self.play_mode.mode == crate::core::bms_player_mode::Mode::Replay,
        );
        // OPTION_GAUGE_GROOVE (42): gauge type <= 2
        s.booleans
            .insert(42, self.gauge.as_ref().is_some_and(|g| g.gauge_type() <= 2));
        // OPTION_GAUGE_HARD (43): gauge type >= 3
        s.booleans
            .insert(43, self.gauge.as_ref().is_some_and(|g| g.gauge_type() >= 3));
        // Loading state (OPTION_LOADING1 = 80)
        s.booleans.insert(80, self.state == PlayState::Preload);
        // OPTION_LOADED (81)
        s.booleans.insert(81, self.state != PlayState::Preload);
        // OPTION_REPLAY_OFF (82)
        s.booleans.insert(
            82,
            self.play_mode.mode != crate::core::bms_player_mode::Mode::Replay,
        );
        // OPTION_REPLAY_PLAYING (84)
        s.booleans.insert(
            84,
            self.play_mode.mode == crate::core::bms_player_mode::Mode::Replay,
        );
        // OPTION_1P_0_9 through OPTION_1P_100 (230-240)
        for bucket in 0..=10 {
            let id = 230 + bucket;
            let val = self.gauge.as_ref().is_some_and(|g| {
                let gauge = g.gauge();
                let max = gauge.property().max;
                let low = bucket as f32 * 0.1 * max;
                let high = (bucket + 1) as f32 * 0.1 * max;
                gauge.value() >= low && gauge.value() < high
            });
            s.booleans.insert(id, val);
        }
        // OPTION_1P_PERFECT (241)
        s.booleans.insert(241, self.judge.now_judge(0) == 1);
        // OPTION_2P_PERFECT (261)
        s.booleans.insert(261, self.judge.now_judge(1) == 1);
        // OPTION_LANECOVER1_ON (271)
        s.booleans.insert(271, live_lanecover > 0.0);
        // OPTION_LIFT1_ON (272)
        s.booleans.insert(272, live_lift > 0.0);
        // OPTION_HIDDEN1_ON (273)
        s.booleans.insert(273, live_hidden > 0.0);
        // OPTION_3P_PERFECT (361)
        s.booleans.insert(361, self.judge.now_judge(2) == 1);
        // OPTION_GAUGE_EX (1046)
        s.booleans.insert(
            1046,
            self.gauge
                .as_ref()
                .is_some_and(|g| matches!(g.gauge_type(), 0 | 1 | 4 | 5 | 7 | 8)),
        );
        // OPTION_STATE_PRACTICE (1080)
        s.booleans.insert(
            1080,
            self.play_mode.mode == crate::core::bms_player_mode::Mode::Practice,
        );
        // OPTION_1P_BORDER_OR_MORE (1240)
        s.booleans
            .insert(1240, self.gauge.as_ref().is_some_and(|g| g.is_qualified()));
        // OPTION_1P_EARLY (1242)
        s.booleans.insert(
            1242,
            self.judge.now_judge(0) > 1 && self.judge.recent_judge_timing(0) > 0,
        );
        // OPTION_1P_LATE (1243)
        s.booleans.insert(
            1243,
            self.judge.now_judge(0) > 1 && self.judge.recent_judge_timing(0) < 0,
        );
        // OPTION_2P_EARLY (1262)
        s.booleans.insert(
            1262,
            self.judge.now_judge(1) > 1 && self.judge.recent_judge_timing(1) > 0,
        );
        // OPTION_2P_LATE (1263)
        s.booleans.insert(
            1263,
            self.judge.now_judge(1) > 1 && self.judge.recent_judge_timing(1) < 0,
        );
        // OPTION_3P_EARLY (1362)
        s.booleans.insert(
            1362,
            self.judge.now_judge(2) > 1 && self.judge.recent_judge_timing(2) > 0,
        );
        // OPTION_3P_LATE (1363)
        s.booleans.insert(
            1363,
            self.judge.now_judge(2) > 1 && self.judge.recent_judge_timing(2) < 0,
        );

        // ================================================================
        // String properties
        // ================================================================
        s.strings.insert(10, self.song_metadata.title.clone());
        s.strings.insert(11, self.song_metadata.subtitle.clone());
        s.strings.insert(12, self.song_metadata.full_title());
        s.strings.insert(13, self.song_metadata.genre.clone());
        s.strings.insert(14, self.song_metadata.artist.clone());
        s.strings.insert(15, self.song_metadata.subartist.clone());
        s.strings.insert(16, {
            if self.song_metadata.subartist.is_empty() {
                self.song_metadata.artist.clone()
            } else {
                format!(
                    "{} {}",
                    self.song_metadata.artist, self.song_metadata.subartist
                )
            }
        });

        // ================================================================
        // Image index properties
        // ================================================================
        // lnmode override (ID 308)
        if let Some(override_val) = self.lnmode_override {
            s.image_indices.insert(308, override_val);
        }

        s
    }

    /// Copy player_config and config mutations from the snapshot back to live state.
    ///
    /// The snapshot contains cloned copies of player_config and config. If the skin
    /// mutates them via `player_config_mut()` or `config_mut()`, those changes must
    /// be propagated back after drain_actions.
    pub(super) fn propagate_snapshot_config(
        &mut self,
        snapshot: &rubato_types::property_snapshot::PropertySnapshot,
    ) {
        if let Some(ref pc) = snapshot.player_config {
            self.player_config = (**pc).clone();
        }
        if let Some(ref c) = snapshot.config {
            self.config = (**c).clone();
        }
    }

    /// Apply queued actions from the snapshot back to live game state.
    pub(super) fn drain_actions(
        &mut self,
        actions: &mut SkinActionQueue,
        timer: &mut TimerManager,
    ) {
        // Timer sets
        for (timer_id, micro_time) in actions.timer_sets.drain(..) {
            timer.set_micro_timer(timer_id, micro_time);
        }

        // State changes
        for state in actions.state_changes.drain(..) {
            self.pending.pending_state_change = Some(state);
        }

        // Audio play/stop
        for (path, volume, is_loop) in actions.audio_plays.drain(..) {
            self.pending
                .pending_audio_path_plays
                .push((path, volume, is_loop));
        }
        for path in actions.audio_stops.drain(..) {
            self.pending.pending_audio_path_stops.push(path);
        }

        // Config propagation (audio config changed)
        if actions.audio_config_changed {
            if let Some(audio) = self.config.audio.clone() {
                self.pending.pending_audio_config = Some(audio);
            }
            actions.audio_config_changed = false;
        }

        // Float writes (volume sliders)
        for (id, value) in actions.float_writes.drain(..) {
            if (17..=19).contains(&id) {
                let clamped = value.clamp(0.0, 1.0);
                match id {
                    17 => self.system_volume = clamped,
                    18 => self.key_volume = clamped,
                    19 => self.bg_volume = clamped,
                    _ => {}
                }
                if let Some(mut audio) = self.config.audio.clone() {
                    match id {
                        17 => audio.systemvolume = clamped,
                        18 => audio.keyvolume = clamped,
                        19 => audio.bgvolume = clamped,
                        _ => {}
                    }
                    self.config.audio = Some(audio.clone());
                    self.pending.pending_audio_config = Some(audio);
                }
            }
        }
    }
}
