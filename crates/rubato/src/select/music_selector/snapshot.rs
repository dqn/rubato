use crate::core::timer_manager::TimerManager;
use rubato_skin::property_snapshot::PropertySnapshot;
use rubato_skin::skin_action_queue::SkinActionQueue;
use rubato_skin::skin_property::*;
use rubato_skin::timer_id::TimerId;

use super::*;

impl MusicSelector {
    /// Build a PropertySnapshot capturing all raw data needed for skin rendering.
    ///
    /// This captures every property value that SelectSkinContext's SkinRenderContext
    /// implementation provides, so that PropertySnapshot (which also implements
    /// SkinRenderContext) produces identical results.
    pub(super) fn build_snapshot(&self, timer: &TimerManager) -> PropertySnapshot {
        let mut s = PropertySnapshot::new();

        // ---- Timing ----
        s.now_time = timer.now_time();
        s.now_micro_time = timer.now_micro_time();
        s.boot_time_millis = timer.boot_time_millis();
        for (i, &val) in timer.timer_values().iter().enumerate() {
            if val != i64::MIN {
                s.timers.insert(TimerId::new(i as i32), val);
            }
        }

        // ---- State identity ----
        s.state_type = Some(rubato_skin::main_state_type::MainStateType::MusicSelect);

        // ---- Config ----
        s.config = Some(Box::new(self.app_config.clone()));
        s.player_config = Some(Box::new(self.config.clone()));

        // ---- Play config (resolve mode from selected bar) ----
        s.play_config = self
            .selected_play_config_mode()
            .map(|mode| Box::new(self.config.play_config_ref(mode).playconfig.clone()));

        // ---- Song / score data ----
        let selected_bar = self.manager.selected();
        let selected_song_data = selected_bar
            .and_then(|b| b.as_song_bar())
            .map(|sb| sb.song_data());
        let selected_score = selected_bar.and_then(|b| b.score());
        let selected_rival_score = selected_bar.and_then(|b| b.rival_score());

        s.song_data = selected_song_data.map(|d| Box::new(d.clone()));
        s.score_data = selected_score.map(|d| Box::new(d.clone()));
        s.rival_score_data = selected_rival_score.map(|d| Box::new(d.clone()));

        // Target score
        {
            let targetid = &self.config.select_settings.targetid;
            let target =
                if targetid.starts_with("RIVAL_RANK_") || targetid.starts_with("RIVAL_NEXT_") {
                    self.cached_target_score.as_ref()
                } else if targetid.starts_with("RIVAL_") {
                    selected_rival_score
                } else if targetid == "MYBEST" {
                    selected_score
                } else {
                    self.cached_target_score.as_ref()
                };
            s.target_score_data = target.map(|d| Box::new(d.clone()));
        }

        // Score data property
        s.score_data_property = self.cached_score_data_property.clone();

        // ---- Player data ----
        s.player_data = self
            .player_resource
            .as_ref()
            .map(|r| *crate::core::player_resource::PlayerResource::player_data(r));

        // ---- Offsets ----
        s.offsets = self.main_state_data.offsets.clone();

        // ---- Mode / sort image indices ----
        {
            let current_mode = self.config.mode();
            let mode_index = MODE.iter().position(|mode| mode.as_ref() == current_mode);
            if let Some(mode_index) = mode_index {
                // LR2 skin image order: 0=all, 1=5k, 2=7k, 3=10k, 4=14k, 5=9k, 6=24k, 7=24kDP
                let lr2_mode_indices = [0, 2, 4, 5, 1, 3, 6, 7];
                s.mode_image_index = Some(
                    lr2_mode_indices
                        .get(mode_index)
                        .copied()
                        .unwrap_or(mode_index as i32),
                );
            }
        }
        s.sort_image_index = Some(self.sort());

        // ---- Ranking data ----
        s.ranking_offset = self.ranking.ranking_offset;
        if let Some(ref ranking) = self.ranking.currentir {
            for slot in 0..10 {
                let index = self.ranking.ranking_offset + slot;
                let clear_type = ranking
                    .score(index)
                    .map(|score| score.clear.id())
                    .unwrap_or(-1);
                s.ranking_clear_types.push(clear_type);
            }
        }

        // ---- Distribution data ----
        if let Some(dir) = selected_bar.and_then(|b| b.as_directory_bar()) {
            s.distribution_data = Some(rubato_skin::distribution_data::DistributionData {
                lamps: *dir.lamps(),
                ranks: *dir.ranks(),
            });
        }

        // ---- Select-specific integers ----
        // Directory lamp sum (300)
        if let Some(dir) = selected_bar.and_then(|b| b.as_directory_bar()) {
            s.integers.insert(
                300,
                dir.lamps.iter().fold(0i32, |acc, &x| acc.saturating_add(x)),
            );
        }
        // Song play/clear/fail counts (77-79)
        if let Some(score) = selected_score {
            s.integers.insert(77, score.playcount);
            s.integers.insert(78, score.clearcount);
            s.integers.insert(79, score.playcount - score.clearcount);
        }

        // ---- Select-specific booleans ----
        // Bar type
        s.booleans.insert(
            OPTION_SONGBAR,
            selected_bar.is_some_and(|b| b.as_song_bar().is_some()),
        );
        s.booleans.insert(
            OPTION_FOLDERBAR,
            selected_bar.is_some_and(|b| b.is_directory_bar()),
        );
        s.booleans.insert(
            OPTION_GRADEBAR,
            selected_bar.is_some_and(|b| b.as_grade_bar().is_some()),
        );
        // Select bar clear conditions
        s.booleans.insert(
            OPTION_SELECT_BAR_NOT_PLAYED,
            selected_bar.is_none_or(|b| b.lamp(true) == 0),
        );
        s.booleans.insert(
            OPTION_SELECT_BAR_FAILED,
            selected_bar.is_some_and(|b| b.lamp(true) == 1),
        );
        s.booleans.insert(
            OPTION_SELECT_BAR_ASSIST_EASY_CLEARED,
            selected_bar.is_some_and(|b| b.lamp(true) == 2),
        );
        s.booleans.insert(
            OPTION_SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED,
            selected_bar.is_some_and(|b| b.lamp(true) == 3),
        );
        s.booleans.insert(
            OPTION_SELECT_BAR_EASY_CLEARED,
            selected_bar.is_some_and(|b| b.lamp(true) == 4),
        );
        s.booleans.insert(
            OPTION_SELECT_BAR_NORMAL_CLEARED,
            selected_bar.is_some_and(|b| b.lamp(true) == 5),
        );
        s.booleans.insert(
            OPTION_SELECT_BAR_HARD_CLEARED,
            selected_bar.is_some_and(|b| b.lamp(true) == 6),
        );
        s.booleans.insert(
            OPTION_SELECT_BAR_EXHARD_CLEARED,
            selected_bar.is_some_and(|b| b.lamp(true) == 7),
        );
        s.booleans.insert(
            OPTION_SELECT_BAR_FULL_COMBO_CLEARED,
            selected_bar.is_some_and(|b| b.lamp(true) == 8),
        );
        s.booleans.insert(
            OPTION_SELECT_BAR_PERFECT_CLEARED,
            selected_bar.is_some_and(|b| b.lamp(true) == 9),
        );
        s.booleans.insert(
            OPTION_SELECT_BAR_MAX_CLEARED,
            selected_bar.is_some_and(|b| b.lamp(true) == 10),
        );
        // Replay data availability per slot
        let replay_exists = |slot: i32| -> bool {
            selected_bar
                .and_then(|b| b.as_selectable_bar())
                .is_some_and(|sb| sb.exists_replay(slot))
        };
        s.booleans.insert(OPTION_REPLAYDATA, replay_exists(0));
        s.booleans.insert(OPTION_REPLAYDATA2, replay_exists(1));
        s.booleans.insert(OPTION_REPLAYDATA3, replay_exists(2));
        s.booleans.insert(OPTION_REPLAYDATA4, replay_exists(3));
        s.booleans.insert(OPTION_NO_REPLAYDATA, !replay_exists(0));
        s.booleans.insert(OPTION_NO_REPLAYDATA2, !replay_exists(1));
        s.booleans.insert(OPTION_NO_REPLAYDATA3, !replay_exists(2));
        s.booleans.insert(OPTION_NO_REPLAYDATA4, !replay_exists(3));
        // Autoplay (always off on select screen)
        s.booleans.insert(33, false); // OPTION_AUTOPLAYON
        s.booleans.insert(32, true); // OPTION_AUTOPLAYOFF
        // Panels (always visible on select)
        s.booleans.insert(21, true); // OPTION_PANEL1

        // ---- Select-specific floats ----
        // Music select scroll position
        s.floats.insert(1, self.manager.selected_position());
        // Ranking scroll position
        s.floats.insert(8, self.ranking_position());
        // Level (0.0-1.0 normalized)
        if let Some(song) = selected_song_data {
            s.floats.insert(103, song.chart.level as f32 / 12.0);
        }
        // Hi-speed (from selected bar's play config)
        if let Some(pc) = self.get_selected_play_config_ref() {
            s.floats.insert(310, pc.hispeed);
        }

        // ---- Select-specific strings ----
        // Search word
        {
            let search_word = self.search.as_ref().map_or_else(String::new, |search| {
                if search.text.is_empty() {
                    search.message_text.clone()
                } else {
                    search.text.clone()
                }
            });
            if !search_word.is_empty() {
                s.strings.insert(30, search_word);
            }
        }
        // Course titles (150-159)
        for i in 0..10 {
            let title = self.course_title_at(i);
            if !title.is_empty() {
                s.strings.insert(150 + i as i32, title);
            }
        }
        // Directory string
        {
            let dir_str = self.manager.directory_string().to_string();
            if !dir_str.is_empty() {
                s.strings.insert(1000, dir_str);
            }
        }
        // Version
        s.strings.insert(
            1010,
            crate::core::version::Version::get_version().to_string(),
        );
        // Song hashes
        if let Some(song) = selected_song_data {
            if !song.file.md5.is_empty() {
                s.strings.insert(1030, song.file.md5.clone());
            }
            if !song.file.sha256.is_empty() {
                s.strings.insert(1031, song.file.sha256.clone());
            }
        }

        s
    }

    /// Helper to compute course title at index (used by build_snapshot).
    /// Same logic as SelectSkinContext::course_title_at.
    fn course_title_at(&self, index: usize) -> String {
        let selected_bar = self.manager.selected();

        if let Some(course_bar) = selected_bar.and_then(|bar| bar.as_grade_bar()) {
            return course_bar
                .song_datas()
                .get(index)
                .map_or_else(String::new, |song| {
                    let title = song.metadata.title.clone();
                    if song.file.path().is_some() {
                        title
                    } else {
                        format!("(no song) {title}")
                    }
                });
        }

        selected_bar
            .and_then(|bar| bar.as_random_course_bar())
            .and_then(|bar| bar.course_data().stage().get(index))
            .map(|stage| stage.title.clone().unwrap_or_else(|| "----".to_string()))
            .unwrap_or_default()
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
            self.pending_state_change = Some(state);
        }

        // Audio: store in pending lists for outbox drain
        for (path, volume, is_loop) in actions.audio_plays.drain(..) {
            self.pending_audio_path_plays.push((path, volume, is_loop));
        }
        for path in actions.audio_stops.drain(..) {
            self.pending_audio_path_stops.push(path);
        }

        // Config propagation (audio config changes)
        if actions.audio_config_changed {
            if let Some(audio) = self.app_config.audio.clone() {
                self.pending_audio_config = Some(audio);
            }
            actions.audio_config_changed = false;
        }

        // Float writes (volume sliders IDs 17-19, scroll position ID 1, ranking position ID 8)
        for (id, value) in actions.float_writes.drain(..) {
            match id {
                1 => self.manager.set_selected_position(value),
                8 => self.set_ranking_position(value),
                17..=19 => {
                    if let Some(audio) = self.app_config.audio.as_mut() {
                        let clamped = value.clamp(0.0, 1.0);
                        match id {
                            17 => audio.systemvolume = clamped,
                            18 => audio.keyvolume = clamped,
                            19 => audio.bgvolume = clamped,
                            _ => unreachable!(),
                        }
                    }
                    // Propagate audio config change via outbox
                    if let Some(audio) = self.app_config.audio.clone() {
                        self.pending_audio_config = Some(audio);
                    }
                }
                _ => {}
            }
        }

        // Option change sound
        if actions.option_change_sound {
            self.play_option_change();
            actions.option_change_sound = false;
        }

        // Bar update after change
        if actions.update_bar_after_change {
            self.refresh_bar_with_context();
            actions.update_bar_after_change = false;
        }

        // Song selection mode requests
        for event_id in actions.select_song_mode_requests.drain(..) {
            let mode = match event_id {
                15 => Some(BMSPlayerMode::PLAY),
                16 => Some(BMSPlayerMode::AUTOPLAY),
                315 => Some(BMSPlayerMode::PRACTICE),
                _ => None,
            };
            if let Some(mode) = mode {
                self.select_song(mode);
            }
        }

        // Custom events (delegated events)
        // These are handled in the custom event replay loop, not here.
    }

    /// Copy player_config back from the snapshot to the selector if it was modified.
    pub(super) fn propagate_player_config(&mut self, snapshot: &PropertySnapshot) {
        if let Some(ref pc) = snapshot.player_config {
            self.config = (**pc).clone();
            self.pending_player_config_dirty = true;
        }
    }

    /// Copy app config (audio) back from the snapshot if it was modified.
    pub(super) fn propagate_app_config(&mut self, snapshot: &PropertySnapshot) {
        if let Some(ref config) = snapshot.config {
            self.app_config = (**config).clone();
        }
    }
}
