use super::*;

impl BMSPlayer {
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

        // GhostBattle seed/option override (Java lines 119-138)
        let mut ghost_battle = crate::ghost_battle_play::consume();
        if let Some(ref mut gb) = ghost_battle {
            self.score.playinfo.randomoption = gb.random;
            // Mirror inversion: if player config is MIRROR, flip ghost's option
            const IDENTITY: i32 = 0; // Random::Identity ordinal
            const MIRROR: i32 = 1; // Random::Mirror ordinal
            const RANDOM: i32 = 2; // Random::Random ordinal
            if config.play_settings.random == MIRROR {
                match gb.random {
                    IDENTITY => self.score.playinfo.randomoption = MIRROR,
                    MIRROR => self.score.playinfo.randomoption = IDENTITY,
                    RANDOM => {
                        // Reverse the decimal digit representation of the lane pattern
                        let reversed: i32 = gb
                            .lanes
                            .to_string()
                            .chars()
                            .rev()
                            .collect::<String>()
                            .parse()
                            .unwrap_or(gb.lanes);
                        gb.lanes = reversed;
                    }
                    _ => {}
                }
            }
        } else if let Some(chart_option) = self.chart_option.take() {
            // ChartOption override (Java lines 140-148)
            self.score.playinfo.randomoption = chart_option.randomoption;
            self.score.playinfo.randomoptionseed = chart_option.randomoptionseed;
            self.score.playinfo.randomoption2 = chart_option.randomoption2;
            self.score.playinfo.randomoption2seed = chart_option.randomoption2seed;
            self.score.playinfo.doubleoption = chart_option.doubleoption;
            self.score.playinfo.rand = chart_option.rand;
        }

        // -- Phase 1: Pre-option modifiers (scroll, LN, mine, extra) --
        let mut pre_mods: Vec<Box<dyn PatternModifier>> = Vec::new();

        if config.display_settings.scroll_mode > 0 {
            pre_mods.push(Box::new(ScrollSpeedModifier::with_params(
                config.display_settings.scroll_mode - 1,
                config.display_settings.scroll_section,
                config.display_settings.scroll_rate,
            )));
        }
        if config.note_modifier_settings.longnote_mode > 0 {
            pre_mods.push(Box::new(LongNoteModifier::with_params(
                config.note_modifier_settings.longnote_mode - 1,
                config.note_modifier_settings.longnote_rate,
            )));
        }
        if config.play_settings.mine_mode > 0 {
            pre_mods.push(Box::new(MineNoteModifier::with_mode(
                config.play_settings.mine_mode - 1,
            )));
        }
        if config.display_settings.extranote_depth > 0 {
            pre_mods.push(Box::new(ExtraNoteModifier::new(
                config.display_settings.extranote_type,
                config.display_settings.extranote_depth,
                config.display_settings.extranote_scratch,
            )));
        }

        // Apply pre-option modifiers and accumulate assist level
        for m in pre_mods.iter_mut() {
            m.modify(&mut self.play.model);
            let assist_level = m.assist_level();
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
        if self.score.playinfo.doubleoption >= 2 {
            let mode = self.play.model.mode().copied().unwrap_or(Mode::BEAT_7K);
            if mode == Mode::BEAT_5K || mode == Mode::BEAT_7K || mode == Mode::KEYBOARD_24K {
                // Convert SP mode to DP mode
                let new_mode = match mode {
                    Mode::BEAT_5K => Mode::BEAT_10K,
                    Mode::BEAT_7K => Mode::BEAT_14K,
                    Mode::KEYBOARD_24K => Mode::KEYBOARD_24K_DOUBLE,
                    _ => unreachable!(),
                };
                self.play.model.set_mode(new_mode);

                // Apply PlayerBattleModifier
                let mut battle_mod = PlayerBattleModifier::new();
                battle_mod.modify(&mut self.play.model);

                // If doubleoption == 3, also add AutoplayModifier for scratch keys
                if self.score.playinfo.doubleoption == 3 {
                    let dp_mode = self.play.model.mode().copied().unwrap_or(Mode::BEAT_14K);
                    let scratch_keys = dp_mode.scratch_key().to_vec();
                    let mut autoplay_mod = AutoplayModifier::new(scratch_keys);
                    autoplay_mod.modify(&mut self.play.model);
                }

                self.assist = self.assist.max(1);
                score = false;
                log::info!("Pattern option: BATTLE (L-ASSIST)");
            } else {
                // Not SP mode, so BATTLE is not applied
                self.score.playinfo.doubleoption = 0;
            }
        }

        // -- Phase 3: Random option modifiers --
        // This section corresponds to Java lines 384-447
        let mode = self.play.model.mode().copied().unwrap_or(Mode::BEAT_7K);
        let player_count = mode.player();
        let mut pattern_array: Vec<Option<Vec<i32>>> = vec![None; player_count as usize];

        let mut random_mods: Vec<Box<dyn PatternModifier>> = Vec::new();

        // DP option modifiers
        if player_count == 2 {
            if self.score.playinfo.doubleoption == 1 {
                random_mods.push(Box::new(PlayerFlipModifier::new()));
            }
            log::info!("Pattern option (DP): {}", self.score.playinfo.doubleoption);

            // 2P random option
            let mut pm2 = rubato_core::pattern::pattern_modifier::create_pattern_modifier(
                self.score.playinfo.randomoption2,
                1,
                &mode,
                config,
            );
            if self.score.playinfo.randomoption2seed != -1 {
                pm2.set_seed(self.score.playinfo.randomoption2seed);
            } else {
                self.score.playinfo.randomoption2seed = pm2.get_seed();
            }
            random_mods.push(pm2);
            log::info!(
                "Pattern option (2P): {}, Seed: {}",
                self.score.playinfo.randomoption2,
                self.score.playinfo.randomoption2seed
            );
        }

        // 1P random option
        let mut pm1 = rubato_core::pattern::pattern_modifier::create_pattern_modifier(
            self.score.playinfo.randomoption,
            0,
            &mode,
            config,
        );
        if self.score.playinfo.randomoptionseed != -1 {
            pm1.set_seed(self.score.playinfo.randomoptionseed);
        } else {
            // GhostBattle/RandomTrainer seed override requires RandomTrainer::getRandomSeedMap()
            // which lives in beatoraja-modmenu (circular dep). The seed map would need to be
            // passed in as an external dependency when GhostBattle or RandomTrainer is active.
            self.score.playinfo.randomoptionseed = pm1.get_seed();
        }
        random_mods.push(pm1);
        log::info!(
            "Pattern option (1P): {}, Seed: {}",
            self.score.playinfo.randomoption,
            self.score.playinfo.randomoptionseed
        );

        // 7to9 mode
        if config.note_modifier_settings.seven_to_nine_pattern >= 1 && mode == Mode::BEAT_7K {
            let mode_mod = ModeModifier::new(Mode::BEAT_7K, Mode::POPN_9K, config.clone());
            random_mods.push(Box::new(mode_mod));
        }

        // Apply all random modifiers
        for m in random_mods.iter_mut() {
            m.modify(&mut self.play.model);

            let assist_level = m.assist_level();
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
                let current_mode = self.play.model.mode().copied().unwrap_or(Mode::BEAT_7K);
                let player_idx = m.player() as usize;
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
            self.score.playinfo.lane_shuffle_pattern = Some(patterns);
        }

        score
    }

    pub fn now_quarter_note_time(&self) -> i64 {
        self.rhythm
            .as_ref()
            .map_or(0, |r| r.now_quarter_note_time())
    }

    pub fn play_skin(&self) -> &PlaySkin {
        &self.play.play_skin
    }

    pub fn play_skin_mut(&mut self) -> &mut PlaySkin {
        &mut self.play.play_skin
    }

    pub fn gaugelog(&self) -> &[Vec<f32>] {
        &self.gaugelog
    }

    /// Restore replay data into playinfo based on key state.
    ///
    /// Corresponds to Java BMSPlayer constructor lines 150-214.
    ///
    /// When in REPLAY mode for a single song:
    /// - If `replay` is `None`: cannot load replay, switch to PLAY mode.
    /// - Key1 held (pattern_key): Copy all pattern options + seeds + rand from replay to playinfo.
    ///   Then switch to PLAY mode (replay pattern mode).
    /// - Key2 held (option_key): Copy pattern options (no seeds, no rand) from replay to playinfo.
    ///   Then switch to PLAY mode (replay option mode).
    /// - Key4 held (hs_key): Save replay's PlayConfig for HS restoration.
    ///   Then switch to PLAY mode.
    /// - If any of the above keys were held, `replay` is discarded and mode becomes PLAY.
    /// - If none of the above keys were held, the replay is kept for keylog playback.
    ///
    /// Returns `ReplayRestoreResult` with whether to stay in replay mode, the replay data,
    /// and any HS config to apply.
    pub fn restore_replay_data(
        &mut self,
        replay: Option<ReplayData>,
        key_state: &ReplayKeyState,
    ) -> ReplayRestoreResult {
        match replay {
            None => {
                // No replay data available -> fall back to PLAY mode
                log::info!("リプレイデータを読み込めなかったため、通常プレイモードに移行");
                ReplayRestoreResult {
                    stay_replay: false,
                    replay: None,
                    hs_replay_config: None,
                }
            }
            Some(replay_data) => {
                let mut is_replay_pattern_play = false;
                let mut hs_config: Option<PlayConfig> = None;

                if key_state.pattern_key {
                    // Replay pattern mode: copy options + seeds + rand
                    log::info!("リプレイ再現モード : 譜面");
                    self.score.playinfo.randomoption = replay_data.randomoption;
                    self.score.playinfo.randomoptionseed = replay_data.randomoptionseed;
                    self.score.playinfo.randomoption2 = replay_data.randomoption2;
                    self.score.playinfo.randomoption2seed = replay_data.randomoption2seed;
                    self.score.playinfo.doubleoption = replay_data.doubleoption;
                    self.score.playinfo.rand = replay_data.rand.clone();
                    is_replay_pattern_play = true;
                } else if key_state.option_key {
                    // Replay option mode: copy options only (no seeds, no rand)
                    log::info!("リプレイ再現モード : オプション");
                    self.score.playinfo.randomoption = replay_data.randomoption;
                    self.score.playinfo.randomoption2 = replay_data.randomoption2;
                    self.score.playinfo.doubleoption = replay_data.doubleoption;
                    is_replay_pattern_play = true;
                }

                if key_state.hs_key {
                    // Replay HS option mode: save replay config
                    log::info!("リプレイ再現モード : ハイスピード");
                    hs_config = replay_data.config.clone();
                    is_replay_pattern_play = true;
                }

                if is_replay_pattern_play {
                    // Switch to PLAY mode, discard replay
                    ReplayRestoreResult {
                        stay_replay: false,
                        replay: None,
                        hs_replay_config: hs_config,
                    }
                } else {
                    // Normal replay mode: keep replay for keylog playback
                    ReplayRestoreResult {
                        stay_replay: true,
                        replay: Some(replay_data),
                        hs_replay_config: None,
                    }
                }
            }
        }
    }

    /// Select the gauge type to use.
    ///
    /// Corresponds to Java BMSPlayer constructor lines 456-466.
    ///
    /// In REPLAY mode with a replay, uses the replay's gauge type.
    /// Additionally, key3/key5 can shift the gauge type upward:
    ///   shift = (key5 ? 1 : 0) + (key3 ? 2 : 0)
    ///   If replay.gauge is not HAZARD or EXHARDCLASS, increment gauge by shift.
    /// In PLAY mode, uses the config gauge type.
    pub fn select_gauge_type(
        replay: Option<&ReplayData>,
        config_gauge: i32,
        key_state: &ReplayKeyState,
    ) -> i32 {
        match replay {
            Some(replay_data) => {
                let mut gauge = replay_data.gauge;
                let shift = (if key_state.gauge_shift_key5 { 1 } else { 0 })
                    + (if key_state.gauge_shift_key3 { 2 } else { 0 });
                for _ in 0..shift {
                    if gauge != rubato_types::groove_gauge::HAZARD
                        && gauge != rubato_types::groove_gauge::EXHARDCLASS
                    {
                        gauge += 1;
                    }
                }
                gauge
            }
            None => config_gauge,
        }
    }

    /// Handle RANDOM syntax (branch chart loading) for replay/play mode.
    ///
    /// Corresponds to Java BMSPlayer constructor lines 225-242.
    ///
    /// If the model has RANDOM branches:
    /// - In REPLAY mode: use replay.rand
    /// - If resource has a saved seed (randomoptionseed != -1): use resource's rand
    /// - If rand is set and non-empty: reload BMS model with that rand
    ///   (actual model reload deferred → Phase 41)
    /// - Store final model.getRandom() into playinfo.rand
    ///
    /// Returns the rand values to use for model reload (if any), or None.
    pub fn handle_random_syntax(
        &mut self,
        is_replay_mode: bool,
        replay: Option<&ReplayData>,
        resource_replay_seed: i64,
        resource_rand: &[i32],
    ) -> Option<Vec<i32>> {
        let model_random = self.play.model.random().map(|r| r.to_vec());
        if let Some(ref random) = model_random
            && !random.is_empty()
        {
            if is_replay_mode {
                if let Some(replay_data) = replay {
                    self.score.playinfo.rand = replay_data.rand.clone();
                }
            } else if resource_replay_seed != -1 {
                // This path is hit on MusicResult / QuickRetry
                self.score.playinfo.rand = resource_rand.to_vec();
            }

            if !self.score.playinfo.rand.is_empty() {
                // Return rand to the caller for model reload via PlayerResource.
                // Caller should: resource.load_bms_model(rand), then update self.play.model
                // and self.score.playinfo.rand = model.random().
                log::info!("譜面分岐 : {:?}", self.score.playinfo.rand);
                return Some(self.score.playinfo.rand.clone());
            }

            // No rand override, store model's random into playinfo
            self.score.playinfo.rand = random.clone();
            log::info!("譜面分岐 : {:?}", self.score.playinfo.rand);
        }
        None
    }

    /// Calculate non-modifier assist flags (BPM guide, custom judge, constant speed).
    ///
    /// Corresponds to Java BMSPlayer constructor lines 269-301.
    /// This method checks assist conditions that are NOT from pattern modifiers:
    /// 1. BPM guide with variable BPM → LightAssist (assist=1)
    /// 2. Custom judge with any window rate > 100 → Assist (assist=2)
    /// 3. Constant speed enabled → Assist (assist=2)
    ///
    /// Accumulates with any existing assist level (e.g., from `build_pattern_modifiers`).
    /// Returns `true` if score submission is still valid (no assist triggered here).
    pub fn calculate_non_modifier_assist(&mut self, config: &PlayerConfig) -> bool {
        let mut score = true;

        // BPM Guide check (Java lines 269-272)
        // BPM変化がなければBPMガイドなし
        if config.display_settings.bpmguide
            && (self.play.model.get_min_bpm() < self.play.model.max_bpm())
        {
            self.assist = self.assist.max(1);
            score = false;
        }

        // Custom Judge check (Java lines 275-280)
        if config.judge_settings.custom_judge
            && (config.judge_settings.key_judge_window_rate_perfect_great > 100
                || config.judge_settings.key_judge_window_rate_great > 100
                || config.judge_settings.key_judge_window_rate_good > 100
                || config
                    .judge_settings
                    .scratch_judge_window_rate_perfect_great
                    > 100
                || config.judge_settings.scratch_judge_window_rate_great > 100
                || config.judge_settings.scratch_judge_window_rate_good > 100)
        {
            self.assist = self.assist.max(2);
            score = false;
        }

        // Constant speed check (Java lines 297-301)
        // Constant considered as assist in Endless Dream
        // This is a community discussion result, see https://github.com/seraxis/lr2oraja-endlessdream/issues/42
        let mode = self.play.model.mode().copied().unwrap_or(Mode::BEAT_7K);
        if config.play_config_ref(mode).playconfig.enable_constant {
            self.assist = self.assist.max(2);
            score = false;
        }

        score
    }

    /// Apply frequency trainer speed modification.
    ///
    /// Corresponds to Java BMSPlayer constructor lines 246-267.
    ///
    /// When freq trainer is enabled in PLAY mode (non-course):
    /// 1. Adjusts playtime based on frequency ratio
    /// 2. Scales chart timing via `BMSModelUtils::change_frequency`
    /// 3. Returns result with freq state and optional global pitch
    ///
    /// Returns `None` if freq trainer should not be applied (freq == 100,
    /// not play mode, or course mode).
    pub fn apply_freq_trainer(
        &mut self,
        freq: i32,
        is_play_mode: bool,
        is_course: bool,
        freq_option: &FrequencyType,
    ) -> Option<FreqTrainerResult> {
        if freq == 100 || freq == 0 || !is_play_mode || is_course {
            return None;
        }

        // Adjust playtime: (lastNoteTime + 1000) * 100 / freq + TIME_MARGIN
        self.play.playtime = (self.play.model.last_note_time() + 1000) * 100 / freq + TIME_MARGIN;

        // Scale chart timing
        bms_model_utils::change_frequency(&mut self.play.model, freq as f32 / 100.0);

        // Determine global pitch
        let global_pitch = match freq_option {
            FrequencyType::FREQUENCY => Some(freq as f32 / 100.0),
            _ => None,
        };

        // Format freq string (matches Java FreqTrainerMenu.getFreqString())
        let rate = freq as f32 / 100.0;
        let freq_string = format!("[{:.02}x]", rate);

        Some(FreqTrainerResult {
            freq_on: true,
            freq_string,
            force_no_ir_send: true,
            global_pitch,
        })
    }

    /// Get the ClearType override for the current assist level.
    ///
    /// Corresponds to Java BMSPlayer assist → ClearType mapping:
    /// - assist == 0 → None (no override)
    /// - assist == 1 → LightAssistEasy
    /// - assist >= 2 → NoPlay
    pub fn clear_type_for_assist(&self) -> Option<ClearType> {
        if self.assist == 0 {
            None
        } else if self.assist == 1 {
            Some(ClearType::LightAssistEasy)
        } else {
            Some(ClearType::NoPlay)
        }
    }

    /// Build guide SE configuration for the audio driver.
    ///
    /// Translated from: BMSPlayer.create() guide SE setup (Java lines 512-524)
    ///
    /// Returns a list of (judge_index, Option<path>) pairs.
    /// When `is_guide_se` is true, each entry contains the resolved path from
    /// `SystemSoundManager::sound_paths()` for the corresponding guide SE type.
    /// When false, all entries contain None (clearing the additional key sounds).
    ///
    /// The caller should apply each entry to the audio driver:
    ///   `audio.set_additional_key_sound(judge, true, path);`
    ///   `audio.set_additional_key_sound(judge, false, path);`
    pub fn build_guide_se_config(
        is_guide_se: bool,
        sound_manager: &rubato_core::system_sound_manager::SystemSoundManager,
    ) -> Vec<(i32, Option<String>)> {
        use rubato_core::system_sound_manager::SoundType;

        let guide_se_types = [
            SoundType::GuidesePg,
            SoundType::GuideseGr,
            SoundType::GuideseGd,
            SoundType::GuideseBd,
            SoundType::GuidesePr,
            SoundType::GuideseMs,
        ];

        let mut config = Vec::with_capacity(6);
        for (i, sound_type) in guide_se_types.iter().enumerate() {
            if is_guide_se {
                let paths = sound_manager.sound_paths(sound_type);
                let path = paths.first().map(|p| p.to_string_lossy().to_string());
                config.push((i as i32, path));
            } else {
                config.push((i as i32, None));
            }
        }
        config
    }

    /// Get mutable reference to playinfo for testing.
    #[cfg(test)]
    pub fn playinfo_mut(&mut self) -> &mut ReplayData {
        &mut self.score.playinfo
    }
}
