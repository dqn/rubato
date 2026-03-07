use super::*;

impl JudgeManager {
    pub fn new() -> Self {
        JudgeManager {
            lntype: LnType::LongNote,
            scoring: ScoreAccumulator::default(),
            windows: JudgeWindows::default(),
            auto_adjust: AutoAdjustState::default(),
            keyassign: Vec::new(),
            sckey: Vec::new(),
            combocond: Vec::new(),
            miss: MissCondition::One,
            judge_vanish: Vec::new(),
            prevmtime: 0,
            autoplay: false,
            auto_presstime: Vec::new(),
            auto_minduration: 80,
            algorithm: JudgeAlgorithm::Combo,
            lane_states: Vec::new(),
            note_states: Vec::new(),
            multi_bad: MultiBadCollector::new(),
            lane_count: 0,
        }
    }

    /// Create a JudgeManager from a JudgeConfig (testable API).
    pub fn from_config(config: &JudgeConfig) -> Self {
        let lp = config
            .lane_property
            .cloned()
            .unwrap_or_else(|| LaneProperty::new(config.mode));

        let lane_count = config.mode.key() as usize;
        let player_count = config.mode.player() as usize;
        let keys_per_player = lane_count / player_count;

        // Build judge windows
        let nmjudge = config.judge_property.judge(
            NoteType::Note,
            config.judge_rank,
            &config.judge_window_rate,
        );
        let cnendmjudge = config.judge_property.judge(
            NoteType::LongnoteEnd,
            config.judge_rank,
            &config.judge_window_rate,
        );
        let smjudge = config.judge_property.judge(
            NoteType::Scratch,
            config.judge_rank,
            &config.scratch_judge_window_rate,
        );
        let scnendmjudge = config.judge_property.judge(
            NoteType::LongscratchEnd,
            config.judge_rank,
            &config.scratch_judge_window_rate,
        );

        let mut mjudgestart: i64 = 0;
        let mut mjudgeend: i64 = 0;
        for l in &nmjudge {
            mjudgestart = mjudgestart.min(l[0]);
            mjudgeend = mjudgeend.max(l[1]);
        }
        for l in &smjudge {
            mjudgestart = mjudgestart.min(l[0]);
            mjudgeend = mjudgeend.max(l[1]);
        }

        // Build per-lane note index lists
        let mut lane_note_indices: Vec<Vec<usize>> = vec![Vec::new(); lane_count];
        for (i, note) in config.notes.iter().enumerate() {
            if note.lane < lane_count {
                lane_note_indices[note.lane].push(i);
            }
        }

        // Build LaneIterState for each lane
        let lane_key_assign = lp.lane_key_assign();
        let lane_scratch = lp.lane_scratch_assign();
        let lane_skin_offset = lp.lane_skin_offset();
        let lane_player = lp.lane_player();
        let mut lane_states = Vec::with_capacity(lane_count);
        for lane in 0..lane_count {
            let laneassign = if lane < lane_key_assign.len() {
                lane_key_assign[lane].iter().map(|&k| k as usize).collect()
            } else {
                vec![lane]
            };
            lane_states.push(LaneIterState {
                _lane: lane,
                player: if lane < lane_player.len() {
                    lane_player[lane] as usize
                } else {
                    0
                },
                offset: if lane < lane_skin_offset.len() {
                    lane_skin_offset[lane] as usize
                } else {
                    lane
                },
                sckey: if lane < lane_scratch.len() {
                    lane_scratch[lane]
                } else {
                    -1
                },
                laneassign,
                note_indices: lane_note_indices[lane].clone(),
                base_pos: 0,
                seek_pos: 0,
                processing: None,
                passing: None,
                inclease: false,
                mpassingcount: 0,
                lnstart_judge: 0,
                lnstart_duration: 0,
                releasetime: i64::MIN,
                lnend_judge: i32::MIN,
            });
        }

        // Count total playable notes for ghost array.
        // Mirrors Java TimeLine.getTotalNotes(lntype): for LNTYPE_LONGNOTE, LN end notes
        // with TYPE_UNDEFINED are not independently counted (only the LN start counts).
        let total_notes = config
            .notes
            .iter()
            .filter(|n| {
                if n.is_long_end()
                    && n.ln_type == TYPE_UNDEFINED
                    && config.ln_type == LNTYPE_LONGNOTE
                {
                    return false;
                }
                n.is_playable()
            })
            .count();

        let keyassign_vec: Vec<i32> = lp.key_lane_assign().to_vec();
        let num_keys = keyassign_vec.len();

        // Scratch key count
        let scratch_count = lp.scratch_key_assign().len();

        let mut jm = JudgeManager {
            lntype: config.ln_type,
            scoring: ScoreAccumulator {
                score: ScoreData::default(),
                combo: 0,
                coursecombo: 0,
                coursemaxcombo: 0,
                judge: vec![vec![0; keys_per_player + 1]; player_count],
                judgenow: vec![0; 1],
                judgecombo: vec![0; 1],
                ghost: vec![JUDGE_PR; total_notes],
                judgefast: vec![0; 1],
                mjudgefast: vec![0; 1],
            },
            windows: JudgeWindows {
                nmjudge,
                mjudgestart,
                mjudgeend,
                cnendmjudge,
                nreleasemargin: config.judge_property.longnote_margin,
                smjudge,
                scnendmjudge,
                sreleasemargin: config.judge_property.longscratch_margin,
            },
            auto_adjust: AutoAdjustState {
                recent_judges: vec![i64::MIN; 100],
                micro_recent_judges: vec![i64::MIN; 100],
                recent_judges_index: 0,
                presses_since_last_autoadjust: 0,
                auto_adjust_enabled: config.auto_adjust_enabled,
                is_play_or_practice: config.is_play_or_practice,
                judgetiming_delta: 0,
            },
            keyassign: keyassign_vec,
            sckey: vec![0; scratch_count],
            combocond: config.judge_property.combo.clone(),
            miss: config.judge_property.miss,
            judge_vanish: config.judge_property.judge_vanish.clone(),
            prevmtime: 0,
            autoplay: config.autoplay,
            auto_presstime: vec![i64::MIN; num_keys],
            auto_minduration: 80,
            algorithm: config.algorithm,
            lane_states,
            note_states: vec![
                NoteJudgeState {
                    state: 0,
                    play_time: 0,
                };
                config.notes.len()
            ],
            multi_bad: MultiBadCollector::new(),
            lane_count,
        };
        jm.scoring.score.notes = total_notes as i32;
        jm
    }
}
