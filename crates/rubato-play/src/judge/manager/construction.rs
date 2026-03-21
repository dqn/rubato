use super::*;

impl JudgeManager {
    pub fn new() -> Self {
        JudgeManager {
            lntype: LnType::LongNote,
            score: ScoreData::default(),
            combo: 0,
            coursecombo: 0,
            coursemaxcombo: 0,
            judge: Vec::new(),
            judgenow: Vec::new(),
            judgecombo: Vec::new(),
            ghost: Vec::new(),
            judgefast: Vec::new(),
            mjudgefast: Vec::new(),
            keyassign: Vec::new(),
            sckey: Vec::new(),
            nmjudge: Vec::new(),
            mjudgestart: 0,
            mjudgeend: 0,
            cnendmjudge: Vec::new(),
            nreleasemargin: 0,
            smjudge: Vec::new(),
            scnendmjudge: Vec::new(),
            sreleasemargin: 0,
            combocond: Vec::new(),
            miss: MissCondition::One,
            judge_vanish: Vec::new(),
            prevmtime: 0,
            autoplay: false,
            auto_presstime: Vec::new(),
            auto_minduration: 80_000,
            algorithm: JudgeAlgorithm::Combo,
            recent_judges: vec![i64::MIN; 100],
            micro_recent_judges: vec![i64::MIN; 100],
            recent_judges_index: 0,
            auto_adjust_enabled: false,
            is_play_or_practice: false,
            judgetiming_delta: 0,
            lane_states: Vec::new(),
            note_states: Vec::new(),
            multi_bad: MultiBadCollector::new(),
            lane_count: 0,
            judged_lanes: Vec::new(),
            judged_events: Vec::new(),
            judged_visual_events: Vec::new(),
            keysound_play_indices: Vec::new(),
            keysound_volume_set_indices: Vec::new(),
        }
    }

    /// Create a JudgeManager from a JudgeConfig (testable API).
    pub fn from_config(config: &JudgeConfig) -> Self {
        let lp = config
            .lane_property
            .cloned()
            .unwrap_or_else(|| LaneProperty::new(config.mode));

        let lane_count = config.mode.key() as usize;
        let player_count = config.mode.player().max(1) as usize;
        debug_assert!(
            lane_count.is_multiple_of(player_count),
            "lane_count ({lane_count}) must be divisible by player_count ({player_count})"
        );
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

        let judgeregion = config.judgeregion.max(1) as usize;
        let mut jm = JudgeManager {
            lntype: config.ln_type,
            score: ScoreData::default(),
            combo: 0,
            coursecombo: 0,
            coursemaxcombo: 0,
            judge: vec![vec![0; keys_per_player + 1]; player_count],
            judgenow: vec![0; judgeregion],
            judgecombo: vec![0; judgeregion],
            ghost: vec![JUDGE_PR; total_notes],
            judgefast: vec![0; judgeregion],
            mjudgefast: vec![0; judgeregion],
            keyassign: keyassign_vec,
            sckey: vec![0; scratch_count],
            nmjudge,
            mjudgestart,
            mjudgeend,
            cnendmjudge,
            nreleasemargin: config.judge_property.longnote_margin,
            smjudge,
            scnendmjudge,
            sreleasemargin: config.judge_property.longscratch_margin,
            combocond: config.judge_property.combo.clone(),
            miss: config.judge_property.miss,
            judge_vanish: config.judge_property.judge_vanish.clone(),
            prevmtime: 0,
            autoplay: config.autoplay,
            auto_presstime: vec![i64::MIN; num_keys],
            auto_minduration: 80_000,
            algorithm: config.algorithm,
            recent_judges: vec![i64::MIN; 100],
            micro_recent_judges: vec![i64::MIN; 100],
            recent_judges_index: 0,
            auto_adjust_enabled: config.auto_adjust_enabled,
            is_play_or_practice: config.is_play_or_practice,
            judgetiming_delta: 0,
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
            judged_lanes: Vec::new(),
            judged_events: Vec::new(),
            judged_visual_events: Vec::new(),
            keysound_play_indices: Vec::new(),
            keysound_volume_set_indices: Vec::new(),
        };
        jm.score.notes = total_notes as i32;

        // Populate play_option fields so scores record which algorithm/rule was used.
        // Mirrors what init() does in accessors.rs.
        jm.score.play_option.judge_algorithm = Some(match config.algorithm {
            JudgeAlgorithm::Combo => rubato_types::judge_algorithm::JudgeAlgorithm::Combo,
            JudgeAlgorithm::Duration => rubato_types::judge_algorithm::JudgeAlgorithm::Duration,
            JudgeAlgorithm::Lowest => rubato_types::judge_algorithm::JudgeAlgorithm::Lowest,
            JudgeAlgorithm::Score => rubato_types::judge_algorithm::JudgeAlgorithm::Score,
        });
        jm.score.play_option.rule = Some(rubato_types::bms_player_rule::BMSPlayerRule::LR2);

        jm
    }
}
