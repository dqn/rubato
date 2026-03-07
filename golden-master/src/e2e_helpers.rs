// E2E simulation helpers: shared between e2e_judge.rs and exhaustive_e2e.rs
//
// Provides BMS loading, autoplay/manual simulation, and assertion utilities
// for integration tests that validate the full pipeline:
// BMS parse -> JudgeManager -> GrooveGauge -> ScoreData

use std::path::Path;

use bms_model::bms_decoder::BMSDecoder;
use bms_model::bms_model::{BMSModel, LNTYPE_LONGNOTE};
use bms_model::chart_information::ChartInformation;
use bms_model::judge_note::{
    JUDGE_BD, JUDGE_GD, JUDGE_GR, JUDGE_MS, JUDGE_PG, JUDGE_PR, JudgeNote,
};
use bms_model::mode::Mode;
use rubato_core::score_data::ScoreData;
use rubato_input::key_input_log::KeyInputLog;
use rubato_play::bms_player_rule::BMSPlayerRule;
use rubato_play::judge_algorithm::JudgeAlgorithm;
use rubato_play::judge_manager::{JudgeConfig, JudgeManager};
use rubato_play::lane_property::LaneProperty;
use rubato_types::groove_gauge::GrooveGauge;

/// Sentinel for "not set" timestamps (matches JudgeManager internal).
pub const NOT_SET: i64 = i64::MIN;

/// Frame step for simulation (1ms = 1000us).
pub const FRAME_STEP: i64 = 1_000;

/// Extra time after last note to finish simulation (1 second).
pub const TAIL_TIME: i64 = 1_000_000;

pub fn test_bms_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test-bms")
        .leak()
}

/// Load and validate a BMS file. Validation normalizes judge_rank and total.
pub fn load_bms(filename: &str) -> BMSModel {
    let path = test_bms_dir().join(filename);
    let info = ChartInformation::new(Some(path), LNTYPE_LONGNOTE, None);
    let mut model = BMSDecoder::new()
        .decode(info)
        .unwrap_or_else(|| panic!("Failed to parse {filename}"));
    BMSPlayerRule::validate(&mut model);
    model
}

pub struct SimulationResult {
    pub score: ScoreData,
    pub max_combo: i32,
    pub ghost: Vec<usize>,
    pub gauge_value: f32,
    pub gauge_qualified: bool,
}

/// Count normal (non-LN) playable notes from JudgeNote array.
pub fn count_normal_notes(notes: &[JudgeNote]) -> usize {
    notes
        .iter()
        .filter(|n| n.is_playable() && !n.is_long())
        .count()
}

/// Run autoplay simulation: JudgeManager with autoplay=true, empty key inputs.
pub fn run_autoplay_simulation(model: &BMSModel, gauge_type: i32) -> SimulationResult {
    let judge_notes = model.build_judge_notes();
    let mode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
    let rule = BMSPlayerRule::for_mode(&mode);

    let config = JudgeConfig {
        notes: &judge_notes,
        mode: &mode,
        ln_type: model.lntype(),
        judge_rank: model.judgerank(),
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &rule.judge,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
    };

    let mut jm = JudgeManager::from_config(&config);
    let mut gauge = GrooveGauge::new(model, gauge_type, &rule.gauge);

    let lp = LaneProperty::new(&mode);
    let physical_key_count = lp.key_lane_assign().len();
    let key_states = vec![false; physical_key_count];
    let key_times = vec![NOT_SET; physical_key_count];

    // Prime JudgeManager: set prev_time to -1 so notes at time_us=0 are not skipped.
    jm.update(-1, &judge_notes, &key_states, &key_times, &mut gauge);

    let last_note_time = judge_notes
        .iter()
        .map(|n| n.time_us.max(n.end_time_us))
        .max()
        .unwrap_or(0);
    let end_time = last_note_time + TAIL_TIME;

    let mut time = 0i64;
    while time <= end_time {
        jm.update(time, &judge_notes, &key_states, &key_times, &mut gauge);
        time += FRAME_STEP;
    }

    SimulationResult {
        score: jm.score().clone(),
        max_combo: jm.max_combo(),
        ghost: jm.ghost_as_usize(),
        gauge_value: gauge.value(),
        gauge_qualified: gauge.is_qualified(),
    }
}

/// Create simple press+release input events for each playable note.
/// For normal notes: press at note time, release 80ms later.
/// LN notes are not generated here (handled by autoplay in JudgeManager).
///
/// Uses LaneProperty to map note lanes to physical key indices, which is
/// required for DP modes (Beat14K) where lane indices differ from key indices.
pub fn create_note_press_log(notes: &[JudgeNote], mode: &Mode, offset_us: i64) -> Vec<KeyInputLog> {
    let lp = LaneProperty::new(mode);
    let lane_keys = lp.lane_key_assign();
    let mut log = Vec::new();
    for note in notes {
        if !note.is_playable() {
            continue;
        }
        if note.is_long() {
            // Skip LN start/end notes for manual tests
            continue;
        }
        // Use lane_to_key mapping to get the correct physical key index
        if note.lane < lane_keys.len() {
            let key = lane_keys[note.lane][0];
            log.push(KeyInputLog::with_data(note.time_us + offset_us, key, true));
            // Release 80ms after press
            log.push(KeyInputLog::with_data(
                note.time_us + offset_us + 80_000,
                key,
                false,
            ));
        }
    }
    log
}

/// Run manual simulation with per-frame key state conversion.
pub fn run_manual_simulation(
    model: &BMSModel,
    input_log: &[KeyInputLog],
    gauge_type: i32,
) -> SimulationResult {
    let judge_notes = model.build_judge_notes();
    let mode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
    let rule = BMSPlayerRule::for_mode(&mode);

    let config = JudgeConfig {
        notes: &judge_notes,
        mode: &mode,
        ln_type: model.lntype(),
        judge_rank: model.judgerank(),
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &rule.judge,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
    };

    let mut jm = JudgeManager::from_config(&config);
    let mut gauge = GrooveGauge::new(model, gauge_type, &rule.gauge);

    let lp = LaneProperty::new(&mode);
    let physical_key_count = lp.key_lane_assign().len();

    let mut sorted_log: Vec<&KeyInputLog> = input_log.iter().collect();
    sorted_log.sort_by_key(|e| e.time());

    let last_note_time = judge_notes
        .iter()
        .map(|n| n.time_us.max(n.end_time_us))
        .max()
        .unwrap_or(0);
    let end_time = last_note_time + TAIL_TIME;

    let mut key_states = vec![false; physical_key_count];
    let mut log_cursor = 0;

    // Prime JudgeManager for notes at time 0
    let empty_key_times = vec![NOT_SET; physical_key_count];
    jm.update(-1, &judge_notes, &key_states, &empty_key_times, &mut gauge);

    let mut time = 0i64;
    while time <= end_time {
        let mut key_changed_times = vec![NOT_SET; physical_key_count];

        while log_cursor < sorted_log.len() && sorted_log[log_cursor].time() <= time {
            let event = sorted_log[log_cursor];
            let key = event.keycode() as usize;
            if key < physical_key_count {
                key_states[key] = event.is_pressed();
                key_changed_times[key] = event.time();
            }
            log_cursor += 1;
        }

        jm.update(
            time,
            &judge_notes,
            &key_states,
            &key_changed_times,
            &mut gauge,
        );
        time += FRAME_STEP;
    }

    SimulationResult {
        score: jm.score().clone(),
        max_combo: jm.max_combo(),
        ghost: jm.ghost_as_usize(),
        gauge_value: gauge.value(),
        gauge_qualified: gauge.is_qualified(),
    }
}

/// Result of a multi-song course simulation with gauge carryover.
pub struct CourseSimulationResult {
    /// Per-song results for each completed song.
    pub stages: Vec<SimulationResult>,
    /// True if the course was completed (all songs played without gauge death).
    pub completed: bool,
}

/// Run a multi-song course simulation with gauge carryover.
///
/// Simulates each song in sequence with autoplay. After each song, the gauge
/// value is carried over to the next song. If the gauge dies (reaches 0) at the
/// end of a song, subsequent songs are skipped.
///
/// This models the real course (dan) behavior where a single GrooveGauge persists
/// across all songs in the course.
pub fn run_course_simulation(models: &[&BMSModel], gauge_type: i32) -> CourseSimulationResult {
    let mut stages = Vec::new();
    let mut carry_gauge: Option<f32> = None;

    for model in models {
        // Check if gauge died in previous song
        if let Some(prev_value) = carry_gauge
            && prev_value < 1e-6
        {
            return CourseSimulationResult {
                stages,
                completed: false,
            };
        }

        let judge_notes = model.build_judge_notes();
        let mode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
        let rule = BMSPlayerRule::for_mode(&mode);

        let config = JudgeConfig {
            notes: &judge_notes,
            mode: &mode,
            ln_type: model.lntype(),
            judge_rank: model.judgerank(),
            judge_window_rate: [100, 100, 100],
            scratch_judge_window_rate: [100, 100, 100],
            algorithm: JudgeAlgorithm::Combo,
            autoplay: true,
            judge_property: &rule.judge,
            lane_property: None,
            auto_adjust_enabled: false,
            is_play_or_practice: false,
        };

        let mut jm = JudgeManager::from_config(&config);
        let mut gauge = GrooveGauge::new(model, gauge_type, &rule.gauge);

        // Apply carried gauge value from previous song
        if let Some(prev_value) = carry_gauge {
            gauge.set_value(prev_value);
        }

        let lp = LaneProperty::new(&mode);
        let physical_key_count = lp.key_lane_assign().len();
        let key_states = vec![false; physical_key_count];
        let key_times = vec![NOT_SET; physical_key_count];

        jm.update(-1, &judge_notes, &key_states, &key_times, &mut gauge);

        let last_note_time = judge_notes
            .iter()
            .map(|n| n.time_us.max(n.end_time_us))
            .max()
            .unwrap_or(0);
        let end_time = last_note_time + TAIL_TIME;

        let mut time = 0i64;
        while time <= end_time {
            jm.update(time, &judge_notes, &key_states, &key_times, &mut gauge);
            time += FRAME_STEP;
        }

        let result = SimulationResult {
            score: jm.score().clone(),
            max_combo: jm.max_combo(),
            ghost: jm.ghost_as_usize(),
            gauge_value: gauge.value(),
            gauge_qualified: gauge.is_qualified(),
        };

        carry_gauge = Some(result.gauge_value);
        stages.push(result);
    }

    CourseSimulationResult {
        stages,
        completed: true,
    }
}

/// Run a multi-song course simulation with manual input (no autoplay).
///
/// Each song uses the provided input logs. Gauge carries over between songs.
/// If gauge dies, subsequent songs are skipped.
pub fn run_course_simulation_manual(
    models: &[&BMSModel],
    input_logs: &[&[KeyInputLog]],
    gauge_type: i32,
) -> CourseSimulationResult {
    assert_eq!(
        models.len(),
        input_logs.len(),
        "Must provide input logs for each song"
    );

    let mut stages = Vec::new();
    let mut carry_gauge: Option<f32> = None;

    for (model, input_log) in models.iter().zip(input_logs.iter()) {
        if let Some(prev_value) = carry_gauge
            && prev_value < 1e-6
        {
            return CourseSimulationResult {
                stages,
                completed: false,
            };
        }

        let judge_notes = model.build_judge_notes();
        let mode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
        let rule = BMSPlayerRule::for_mode(&mode);

        let config = JudgeConfig {
            notes: &judge_notes,
            mode: &mode,
            ln_type: model.lntype(),
            judge_rank: model.judgerank(),
            judge_window_rate: [100, 100, 100],
            scratch_judge_window_rate: [100, 100, 100],
            algorithm: JudgeAlgorithm::Combo,
            autoplay: false,
            judge_property: &rule.judge,
            lane_property: None,
            auto_adjust_enabled: false,
            is_play_or_practice: false,
        };

        let mut jm = JudgeManager::from_config(&config);
        let mut gauge = GrooveGauge::new(model, gauge_type, &rule.gauge);

        if let Some(prev_value) = carry_gauge {
            gauge.set_value(prev_value);
        }

        let lp = LaneProperty::new(&mode);
        let physical_key_count = lp.key_lane_assign().len();

        let mut sorted_log: Vec<&KeyInputLog> = input_log.iter().collect();
        sorted_log.sort_by_key(|e| e.time());

        let last_note_time = judge_notes
            .iter()
            .map(|n| n.time_us.max(n.end_time_us))
            .max()
            .unwrap_or(0);
        let end_time = last_note_time + TAIL_TIME;

        let mut key_states = vec![false; physical_key_count];
        let mut log_cursor = 0;

        let empty_key_times = vec![NOT_SET; physical_key_count];
        jm.update(-1, &judge_notes, &key_states, &empty_key_times, &mut gauge);

        let mut time = 0i64;
        while time <= end_time {
            let mut key_changed_times = vec![NOT_SET; physical_key_count];

            while log_cursor < sorted_log.len() && sorted_log[log_cursor].time() <= time {
                let event = sorted_log[log_cursor];
                let key = event.keycode() as usize;
                if key < physical_key_count {
                    key_states[key] = event.is_pressed();
                    key_changed_times[key] = event.time();
                }
                log_cursor += 1;
            }

            jm.update(
                time,
                &judge_notes,
                &key_states,
                &key_changed_times,
                &mut gauge,
            );
            time += FRAME_STEP;
        }

        let result = SimulationResult {
            score: jm.score().clone(),
            max_combo: jm.max_combo(),
            ghost: jm.ghost_as_usize(),
            gauge_value: gauge.value(),
            gauge_qualified: gauge.is_qualified(),
        };

        carry_gauge = Some(result.gauge_value);
        stages.push(result);
    }

    CourseSimulationResult {
        stages,
        completed: true,
    }
}

/// Assert the autoplay invariant: all notes are PGREAT.
pub fn assert_all_pgreat(result: &SimulationResult, total_notes: usize, label: &str) {
    let score = &result.score;
    let pg_count = score.judge_count_total(JUDGE_PG);

    assert!(
        pg_count > 0,
        "{label}: expected PG count > 0, got {pg_count}"
    );

    for &judge in &[JUDGE_GR, JUDGE_GD, JUDGE_BD, JUDGE_PR, JUDGE_MS] {
        assert_eq!(
            score.judge_count_total(judge),
            0,
            "{label}: judge {judge} should be 0, got {} \
             (PG={}, GR={}, GD={}, BD={}, PR={}, MS={})",
            score.judge_count_total(judge),
            score.judge_count_total(JUDGE_PG),
            score.judge_count_total(JUDGE_GR),
            score.judge_count_total(JUDGE_GD),
            score.judge_count_total(JUDGE_BD),
            score.judge_count_total(JUDGE_PR),
            score.judge_count_total(JUDGE_MS),
        );
    }

    assert!(
        result.max_combo >= total_notes as i32,
        "{label}: max_combo {} < total_notes {}",
        result.max_combo,
        total_notes
    );

    for (i, &g) in result.ghost.iter().enumerate() {
        assert_eq!(
            g, JUDGE_PG as usize,
            "{label}: ghost[{i}] = {g}, expected PG (0)"
        );
    }

    assert!(
        result.gauge_qualified,
        "{label}: gauge not qualified (value={})",
        result.gauge_value
    );
}
