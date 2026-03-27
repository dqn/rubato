// Golden master tests for replay data.
//
// Compares Rust implementation output against Java-generated fixtures.
//
// NOTE: Ghost decode test (compare_lr2_ghost_decode) is NOT included here because
// it requires beatoraja-ir which is not a dependency of the golden-master crate.
// That test remains in tests/pending/compare_replay.rs.

use std::path::Path;

use serde::Deserialize;

use rubato_game::core::pattern::lr2_random::LR2Random;
use rubato_types::KeyInputLog;
use rubato_types::replay_data::ReplayData;
use rubato_types::validatable::Validatable;

// =========================================================================
// LR2Random tests
// =========================================================================

#[derive(Deserialize)]
struct LR2RandomCase {
    seed: i64,
    raw_sequence: Vec<u64>,
    next_int: Vec<NextIntEntry>,
}

#[derive(Deserialize)]
struct NextIntEntry {
    bound: i32,
    values: Vec<i32>,
}

#[test]
fn compare_lr2_random() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/lr2_random.json");
    let data = std::fs::read_to_string(&fixture_path).expect("Failed to read lr2_random.json");
    let cases: Vec<LR2RandomCase> = serde_json::from_str(&data).expect("Failed to parse fixture");

    let mut total_tests = 0;
    for case in &cases {
        let seed = case.seed as i32;
        // Test raw sequence (700 values, crosses N=624 buffer boundary)
        let mut rng = LR2Random::with_seed(seed);
        for (i, &expected) in case.raw_sequence.iter().enumerate() {
            let actual = rng.rand_mt() as u32 as u64;
            assert_eq!(
                actual, expected,
                "LR2Random raw_sequence mismatch at seed={}, index={}: got {} expected {}",
                seed, i, actual, expected
            );
            total_tests += 1;
        }

        // Test nextInt with various bounds
        let mut rng2 = LR2Random::with_seed(seed);
        for entry in &case.next_int {
            for (i, &expected) in entry.values.iter().enumerate() {
                let actual = rng2.next_int(entry.bound);
                assert_eq!(
                    actual, expected,
                    "LR2Random nextInt mismatch at seed={}, bound={}, index={}: got {} expected {}",
                    seed, entry.bound, i, actual, expected
                );
                total_tests += 1;
            }
        }
    }
    println!(
        "LR2Random: {} seeds x raw+nextInt = {} assertions passed",
        cases.len(),
        total_tests
    );
}

// =========================================================================
// Keylog shrink/validate round-trip tests
// =========================================================================

#[derive(Deserialize)]
struct KeylogCase {
    name: String,
    keylog: Vec<KeylogEntry>,
    keyinput: String,
    validated: Vec<KeylogEntry>,
}

#[derive(Deserialize)]
struct KeylogEntry {
    presstime: i64,
    keycode: i32,
    pressed: bool,
}

#[test]
fn compare_replay_keylog_round_trip() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/replay_keylog.json");
    let data = std::fs::read_to_string(&fixture_path).expect("Failed to read replay_keylog.json");
    let cases: Vec<KeylogCase> = serde_json::from_str(&data).expect("Failed to parse fixture");

    let mut passed = 0;
    for case in &cases {
        // Test 1: Rust shrink -> validate round-trip preserves keylog
        let keylog: Vec<KeyInputLog> = case
            .keylog
            .iter()
            .map(|e| KeyInputLog {
                time: e.presstime,
                keycode: e.keycode,
                pressed: e.pressed,
            })
            .collect();

        let mut replay = ReplayData {
            keylog: keylog.clone(),
            ..Default::default()
        };
        replay.shrink();
        assert!(replay.keyinput.is_some(), "shrink should produce keyinput");
        assert!(replay.keylog.is_empty(), "shrink should clear keylog");

        replay.validate();
        assert!(replay.keyinput.is_none(), "validate should clear keyinput");
        assert_eq!(
            replay.keylog.len(),
            keylog.len(),
            "round-trip should preserve keylog length for '{}'",
            case.name
        );

        for (i, (actual, original)) in replay.keylog.iter().zip(keylog.iter()).enumerate() {
            assert_eq!(
                actual.time, original.time,
                "time mismatch at index {} for '{}'",
                i, case.name
            );
            assert_eq!(
                actual.keycode, original.keycode,
                "keycode mismatch at index {} for '{}'",
                i, case.name
            );
            assert_eq!(
                actual.pressed, original.pressed,
                "pressed mismatch at index {} for '{}'",
                i, case.name
            );
        }

        // Test 2: Java keyinput -> Rust validate should produce same keylog as Java validated
        let mut replay2 = ReplayData {
            keyinput: Some(case.keyinput.clone()),
            ..Default::default()
        };
        replay2.validate();

        assert_eq!(
            replay2.keylog.len(),
            case.validated.len(),
            "Java keyinput validate should produce same length for '{}'",
            case.name
        );

        for (i, (actual, expected)) in replay2.keylog.iter().zip(case.validated.iter()).enumerate()
        {
            assert_eq!(
                actual.time, expected.presstime,
                "Java validate time mismatch at index {} for '{}'",
                i, case.name
            );
            assert_eq!(
                actual.keycode, expected.keycode,
                "Java validate keycode mismatch at index {} for '{}'",
                i, case.name
            );
            assert_eq!(
                actual.pressed, expected.pressed,
                "Java validate pressed mismatch at index {} for '{}'",
                i, case.name
            );
        }

        passed += 1;
    }
    println!("Keylog round-trip: {} cases passed", passed);
}

// =========================================================================
// Lane order tests
// =========================================================================

#[derive(Deserialize)]
struct LaneOrderCase {
    seed: i32,
    encoded_lanes: i32,
}

#[test]
fn compare_lr2_lane_order() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/lr2_lane_order.json");
    let data = std::fs::read_to_string(&fixture_path).expect("Failed to read lr2_lane_order.json");
    let cases: Vec<LaneOrderCase> = serde_json::from_str(&data).expect("Failed to parse fixture");

    let mut passed = 0;
    for case in &cases {
        // Replicate the lane ordering computation from LR2GhostData
        let mut rng = LR2Random::with_seed(case.seed);
        let mut targets = [0i32, 1, 2, 3, 4, 5, 6, 7];
        for lane in 1..7 {
            let swap = lane + rng.next_int(7 - lane as i32 + 1) as usize;
            targets.swap(lane, swap);
        }
        let mut lanes = [0i32, 1, 2, 3, 4, 5, 6, 7];
        for i in 1..8 {
            lanes[targets[i] as usize] = i as i32;
        }
        let mut encoded = 0i32;
        for &lane_val in &lanes[1..8] {
            encoded = encoded * 10 + lane_val;
        }

        assert_eq!(
            encoded, case.encoded_lanes,
            "Lane order mismatch for seed {}: got {} expected {}",
            case.seed, encoded, case.encoded_lanes
        );
        passed += 1;
    }
    println!("Lane order: {} cases passed", passed);
}
