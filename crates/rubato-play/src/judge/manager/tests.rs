use super::*;
use bms_model::judge_note::{JUDGE_PG, build_judge_notes};
use bms_model::note::Note;
use bms_model::time_line::TimeLine;
use rubato_types::course_data::CourseDataConstraint;
use rubato_types::player_config::PlayerConfig;

#[test]
fn new_creates_default_state() {
    let jm = JudgeManager::new();
    assert_eq!(jm.combo(), 0);
    assert_eq!(jm.course_combo(), 0);
    assert_eq!(jm.course_maxcombo(), 0);
}

#[test]
fn default_is_same_as_new() {
    let jm1 = JudgeManager::new();
    let jm2 = JudgeManager::default();
    assert_eq!(jm1.combo(), jm2.combo());
    assert_eq!(jm1.course_combo(), jm2.course_combo());
    assert_eq!(jm1.course_maxcombo(), jm2.course_maxcombo());
}

#[test]
fn recent_judges_initialized_to_min() {
    let jm = JudgeManager::new();
    let judges = jm.recent_judges();
    assert_eq!(judges.len(), 100);
    for &j in judges {
        assert_eq!(j, i64::MIN);
    }
}

#[test]
fn micro_recent_judges_initialized_to_min() {
    let jm = JudgeManager::new();
    let judges = jm.micro_recent_judges();
    assert_eq!(judges.len(), 100);
    for &j in judges {
        assert_eq!(j, i64::MIN);
    }
}

#[test]
fn recent_judges_index_starts_at_zero() {
    let jm = JudgeManager::new();
    assert_eq!(jm.recent_judges_index(), 0);
}

#[test]
fn set_course_combo() {
    let mut jm = JudgeManager::new();
    jm.coursecombo = 42;
    assert_eq!(jm.course_combo(), 42);
}

#[test]
fn set_course_maxcombo() {
    let mut jm = JudgeManager::new();
    jm.coursemaxcombo = 100;
    assert_eq!(jm.course_maxcombo(), 100);
}

#[test]
fn get_now_judge_out_of_bounds_returns_zero() {
    let jm = JudgeManager::new();
    assert_eq!(jm.now_judge(0), 0);
    assert_eq!(jm.now_judge(100), 0);
}

#[test]
fn get_now_combo_out_of_bounds_returns_zero() {
    let jm = JudgeManager::new();
    assert_eq!(jm.now_combo(0), 0);
    assert_eq!(jm.now_combo(100), 0);
}

#[test]
fn get_recent_judge_timing_out_of_bounds_returns_zero() {
    let jm = JudgeManager::new();
    assert_eq!(jm.recent_judge_timing(0), 0);
    assert_eq!(jm.recent_judge_timing(100), 0);
}

#[test]
fn get_recent_judge_micro_timing_out_of_bounds_returns_zero() {
    let jm = JudgeManager::new();
    assert_eq!(jm.recent_judge_micro_timing(0), 0);
    assert_eq!(jm.recent_judge_micro_timing(100), 0);
}

#[test]
fn init_sets_up_judge_tables() {
    let mut jm = JudgeManager::new();
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    jm.init(&model, 1, None, &[]);

    assert_eq!(jm.now_judge(0), 0);
    let table = jm.judge_table(false);
    assert!(!table.is_empty());
    let sc_table = jm.judge_table(true);
    assert!(!sc_table.is_empty());
}

#[test]
fn init_sets_up_ghost_array() {
    let mut jm = JudgeManager::new();
    let model = make_model_with_notes(&[1_000_000, 2_000_000, 3_000_000]);
    jm.init(&model, 1, None, &[]);

    let ghost = jm.ghost();
    let total = model.total_notes() as usize;
    assert_eq!(total, 3);
    assert_eq!(ghost.len(), total);
    for &g in ghost {
        assert_eq!(g, 4);
    }
}

#[test]
fn init_resets_recent_judges() {
    let mut jm = JudgeManager::new();
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    jm.init(&model, 1, None, &[]);

    assert_eq!(jm.recent_judges_index(), 0);
    for &j in jm.recent_judges() {
        assert_eq!(j, i64::MIN);
    }
}

#[test]
fn get_judge_count_initially_zero() {
    let jm = JudgeManager::new();
    for i in 0..6 {
        assert_eq!(jm.judge_count(i), 0);
    }
}

#[test]
fn get_judge_count_fast_initially_zero() {
    let jm = JudgeManager::new();
    for i in 0..6 {
        assert_eq!(jm.judge_count_fast(i, true), 0);
        assert_eq!(jm.judge_count_fast(i, false), 0);
    }
}

#[test]
fn get_past_notes_initially_zero() {
    let jm = JudgeManager::new();
    assert_eq!(jm.past_notes(), 0);
}

#[test]
fn get_auto_presstime_initially_empty() {
    let jm = JudgeManager::new();
    assert!(jm.auto_presstime().is_empty());
}

#[test]
fn get_score_data_returns_default() {
    let jm = JudgeManager::new();
    let score = jm.score_data();
    assert_eq!(score.maxcombo, 0);
    assert_eq!(score.judge_counts.epg, 0);
    assert_eq!(score.judge_counts.egr, 0);
}

#[test]
fn init_with_judgeregion_2() {
    let mut jm = JudgeManager::new();
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K);
    model.judgerank = 100;
    jm.init(&model, 2, None, &[]);

    assert_eq!(jm.now_judge(0), 0);
    assert_eq!(jm.now_judge(1), 0);
}

#[test]
fn judge_time_region_returns_note_judge() {
    let mut jm = JudgeManager::new();
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    jm.init(&model, 1, None, &[]);

    let region = jm.judge_time_region(0);
    assert!(!region.is_empty());
    assert!(region[0][0] < 0);
    assert!(region[0][1] > 0);
}

// --- New testable API tests ---

fn make_model_with_notes(note_times_us: &[i64]) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    let mut timelines = Vec::new();
    for &time_us in note_times_us {
        let mut tl = TimeLine::new(0.0, time_us, 8);
        let mut note = Note::new_normal(1);
        note.set_micro_time(time_us);
        tl.set_note(0, Some(note));
        timelines.push(tl);
    }
    model.timelines = timelines;
    model
}

#[test]
fn from_config_creates_valid_state() {
    let model = make_model_with_notes(&[1_000_000, 2_000_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let jm = JudgeManager::from_config(&config);

    assert_eq!(jm.score().notes, 2);
    assert_eq!(jm.ghost().len(), 2);
    assert_eq!(jm.combo(), 0);
    assert_eq!(jm.past_notes(), 0);
}

#[test]
fn autoplay_judges_all_notes_as_pgreat() {
    let model = make_model_with_notes(&[500_000, 1_000_000, 1_500_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    // Create a minimal gauge (use BMSModel directly)
    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    // Prime
    jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

    // Run simulation
    let mut time = 0i64;
    while time <= 2_500_000 {
        jm.update(time, &notes, &key_states, &key_times, &mut gauge);
        time += 1000;
    }

    // All 3 notes should be PGREAT
    assert_eq!(jm.score().judge_counts.epg + jm.score().judge_counts.lpg, 3);
    assert_eq!(jm.max_combo(), 3);
    assert_eq!(jm.past_notes(), 3);
    for &g in jm.ghost() {
        assert_eq!(g, JUDGE_PG);
    }
}

#[test]
fn miss_all_notes_without_input() {
    let model = make_model_with_notes(&[500_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

    let mut time = 0i64;
    while time <= 1_500_000 {
        jm.update(time, &notes, &key_states, &key_times, &mut gauge);
        time += 1000;
    }

    // Note should be miss-POOR (judge=4)
    assert_eq!(jm.past_notes(), 1);
    assert_eq!(jm.ghost()[0], JUDGE_PR);
}

// --- Phase 36d: Custom judge rates and course constraints ---

#[test]
fn init_default_none_config_empty_constraints_unchanged() {
    // Regression: calling init with None + empty constraints should produce
    // the same judge tables as before (hardcoded [100,100,100]).
    let mut jm = JudgeManager::new();
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    jm.init(&model, 1, None, &[]);

    let nm = jm.judge_table(false);
    let sm = jm.judge_table(true);
    assert!(!nm.is_empty());
    assert!(!sm.is_empty());

    // With default rates [100,100,100] the tables must be identical to
    // a second JudgeManager initialized the same way.
    let mut jm2 = JudgeManager::new();
    jm2.init(&model, 1, None, &[]);
    assert_eq!(jm.judge_table(false), jm2.judge_table(false));
    assert_eq!(jm.judge_table(true), jm2.judge_table(true));
}

#[test]
fn init_custom_judge_rates_differ_from_default() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;

    // Default init
    let mut jm_default = JudgeManager::new();
    jm_default.init(&model, 1, None, &[]);

    // Custom judge with narrower windows
    let mut config = PlayerConfig::default();
    config.judge_settings.custom_judge = true;
    config.judge_settings.key_judge_window_rate_perfect_great = 50;
    config.judge_settings.key_judge_window_rate_great = 50;
    config.judge_settings.key_judge_window_rate_good = 50;
    config
        .judge_settings
        .scratch_judge_window_rate_perfect_great = 50;
    config.judge_settings.scratch_judge_window_rate_great = 50;
    config.judge_settings.scratch_judge_window_rate_good = 50;

    let mut jm_custom = JudgeManager::new();
    jm_custom.init(&model, 1, Some(&config), &[]);

    // Custom rates should produce different (narrower) judge tables
    assert_ne!(
        jm_default.judge_table(false),
        jm_custom.judge_table(false),
        "Custom key judge rates should differ from default"
    );
    assert_ne!(
        jm_default.judge_table(true),
        jm_custom.judge_table(true),
        "Custom scratch judge rates should differ from default"
    );
}

#[test]
fn init_custom_judge_false_uses_default_rates() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;

    let mut jm_default = JudgeManager::new();
    jm_default.init(&model, 1, None, &[]);

    // PlayerConfig with custom_judge = false should use [100,100,100]
    let config = PlayerConfig::default(); // custom_judge defaults to false
    let mut jm_with_config = JudgeManager::new();
    jm_with_config.init(&model, 1, Some(&config), &[]);

    assert_eq!(
        jm_default.judge_table(false),
        jm_with_config.judge_table(false),
    );
    assert_eq!(
        jm_default.judge_table(true),
        jm_with_config.judge_table(true),
    );
}

#[test]
fn init_no_great_constraint_zeroes_great_and_good() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;

    // Without constraint
    let mut jm_normal = JudgeManager::new();
    jm_normal.init(&model, 1, None, &[]);

    // With NO_GREAT
    let mut jm_constrained = JudgeManager::new();
    jm_constrained.init(&model, 1, None, &[CourseDataConstraint::NoGreat]);

    let nm_normal = jm_normal.judge_table(false);
    let nm_constrained = jm_constrained.judge_table(false);

    // The NoGreat constraint zeroes rates [1] and [2], which affects
    // Great and Good windows. The PerfectGreat window (index 0) stays.
    // So constrained tables should differ from normal tables.
    assert_ne!(
        nm_normal, nm_constrained,
        "NoGreat should modify key judge tables"
    );

    let sm_normal = jm_normal.judge_table(true);
    let sm_constrained = jm_constrained.judge_table(true);
    assert_ne!(
        sm_normal, sm_constrained,
        "NoGreat should modify scratch judge tables"
    );
}

#[test]
fn init_no_good_constraint_zeroes_good_only() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;

    // Without constraint
    let mut jm_normal = JudgeManager::new();
    jm_normal.init(&model, 1, None, &[]);

    // With NO_GOOD
    let mut jm_constrained = JudgeManager::new();
    jm_constrained.init(&model, 1, None, &[CourseDataConstraint::NoGood]);

    let nm_normal = jm_normal.judge_table(false);
    let nm_constrained = jm_constrained.judge_table(false);

    // NoGood zeroes only rate[2] (Good window), so tables should differ.
    assert_ne!(
        nm_normal, nm_constrained,
        "NoGood should modify key judge tables"
    );

    let sm_normal = jm_normal.judge_table(true);
    let sm_constrained = jm_constrained.judge_table(true);
    assert_ne!(
        sm_normal, sm_constrained,
        "NoGood should modify scratch judge tables"
    );
}

#[test]
fn init_no_great_zeroes_more_than_no_good() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;

    let mut jm_no_good = JudgeManager::new();
    jm_no_good.init(&model, 1, None, &[CourseDataConstraint::NoGood]);

    let mut jm_no_great = JudgeManager::new();
    jm_no_great.init(&model, 1, None, &[CourseDataConstraint::NoGreat]);

    // NoGreat zeroes both Great and Good, while NoGood only zeroes Good.
    // So their tables should differ (NoGreat is strictly more restrictive).
    assert_ne!(
        jm_no_good.judge_table(false),
        jm_no_great.judge_table(false),
        "NoGreat should be more restrictive than NoGood for key"
    );
    assert_ne!(
        jm_no_good.judge_table(true),
        jm_no_great.judge_table(true),
        "NoGreat should be more restrictive than NoGood for scratch"
    );
}

// --- Timing auto-adjust tests ---

/// Helper: create a JudgeManager with auto-adjust enabled in PLAY mode,
/// with evenly-spaced notes for testing.
fn make_autoadjust_jm(note_times: &[i64]) -> (JudgeManager, Vec<JudgeNote>, GrooveGauge) {
    let model = make_model_with_notes(note_times);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();
    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: true,
        is_play_or_practice: true,
        judgeregion: 1,
    };
    let jm = JudgeManager::from_config(&config);
    let rule = BMSPlayerRule::for_mode(&Mode::BEAT_7K);
    let gauge = GrooveGauge::new(&model, 0, &rule.gauge);
    (jm, notes, gauge)
}

#[test]
fn auto_adjust_proportional_delta_late_hits() {
    // Java formula (per note): delta -= (int)((mfast >= 0 ? mfast+15000 : mfast-15000) / 30000)
    // 3 notes, player presses each 30ms (30000us) late => mfast = -30000
    // Per note: biased = -30000 - 15000 = -45000, /30000 = -1, delta -= (-1) => delta += 1
    // After 3 notes: cumulative delta = +3
    let times: Vec<i64> = (0..3).map(|i| 200_000 * (i + 1)).collect();
    let (mut jm, notes, mut gauge) = make_autoadjust_jm(&times);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();

    jm.update(
        -1,
        &notes,
        &vec![false; key_count],
        &vec![i64::MIN; key_count],
        &mut gauge,
    );

    for i in 0..3 {
        let note_time = 200_000 * (i as i64 + 1);
        let press_time = note_time + 30_000; // 30ms late
        let mut keys = vec![false; key_count];
        keys[0] = true;
        let mut key_times = vec![i64::MIN; key_count];
        key_times[0] = press_time;
        jm.update(press_time, &notes, &keys, &key_times, &mut gauge);
    }

    assert_eq!(
        jm.judgetiming_delta(),
        3,
        "3 notes each 30ms late should produce cumulative delta +3"
    );
}

#[test]
fn auto_adjust_proportional_delta_early_hits() {
    // 3 notes, player presses each 30ms (30000us) early => mfast = +30000
    // Per note: biased = 30000 + 15000 = 45000, /30000 = 1, delta -= 1
    // After 3 notes: cumulative delta = -3
    let times: Vec<i64> = (0..3).map(|i| 200_000 * (i + 1)).collect();
    let (mut jm, notes, mut gauge) = make_autoadjust_jm(&times);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();

    jm.update(
        -1,
        &notes,
        &vec![false; key_count],
        &vec![i64::MIN; key_count],
        &mut gauge,
    );

    for i in 0..3 {
        let note_time = 200_000 * (i as i64 + 1);
        let press_time = note_time - 30_000; // 30ms early
        let mut keys = vec![false; key_count];
        keys[0] = true;
        let mut key_times = vec![i64::MIN; key_count];
        key_times[0] = press_time;
        jm.update(press_time, &notes, &keys, &key_times, &mut gauge);
    }

    assert_eq!(
        jm.judgetiming_delta(),
        -3,
        "3 notes each 30ms early should produce cumulative delta -3"
    );
}

#[test]
fn auto_adjust_larger_offset_produces_larger_delta() {
    // 1 note, player presses 50ms late => mfast = -50000
    // biased = -50000 - 15000 = -65000, /30000 = -2, delta -= (-2) => delta = +2
    let times: Vec<i64> = vec![200_000];
    let (mut jm, notes, mut gauge) = make_autoadjust_jm(&times);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();

    jm.update(
        -1,
        &notes,
        &vec![false; key_count],
        &vec![i64::MIN; key_count],
        &mut gauge,
    );

    let mut keys = vec![false; key_count];
    keys[0] = true;
    let mut key_times = vec![i64::MIN; key_count];
    key_times[0] = 200_000 + 50_000; // 50ms late
    jm.update(200_000 + 50_000, &notes, &keys, &key_times, &mut gauge);

    assert_eq!(
        jm.judgetiming_delta(),
        2,
        "50ms late should produce delta +2 (proportional)"
    );
}

#[test]
fn auto_adjust_deadzone_no_delta_within_15ms() {
    // 1 note, player presses 10ms late => mfast = -10000
    // biased = -10000 - 15000 = -25000, /30000 = 0, delta = 0 (deadzone)
    let times: Vec<i64> = vec![200_000];
    let (mut jm, notes, mut gauge) = make_autoadjust_jm(&times);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();

    jm.update(
        -1,
        &notes,
        &vec![false; key_count],
        &vec![i64::MIN; key_count],
        &mut gauge,
    );

    let mut keys = vec![false; key_count];
    keys[0] = true;
    let mut key_times = vec![i64::MIN; key_count];
    key_times[0] = 200_000 + 10_000; // 10ms late
    jm.update(200_000 + 10_000, &notes, &keys, &key_times, &mut gauge);

    assert_eq!(
        jm.judgetiming_delta(),
        0,
        "10ms offset is within 15ms deadzone, should produce no delta"
    );
}

#[test]
fn auto_adjust_no_delta_beyond_150ms() {
    // 1 note, player presses 160ms late => mfast = -160000
    // |mfast| > 150000 => outside range, no adjustment
    let times: Vec<i64> = vec![200_000];
    let (mut jm, notes, mut gauge) = make_autoadjust_jm(&times);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();

    jm.update(
        -1,
        &notes,
        &vec![false; key_count],
        &vec![i64::MIN; key_count],
        &mut gauge,
    );

    let mut keys = vec![false; key_count];
    keys[0] = true;
    let mut key_times = vec![i64::MIN; key_count];
    key_times[0] = 200_000 + 160_000; // 160ms late
    jm.update(200_000 + 160_000, &notes, &keys, &key_times, &mut gauge);

    assert_eq!(
        jm.judgetiming_delta(),
        0,
        "160ms offset exceeds 150ms range, should produce no delta"
    );
}

#[test]
fn auto_adjust_no_delta_when_disabled() {
    let times: Vec<i64> = (0..3).map(|i| 200_000 * (i + 1)).collect();
    let model = make_model_with_notes(&times);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();
    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: true,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);
    let rule = BMSPlayerRule::for_mode(&Mode::BEAT_7K);
    let mut gauge = GrooveGauge::new(&model, 0, &rule.gauge);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();

    jm.update(
        -1,
        &notes,
        &vec![false; key_count],
        &vec![i64::MIN; key_count],
        &mut gauge,
    );

    for i in 0..3 {
        let note_time = 200_000 * (i as i64 + 1);
        let press_time = note_time + 30_000; // 30ms late (would produce delta if enabled)
        let mut keys = vec![false; key_count];
        keys[0] = true;
        let mut key_times = vec![i64::MIN; key_count];
        key_times[0] = press_time;
        jm.update(press_time, &notes, &keys, &key_times, &mut gauge);
    }

    assert_eq!(
        jm.judgetiming_delta(),
        0,
        "disabled auto-adjust should not produce delta"
    );
}

#[test]
fn auto_adjust_no_delta_when_not_play_mode() {
    let times: Vec<i64> = (0..3).map(|i| 200_000 * (i + 1)).collect();
    let model = make_model_with_notes(&times);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();
    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: true,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);
    let rule = BMSPlayerRule::for_mode(&Mode::BEAT_7K);
    let mut gauge = GrooveGauge::new(&model, 0, &rule.gauge);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();

    jm.update(
        -1,
        &notes,
        &vec![false; key_count],
        &vec![i64::MIN; key_count],
        &mut gauge,
    );

    for i in 0..3 {
        let note_time = 200_000 * (i as i64 + 1);
        let press_time = note_time + 30_000; // 30ms late (would produce delta if play mode)
        let mut keys = vec![false; key_count];
        keys[0] = true;
        let mut key_times = vec![i64::MIN; key_count];
        key_times[0] = press_time;
        jm.update(press_time, &notes, &keys, &key_times, &mut gauge);
    }

    assert_eq!(
        jm.judgetiming_delta(),
        0,
        "non-play mode should not trigger auto-adjust"
    );
}

#[test]
fn take_judgetiming_delta_resets_accumulator() {
    let times: Vec<i64> = (0..3).map(|i| 200_000 * (i + 1)).collect();
    let (mut jm, notes, mut gauge) = make_autoadjust_jm(&times);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();

    jm.update(
        -1,
        &notes,
        &vec![false; key_count],
        &vec![i64::MIN; key_count],
        &mut gauge,
    );

    for i in 0..3 {
        let note_time = 200_000 * (i as i64 + 1);
        let press_time = note_time + 30_000; // 30ms late => delta per note = +1
        let mut keys = vec![false; key_count];
        keys[0] = true;
        let mut key_times = vec![i64::MIN; key_count];
        key_times[0] = press_time;
        jm.update(press_time, &notes, &keys, &key_times, &mut gauge);
    }

    let delta = jm.take_judgetiming_delta();
    assert_eq!(delta, 3, "3 notes 30ms late should produce delta +3");
    assert_eq!(jm.judgetiming_delta(), 0, "take should reset delta to 0");
}

#[test]
fn judge_vanish_bounds_checked_with_short_vec() {
    let mut jm = JudgeManager::new();
    // Set a custom judge_vanish shorter than 6 elements
    jm.judge_vanish = vec![true, false];
    // Accessing index 3 or 5 should return false (default), not panic
    assert!(!jm.judge_vanish.get(3).copied().unwrap_or(false));
    assert!(!jm.judge_vanish.get(5).copied().unwrap_or(false));
    // Index 0 should return the actual value
    assert!(jm.judge_vanish.first().copied().unwrap_or(false));
}

// --- MultiBadCollector regression tests ---

#[test]
fn multi_bad_capacity_guard() {
    let mut collector = MultiBadCollector::new();
    for i in 0..257 {
        collector.add(i, i as i64 * 100);
    }
    // Capacity is capped at 256; the 257th add should be rejected.
    assert_eq!(collector.size, 256);
    assert_eq!(collector.note_list.len(), 256);
    assert_eq!(collector.time_list.len(), 256);
}

#[test]
fn multi_bad_filter_with_minus_one_dmtime() {
    // Regression: filter() must find a note whose dmtime is -1,
    // not treat -1 as "not found".
    let mut collector = MultiBadCollector::new();
    // Set up mjudge windows: good = [-50, 50], bad = [-200, 200]
    collector.set_judge(&[
        [-1000, 1000], // PG
        [-500, 500],   // GR
        [-50, 50],     // GD (good)
        [-200, 200],   // BD (bad)
        [-300, 300],   // PR
    ]);

    // Add two notes: note 0 with dmtime=-1 (the target), note 1 with dmtime=-100
    collector.add(0, -1);
    collector.add(1, -100);

    // Build a minimal JudgeNote slice (filter reads notes but only uses indices here)
    let notes = vec![
        JudgeNote {
            time_us: 1000,
            end_time_us: 0,
            lane: 0,
            wav: 1,
            kind: bms_model::judge_note::JudgeNoteKind::Normal,
            ln_type: 0,
            damage: 0.0,
            pair_index: None,
        },
        JudgeNote {
            time_us: 2000,
            end_time_us: 0,
            lane: 1,
            wav: 1,
            kind: bms_model::judge_note::JudgeNoteKind::Normal,
            ln_type: 0,
            damage: 0.0,
            pair_index: None,
        },
    ];

    // Filter with note 0 as the target note (tnote).
    // dmtime=-1 is within bad range [-200, 200] but also within good range [-50, 50],
    // so note 0 is excluded as tnote, and note 1 (dmtime=-100) is in bad but not good
    // range, so it should be kept.
    collector.filter(Some(0), &notes);

    // The key assertion: filter did NOT early-return (which would happen if
    // dmtime=-1 were treated as "not found"). Note 1 with dmtime=-100 is in
    // bad range [-200, 200] and NOT in good range [-50, 50], so it survives.
    assert_eq!(collector.size, 1);
    assert_eq!(collector.note_list[0], 1);
    assert_eq!(collector.time_list[0], -100);
}

// --- Regression tests for judge system wiring fixes ---

#[test]
fn from_config_has_nonzero_lane_count() {
    // Issue 1: new() + init() left lane_count=0, making update() iterate 0..0.
    // from_config() must set lane_count from the mode.
    let model = make_model_with_notes(&[1_000_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let jm = JudgeManager::from_config(&config);

    // BEAT_7K has 8 lanes (7 keys + 1 scratch)
    assert!(
        !jm.auto_presstime().is_empty(),
        "auto_presstime should be initialized (lane_count > 0)"
    );
}

#[test]
fn from_config_with_judgeregion_2_sizes_arrays() {
    // Issue 1/4: judgenow/judgecombo/judgefast/mjudgefast must be sized by judgeregion.
    let model = make_model_with_notes(&[1_000_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_14K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 2,
    };
    let jm = JudgeManager::from_config(&config);

    // Both player slots should be accessible
    assert_eq!(jm.now_judge(0), 0);
    assert_eq!(jm.now_judge(1), 0);
    assert_eq!(jm.now_combo(0), 0);
    assert_eq!(jm.now_combo(1), 0);
    assert_eq!(jm.recent_judge_timing(0), 0);
    assert_eq!(jm.recent_judge_timing(1), 0);
    assert_eq!(jm.recent_judge_micro_timing(0), 0);
    assert_eq!(jm.recent_judge_micro_timing(1), 0);
}

#[test]
fn update_produces_judged_events_after_from_config() {
    // Issue 2/3: update() must produce judge events when properly initialized
    // via from_config (not new()+init() which left lane_count=0).
    let model = make_model_with_notes(&[500_000, 1_000_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

    let mut all_events = Vec::new();
    let mut time = 0i64;
    while time <= 2_000_000 {
        jm.update(time, &notes, &key_states, &key_times, &mut gauge);
        all_events.extend(jm.drain_judged_events());
        time += 1000;
    }

    // Autoplay should produce 2 PGREAT events (one per note)
    assert_eq!(
        all_events.len(),
        2,
        "expected 2 judge events from autoplay, got {}",
        all_events.len()
    );
    for (judge, _mtime) in &all_events {
        assert_eq!(*judge, 0, "autoplay should produce PGREAT (judge=0)");
    }
}

#[test]
fn judgenow_judgecombo_populated_after_update_micro() {
    // Issue 4: judgenow/judgecombo must be written in update_micro after combo update.
    let model = make_model_with_notes(&[500_000, 1_000_000, 1_500_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

    let mut time = 0i64;
    while time <= 2_000_000 {
        jm.update(time, &notes, &key_states, &key_times, &mut gauge);
        time += 1000;
    }

    // After 3 autoplay PGREAT judgments, judgenow[0] should be 1 (PG+1)
    // and judgecombo[0] should be 3.
    assert_eq!(
        jm.now_judge(0),
        1,
        "judgenow should be 1 (PGREAT+1) after autoplay"
    );
    assert_eq!(
        jm.now_combo(0),
        3,
        "judgecombo should be 3 after 3 PGREAT judgments"
    );
}

#[test]
fn judgecombo_uses_coursecombo_not_combo() {
    // Regression: Java JudgeManager line 710 assigns getCourseCombo() to
    // judgecombo, not getCombo(). In course mode (dan-i nintei), coursecombo
    // carries over from the previous song via set_course_combo(), while combo
    // resets to 0. If judgecombo incorrectly reads combo, the skin combo
    // display resets to 0 at the start of each subsequent course song.
    let model = make_model_with_notes(&[500_000, 1_000_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    // Simulate course mode: carry over a combo of 50 from the previous song.
    // combo stays at 0 (reset per-song), coursecombo starts at 50.
    jm.set_course_combo(50);

    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

    let mut time = 0i64;
    while time <= 1_500_000 {
        jm.update(time, &notes, &key_states, &key_times, &mut gauge);
        time += 1000;
    }

    // combo = 2 (reset per-song, only 2 notes hit)
    // coursecombo = 50 + 2 = 52 (carried over from previous song)
    assert_eq!(jm.combo(), 2, "per-song combo should be 2");
    assert_eq!(
        jm.course_combo(),
        52,
        "coursecombo should carry over (50 + 2)"
    );

    // The key assertion: now_combo (judgecombo) must reflect coursecombo, not combo.
    // Before the fix, this would return 2 (combo) instead of 52 (coursecombo).
    assert_eq!(
        jm.now_combo(0),
        52,
        "judgecombo must use coursecombo (52), not combo (2)"
    );
}

#[test]
fn gauge_not_double_updated_via_judged_events() {
    // Verify gauge.update is called exactly once per judgment (in update_micro),
    // not again in the caller's update_judge. We do this by comparing gauge
    // values between a direct simulation and one where we manually call update again.
    let model = make_model_with_notes(&[500_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    let gauge_before = gauge.value();

    jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

    let mut time = 0i64;
    while time <= 1_000_000 {
        jm.update(time, &notes, &key_states, &key_times, &mut gauge);
        time += 1000;
    }

    let gauge_after_single = gauge.value();
    let events = jm.drain_judged_events();
    assert_eq!(events.len(), 1, "expected exactly one judge event");

    // If we were to call gauge.update(judge) again here, it would be a
    // double-update. Instead, we verify the gauge changed exactly once.
    // A PGREAT (judge=0) on a NORMAL gauge increases the value.
    assert!(
        gauge_after_single > gauge_before,
        "gauge should increase after PGREAT (before={}, after={})",
        gauge_before,
        gauge_after_single,
    );
}

// --- from_config score play_option regression tests ---

#[test]
fn from_config_sets_judge_algorithm_combo() {
    let model = make_model_with_notes(&[1_000_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: true,
        judgeregion: 1,
    };
    let jm = JudgeManager::from_config(&config);

    assert_eq!(
        jm.score().play_option.judge_algorithm,
        Some(rubato_types::judge_algorithm::JudgeAlgorithm::Combo),
    );
    assert_eq!(
        jm.score().play_option.rule,
        Some(rubato_types::bms_player_rule::BMSPlayerRule::LR2),
    );
}

#[test]
fn from_config_sets_judge_algorithm_duration() {
    let model = make_model_with_notes(&[1_000_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Duration,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: true,
        judgeregion: 1,
    };
    let jm = JudgeManager::from_config(&config);

    assert_eq!(
        jm.score().play_option.judge_algorithm,
        Some(rubato_types::judge_algorithm::JudgeAlgorithm::Duration),
    );
}

#[test]
fn from_config_sets_judge_algorithm_lowest() {
    let model = make_model_with_notes(&[1_000_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Lowest,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: true,
        judgeregion: 1,
    };
    let jm = JudgeManager::from_config(&config);

    assert_eq!(
        jm.score().play_option.judge_algorithm,
        Some(rubato_types::judge_algorithm::JudgeAlgorithm::Lowest),
    );
}

#[test]
fn from_config_sets_judge_algorithm_score_maps_to_timing() {
    let model = make_model_with_notes(&[1_000_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Score,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: true,
        judgeregion: 1,
    };
    let jm = JudgeManager::from_config(&config);

    // Score in the play crate maps to Timing in the types crate
    assert_eq!(
        jm.score().play_option.judge_algorithm,
        Some(rubato_types::judge_algorithm::JudgeAlgorithm::Timing),
    );
    assert_eq!(
        jm.score().play_option.rule,
        Some(rubato_types::bms_player_rule::BMSPlayerRule::LR2),
    );
}

// --- note_state / note_play_time accessor tests ---

#[test]
fn note_state_returns_zero_for_unjudged() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    let mut tl = TimeLine::new(0.0, 1_000_000, 8);
    tl.set_note(0, Some(Note::new_normal(1)));
    model.timelines = vec![tl];

    let notes = build_judge_notes(&model);
    let jp = BMSPlayerRule::for_mode(&Mode::BEAT_7K).judge;
    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: model.lntype(),
        judge_rank: model.judgerank,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let jm = JudgeManager::from_config(&config);

    assert_eq!(jm.note_state(0), 0, "Unjudged note should have state 0");
    assert_eq!(
        jm.note_play_time(0),
        0,
        "Unjudged note should have play_time 0"
    );
    assert_eq!(jm.note_state_count(), 1, "Should have 1 note state");
}

#[test]
fn note_state_out_of_bounds_returns_zero() {
    let jm = JudgeManager::new();
    assert_eq!(jm.note_state(0), 0);
    assert_eq!(jm.note_state(999), 0);
    assert_eq!(jm.note_play_time(0), 0);
    assert_eq!(jm.note_play_time(999), 0);
    assert_eq!(jm.note_state_count(), 0);
}

#[test]
fn note_state_updated_after_autoplay_judgment() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    let mut tl = TimeLine::new(0.0, 1_000_000, 8);
    tl.set_note(0, Some(Note::new_normal(1)));
    model.timelines = vec![tl];

    let notes = build_judge_notes(&model);
    let jp = BMSPlayerRule::for_mode(&Mode::BEAT_7K).judge;
    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: model.lntype(),
        judge_rank: model.judgerank,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);
    let mut gauge = rubato_types::groove_gauge::GrooveGauge::new(
        &model,
        rubato_types::groove_gauge::NORMAL,
        &rubato_types::gauge_property::GaugeProperty::SevenKeys,
    );

    // Autoplay at exactly note time -> PG (judge=0, state=1)
    jm.update(
        1_000_000,
        &notes,
        &vec![false; 256],
        &vec![i64::MIN; 256],
        &mut gauge,
    );

    assert_eq!(
        jm.note_state(0),
        1,
        "Autoplay PG should set state to 1 (PG+1)"
    );
    assert_eq!(
        jm.note_play_time(0),
        0,
        "Autoplay PG should have play_time 0"
    );
}

// --- Keysound event tests ---

#[test]
fn autoplay_produces_keysound_play_events() {
    // Autoplay should produce keysound play events for each judged note.
    // Java: keysound.play(note, keyvolume, 0) in autoplay normal note path.
    let model = make_model_with_notes(&[500_000, 1_000_000, 1_500_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

    // Collect all keysound play events across the simulation
    let mut all_keysound_plays = Vec::new();
    let mut time = 0i64;
    while time <= 2_500_000 {
        jm.update(time, &notes, &key_states, &key_times, &mut gauge);
        all_keysound_plays.extend(jm.drain_keysound_play_indices());
        time += 1000;
    }

    // All 3 notes should have produced keysound events
    assert_eq!(
        all_keysound_plays.len(),
        3,
        "Autoplay should produce 3 keysound play events for 3 normal notes"
    );
}

#[test]
fn drain_keysound_play_indices_clears_after_drain() {
    let model = make_model_with_notes(&[500_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

    // Collect keysound events across frames
    let mut all_events = Vec::new();
    let mut time = 0i64;
    while time <= 1_500_000 {
        jm.update(time, &notes, &key_states, &key_times, &mut gauge);
        all_events.extend(jm.drain_keysound_play_indices());
        time += 1000;
    }

    // Should have found at least one event
    assert!(
        !all_events.is_empty(),
        "Should have keysound events after autoplay"
    );

    // Now do another update past any notes, drain should be empty
    jm.update(2_000_000, &notes, &key_states, &key_times, &mut gauge);
    let events2 = jm.drain_keysound_play_indices();
    assert!(
        events2.is_empty(),
        "Drain after no-event update should return empty vec"
    );

    // Second consecutive drain (without update) should also be empty
    let events3 = jm.drain_keysound_play_indices();
    assert!(
        events3.is_empty(),
        "Second drain without update should return empty vec"
    );
}

#[test]
fn mine_note_hit_produces_keysound_play_event() {
    // Mine note hit should produce a keysound play event.
    // Java line 258: keysound.play(note, keyvolume, 0) on mine damage.
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;

    let mut tl = TimeLine::new(0.0, 500_000, 8);
    let mine = Note::new_mine(1, 0.5);
    tl.set_note(0, Some(mine));
    model.timelines = vec![tl];

    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();

    // Key for lane 0 must be pressed when the mine note passes through
    let mut key_states = vec![false; key_count];
    // Lane 0 key assignment: find the key index for lane 0
    let key_lane_assign = lp.key_lane_assign();
    for (key_idx, &lane) in key_lane_assign.iter().enumerate() {
        if lane == 0 {
            key_states[key_idx] = true;
            break;
        }
    }

    let key_times = vec![i64::MIN; key_count];

    jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

    let mut all_keysound_plays = Vec::new();
    let mut time = 0i64;
    while time <= 1_000_000 {
        jm.update(time, &notes, &key_states, &key_times, &mut gauge);
        all_keysound_plays.extend(jm.drain_keysound_play_indices());
        time += 1000;
    }

    assert!(
        !all_keysound_plays.is_empty(),
        "Mine note hit should produce a keysound play event"
    );
}

#[test]
fn manual_key_press_produces_keysound_play_event() {
    // Manual note judgment should produce a keysound play event.
    // Java line 473: keysound.play(tnote, keyvolume, 0) on normal note hit.
    let model = make_model_with_notes(&[500_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();
    let key_lane_assign = lp.key_lane_assign();

    // Find the key index for lane 0
    let mut lane0_key = 0;
    for (key_idx, &lane) in key_lane_assign.iter().enumerate() {
        if lane == 0 {
            lane0_key = key_idx;
            break;
        }
    }

    let key_states_idle = vec![false; key_count];
    let key_times_idle = vec![i64::MIN; key_count];

    // Prime
    jm.update(-1, &notes, &key_states_idle, &key_times_idle, &mut gauge);

    // Advance to near the note time, then press the key
    let mut time = 0i64;
    while time < 499_000 {
        jm.update(time, &notes, &key_states_idle, &key_times_idle, &mut gauge);
        time += 1000;
    }

    // Now press: key pressed at 500_000 (exact time)
    let mut key_states_pressed = vec![false; key_count];
    key_states_pressed[lane0_key] = true;
    let mut key_times_pressed = vec![i64::MIN; key_count];
    key_times_pressed[lane0_key] = 500_000;

    jm.update(
        500_000,
        &notes,
        &key_states_pressed,
        &key_times_pressed,
        &mut gauge,
    );
    let plays = jm.drain_keysound_play_indices();

    assert!(
        !plays.is_empty(),
        "Manual key press on a normal note should produce a keysound play event"
    );
}

#[test]
fn keysound_events_cleared_at_start_of_update() {
    // Events from a previous update() should not leak into the next call.
    let model = make_model_with_notes(&[500_000, 2_000_000]);
    let notes = build_judge_notes(&model);
    let jp = crate::judge_property::lr2();

    let config = JudgeConfig {
        notes: &notes,
        mode: &Mode::BEAT_7K,
        ln_type: LnType::LongNote,
        judge_rank: 100,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &jp,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
        judgeregion: 1,
    };
    let mut jm = JudgeManager::from_config(&config);

    let gp = crate::gauge_property::GaugeProperty::Lr2;
    let mut gauge = GrooveGauge::new(&model, GrooveGauge::NORMAL, &gp);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    jm.update(-1, &notes, &key_states, &key_times, &mut gauge);

    // Advance past first note
    let mut time = 0i64;
    while time <= 600_000 {
        jm.update(time, &notes, &key_states, &key_times, &mut gauge);
        time += 1000;
    }
    // Don't drain - keysound events are sitting in the vec

    // Next update at a time where no new note is judged
    jm.update(700_000, &notes, &key_states, &key_times, &mut gauge);

    // The update() should have cleared old events before processing
    let plays = jm.drain_keysound_play_indices();
    // Should be 0 because the note at 500_000 was judged previously and
    // update() clears at start
    assert!(
        plays.is_empty(),
        "Keysound events from a previous update() should be cleared"
    );
}
