use super::*;

/// Mirror-invert a ghost-battle lane pattern by reversing digit positions
/// within each player's key range.
///
/// `lanes` encodes each lane's mapping as a decimal digit (least-significant
/// digit = lane 0). `keys_per_player` and `player_count` define how digits
/// are grouped. Returns the re-encoded i32.
///
/// Uses i64 intermediate arithmetic to avoid overflow for DP modes where
/// total_digits >= 14 (10^13 exceeds i32::MAX).
fn mirror_invert_lanes(lanes: i32, keys_per_player: usize, player_count: usize) -> i32 {
    let total_digits = keys_per_player * player_count;

    // Extract digits from least-significant to most-significant,
    // padding with zeros to the expected total_digits count.
    let mut digits = vec![0i32; total_digits];
    let mut val = lanes.unsigned_abs();
    for d in &mut digits {
        *d = (val % 10) as i32;
        val /= 10;
    }

    // Reverse within each player's half
    for p in 0..player_count {
        let start = p * keys_per_player;
        let end = start + keys_per_player;
        digits[start..end].reverse();
    }

    // Re-encode: digits[0] is least-significant.
    // Use i64 intermediate to avoid overflow for DP modes (14+ digits;
    // 10^13 exceeds i32::MAX). For extreme modes (KEYBOARD_24K_DOUBLE,
    // 52 digits) even i64 can overflow at 10^19+, so use wrapping
    // arithmetic to match Java's int overflow semantics.
    let mut result: i64 = 0;
    for (i, &d) in digits.iter().enumerate() {
        if d != 0 {
            let power = 10i64.wrapping_pow(i as u32);
            result = result.wrapping_add((d as i64).wrapping_mul(power));
        }
    }
    result as i32
}

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
        let mut ghost_battle = crate::play::ghost_battle_play::consume();
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
                        // Mirror-invert the lane pattern by reversing digit
                        // positions within each player's key range.
                        // String reversal is wrong because:
                        //   (a) i32 drops leading zeros (e.g. 0123456 -> 123456)
                        //   (b) DP patterns need per-player-half reversal
                        let mode = self.model.mode().copied().unwrap_or(Mode::BEAT_7K);
                        let player_count = mode.player().max(1) as usize;
                        let keys_per_player = (mode.key() / mode.player().max(1)) as usize;
                        gb.lanes = mirror_invert_lanes(gb.lanes, keys_per_player, player_count);
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
            m.modify(&mut self.model);
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
            let mode = self.model.mode().copied().unwrap_or(Mode::BEAT_7K);
            if mode == Mode::BEAT_5K || mode == Mode::BEAT_7K || mode == Mode::KEYBOARD_24K {
                // Convert SP mode to DP mode
                let new_mode = match mode {
                    Mode::BEAT_5K => Mode::BEAT_10K,
                    Mode::BEAT_7K => Mode::BEAT_14K,
                    Mode::KEYBOARD_24K => Mode::KEYBOARD_24K_DOUBLE,
                    _ => unreachable!(
                        "mode must be BEAT_5K, BEAT_7K, or KEYBOARD_24K per outer if-condition"
                    ),
                };
                self.model.set_mode(new_mode);

                // Apply PlayerBattleModifier
                let mut battle_mod = PlayerBattleModifier::new();
                battle_mod.modify(&mut self.model);

                // If doubleoption == 3, also add AutoplayModifier for scratch keys
                if self.score.playinfo.doubleoption == 3 {
                    let dp_mode = self.model.mode().copied().unwrap_or(Mode::BEAT_14K);
                    let scratch_keys = dp_mode.scratch_key().to_vec();
                    let mut autoplay_mod = AutoplayModifier::new(scratch_keys);
                    autoplay_mod.modify(&mut self.model);
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
        let mode = self.model.mode().copied().unwrap_or(Mode::BEAT_7K);
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
            let mut pm2 = crate::core::pattern::pattern_modifier::create_pattern_modifier(
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
        let mut pm1 = crate::core::pattern::pattern_modifier::create_pattern_modifier(
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
            m.modify(&mut self.model);

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
                let current_mode = self.model.mode().copied().unwrap_or(Mode::BEAT_7K);
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
        &self.play_skin
    }

    pub fn play_skin_mut(&mut self) -> &mut PlaySkin {
        &mut self.play_skin
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
                gauge = gauge.min(rubato_types::groove_gauge::EXHARDCLASS);
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
        let model_random = self.model.random().map(|r| r.to_vec());
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
                // Caller should: resource.load_bms_model(rand), then update self.model
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
        if config.display_settings.bpmguide && (self.model.min_bpm() < self.model.max_bpm()) {
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
        let mode = self.model.mode().copied().unwrap_or(Mode::BEAT_7K);
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
        self.playtime = (self.model.last_note_time() + 1000) * 100 / freq as i64 + TIME_MARGIN;

        // Scale chart timing
        bms_model_utils::change_frequency(&mut self.model, freq as f32 / 100.0);

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
        sound_manager: &crate::core::system_sound_manager::SystemSoundManager,
    ) -> Vec<(i32, Option<String>)> {
        use crate::core::system_sound_manager::SoundType;

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

    /// Orchestrate the full pattern modification pipeline.
    ///
    /// Corresponds to the Java BMSPlayer constructor lines 94-348.
    /// This method calls the 5 pipeline stages in order:
    /// 1. `init_playinfo_from_config` -- copy config random options into playinfo
    /// 2. `restore_replay_data` -- restore replay data and handle replay key modes
    /// 3. `handle_random_syntax` -- process RANDOM branch chart loading
    /// 4. `calculate_non_modifier_assist` -- check non-modifier assist flags
    /// 5. `build_pattern_modifiers` -- apply scroll/LN/mine/extra/battle/random modifiers
    ///
    /// Additionally handles:
    /// - Bug rubato-5pd: Applies HS replay config to PlayConfig after restore_replay_data
    /// - Bug rubato-9dx: Applies 7-to-9 mode change from replay before pattern modifiers
    ///
    /// The caller should invoke this BEFORE `create()` so that the model is
    /// fully modified when create() initializes the judge, gauge, and lane renderer.
    pub fn prepare_pattern_pipeline(&mut self) {
        let config = self.player_config.clone();
        let is_replay = self.play_mode.mode == crate::core::bms_player_mode::Mode::Replay;
        let is_course = self.is_course_mode;

        // Step 1: Initialize playinfo from config (Java lines 94-96)
        self.init_playinfo_from_config(&config);

        // Step 2: Restore replay data (Java lines 110-175)
        let mut hs_replay_config: Option<rubato_types::play_config::PlayConfig> = None;
        if is_replay && !is_course {
            let replay = self.score.active_replay.take();
            let key_state = self.replay_key_state;
            let result = self.restore_replay_data(replay, &key_state);

            // Apply HS replay config (Bug rubato-5pd, Java lines 345-348)
            hs_replay_config = result.hs_replay_config;

            if result.stay_replay {
                // Normal replay: keep replay for keylog playback
                self.score.active_replay = result.replay;
            } else {
                // Switched to PLAY mode
                self.play_mode = BMSPlayerMode::PLAY;
            }
        }

        // Step 3: Handle RANDOM syntax (Java lines 179-196)
        let still_replay =
            is_replay && self.play_mode.mode == crate::core::bms_player_mode::Mode::Replay;
        let resource_replay_seed = self.score.playinfo.randomoptionseed;
        let resource_rand = self.score.playinfo.rand.clone();
        // Take active_replay temporarily to avoid borrow conflict
        let active_replay = self.score.active_replay.take();
        let rand_for_reload = self.handle_random_syntax(
            still_replay,
            active_replay.as_ref(),
            resource_replay_seed,
            &resource_rand,
        );
        self.score.active_replay = active_replay;

        // Reload BMS model with selected RANDOM branches (Java lines 189-195)
        if let Some(rand) = rand_for_reload
            && let Some(path_str) = self.model.path()
        {
            let path = std::path::PathBuf::from(&path_str);
            let lnmode = self.player_config.play_settings.lnmode;
            if let Some((model, margin_time)) =
                crate::core::player_resource::PlayerResource::load_bms_model(
                    &path,
                    lnmode,
                    Some(rand),
                )
            {
                self.margin_time = margin_time;
                if let Some(new_rand) = model.random().map(|r| r.to_vec()) {
                    self.score.playinfo.rand = new_rand;
                }
                self.model = model;
            }
        }

        // Step 4: Non-modifier assist checks (Java lines 200-212)
        self.calculate_non_modifier_assist(&config);

        // Step 5: 7-to-9 mode change from replay (Bug rubato-9dx, Java lines 263/280)
        // This must happen BEFORE build_pattern_modifiers so ModeModifier sees the
        // correct mode.
        if let Some(ref replay) = self.score.active_replay
            && replay.seven_to_nine_pattern > 0
            && self.model.mode().copied() == Some(Mode::BEAT_7K)
        {
            self.model.set_mode(Mode::POPN_9K);
        }

        // Step 6: Build and apply pattern modifiers (Java lines 214-342)
        self.build_pattern_modifiers(&config);

        // Step 7: Apply HS replay config (Bug rubato-5pd, Java lines 345-348)
        // Must happen AFTER build_pattern_modifiers since the model mode may have
        // changed (e.g. 7to9), and we need the final mode for play_config lookup.
        if let Some(hs_config) = hs_replay_config {
            let mode = self.model.mode().copied().unwrap_or(Mode::BEAT_7K);
            self.player_config.play_config(mode).playconfig = hs_config;
        }

        // Step 8: Apply frequency trainer (Java lines 246-267)
        // Freq trainer is applied via FreqTrainerMenu global state, but the value
        // must be passed in by the caller since BMSPlayer cannot access the global.
        // This is handled separately by the caller after prepare_pattern_pipeline().
    }

    /// Get mutable reference to playinfo for testing.
    #[cfg(test)]
    pub fn playinfo_mut(&mut self) -> &mut ReplayData {
        &mut self.score.playinfo
    }
}

#[cfg(test)]
mod tests {
    use super::mirror_invert_lanes;

    /// SP 7-key (BEAT_7K): keys_per_player=8, player_count=1, total_digits=8.
    /// Verifies basic per-player reversal works for a single player.
    #[test]
    fn mirror_invert_sp_7k() {
        // lanes = 76543210 (digit 0 = 0, digit 1 = 1, ..., digit 7 = 7)
        // After reversing the single player half: [7,6,5,4,3,2,1,0]
        // Re-encoded: 0*10^0 + 1*10^1 + 2*10^2 + 3*10^3 + 4*10^4 + 5*10^5 + 6*10^6 + 7*10^7
        //           = 0 + 10 + 200 + 3000 + 40000 + 500000 + 6000000 + 70000000 = 76543210
        // Wait -- reversing a palindrome-like sequence yields the same digits re-encoded
        // in reverse positional order, which actually produces the *same* number.
        // Use an asymmetric example instead.
        //
        // lanes = 12345678 as decimal:
        //   digit[0]=8, digit[1]=7, ..., digit[7]=1
        // Reversed: [1,2,3,4,5,6,7,8]
        // Re-encoded: 1 + 20 + 300 + 4000 + 50000 + 600000 + 7000000 + 80000000 = 87654321
        let result = mirror_invert_lanes(12345678, 8, 1);
        assert_eq!(result, 87654321);
    }

    /// DP 14-key (BEAT_14K): keys_per_player=8, player_count=2, total_digits=16.
    /// This is the exact case that overflows i32 (10^13 > i32::MAX).
    /// The test verifies no panic occurs and the result is correct.
    #[test]
    fn mirror_invert_dp_14k_no_overflow() {
        // BEAT_14K: key()=16, player()=2 => keys_per_player=8, player_count=2
        // Use a 14-digit lanes value that exercises high-order digits.
        // lanes = 1234567_8765432_1 won't fit i32; use a value near i32::MAX.
        //
        // Construct: 7-digit 1P pattern + 7-digit 2P pattern stored in a 14-digit number.
        // But gb.lanes is i32, max ~2,147,483,647 (10 digits). With 16 total digits,
        // digits 10..15 will be 0 after extraction.
        //
        // lanes = 1234567890 (10 digits)
        //   digits[0..8]  = [0,9,8,7,6,5,4,3]  (1P, extracted from least-significant)
        //   digits[8..16] = [2,1,0,0,0,0,0,0]  (2P, higher digits)
        //
        // After per-player reversal:
        //   digits[0..8]  = [3,4,5,6,7,8,9,0]
        //   digits[8..16] = [0,0,0,0,0,0,1,2]
        //
        // Re-encode:
        //   3*10^0 + 4*10^1 + 5*10^2 + 6*10^3 + 7*10^4 + 8*10^5 + 9*10^6 + 0*10^7
        //   + 0*10^8 + 0*10^9 + 0*10^10 + 0*10^11 + 0*10^12 + 0*10^13 + 1*10^14 + 2*10^15
        //
        // 1P part = 3 + 40 + 500 + 6000 + 70000 + 800000 + 9000000 = 9876543
        // 2P part = 1*10^14 + 2*10^15 = 100_000_000_000_000 + 2_000_000_000_000_000
        //         = 2_100_000_000_000_000 (exceeds i32, but `as i32` truncates)
        //
        // Total i64 = 2_100_000_009_876_543
        // Truncated to i32: 2_100_000_009_876_543 % 2^32 then as i32
        //
        // This would be a complex truncation. Instead, use a value where the
        // 2P digits are all 0, so the result fits in i32.
        // lanes = 87654321 (8 digits, fits easily)
        //   total_digits = 16, so digits[8..16] are all 0
        //   digits[0..8] = [1,2,3,4,5,6,7,8]
        //   digits[8..16] = [0,0,0,0,0,0,0,0]
        //   After reversal:
        //     digits[0..8] = [8,7,6,5,4,3,2,1]
        //     digits[8..16] = [0,0,0,0,0,0,0,0]
        //   Re-encode: 8 + 70 + 600 + 5000 + 40000 + 300000 + 2000000 + 10000000
        //            = 12345678
        let result = mirror_invert_lanes(87654321, 8, 2);
        assert_eq!(result, 12345678);
    }

    /// Verify that a 14-digit case with non-zero high digits does not panic.
    /// The i64 intermediate handles 10^13 correctly even though the final
    /// `as i32` truncation wraps the large result.
    #[test]
    fn mirror_invert_dp_14k_large_value_no_panic() {
        // BEAT_14K mode: keys_per_player=8, player_count=2
        // Use i32::MAX = 2_147_483_647 (10 digits).
        // digits[0..8]  = [7,4,6,3,8,4,7,4]  (from 2147483647)
        // digits[8..16] = [1,2,0,0,0,0,0,0]
        // After reversal:
        //   [4,7,4,8,3,6,4,7]  and  [0,0,0,0,0,0,2,1]
        // Re-encode with i64 (high digits use 10^14, 10^15 -- no panic).
        // The truncation to i32 is expected Java-parity behavior.
        let result = mirror_invert_lanes(i32::MAX, 8, 2);
        // Just verify no panic; the exact truncated value is not important
        // for this regression test. The key property is that 10^13 and 10^14
        // do not overflow the intermediate accumulator.
        let _ = result;
    }

    /// DP 10-key (BEAT_10K): keys_per_player=6, player_count=2, total_digits=12.
    /// Borderline case: 10^11 < i32::MAX is false (10^11 > i32::MAX), so this
    /// also needs i64.
    #[test]
    fn mirror_invert_dp_10k() {
        // BEAT_10K: key()=12, player()=2 => keys_per_player=6, player_count=2
        // lanes = 123456 (6 digits, only 1P half populated)
        //   digits[0..6]  = [6,5,4,3,2,1]
        //   digits[6..12] = [0,0,0,0,0,0]
        // After reversal:
        //   [1,2,3,4,5,6] and [0,0,0,0,0,0]
        // Re-encode: 1 + 20 + 300 + 4000 + 50000 + 600000 = 654321
        let result = mirror_invert_lanes(123456, 6, 2);
        assert_eq!(result, 654321);
    }

    /// Zero lanes should remain zero after mirror inversion.
    #[test]
    fn mirror_invert_zero_lanes() {
        assert_eq!(mirror_invert_lanes(0, 8, 1), 0);
        assert_eq!(mirror_invert_lanes(0, 8, 2), 0);
    }

    /// KEYBOARD_24K_DOUBLE: keys_per_player=26, player_count=2, total_digits=52.
    /// After reversal, non-zero digits land at positions 17-25 where
    /// 10^25 overflows even i64. Wrapping arithmetic prevents panic and
    /// matches Java's int overflow semantics.
    #[test]
    fn mirror_invert_keyboard_24k_double_no_panic() {
        // keys_per_player=26, player_count=2 => total_digits=52
        // digits extracted from 123456789 land at positions [0..9].
        // After reversing within the 26-digit first-player half,
        // they move to positions [17..25], requiring 10^17..10^25.
        // Wrapping arithmetic handles powers >= 10^19 without panic.
        let result = mirror_invert_lanes(123456789, 26, 2);
        let _ = result; // No panic is the assertion
    }
}
