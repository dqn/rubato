// Wiring invariant tests: verify that components are connected correctly
// across the judge -> key beam -> timer pipeline.
//
// These tests would have caught bugs #3 and #4 from the play-screen bug batch:
// - Bug #3: JudgeManager never called input_key_on() for key beam timers
// - Bug #4: input() key-release branch not guarded by is_judge_started

use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;
use rubato_core::timer_manager::TimerManager;
use rubato_play::bms_player_rule::BMSPlayerRule;
use rubato_play::groove_gauge::create_groove_gauge;
use rubato_play::judge_algorithm::JudgeAlgorithm;
use rubato_play::judge_manager::{JudgeConfig, JudgeManager};
use rubato_play::key_input_processor::{InputContext, KeyInputProccessor};
use rubato_play::lane_property::LaneProperty;
use rubato_types::groove_gauge::NORMAL;
use rubato_types::timer_id::TimerId;

// Timer ID constants (must match key_input.rs private constants)
const TIMER_KEYON_1P_SCRATCH: i32 = 100;
const TIMER_KEYOFF_1P_SCRATCH: i32 = 120;

/// Compute expected key-on timer ID for a lane in BEAT_7K mode.
/// BEAT_7K lane_to_skin_offset = [1, 2, 3, 4, 5, 6, 7, 0]
/// All lanes are player 0.
fn keyon_timer_for_lane(lane: usize) -> TimerId {
    let offsets = [1, 2, 3, 4, 5, 6, 7, 0]; // BEAT_7K
    let offset = offsets[lane];
    TimerId::new(TIMER_KEYON_1P_SCRATCH + offset)
}

fn keyoff_timer_for_lane(lane: usize) -> TimerId {
    let offsets = [1, 2, 3, 4, 5, 6, 7, 0];
    let offset = offsets[lane];
    TimerId::new(TIMER_KEYOFF_1P_SCRATCH + offset)
}

fn make_model_with_note_on_lane(lane: usize) -> BMSModel {
    let key_count = Mode::BEAT_7K.key();
    let mut tl = TimeLine::new(0.0, 1_000_000, key_count);
    tl.set_note(lane as i32, Some(Note::new_normal(1)));
    let mut model = BMSModel::new();
    model.timelines = vec![tl];
    model.set_mode(Mode::BEAT_7K);
    model
}

fn make_judge_manager(model: &BMSModel) -> (JudgeManager, rubato_play::groove_gauge::GrooveGauge) {
    let judge_notes = model.build_judge_notes();
    let mode = Mode::BEAT_7K;
    let rule = BMSPlayerRule::for_mode(&mode);
    let config = JudgeConfig {
        notes: &judge_notes,
        mode: &mode,
        ln_type: model.lntype(),
        judge_rank: model.judgerank,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &rule.judge,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
    };
    let jm = JudgeManager::from_config(&config);
    let gg = create_groove_gauge(model, NORMAL, 0, None).unwrap();
    (jm, gg)
}

// ===========================================================================
// Test 1: JudgeManager emits judged lanes on hit
// ===========================================================================

#[test]
fn judge_update_emits_judged_lanes_on_hit() {
    let model = make_model_with_note_on_lane(0);
    let judge_notes = model.build_judge_notes();
    let (mut jm, mut gg) = make_judge_manager(&model);
    let key_count = Mode::BEAT_7K.key() as usize;

    // Prime
    let empty_states = vec![false; key_count];
    let empty_times = vec![i64::MIN; key_count];
    jm.update(-1, &judge_notes, &empty_states, &empty_times, &mut gg);

    // Verify no judged lanes before any hit
    let before = jm.drain_judged_lanes();
    assert!(before.is_empty(), "no lanes should be judged before input");

    // Press key 0 at exactly note time
    let note_time = 1_000_000i64;
    let mut key_states = vec![false; key_count];
    key_states[0] = true;
    let mut key_times = vec![i64::MIN; key_count];
    key_times[0] = note_time;

    jm.update(note_time, &judge_notes, &key_states, &key_times, &mut gg);

    let judged = jm.drain_judged_lanes();
    assert!(
        !judged.is_empty(),
        "drain_judged_lanes() must be non-empty after a hit"
    );
    assert_eq!(judged[0], 0, "lane 0 should be in the judged lanes");
}

// ===========================================================================
// Test 2: input_key_on() sets the correct KEYON timer
// ===========================================================================

#[test]
fn input_key_on_sets_keyon_timer() {
    let lane_property = LaneProperty::new(&Mode::BEAT_7K);
    let mut processor = KeyInputProccessor::new(&lane_property);
    let mut timer = TimerManager::new();

    // Verify timer is off initially
    let timer_on = keyon_timer_for_lane(0);
    assert!(
        !timer.is_timer_on(timer_on),
        "timer should be off before input_key_on"
    );

    // Call input_key_on for lane 0
    processor.input_key_on(0, &mut timer);

    // Verify timer is now on
    assert!(
        timer.is_timer_on(timer_on),
        "timer should be on after input_key_on"
    );

    // Verify the off timer is off
    let timer_off = keyoff_timer_for_lane(0);
    assert!(
        !timer.is_timer_on(timer_off),
        "off timer should be off after input_key_on"
    );
}

#[test]
fn input_key_on_works_for_multiple_lanes() {
    let lane_property = LaneProperty::new(&Mode::BEAT_7K);
    let mut processor = KeyInputProccessor::new(&lane_property);
    let mut timer = TimerManager::new();

    for lane in 0..8 {
        processor.input_key_on(lane, &mut timer);
        assert!(
            timer.is_timer_on(keyon_timer_for_lane(lane)),
            "keyon timer should be set for lane {lane}"
        );
    }
}

// ===========================================================================
// Test 3: input() does NOT clear timer during play (regression for bug #4)
// ===========================================================================

#[test]
fn input_does_not_clear_timer_during_play() {
    let lane_property = LaneProperty::new(&Mode::BEAT_7K);
    let mut processor = KeyInputProccessor::new(&lane_property);
    let mut timer = TimerManager::new();

    // Set key beam timer via input_key_on (simulating judge hit)
    processor.input_key_on(0, &mut timer);
    let timer_on = keyon_timer_for_lane(0);
    assert!(timer.is_timer_on(timer_on), "precondition: timer is on");

    // Start judge (sets is_judge_started = true)
    processor.start_judge(10_000_000, None, 0);

    // Call input() with all keys released — this should NOT clear the timer
    // because is_judge_started is true and autoplay is false
    let key_count = Mode::BEAT_7K.key() as usize;
    let key_states = vec![false; key_count];
    let auto_presstime = vec![i64::MIN; key_count];
    let mut ctx = InputContext {
        now: 100_000,
        key_states: &key_states,
        auto_presstime: &auto_presstime,
        is_autoplay: false,
        timer: &mut timer,
    };
    processor.input(&mut ctx);

    // Timer should still be on — the fix ensures the else-if branch
    // is guarded by !is_judge_started
    assert!(
        ctx.timer.is_timer_on(timer_on),
        "BUG: input() cleared the key beam timer during play! \
         The else-if branch must be guarded by !is_judge_started"
    );
}

// ===========================================================================
// Test 4: Full key beam lifecycle during play
// ===========================================================================

#[test]
fn key_beam_lifecycle_during_play() {
    let model = make_model_with_note_on_lane(0);
    let judge_notes = model.build_judge_notes();
    let (mut jm, mut gg) = make_judge_manager(&model);

    let mode = Mode::BEAT_7K;
    let key_count = mode.key() as usize;
    let lane_property = LaneProperty::new(&mode);
    let mut processor = KeyInputProccessor::new(&lane_property);
    let mut timer = TimerManager::new();

    // Start judge
    processor.start_judge(10_000_000, None, 0);

    // Prime judge
    let empty_states = vec![false; key_count];
    let empty_times = vec![i64::MIN; key_count];
    jm.update(-1, &judge_notes, &empty_states, &empty_times, &mut gg);

    // Simulate key press at note time
    let note_time = 1_000_000i64;
    let mut key_states = vec![false; key_count];
    key_states[0] = true;
    let mut key_times = vec![i64::MIN; key_count];
    key_times[0] = note_time;

    jm.update(note_time, &judge_notes, &key_states, &key_times, &mut gg);

    // Drain judged lanes and trigger key beam timers (the bridge)
    let judged = jm.drain_judged_lanes();
    assert!(!judged.is_empty(), "should have judged lanes after a hit");
    for lane in &judged {
        processor.input_key_on(*lane, &mut timer);
    }

    // Verify key beam timer is on
    let timer_on = keyon_timer_for_lane(0);
    assert!(
        timer.is_timer_on(timer_on),
        "key beam timer should be on after judge hit"
    );

    // Simulate next frame: key is released, but timer should persist
    let released_states = vec![false; key_count];
    let auto_presstime = vec![i64::MIN; key_count];
    let mut ctx = InputContext {
        now: note_time / 1000 + 16, // ~1 frame later
        key_states: &released_states,
        auto_presstime: &auto_presstime,
        is_autoplay: false,
        timer: &mut timer,
    };
    processor.input(&mut ctx);

    // Key beam should still be visible (timer preserved during play)
    assert!(
        ctx.timer.is_timer_on(timer_on),
        "key beam timer must persist during play even after key release"
    );
}

// ===========================================================================
// Test 5: Before judge starts, key release DOES clear timer (normal behavior)
// ===========================================================================

#[test]
fn input_clears_timer_before_judge_starts() {
    let lane_property = LaneProperty::new(&Mode::BEAT_7K);
    let mut processor = KeyInputProccessor::new(&lane_property);
    let mut timer = TimerManager::new();

    let key_count = Mode::BEAT_7K.key() as usize;
    let timer_on = keyon_timer_for_lane(0);

    // Press key (judge not started, so input() sets timer)
    let mut key_states = vec![false; key_count];
    key_states[0] = true;
    let auto_presstime = vec![i64::MIN; key_count];
    let mut ctx = InputContext {
        now: 100,
        key_states: &key_states,
        auto_presstime: &auto_presstime,
        is_autoplay: false,
        timer: &mut timer,
    };
    processor.input(&mut ctx);
    assert!(
        ctx.timer.is_timer_on(timer_on),
        "timer should be on after key press"
    );

    // Release key (judge not started, so input() should clear timer)
    let released_states = vec![false; key_count];
    let mut ctx2 = InputContext {
        now: 200,
        key_states: &released_states,
        auto_presstime: &auto_presstime,
        is_autoplay: false,
        timer: ctx.timer,
    };
    processor.input(&mut ctx2);
    assert!(
        !ctx2.timer.is_timer_on(timer_on),
        "before judge starts, key release should clear the timer"
    );
}
