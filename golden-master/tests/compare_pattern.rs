// Golden master tests for Phase 3: Pattern Shuffle
//
// Compares Rust lane shuffle implementations against Java fixture output.

use std::path::Path;

use bms::model::bms_model::BMSModel;
use bms::model::mode::Mode;
use golden_master::pattern_fixtures::LaneShuffleFixture;
use rubato_game::core::pattern::lane_shuffle_modifier::{
    LaneCrossShuffleModifier, LaneMirrorShuffleModifier, LaneRandomShuffleModifier,
    LaneRotateShuffleModifier, PlayerFlipModifier,
};

fn fixture_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .leak()
}

/// Create a minimal BMSModel with the specified mode for shuffle testing.
fn make_model_for_mode(mode: &Mode) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(*mode);
    model
}

// =========================================================================
// Lane Shuffle Mapping Tests
// =========================================================================

#[test]
fn golden_master_lane_shuffle_mappings() {
    let fixture_path = fixture_dir().join("pattern_lane_shuffle.json");
    if !fixture_path.exists() {
        eprintln!(
            "Fixture not found: {}. Run `just golden-master-pattern-gen` first.",
            fixture_path.display()
        );
        return;
    }

    let content = std::fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let fixture: LaneShuffleFixture =
        serde_json::from_str(&content).expect("Failed to parse fixture");

    let mut pass = 0;
    let mut fail = 0;

    for (i, tc) in fixture.test_cases.iter().enumerate() {
        let mode = golden_master::mode_hint_to_mode(&tc.mode)
            .unwrap_or_else(|| panic!("Unknown mode: {}", tc.mode));

        let model = make_model_for_mode(&mode);
        let keys: Vec<i32> = tc.keys.iter().map(|&k| k as i32).collect();
        let seed = tc.seed.unwrap_or(0);

        let rust_mapping_i32 = match tc.modifier_type.as_str() {
            "MIRROR" => LaneMirrorShuffleModifier::make_random(&keys, &model, seed),
            "ROTATE" => LaneRotateShuffleModifier::make_random(&keys, &model, seed),
            "RANDOM" => LaneRandomShuffleModifier::make_random(&keys, &model, seed),
            "CROSS" => LaneCrossShuffleModifier::make_random(&keys, &model, seed),
            "FLIP" => PlayerFlipModifier::make_random(&keys, &model, seed),
            other => panic!("Unknown modifier type: {other}"),
        };

        // Convert i32 mapping to usize for comparison with fixture
        let rust_mapping: Vec<usize> = rust_mapping_i32.iter().map(|&v| v as usize).collect();

        if rust_mapping == tc.mapping {
            pass += 1;
        } else {
            fail += 1;
            eprintln!(
                "FAIL case[{i}] {modifier} mode={mode:?} seed={seed:?} scratch={scratch} player={player}",
                modifier = tc.modifier_type,
                mode = tc.mode,
                seed = tc.seed,
                scratch = tc.contains_scratch,
                player = tc.player,
            );
            eprintln!("  expected: {:?}", tc.mapping);
            eprintln!("  actual:   {:?}", rust_mapping);
        }
    }

    println!(
        "\nLane shuffle mapping results: {pass} passed, {fail} failed (total {})",
        fixture.test_cases.len()
    );
    assert_eq!(fail, 0, "{fail} lane shuffle mapping test(s) failed");
}
