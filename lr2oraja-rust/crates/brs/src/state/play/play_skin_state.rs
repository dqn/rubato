// Play-specific skin state synchronization.
//
// Updates SharedGameState integers/booleans with play-specific values
// (gauge, score, combo, BPM, judge counts, hispeed, duration, time,
// score comparison, gauge range, rank, offsets, etc.) each frame.

use bms_config::play_config::PlayConfig;
use bms_database::score_data_property::ScoreDataProperty;
use bms_model::LaneProperty;
use bms_rule::GrooveGauge;
use bms_rule::judge_manager::JudgeManager;
use bms_skin::property_id::{
    NUMBER_BAD, NUMBER_COMBO, NUMBER_EARLY_BAD, NUMBER_EARLY_GOOD, NUMBER_EARLY_GREAT,
    NUMBER_EARLY_MISS, NUMBER_EARLY_PERFECT, NUMBER_EARLY_POOR, NUMBER_GOOD, NUMBER_GREAT,
    NUMBER_GROOVEGAUGE, NUMBER_GROOVEGAUGE_AFTERDOT, NUMBER_LATE_BAD, NUMBER_LATE_GOOD,
    NUMBER_LATE_GREAT, NUMBER_LATE_MISS, NUMBER_LATE_PERFECT, NUMBER_LATE_POOR, NUMBER_MAXCOMBO2,
    NUMBER_MISS, NUMBER_NOWBPM, NUMBER_PERFECT, NUMBER_POOR, NUMBER_SCORE2, NUMBER_TOTALNOTES2,
};
use bms_skin::skin_object::SkinOffset;

use super::PlayPhase;
use crate::game_state::SharedGameState;

// ---------------------------------------------------------------------------
// Scratch angle state (ported from Java KeyInputProccessor.java lines 99-137)
// ---------------------------------------------------------------------------

/// Tracks per-scratch turntable angle and animation speed.
///
/// Faithfully ports the Java scratch angle algorithm:
/// - CW key: targetSpeed = -0.75, moveTowardsSpeed = 16.0, clamp speed <= 0
/// - CCW key: targetSpeed = 2.0, moveTowardsSpeed = 16.0, clamp speed >= 0
/// - No input: targetSpeed = 1.0, moveTowardsSpeed = 4.0
/// - Smooth interpolation toward target, then apply rotation at 270 deg/s.
pub struct ScratchAngleState {
    /// Per-scratch current angle (0.0-360.0 degrees).
    angles: Vec<f32>,
    /// Per-scratch graphic speed.
    speeds: Vec<f32>,
    /// Previous frame time in milliseconds (-1 = not yet set).
    prev_time_ms: i64,
}

impl ScratchAngleState {
    pub fn new(scratch_count: usize) -> Self {
        Self {
            angles: vec![0.0; scratch_count],
            speeds: vec![0.0; scratch_count],
            prev_time_ms: -1,
        }
    }

    /// Update scratch angles based on key states and delta time.
    ///
    /// # Arguments
    /// * `now_ms` - Current frame time in milliseconds
    /// * `lane_property` - Lane/key mapping
    /// * `key_states` - Per-physical-key pressed state
    /// * `auto_presstime` - Per-physical-key autoplay press time (NOT_SET = not pressed)
    /// * `is_autoplay` - Whether in full autoplay mode
    pub fn update(
        &mut self,
        now_ms: i64,
        lane_property: &LaneProperty,
        key_states: &[bool],
        auto_presstime: &[i64],
        is_autoplay: bool,
    ) {
        if self.prev_time_ms < 0 {
            self.prev_time_ms = now_ms;
            return;
        }

        let delta_s = (now_ms - self.prev_time_ms) as f32 / 1000.0;
        self.prev_time_ms = now_ms;

        for s in 0..self.angles.len() {
            let [key_cw, key_ccw] = lane_property.scratch_keys(s);

            let mut target_speed = 1.0f32;
            let mut move_towards_speed = 4.0f32;

            if !is_autoplay {
                let cw_pressed = key_states.get(key_ccw).copied().unwrap_or(false)
                    || auto_presstime.get(key_ccw).copied().unwrap_or(i64::MIN) != i64::MIN;
                let ccw_pressed = key_states.get(key_cw).copied().unwrap_or(false)
                    || auto_presstime.get(key_cw).copied().unwrap_or(i64::MIN) != i64::MIN;

                if cw_pressed {
                    target_speed = -0.75;
                    move_towards_speed = 16.0;
                    self.speeds[s] = self.speeds[s].min(0.0);
                } else if ccw_pressed {
                    target_speed = 2.0;
                    move_towards_speed = 16.0;
                    self.speeds[s] = self.speeds[s].max(0.0);
                }
            }

            // Move towards target speed
            let diff = target_speed - self.speeds[s];
            if diff.abs() <= delta_s * move_towards_speed {
                self.speeds[s] = target_speed;
            } else {
                self.speeds[s] += diff.signum() * delta_s * move_towards_speed;
            }

            // Apply rotation
            if self.speeds[s] > 0.0 {
                self.angles[s] += 360.0 - self.speeds[s] * delta_s * 270.0;
            } else if self.speeds[s] < 0.0 {
                self.angles[s] += -self.speeds[s] * delta_s * 270.0;
            }

            self.angles[s] %= 360.0;
            if self.angles[s] < 0.0 {
                self.angles[s] += 360.0;
            }
        }
    }

    /// Get the angle for a scratch controller.
    pub fn angle(&self, scratch_idx: usize) -> f32 {
        self.angles.get(scratch_idx).copied().unwrap_or(0.0)
    }
}

/// Synchronize play-specific state into SharedGameState for skin rendering.
///
/// Called once per frame during the Playing phase.
pub fn sync_play_state(
    state: &mut SharedGameState,
    jm: &JudgeManager,
    gauge: &GrooveGauge,
    current_bpm: i32,
) {
    let score = jm.score();

    // Gauge value (integer part and fractional part)
    let gauge_val = gauge.value();
    state.integers.insert(NUMBER_GROOVEGAUGE, gauge_val as i32);
    state.integers.insert(
        NUMBER_GROOVEGAUGE_AFTERDOT,
        ((gauge_val % 1.0) * 100.0) as i32,
    );

    // Score
    state.integers.insert(NUMBER_SCORE2, score.exscore());

    // Combo
    state.integers.insert(NUMBER_COMBO, jm.combo());
    state.integers.insert(NUMBER_MAXCOMBO2, jm.max_combo());

    // Total notes
    state.integers.insert(NUMBER_TOTALNOTES2, score.notes);

    // BPM
    state.integers.insert(NUMBER_NOWBPM, current_bpm);

    // Judge counts (total)
    state
        .integers
        .insert(NUMBER_PERFECT, score.judge_count(bms_rule::JUDGE_PG));
    state
        .integers
        .insert(NUMBER_GREAT, score.judge_count(bms_rule::JUDGE_GR));
    state
        .integers
        .insert(NUMBER_GOOD, score.judge_count(bms_rule::JUDGE_GD));
    state
        .integers
        .insert(NUMBER_BAD, score.judge_count(bms_rule::JUDGE_BD));
    state
        .integers
        .insert(NUMBER_POOR, score.judge_count(bms_rule::JUDGE_PR));
    state
        .integers
        .insert(NUMBER_MISS, score.judge_count(bms_rule::JUDGE_MS));

    // Judge counts (early/late)
    state.integers.insert(
        NUMBER_EARLY_PERFECT,
        score.judge_count_early(bms_rule::JUDGE_PG),
    );
    state.integers.insert(
        NUMBER_LATE_PERFECT,
        score.judge_count_late(bms_rule::JUDGE_PG),
    );
    state.integers.insert(
        NUMBER_EARLY_GREAT,
        score.judge_count_early(bms_rule::JUDGE_GR),
    );
    state.integers.insert(
        NUMBER_LATE_GREAT,
        score.judge_count_late(bms_rule::JUDGE_GR),
    );
    state.integers.insert(
        NUMBER_EARLY_GOOD,
        score.judge_count_early(bms_rule::JUDGE_GD),
    );
    state
        .integers
        .insert(NUMBER_LATE_GOOD, score.judge_count_late(bms_rule::JUDGE_GD));
    state.integers.insert(
        NUMBER_EARLY_BAD,
        score.judge_count_early(bms_rule::JUDGE_BD),
    );
    state
        .integers
        .insert(NUMBER_LATE_BAD, score.judge_count_late(bms_rule::JUDGE_BD));
    state.integers.insert(
        NUMBER_EARLY_POOR,
        score.judge_count_early(bms_rule::JUDGE_PR),
    );
    state
        .integers
        .insert(NUMBER_LATE_POOR, score.judge_count_late(bms_rule::JUDGE_PR));
    state.integers.insert(
        NUMBER_EARLY_MISS,
        score.judge_count_early(bms_rule::JUDGE_MS),
    );
    state
        .integers
        .insert(NUMBER_LATE_MISS, score.judge_count_late(bms_rule::JUDGE_MS));

    // Gauge type float (for skin gauge rendering)
    state
        .floats
        .insert(bms_skin::property_id::FLOAT_GROOVEGAUGE_1P, gauge_val);
}

/// Synchronize play option booleans into SharedGameState.
pub fn sync_play_options(
    state: &mut SharedGameState,
    is_autoplay: bool,
    gauge_type: i32,
    bga_on: bool,
) {
    use bms_skin::property_id::{
        OPTION_AUTOPLAYOFF, OPTION_AUTOPLAYON, OPTION_BGAOFF, OPTION_BGAON, OPTION_GAUGE_EX,
        OPTION_GAUGE_GROOVE, OPTION_GAUGE_HARD,
    };

    // Autoplay flags
    state.booleans.insert(OPTION_AUTOPLAYON, is_autoplay);
    state.booleans.insert(OPTION_AUTOPLAYOFF, !is_autoplay);

    // Gauge type flags (mutually exclusive)
    // gauge_type: 0=AssistEasy, 1=Easy, 2=Normal, 3=Hard, 4=ExHard, etc.
    state.booleans.insert(OPTION_GAUGE_GROOVE, gauge_type <= 2);
    state.booleans.insert(OPTION_GAUGE_HARD, gauge_type == 3);
    state.booleans.insert(OPTION_GAUGE_EX, gauge_type >= 4);

    // BGA flags
    state.booleans.insert(OPTION_BGAON, bga_on);
    state.booleans.insert(OPTION_BGAOFF, !bga_on);
}

// ---------------------------------------------------------------------------
// 23-2: Hispeed / Duration / Lanecover sync
// ---------------------------------------------------------------------------

/// Calculate note fall duration in milliseconds.
///
/// `cover` is the combined lanecover + lift ratio (0.0-1.0) when enabled.
fn calc_duration(hispeed: f32, bpm: f64, cover: f64) -> i32 {
    if hispeed <= 0.0 || bpm <= 0.0 {
        return 0;
    }
    (0.5 + (240_000.0 / (hispeed as f64 * bpm)) * (1.0 - cover)) as i32
}

/// Calculate green number (LR2-style duration display).
fn calc_green(duration: i32, bpm: f64) -> i32 {
    (0.5 + duration as f64 * bpm / 240.0) as i32
}

/// Synchronize hispeed, duration, and lanecover values.
pub fn sync_play_hispeed_duration(
    state: &mut SharedGameState,
    play_config: &PlayConfig,
    now_bpm: f64,
    main_bpm: f64,
    min_bpm: f64,
    max_bpm: f64,
) {
    use bms_skin::property_id::*;

    let hs = play_config.hispeed;

    // Hispeed values
    state.integers.insert(NUMBER_HISPEED, hs as i32);
    state
        .integers
        .insert(NUMBER_HISPEED_LR2, (hs * 100.0) as i32);
    state
        .integers
        .insert(NUMBER_HISPEED_AFTERDOT, ((hs * 100.0) as i32) % 100);
    state.floats.insert(FLOAT_HISPEED, hs);

    // Lanecover / Lift / Hidden integer values (0-1000 scale)
    state
        .integers
        .insert(NUMBER_LANECOVER1, (play_config.lanecover * 1000.0) as i32);
    state
        .integers
        .insert(NUMBER_LIFT1, (play_config.lift * 1000.0) as i32);
    state
        .integers
        .insert(NUMBER_HIDDEN1, (play_config.hidden * 1000.0) as i32);
    // LANECOVER2 is for 2P; mirror 1P for now
    state
        .integers
        .insert(NUMBER_LANECOVER2, (play_config.lanecover * 1000.0) as i32);

    // Lanecover rate (0.0-1.0)
    state.floats.insert(RATE_LANECOVER, play_config.lanecover);
    state.floats.insert(RATE_LANECOVER2, play_config.lanecover);

    // Cover ratio for duration calculation
    let cover_on = {
        let mut c = 0.0f64;
        if play_config.enablelanecover {
            c += play_config.lanecover as f64;
        }
        if play_config.enablelift {
            c += play_config.lift as f64;
        }
        c
    };
    let cover_off = 0.0;

    // Base duration at current BPM (no cover)
    let dur_base = calc_duration(hs, now_bpm, 0.0);
    let green_base = calc_green(dur_base, now_bpm);
    state.integers.insert(NUMBER_DURATION, dur_base);
    state.integers.insert(NUMBER_DURATION_GREEN, green_base);

    // 16 duration variants: [now, main, min, max] x [cover_on, green_on, cover_off, green_off]
    let bpms = [now_bpm, main_bpm, min_bpm, max_bpm];
    let base_ids = [
        NUMBER_DURATION_LANECOVER_ON,         // 1312
        NUMBER_MAINBPM_DURATION_LANECOVER_ON, // 1316
        NUMBER_MINBPM_DURATION_LANECOVER_ON,  // 1320
        NUMBER_MAXBPM_DURATION_LANECOVER_ON,  // 1324
    ];

    for (i, &bpm) in bpms.iter().enumerate() {
        let base = base_ids[i];
        let dur_on = calc_duration(hs, bpm, cover_on);
        let green_on = calc_green(dur_on, bpm);
        let dur_off = calc_duration(hs, bpm, cover_off);
        let green_off = calc_green(dur_off, bpm);

        state.integers.insert(base, dur_on); // _LANECOVER_ON
        state.integers.insert(base + 1, green_on); // _GREEN_LANECOVER_ON
        state.integers.insert(base + 2, dur_off); // _LANECOVER_OFF
        state.integers.insert(base + 3, green_off); // _GREEN_LANECOVER_OFF
    }

    // Music progress as slider rate
    state
        .floats
        .insert(RATE_MUSIC_PROGRESS, play_config.lanecover);
}

// ---------------------------------------------------------------------------
// 23-3: Play time / Music progress sync
// ---------------------------------------------------------------------------

/// Synchronize play time, time left, song length, and music progress.
pub fn sync_play_time(state: &mut SharedGameState, play_elapsed_us: i64, total_time_us: i64) {
    use bms_skin::property_id::*;

    let elapsed_s = (play_elapsed_us / 1_000_000).max(0);
    let total_s = (total_time_us / 1_000_000).max(0);
    let remaining_s = (total_s - elapsed_s).max(0);

    // Play time (elapsed)
    state
        .integers
        .insert(NUMBER_PLAYTIME_MINUTE, (elapsed_s / 60) as i32);
    state
        .integers
        .insert(NUMBER_PLAYTIME_SECOND, (elapsed_s % 60) as i32);

    // Time left
    state
        .integers
        .insert(NUMBER_TIMELEFT_MINUTE, (remaining_s / 60) as i32);
    state
        .integers
        .insert(NUMBER_TIMELEFT_SECOND, (remaining_s % 60) as i32);

    // Song total length
    state
        .integers
        .insert(NUMBER_SONGLENGTH_MINUTE, (total_s / 60) as i32);
    state
        .integers
        .insert(NUMBER_SONGLENGTH_SECOND, (total_s % 60) as i32);

    // Music progress rate (0.0-1.0)
    let progress = if total_time_us > 0 {
        (play_elapsed_us as f32 / total_time_us as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    state.floats.insert(RATE_MUSIC_PROGRESS, progress);
    state.floats.insert(RATE_MUSIC_PROGRESS_BAR, progress);
}

// ---------------------------------------------------------------------------
// 23-4: Score comparison / Score rate sync
// ---------------------------------------------------------------------------

/// Synchronize score comparison and score rate values.
pub fn sync_play_score_comparison(
    state: &mut SharedGameState,
    sdp: &ScoreDataProperty,
    jm: &JudgeManager,
) {
    use bms_skin::property_id::*;

    let exscore = jm.score().exscore();

    // Score rate (integer + fractional parts)
    state.integers.insert(NUMBER_SCORE_RATE, sdp.now_rate_int());
    state
        .integers
        .insert(NUMBER_SCORE_RATE_AFTERDOT, sdp.now_rate_after_dot());

    // Total rate (final rate based on all notes)
    state.integers.insert(NUMBER_TOTAL_RATE, sdp.rate_int());
    state
        .integers
        .insert(NUMBER_TOTAL_RATE_AFTERDOT, sdp.rate_after_dot());

    // Float versions
    state.floats.insert(FLOAT_SCORE_RATE, sdp.now_rate());
    state.floats.insert(FLOAT_TOTAL_RATE, sdp.rate());

    // High score
    state.integers.insert(NUMBER_HIGHSCORE, sdp.best_score());

    // Target (rival) score
    state
        .integers
        .insert(NUMBER_TARGET_SCORE, sdp.rival_score());

    // Score differences
    state
        .integers
        .insert(NUMBER_DIFF_HIGHSCORE, exscore - sdp.now_best_score());
    state
        .integers
        .insert(NUMBER_DIFF_EXSCORE, exscore - sdp.now_best_score());
    state
        .integers
        .insert(NUMBER_DIFF_TARGETSCORE, exscore - sdp.now_rival_score());
    state.integers.insert(NUMBER_DIFF_NEXTRANK, sdp.next_rank());
}

// ---------------------------------------------------------------------------
// 23-5: Gauge range / Realtime rank / Extended booleans
// ---------------------------------------------------------------------------

/// Synchronize gauge range booleans (10% increments).
pub fn sync_play_gauge_range(state: &mut SharedGameState, gauge: &GrooveGauge) {
    use bms_skin::property_id::*;

    let gauge_val = gauge.value();
    let gauge_pct = gauge_val as i32; // 0-100

    // OPTION_1P_0_9 (230) through OPTION_1P_90_99 (239)
    // Each is true if gauge falls within that 10% range
    let range_ids = [
        OPTION_1P_0_9,   // 230: 0-9
        OPTION_1P_10_19, // 231: 10-19
        OPTION_1P_20_29, // 232: 20-29
        OPTION_1P_30_39, // 233: 30-39
        OPTION_1P_40_49, // 234: 40-49
        OPTION_1P_50_59, // 235: 50-59
        OPTION_1P_60_69, // 236: 60-69
        OPTION_1P_70_79, // 237: 70-79
        OPTION_1P_80_89, // 238: 80-89
        OPTION_1P_90_99, // 239: 90-99
    ];

    for (i, &id) in range_ids.iter().enumerate() {
        let lo = (i * 10) as i32;
        let hi = lo + 9;
        state
            .booleans
            .insert(id, gauge_pct >= lo && gauge_pct <= hi);
    }

    // OPTION_1P_100: gauge == 100
    state.booleans.insert(OPTION_1P_100, gauge_val >= 100.0);

    // OPTION_1P_BORDER_OR_MORE: gauge is at or above clear border
    state
        .booleans
        .insert(OPTION_1P_BORDER_OR_MORE, gauge.is_qualified());
}

/// Synchronize realtime rank booleans.
///
/// Rank thresholds based on exscore rate (27-division system):
/// AAA = 24/27, AA = 21/27, A = 18/27, B = 15/27, C = 12/27, D = 9/27, E = 6/27, F = below E
pub fn sync_play_realtime_rank(state: &mut SharedGameState, sdp: &ScoreDataProperty) {
    use bms_skin::property_id::*;

    // nowrank indices: AAA=24, AA=21, A=18, B=15, C=12, D=9, E=6
    let rank_ids = [
        (OPTION_NOW_AAA_1P, 24),
        (OPTION_NOW_AA_1P, 21),
        (OPTION_NOW_A_1P, 18),
        (OPTION_NOW_B_1P, 15),
        (OPTION_NOW_C_1P, 12),
        (OPTION_NOW_D_1P, 9),
        (OPTION_NOW_E_1P, 6),
    ];

    // Determine current rank: highest qualifying rank is active, others false
    let mut current_rank_id = OPTION_NOW_F_1P;
    for &(id, threshold) in &rank_ids {
        if sdp.qualify_now_rank(threshold) {
            current_rank_id = id;
            break;
        }
    }

    for &(id, _) in &rank_ids {
        state.booleans.insert(id, id == current_rank_id);
    }
    state
        .booleans
        .insert(OPTION_NOW_F_1P, current_rank_id == OPTION_NOW_F_1P);
}

/// Synchronize extended option booleans (loading, replay, lanecover state).
pub fn sync_play_extended_options(
    state: &mut SharedGameState,
    phase: PlayPhase,
    is_replay: bool,
    play_config: &PlayConfig,
) {
    use bms_skin::property_id::*;

    // Loading state
    state
        .booleans
        .insert(OPTION_NOW_LOADING, phase == PlayPhase::Preload);
    state
        .booleans
        .insert(OPTION_LOADED, phase != PlayPhase::Preload);

    // Replay state
    state.booleans.insert(OPTION_REPLAY_OFF, !is_replay);
    state.booleans.insert(OPTION_REPLAY_PLAYING, is_replay);

    // Practice mode (not implemented yet)
    state.booleans.insert(OPTION_STATE_PRACTICE, false);

    // Lanecover state flags
    // OPTION_LANECOVER1_CHANGING is set externally when user is adjusting
    // (stub: always false for now since we don't track lanecover adjustment state)
    state.booleans.insert(OPTION_LANECOVER1_CHANGING, false);
    state
        .booleans
        .insert(OPTION_LANECOVER1_ON, play_config.enablelanecover);
    state
        .booleans
        .insert(OPTION_LIFT1_ON, play_config.enablelift);
    state
        .booleans
        .insert(OPTION_HIDDEN1_ON, play_config.enablehidden);
}

// ---------------------------------------------------------------------------
// 23-6: Offsets / VALUE_JUDGE sync
// ---------------------------------------------------------------------------

/// Synchronize skin offsets for lanecover, lift, hidden, and scratch angle.
pub fn sync_play_offsets(
    state: &mut SharedGameState,
    play_config: &PlayConfig,
    scratch: &ScratchAngleState,
) {
    use bms_skin::property_id::*;

    // OFFSET_LANECOVER: y = lanecover value (0.0-1.0 range, used by skin as y-offset)
    state.offsets.insert(
        OFFSET_LANECOVER,
        SkinOffset {
            y: play_config.lanecover,
            ..Default::default()
        },
    );

    // OFFSET_LIFT: y = lift value
    state.offsets.insert(
        OFFSET_LIFT,
        SkinOffset {
            y: play_config.lift,
            ..Default::default()
        },
    );

    // OFFSET_HIDDEN_COVER: y = hidden value
    state.offsets.insert(
        OFFSET_HIDDEN_COVER,
        SkinOffset {
            y: play_config.hidden,
            ..Default::default()
        },
    );

    // OFFSET_SCRATCHANGLE_1P/2P: rotation angle in degrees
    // Java: main.getOffset(OFFSET_SCRATCHANGLE_1P + s).r = scratch[s];
    state.offsets.insert(
        OFFSET_SCRATCHANGLE_1P,
        SkinOffset {
            r: scratch.angle(0),
            ..Default::default()
        },
    );
    state.offsets.insert(
        OFFSET_SCRATCHANGLE_2P,
        SkinOffset {
            r: scratch.angle(1),
            ..Default::default()
        },
    );
}

/// Synchronize per-key and overall judge values.
pub fn sync_play_judge_per_key(
    state: &mut SharedGameState,
    jm: &JudgeManager,
    lane_property: &bms_model::LaneProperty,
) {
    use bms_skin::property_id::*;

    // Overall latest judge for 1P
    let now_judge = jm.now_judge(0) as i32;
    state.integers.insert(VALUE_JUDGE_1P, now_judge);
    // 2P: use player 1 if available, else 0
    state
        .integers
        .insert(VALUE_JUDGE_2P, jm.now_judge(1) as i32);

    // Per-key judge: use per-lane judge values from JudgeManager
    // Map lane -> skin offset, then set VALUE_JUDGE_1P_SCRATCH + offset
    for lane in 0..lane_property.lane_count() {
        let offset = lane_property.lane_skin_offset(lane);
        if (0..10).contains(&offset) {
            state
                .integers
                .insert(VALUE_JUDGE_1P_SCRATCH + offset, jm.lane_judge(lane));
        }
    }

    // VALUE_JUDGE_1P_DURATION: timing difference of latest judge (from recent_judges)
    let recent = jm.recent_judges();
    let idx = jm.recent_judges_index();
    let duration = if !recent.is_empty() {
        recent[idx.wrapping_sub(1) % recent.len()]
    } else {
        0
    };
    // Convert microseconds to milliseconds for skin
    state
        .integers
        .insert(VALUE_JUDGE_1P_DURATION, (duration / 1000) as i32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::{LaneProperty, PlayMode};
    use bms_rule::gauge_property::GaugeType;
    use bms_rule::judge_manager::JudgeConfig;
    use bms_rule::{JudgeAlgorithm, PlayerRule};

    fn make_judge_manager(autoplay: bool) -> (JudgeManager, Vec<bms_model::Note>, GrooveGauge) {
        // Create a minimal set of notes for testing
        let notes = vec![bms_model::Note {
            lane: 1,
            note_type: bms_model::NoteType::Normal,
            time_us: 1_000_000,
            end_time_us: 0,
            end_wav_id: 0,
            wav_id: 1,
            damage: 0,
            pair_index: usize::MAX,
            micro_starttime: 0,
            micro_duration: 0,
        }];

        let rule = PlayerRule::lr2();
        let gauge = GrooveGauge::new(&rule.gauge, GaugeType::Normal, 300.0, 1);

        let lp = LaneProperty::new(PlayMode::Beat7K);
        let config = JudgeConfig {
            notes: &notes,
            play_mode: PlayMode::Beat7K,
            ln_type: bms_model::LnType::LongNote,
            judge_rank: 100,
            judge_window_rate: [100, 100, 100],
            scratch_judge_window_rate: [100, 100, 100],
            algorithm: JudgeAlgorithm::Combo,
            autoplay,
            judge_property: &rule.judge,
            lane_property: Some(&lp),
        };
        let jm = JudgeManager::new(&config);
        (jm, notes, gauge)
    }

    #[test]
    fn sync_populates_gauge_value() {
        let (jm, _notes, gauge) = make_judge_manager(true);
        let mut state = SharedGameState::default();

        sync_play_state(&mut state, &jm, &gauge, 150);

        // Normal gauge starts at 20
        assert_eq!(*state.integers.get(&NUMBER_GROOVEGAUGE).unwrap(), 20);
    }

    #[test]
    fn sync_populates_bpm() {
        let (jm, _notes, gauge) = make_judge_manager(true);
        let mut state = SharedGameState::default();

        sync_play_state(&mut state, &jm, &gauge, 175);

        assert_eq!(*state.integers.get(&NUMBER_NOWBPM).unwrap(), 175);
    }

    #[test]
    fn sync_populates_judge_counts() {
        let (jm, _notes, gauge) = make_judge_manager(true);
        let mut state = SharedGameState::default();

        sync_play_state(&mut state, &jm, &gauge, 150);

        // Initially all zeros
        assert_eq!(*state.integers.get(&NUMBER_PERFECT).unwrap(), 0);
        assert_eq!(*state.integers.get(&NUMBER_GREAT).unwrap(), 0);
    }

    #[test]
    fn sync_populates_score() {
        let (jm, _notes, gauge) = make_judge_manager(true);
        let mut state = SharedGameState::default();

        sync_play_state(&mut state, &jm, &gauge, 150);

        assert_eq!(*state.integers.get(&NUMBER_SCORE2).unwrap(), 0);
        assert_eq!(*state.integers.get(&NUMBER_COMBO).unwrap(), 0);
    }

    #[test]
    fn sync_populates_gauge_float() {
        let (jm, _notes, gauge) = make_judge_manager(true);
        let mut state = SharedGameState::default();

        sync_play_state(&mut state, &jm, &gauge, 150);

        let val = *state
            .floats
            .get(&bms_skin::property_id::FLOAT_GROOVEGAUGE_1P)
            .unwrap();
        assert!((val - 20.0).abs() < 1e-6);
    }

    #[test]
    fn sync_play_options_autoplay_on() {
        let mut state = SharedGameState::default();
        sync_play_options(&mut state, true, 2, true);
        assert!(
            *state
                .booleans
                .get(&bms_skin::property_id::OPTION_AUTOPLAYON)
                .unwrap()
        );
        assert!(
            !*state
                .booleans
                .get(&bms_skin::property_id::OPTION_AUTOPLAYOFF)
                .unwrap()
        );
    }

    #[test]
    fn sync_play_options_gauge_hard() {
        let mut state = SharedGameState::default();
        sync_play_options(&mut state, false, 3, true);
        assert!(
            !*state
                .booleans
                .get(&bms_skin::property_id::OPTION_GAUGE_GROOVE)
                .unwrap()
        );
        assert!(
            *state
                .booleans
                .get(&bms_skin::property_id::OPTION_GAUGE_HARD)
                .unwrap()
        );
        assert!(
            !*state
                .booleans
                .get(&bms_skin::property_id::OPTION_GAUGE_EX)
                .unwrap()
        );
    }

    #[test]
    fn sync_play_options_bga_off() {
        let mut state = SharedGameState::default();
        sync_play_options(&mut state, false, 2, false);
        assert!(
            !*state
                .booleans
                .get(&bms_skin::property_id::OPTION_BGAON)
                .unwrap()
        );
        assert!(
            *state
                .booleans
                .get(&bms_skin::property_id::OPTION_BGAOFF)
                .unwrap()
        );
    }

    // --- 23-2: Hispeed / Duration tests ---

    #[test]
    fn sync_hispeed_duration_default() {
        let mut state = SharedGameState::default();
        let pc = PlayConfig::default(); // hispeed=1.0
        sync_play_hispeed_duration(&mut state, &pc, 120.0, 120.0, 120.0, 120.0);

        assert_eq!(
            *state
                .integers
                .get(&bms_skin::property_id::NUMBER_HISPEED)
                .unwrap(),
            1
        );
        assert!(
            (*state
                .floats
                .get(&bms_skin::property_id::FLOAT_HISPEED)
                .unwrap()
                - 1.0)
                .abs()
                < f32::EPSILON
        );
        // Duration at BPM=120, hs=1.0, no cover: 240000/(1.0*120) = 2000
        assert_eq!(
            *state
                .integers
                .get(&bms_skin::property_id::NUMBER_DURATION)
                .unwrap(),
            2000
        );
    }

    #[test]
    fn sync_duration_variants() {
        let mut state = SharedGameState::default();
        let pc = PlayConfig {
            hispeed: 1.0,
            lanecover: 0.2,
            enablelanecover: true,
            lift: 0.0,
            enablelift: false,
            ..Default::default()
        };
        sync_play_hispeed_duration(&mut state, &pc, 120.0, 120.0, 100.0, 150.0);

        // Duration at BPM=120, hs=1.0, cover=0.2: 240000/(1*120)*(1-0.2) = 1600
        let dur_on = *state
            .integers
            .get(&bms_skin::property_id::NUMBER_DURATION_LANECOVER_ON)
            .unwrap();
        assert_eq!(dur_on, 1600);

        // Duration without cover: 240000/(1*120) = 2000
        let dur_off = *state
            .integers
            .get(&bms_skin::property_id::NUMBER_DURATION_LANECOVER_OFF)
            .unwrap();
        assert_eq!(dur_off, 2000);

        // Green number at BPM=120, duration=1600: round(1600*120/240) = 800
        let green_on = *state
            .integers
            .get(&bms_skin::property_id::NUMBER_DURATION_GREEN_LANECOVER_ON)
            .unwrap();
        assert_eq!(green_on, 800);
    }

    // --- 23-3: Play time tests ---

    #[test]
    fn sync_play_time_basic() {
        let mut state = SharedGameState::default();
        // 65 seconds elapsed, 180 seconds total
        sync_play_time(&mut state, 65_000_000, 180_000_000);

        assert_eq!(
            *state
                .integers
                .get(&bms_skin::property_id::NUMBER_PLAYTIME_MINUTE)
                .unwrap(),
            1
        ); // 65/60
        assert_eq!(
            *state
                .integers
                .get(&bms_skin::property_id::NUMBER_PLAYTIME_SECOND)
                .unwrap(),
            5
        ); // 65%60
        assert_eq!(
            *state
                .integers
                .get(&bms_skin::property_id::NUMBER_TIMELEFT_MINUTE)
                .unwrap(),
            1
        ); // 115/60
        assert_eq!(
            *state
                .integers
                .get(&bms_skin::property_id::NUMBER_TIMELEFT_SECOND)
                .unwrap(),
            55
        ); // 115%60
    }

    #[test]
    fn sync_music_progress() {
        let mut state = SharedGameState::default();
        sync_play_time(&mut state, 50_000_000, 100_000_000);

        let progress = *state
            .floats
            .get(&bms_skin::property_id::RATE_MUSIC_PROGRESS)
            .unwrap();
        assert!((progress - 0.5).abs() < 0.01);
    }

    // --- 23-4: Score comparison tests ---

    #[test]
    fn sync_score_comparison_basic() {
        let (jm, _notes, _gauge) = make_judge_manager(true);
        let mut state = SharedGameState::default();
        let mut sdp = ScoreDataProperty::new();
        sdp.set_target_score(100, None, 80, None, 1);

        sync_play_score_comparison(&mut state, &sdp, &jm);

        assert_eq!(
            *state
                .integers
                .get(&bms_skin::property_id::NUMBER_HIGHSCORE)
                .unwrap(),
            100
        );
        assert_eq!(
            *state
                .integers
                .get(&bms_skin::property_id::NUMBER_TARGET_SCORE)
                .unwrap(),
            80
        );
    }

    // --- 23-5: Gauge range / Rank tests ---

    #[test]
    fn sync_gauge_range_mid() {
        let rule = PlayerRule::lr2();
        let mut gauge = GrooveGauge::new(&rule.gauge, GaugeType::Normal, 300.0, 1);
        // Normal gauge starts at 20. Set to ~45
        gauge.add_value(25.0);
        let mut state = SharedGameState::default();

        sync_play_gauge_range(&mut state, &gauge);

        // gauge is ~45, so OPTION_1P_40_49 should be true
        assert!(
            *state
                .booleans
                .get(&bms_skin::property_id::OPTION_1P_40_49)
                .unwrap()
        );
        assert!(
            !*state
                .booleans
                .get(&bms_skin::property_id::OPTION_1P_50_59)
                .unwrap()
        );
    }

    #[test]
    fn sync_gauge_range_100() {
        let rule = PlayerRule::lr2();
        let mut gauge = GrooveGauge::new(&rule.gauge, GaugeType::Normal, 300.0, 1);
        gauge.add_value(80.0); // Should hit 100 cap
        let mut state = SharedGameState::default();

        sync_play_gauge_range(&mut state, &gauge);

        assert!(
            *state
                .booleans
                .get(&bms_skin::property_id::OPTION_1P_100)
                .unwrap()
        );
    }

    #[test]
    fn sync_realtime_rank_high_score() {
        let mut state = SharedGameState::default();
        let mut sdp = ScoreDataProperty::new();
        // All PG = 100% rate -> qualifies for AAA (24/27 = 88.9%)
        let score = bms_rule::ScoreData {
            mode: 7,
            epg: 100,
            lpg: 100,
            notes: 200,
            ..Default::default()
        };
        sdp.update(&score, 200);

        sync_play_realtime_rank(&mut state, &sdp);

        assert!(
            *state
                .booleans
                .get(&bms_skin::property_id::OPTION_NOW_AAA_1P)
                .unwrap()
        );
        assert!(
            !*state
                .booleans
                .get(&bms_skin::property_id::OPTION_NOW_F_1P)
                .unwrap()
        );
    }

    #[test]
    fn sync_extended_options_loading() {
        let mut state = SharedGameState::default();
        let pc = PlayConfig::default();

        sync_play_extended_options(&mut state, PlayPhase::Preload, false, &pc);

        assert!(
            *state
                .booleans
                .get(&bms_skin::property_id::OPTION_NOW_LOADING)
                .unwrap()
        );
        assert!(
            !*state
                .booleans
                .get(&bms_skin::property_id::OPTION_LOADED)
                .unwrap()
        );
    }

    #[test]
    fn sync_extended_options_playing() {
        let mut state = SharedGameState::default();
        let pc = PlayConfig {
            enablelanecover: true,
            enablelift: false,
            enablehidden: true,
            ..Default::default()
        };

        sync_play_extended_options(&mut state, PlayPhase::Playing, false, &pc);

        assert!(
            !*state
                .booleans
                .get(&bms_skin::property_id::OPTION_NOW_LOADING)
                .unwrap()
        );
        assert!(
            *state
                .booleans
                .get(&bms_skin::property_id::OPTION_LOADED)
                .unwrap()
        );
        assert!(
            *state
                .booleans
                .get(&bms_skin::property_id::OPTION_LANECOVER1_ON)
                .unwrap()
        );
        assert!(
            !*state
                .booleans
                .get(&bms_skin::property_id::OPTION_LIFT1_ON)
                .unwrap()
        );
        assert!(
            *state
                .booleans
                .get(&bms_skin::property_id::OPTION_HIDDEN1_ON)
                .unwrap()
        );
    }

    // --- 23-6: Offsets / Judge tests ---

    #[test]
    fn sync_offsets_lanecover() {
        let mut state = SharedGameState::default();
        let pc = PlayConfig {
            lanecover: 0.3,
            lift: 0.1,
            hidden: 0.05,
            ..Default::default()
        };
        let scratch = ScratchAngleState::new(1);

        sync_play_offsets(&mut state, &pc, &scratch);

        let lc = state
            .offsets
            .get(&bms_skin::property_id::OFFSET_LANECOVER)
            .unwrap();
        assert!((lc.y - 0.3).abs() < f32::EPSILON);

        let lift = state
            .offsets
            .get(&bms_skin::property_id::OFFSET_LIFT)
            .unwrap();
        assert!((lift.y - 0.1).abs() < f32::EPSILON);
    }

    // --- Scratch angle tests ---

    #[test]
    fn scratch_angle_cw_increases_angle() {
        let lp = LaneProperty::new(PlayMode::Beat7K);
        let mut sa = ScratchAngleState::new(lp.scratch_count());
        let [key_cw, _key_ccw] = lp.scratch_keys(0);

        let mut key_states = vec![false; lp.physical_key_count()];
        let auto_pt = vec![i64::MIN; lp.physical_key_count()];

        // First call sets prev_time
        sa.update(0, &lp, &key_states, &auto_pt, false);

        // Press CCW key (key_cw in scratch_keys[0] = second key index)
        key_states[key_cw] = true;
        sa.update(100, &lp, &key_states, &auto_pt, false);

        assert!(
            sa.angle(0) > 0.0,
            "Angle should increase, got {}",
            sa.angle(0)
        );
    }

    #[test]
    fn scratch_angle_ccw_changes_angle() {
        let lp = LaneProperty::new(PlayMode::Beat7K);
        let mut sa = ScratchAngleState::new(lp.scratch_count());
        let [_key_cw, key_ccw] = lp.scratch_keys(0);

        let mut key_states = vec![false; lp.physical_key_count()];
        let auto_pt = vec![i64::MIN; lp.physical_key_count()];

        sa.update(0, &lp, &key_states, &auto_pt, false);

        key_states[key_ccw] = true;
        sa.update(100, &lp, &key_states, &auto_pt, false);

        // CCW should also change angle (reversed direction)
        assert!(
            sa.angle(0) > 0.0,
            "Angle should change, got {}",
            sa.angle(0)
        );
    }

    #[test]
    fn scratch_angle_wraps_at_360() {
        let lp = LaneProperty::new(PlayMode::Beat7K);
        let mut sa = ScratchAngleState::new(lp.scratch_count());
        let key_states = vec![false; lp.physical_key_count()];
        let auto_pt = vec![i64::MIN; lp.physical_key_count()];

        sa.update(0, &lp, &key_states, &auto_pt, false);
        // Large time step to force wrapping
        sa.update(10_000, &lp, &key_states, &auto_pt, false);

        assert!(sa.angle(0) >= 0.0 && sa.angle(0) < 360.0);
    }

    #[test]
    fn scratch_angle_no_change_at_zero_delta() {
        let lp = LaneProperty::new(PlayMode::Beat7K);
        let mut sa = ScratchAngleState::new(lp.scratch_count());
        let key_states = vec![false; lp.physical_key_count()];
        let auto_pt = vec![i64::MIN; lp.physical_key_count()];

        sa.update(100, &lp, &key_states, &auto_pt, false);
        let angle_before = sa.angle(0);
        sa.update(100, &lp, &key_states, &auto_pt, false);

        assert!((sa.angle(0) - angle_before).abs() < f32::EPSILON);
    }

    #[test]
    fn sync_judge_per_key_populates() {
        let (jm, _notes, _gauge) = make_judge_manager(true);
        let lp = LaneProperty::new(PlayMode::Beat7K);
        let mut state = SharedGameState::default();

        sync_play_judge_per_key(&mut state, &jm, &lp);

        assert!(
            state
                .integers
                .contains_key(&bms_skin::property_id::VALUE_JUDGE_1P)
        );
        assert!(
            state
                .integers
                .contains_key(&bms_skin::property_id::VALUE_JUDGE_1P_SCRATCH)
        );
    }
}
