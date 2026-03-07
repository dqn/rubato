use super::*;

impl JudgeManager {
    // --- Legacy API (backward compat) ---

    pub fn init(
        &mut self,
        model: &BMSModel,
        judgeregion: i32,
        player_config: Option<&PlayerConfig>,
        constraints: &[CourseDataConstraint],
    ) {
        self.prevmtime = 0;
        self.scoring.judgenow = vec![0; judgeregion as usize];
        self.scoring.judgecombo = vec![0; judgeregion as usize];
        self.scoring.judgefast = vec![0; judgeregion as usize];
        self.scoring.mjudgefast = vec![0; judgeregion as usize];

        let orgmode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
        self.scoring.score = ScoreData::default();
        self.scoring.score.notes = model.total_notes();
        self.scoring.score.play_option.judge_algorithm = Some(match self.algorithm {
            JudgeAlgorithm::Combo => rubato_types::judge_algorithm::JudgeAlgorithm::Combo,
            JudgeAlgorithm::Duration => rubato_types::judge_algorithm::JudgeAlgorithm::Duration,
            JudgeAlgorithm::Lowest => rubato_types::judge_algorithm::JudgeAlgorithm::Lowest,
            JudgeAlgorithm::Score => rubato_types::judge_algorithm::JudgeAlgorithm::Timing,
        });
        // BMSPlayerRule::get_bms_player_rule always returns the LR2 ruleset in the current
        // implementation (bms_player_rule_set_lr2). Map to the types-level enum accordingly.
        let _ = BMSPlayerRule::for_mode(&orgmode);
        self.scoring.score.play_option.rule =
            Some(rubato_types::bms_player_rule::BMSPlayerRule::LR2);

        self.scoring.ghost = vec![4; model.total_notes() as usize];
        self.lntype = model.lntype();

        let rule = BMSPlayerRule::for_mode(&orgmode);
        let judgerank = model.judgerank();

        let mut key_judge_window_rate = if let Some(config) = player_config {
            if config.is_custom_judge() {
                [
                    config.judge_settings.key_judge_window_rate_perfect_great,
                    config.judge_settings.key_judge_window_rate_great,
                    config.judge_settings.key_judge_window_rate_good,
                ]
            } else {
                [100, 100, 100]
            }
        } else {
            [100, 100, 100]
        };
        let mut scratch_judge_window_rate = if let Some(config) = player_config {
            if config.is_custom_judge() {
                [
                    config
                        .judge_settings
                        .scratch_judge_window_rate_perfect_great,
                    config.judge_settings.scratch_judge_window_rate_great,
                    config.judge_settings.scratch_judge_window_rate_good,
                ]
            } else {
                [100, 100, 100]
            }
        } else {
            [100, 100, 100]
        };

        for con in constraints {
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

        self.combocond = rule.judge.combo.clone();
        self.miss = rule.judge.miss;
        self.judge_vanish = rule.judge.judge_vanish.clone();

        self.windows.nmjudge = rule
            .judge
            .judge(NoteType::Note, judgerank, &key_judge_window_rate);
        self.windows.cnendmjudge =
            rule.judge
                .judge(NoteType::LongnoteEnd, judgerank, &key_judge_window_rate);
        self.windows.nreleasemargin = rule.judge.longnote_margin;
        self.windows.smjudge = rule
            .judge
            .judge(NoteType::Scratch, judgerank, &scratch_judge_window_rate);
        self.windows.scnendmjudge = rule.judge.judge(
            NoteType::LongscratchEnd,
            judgerank,
            &scratch_judge_window_rate,
        );
        self.windows.sreleasemargin = rule.judge.longscratch_margin;

        self.windows.mjudgestart = 0;
        self.windows.mjudgeend = 0;
        for l in &self.windows.nmjudge {
            self.windows.mjudgestart = self.windows.mjudgestart.min(l[0]);
            self.windows.mjudgeend = self.windows.mjudgeend.max(l[1]);
        }
        for l in &self.windows.smjudge {
            self.windows.mjudgestart = self.windows.mjudgestart.min(l[0]);
            self.windows.mjudgeend = self.windows.mjudgeend.max(l[1]);
        }

        let player_count = orgmode.player();
        let keys_per_player = orgmode.key() / player_count;
        self.scoring.judge =
            vec![vec![0; keys_per_player as usize + 1]; player_count as usize];

        self.auto_adjust.recent_judges = vec![i64::MIN; 100];
        self.auto_adjust.micro_recent_judges = vec![i64::MIN; 100];
        self.auto_adjust.recent_judges_index = 0;
        self.auto_adjust.presses_since_last_autoadjust = 0;
        self.auto_adjust.judgetiming_delta = 0;
    }

    // --- Getters ---

    pub fn score(&self) -> &ScoreData {
        &self.scoring.score
    }

    pub fn max_combo(&self) -> i32 {
        self.scoring.score.maxcombo
    }

    pub fn ghost_as_usize(&self) -> Vec<usize> {
        self.scoring.ghost.iter().map(|&g| g as usize).collect()
    }

    pub fn past_notes(&self) -> i32 {
        self.scoring.score.passnotes
    }

    /// Returns the accumulated judge timing delta from auto-adjust.
    /// The caller should apply this to PlayerConfig.judgetiming and then call
    /// `take_judgetiming_delta()` to consume it.
    pub fn judgetiming_delta(&self) -> i32 {
        self.auto_adjust.judgetiming_delta
    }

    /// Consumes and resets the accumulated judge timing delta.
    pub fn take_judgetiming_delta(&mut self) -> i32 {
        let delta = self.auto_adjust.judgetiming_delta;
        self.auto_adjust.judgetiming_delta = 0;
        delta
    }

    pub fn recent_judges(&self) -> &[i64] {
        &self.auto_adjust.recent_judges
    }

    pub fn micro_recent_judges(&self) -> &[i64] {
        &self.auto_adjust.micro_recent_judges
    }

    pub fn recent_judges_index(&self) -> usize {
        self.auto_adjust.recent_judges_index
    }

    pub fn recent_judge_timing(&self, player: usize) -> i64 {
        if player < self.scoring.judgefast.len() {
            self.scoring.judgefast[player]
        } else {
            0
        }
    }

    pub fn recent_judge_micro_timing(&self, player: usize) -> i64 {
        if player < self.scoring.mjudgefast.len() {
            self.scoring.mjudgefast[player]
        } else {
            0
        }
    }

    pub fn processing_long_note(&self, lane: usize) -> Option<usize> {
        if lane < self.lane_states.len() {
            self.lane_states[lane].processing
        } else {
            None
        }
    }

    pub fn passing_long_note(&self, lane: usize) -> Option<usize> {
        if lane < self.lane_states.len() {
            self.lane_states[lane].passing
        } else {
            None
        }
    }

    pub fn hell_charge_judge(&self, lane: usize) -> bool {
        if lane < self.lane_states.len() {
            self.lane_states[lane].inclease
        } else {
            false
        }
    }

    pub fn auto_presstime(&self) -> &[i64] {
        &self.auto_presstime
    }

    pub fn combo(&self) -> i32 {
        self.scoring.combo
    }

    pub fn course_combo(&self) -> i32 {
        self.scoring.coursecombo
    }

    pub fn course_maxcombo(&self) -> i32 {
        self.scoring.coursemaxcombo
    }

    pub fn judge_time_region(&self, lane: usize) -> &[[i64; 2]] {
        if lane < self.lane_states.len() && self.lane_states[lane].sckey >= 0 {
            &self.windows.smjudge
        } else {
            &self.windows.nmjudge
        }
    }

    pub fn score_data(&self) -> &ScoreData {
        &self.scoring.score
    }

    /// Get mutable reference to score data (for testing).
    #[cfg(test)]
    pub fn score_data_mut(&mut self) -> &mut ScoreData {
        &mut self.scoring.score
    }

    pub fn judge_count(&self, judge: i32) -> i32 {
        self.scoring.score.judge_count_total(judge)
    }

    pub fn judge_count_fast(&self, judge: i32, fast: bool) -> i32 {
        self.scoring.score.judge_count(judge, fast)
    }

    pub fn now_judge(&self, player: usize) -> i32 {
        if player < self.scoring.judgenow.len() {
            self.scoring.judgenow[player]
        } else {
            0
        }
    }

    pub fn now_combo(&self, player: usize) -> i32 {
        if player < self.scoring.judgecombo.len() {
            self.scoring.judgecombo[player]
        } else {
            0
        }
    }

    pub fn judge_table(&self, sc: bool) -> &[[i64; 2]] {
        if sc {
            &self.windows.smjudge
        } else {
            &self.windows.nmjudge
        }
    }

    pub fn ghost(&self) -> &[i32] {
        &self.scoring.ghost
    }

    /// Test-only: set judge_vanish for testing bounds checking.
    #[cfg(test)]
    pub fn set_judge_vanish_for_test(&mut self, vanish: Vec<bool>) {
        self.judge_vanish = vanish;
    }

    /// Test-only: get judge_vanish reference.
    #[cfg(test)]
    pub fn judge_vanish_ref(&self) -> &[bool] {
        &self.judge_vanish
    }
}
