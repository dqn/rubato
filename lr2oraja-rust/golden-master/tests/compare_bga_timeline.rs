// Golden master tests: BgaProcessor timeline against parsed BMS data.
//
// Loads bga_test.bms, builds BGAProcessor, and verifies BGA/layer state
// at known time points. Uses programmatic verification (not Java fixture
// comparison) because no Java BGA exporter exists yet.
//
// Known semantic differences (documented in AGENTS.md):
//   - Channel 06/07 swap: Rust parser maps ch06→Poor, ch07→Layer.
//     The BMS spec (and Java/beatoraja) also maps ch06→Poor, ch07→Layer.
//     This test exercises both BGA (ch04) and Layer (ch07).

use std::path::{Path, PathBuf};

use beatoraja_play::bga::bga_processor::BGAProcessor;
use bms_model::bms_decoder::BMSDecoder;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_bms_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test-bms")
}

// ===========================================================================
// BGA timeline tests (using bga_test.bms)
// ===========================================================================
//
// bga_test.bms layout (BPM=120, 1 measure = 2000ms = 2_000_000μs):
//
//   #BMP01 bg1.bmp      -> bgamap index 0
//   #BMP02 bg2.bmp      -> bgamap index 1
//   #BMP03 bg3.bmp      -> bgamap index 2
//   #BMP04 layer1.bmp   -> bgamap index 3
//   #BMP05 layer2.bmp   -> bgamap index 4
//
//   Measure 1 (t=2000ms): BGA=01 -> set_bga(0)
//   Measure 2 (t=4000ms): BGA=02 -> set_bga(1)
//   Measure 2 mid (t=5000ms): Layer=04 -> set_layer(3)
//   Measure 3 (t=6000ms): BGA=03 -> set_bga(2), Layer=05 -> set_layer(4)
//   Measure 4 (t=8000ms): BGA=01 -> set_bga(0)

#[test]
fn bga_parse_model_has_bga_data() {
    let path = test_bms_dir().join("bga_test.bms");
    let model = BMSDecoder::new()
        .decode_path(&path)
        .expect("Failed to parse BMS");

    // BGA list should contain the BMP definitions
    let bga_list = model.get_bga_list();
    assert_eq!(bga_list.len(), 5, "should have 5 BMP definitions");
    assert_eq!(bga_list[0], "bg1.bmp");
    assert_eq!(bga_list[1], "bg2.bmp");
    assert_eq!(bga_list[2], "bg3.bmp");
    assert_eq!(bga_list[3], "layer1.bmp");
    assert_eq!(bga_list[4], "layer2.bmp");

    // Timelines should contain BGA data
    let timelines_with_bga: Vec<_> = model
        .get_all_time_lines()
        .iter()
        .filter(|tl| tl.get_bga() != -1 || tl.get_layer() != -1)
        .collect();
    assert!(
        timelines_with_bga.len() >= 5,
        "should have at least 5 timelines with BGA/layer data, got {}",
        timelines_with_bga.len()
    );
}

#[test]
fn bga_processor_initial_state() {
    let path = test_bms_dir().join("bga_test.bms");
    let model = BMSDecoder::new()
        .decode_path(&path)
        .expect("Failed to parse BMS");

    let proc = BGAProcessor::from_model(&model);
    assert_eq!(proc.current_bga_id(), -1, "initial BGA should be -1");
    assert_eq!(proc.current_layer_id(), -1, "initial layer should be -1");
}

#[test]
fn bga_processor_before_first_event() {
    let path = test_bms_dir().join("bga_test.bms");
    let model = BMSDecoder::new()
        .decode_path(&path)
        .expect("Failed to parse BMS");

    let mut proc = BGAProcessor::from_model(&model);
    // Before measure 1 (first BGA at t=2000ms)
    proc.update(1_000_000); // 1000ms
    assert_eq!(proc.current_bga_id(), -1, "no BGA before measure 1");
    assert_eq!(proc.current_layer_id(), -1, "no layer before measure 1");
}

#[test]
fn bga_processor_measure_transitions() {
    let path = test_bms_dir().join("bga_test.bms");
    let model = BMSDecoder::new()
        .decode_path(&path)
        .expect("Failed to parse BMS");

    let mut proc = BGAProcessor::from_model(&model);

    // After measure 1 start: BGA=01 -> bga_id=0
    proc.update(2_500_000); // 2500ms
    assert_eq!(
        proc.current_bga_id(),
        0,
        "measure 1: BGA should be 0 (bg1.bmp)"
    );
    assert_eq!(proc.current_layer_id(), -1, "measure 1: no layer yet");

    // After measure 2 start: BGA=02 -> bga_id=1
    proc.update(4_500_000); // 4500ms
    assert_eq!(
        proc.current_bga_id(),
        1,
        "measure 2: BGA should be 1 (bg2.bmp)"
    );
    assert_eq!(proc.current_layer_id(), -1, "measure 2 start: no layer yet");

    // After measure 2 midpoint: Layer=04 -> layer_id=3
    proc.update(5_500_000); // 5500ms
    assert_eq!(proc.current_bga_id(), 1, "measure 2 mid: BGA unchanged");
    assert_eq!(
        proc.current_layer_id(),
        3,
        "measure 2 mid: layer should be 3 (layer1.bmp)"
    );

    // After measure 3 start: BGA=03 -> bga_id=2, Layer=05 -> layer_id=4
    proc.update(6_500_000); // 6500ms
    assert_eq!(
        proc.current_bga_id(),
        2,
        "measure 3: BGA should be 2 (bg3.bmp)"
    );
    assert_eq!(
        proc.current_layer_id(),
        4,
        "measure 3: layer should be 4 (layer2.bmp)"
    );

    // After measure 4 start: BGA=01 -> bga_id=0 (cycles back)
    proc.update(8_500_000); // 8500ms
    assert_eq!(
        proc.current_bga_id(),
        0,
        "measure 4: BGA should be 0 (bg1.bmp again)"
    );
    // Layer persists from measure 3
    assert_eq!(proc.current_layer_id(), 4, "measure 4: layer persists");
}

#[test]
fn bga_processor_reset_via_prepare() {
    let path = test_bms_dir().join("bga_test.bms");
    let model = BMSDecoder::new()
        .decode_path(&path)
        .expect("Failed to parse BMS");

    let mut proc = BGAProcessor::from_model(&model);

    // Advance past some events
    proc.update(6_500_000);
    assert_eq!(proc.current_bga_id(), 2);

    // Reset via prepare()
    proc.prepare(&());
    assert_eq!(proc.current_bga_id(), -1, "BGA reset after prepare");
    assert_eq!(proc.current_layer_id(), -1, "layer reset after prepare");

    // Can replay from the start
    proc.update(2_500_000);
    assert_eq!(proc.current_bga_id(), 0, "BGA replays after reset");
}
