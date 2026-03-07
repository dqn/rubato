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
    jm.scoring.coursecombo = 42;
    assert_eq!(jm.course_combo(), 42);
}

#[test]
fn set_course_maxcombo() {
    let mut jm = JudgeManager::new();
    jm.scoring.coursemaxcombo = 100;
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
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    jm.init(&model, 1, None, &[]);

    let ghost = jm.ghost();
    let total = model.total_notes() as usize;
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
        assert_eq!(g, JUDGE_PG as i32);
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
    assert_eq!(jm.ghost()[0], JUDGE_PR as i32);
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
    };
    let jm = JudgeManager::from_config(&config);
    let rule = BMSPlayerRule::for_mode(&Mode::BEAT_7K);
    let gauge = GrooveGauge::new(&model, 0, &rule.gauge);
    (jm, notes, gauge)
}

#[test]
fn auto_adjust_increments_delta_when_consistently_late() {
    // 10 notes spaced 200ms apart, player presses 1ms late
    // mfast = note_time - press_time < 0 (negative = late)
    // Java: mfast < 0 → judgetiming += 1 (compensate lateness)
    let times: Vec<i64> = (0..10).map(|i| 200_000 * (i + 1)).collect();
    let (mut jm, notes, mut gauge) = make_autoadjust_jm(&times);

    let lp = LaneProperty::new(&Mode::BEAT_7K);
    let key_count = lp.key_lane_assign().len();

    // Prime with -1 update
    jm.update(
        -1,
        &notes,
        &vec![false; key_count],
        &vec![i64::MIN; key_count],
        &mut gauge,
    );

    // Press each note 1ms (1000μs) late => mfast = -(1000) < 0 => delta = +1
    for i in 0..10 {
        let note_time = 200_000 * (i as i64 + 1);
        let press_time = note_time + 1000; // 1ms late
        let mut keys = vec![false; key_count];
        keys[0] = true; // Press key 0 (lane 0)
        let mut key_times = vec![i64::MIN; key_count];
        key_times[0] = press_time;
        jm.update(press_time, &notes, &keys, &key_times, &mut gauge);
    }

    // After 10 good+ judgments with |mfast| >= 500, delta should be +1
    // (mfast < 0 means late, Java compensates by increasing judgetiming)
    assert_eq!(
        jm.judgetiming_delta(),
        1,
        "late hits should increase judgetiming"
    );
}

#[test]
fn auto_adjust_no_delta_when_disabled() {
    let times: Vec<i64> = (0..10).map(|i| 200_000 * (i + 1)).collect();
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

    for i in 0..10 {
        let note_time = 200_000 * (i as i64 + 1);
        let press_time = note_time + 1000;
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
    let times: Vec<i64> = (0..10).map(|i| 200_000 * (i + 1)).collect();
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

    for i in 0..10 {
        let note_time = 200_000 * (i as i64 + 1);
        let press_time = note_time + 1000;
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
    let times: Vec<i64> = (0..10).map(|i| 200_000 * (i + 1)).collect();
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

    for i in 0..10 {
        let note_time = 200_000 * (i as i64 + 1);
        let press_time = note_time + 1000;
        let mut keys = vec![false; key_count];
        keys[0] = true;
        let mut key_times = vec![i64::MIN; key_count];
        key_times[0] = press_time;
        jm.update(press_time, &notes, &keys, &key_times, &mut gauge);
    }

    let delta = jm.take_judgetiming_delta();
    assert_ne!(delta, 0);
    assert_eq!(jm.judgetiming_delta(), 0, "take should reset delta to 0");
}

#[test]
fn judge_vanish_bounds_checked_with_short_vec() {
    let mut jm = JudgeManager::new();
    // Set a custom judge_vanish shorter than 6 elements
    jm.set_judge_vanish_for_test(vec![true, false]);
    // Accessing index 3 or 5 should return false (default), not panic
    let vanish = jm.judge_vanish_ref();
    assert!(!vanish.get(3).copied().unwrap_or(false));
    assert!(!vanish.get(5).copied().unwrap_or(false));
    // Index 0 should return the actual value
    assert!(vanish.first().copied().unwrap_or(false));
}
