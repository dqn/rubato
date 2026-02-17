// Golden master tests for Phase 3: Pattern Shuffle
//
// Compares Rust lane shuffle implementations against Java fixture output.

use std::path::Path;

use golden_master::pattern_fixtures::{
    BattleFixture, BattleNote, LaneShuffleFixture, PlayableRandomFixture,
};

use bms_pattern::lane_shuffle::{
    LaneCrossShuffle, LaneMirrorShuffle, LanePlayableRandomShuffle, LaneRandomShuffle,
    LaneRotateShuffle, PlayerBattleShuffle, PlayerFlipShuffle,
};
use bms_pattern::modifier::{PatternModifier, get_keys};

fn fixture_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .leak()
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
        let mode = golden_master::mode_hint_to_play_mode(&tc.mode)
            .unwrap_or_else(|| panic!("Unknown mode: {}", tc.mode));

        let rust_mapping = match tc.modifier_type.as_str() {
            "MIRROR" => {
                let shuffle = LaneMirrorShuffle::new(tc.player, tc.contains_scratch);
                let keys = get_keys(mode, tc.player, tc.contains_scratch);
                shuffle.make_random(&keys, tc.key_count)
            }
            "ROTATE" => {
                let seed = tc.seed.expect("Rotate requires seed");
                let shuffle = LaneRotateShuffle::new(tc.player, tc.contains_scratch, seed);
                let keys = get_keys(mode, tc.player, tc.contains_scratch);
                shuffle.make_random(&keys, tc.key_count)
            }
            "RANDOM" => {
                let seed = tc.seed.expect("Random requires seed");
                let shuffle = LaneRandomShuffle::new(tc.player, tc.contains_scratch, seed);
                let keys = get_keys(mode, tc.player, tc.contains_scratch);
                shuffle.make_random(&keys, tc.key_count)
            }
            "CROSS" => {
                let shuffle = LaneCrossShuffle::new(tc.player, tc.contains_scratch);
                let keys = get_keys(mode, tc.player, tc.contains_scratch);
                shuffle.make_random(&keys, tc.key_count)
            }
            "FLIP" => {
                let shuffle = PlayerFlipShuffle::new();
                shuffle.make_random(tc.key_count, mode.player_count())
            }
            other => panic!("Unknown modifier type: {other}"),
        };

        if rust_mapping == tc.mapping {
            pass += 1;
        } else {
            fail += 1;
            eprintln!(
                "FAIL case[{i}] {modifier} mode={mode} seed={seed:?} scratch={scratch} player={player}",
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

// =========================================================================
// Playable Random Tests
// =========================================================================

#[test]
fn golden_master_playable_random() {
    let fixture_path = fixture_dir().join("pattern_playable_random.json");
    if !fixture_path.exists() {
        eprintln!(
            "Fixture not found: {}. Run `just golden-master-pattern-gen` first.",
            fixture_path.display()
        );
        return;
    }

    let content = std::fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let fixture: PlayableRandomFixture =
        serde_json::from_str(&content).expect("Failed to parse fixture");

    let mut pass = 0;
    let mut fail = 0;

    for (i, tc) in fixture.test_cases.iter().enumerate() {
        let mode = golden_master::mode_hint_to_play_mode(&tc.mode)
            .unwrap_or_else(|| panic!("Unknown mode: {}", tc.mode));

        // Build a minimal BmsModel with the chord patterns encoded as notes
        let model = build_model_from_chord_patterns(mode, &tc.chord_patterns);

        let keys = get_keys(mode, 0, false);
        let shuffle = LanePlayableRandomShuffle::new(0, false, tc.seed);
        let rust_mapping = shuffle.make_random(&keys, &model);

        // Verify candidate count by running the search directly
        let candidates =
            bms_pattern::lane_shuffle::search_no_murioshi_combinations(&tc.chord_patterns);
        let rust_candidate_count = candidates.len();

        let mapping_ok = rust_mapping == tc.mapping;
        let count_ok = rust_candidate_count == tc.candidate_count;

        if mapping_ok && count_ok {
            pass += 1;
        } else {
            fail += 1;
            eprintln!(
                "FAIL case[{i}] seed={seed} is_fallback={fallback}",
                seed = tc.seed,
                fallback = tc.is_fallback,
            );
            if !mapping_ok {
                eprintln!("  mapping expected: {:?}", tc.mapping);
                eprintln!("  mapping actual:   {:?}", rust_mapping);
            }
            if !count_ok {
                eprintln!(
                    "  candidate_count expected: {}, actual: {}",
                    tc.candidate_count, rust_candidate_count
                );
            }
        }
    }

    println!(
        "\nPlayable random results: {pass} passed, {fail} failed (total {})",
        fixture.test_cases.len()
    );
    assert_eq!(fail, 0, "{fail} playable random test(s) failed");
}

// =========================================================================
// Battle Tests
// =========================================================================

#[test]
fn golden_master_battle() {
    let fixture_path = fixture_dir().join("pattern_battle.json");
    if !fixture_path.exists() {
        eprintln!(
            "Fixture not found: {}. Run `just golden-master-pattern-gen` first.",
            fixture_path.display()
        );
        return;
    }

    let content = std::fs::read_to_string(&fixture_path).expect("Failed to read fixture");
    let fixture: BattleFixture = serde_json::from_str(&content).expect("Failed to parse fixture");

    let mut pass = 0;
    let mut fail = 0;

    for (i, tc) in fixture.test_cases.iter().enumerate() {
        let mode = golden_master::mode_hint_to_play_mode(&tc.mode)
            .unwrap_or_else(|| panic!("Unknown mode: {}", tc.mode));

        // Build a BmsModel from input_notes
        let mut model = build_model_from_battle_notes(mode, &tc.input_notes);

        // Apply Battle modifier
        let mut shuffle = PlayerBattleShuffle::new();
        shuffle.modify(&mut model);

        // Extract output notes and sort by (time_us, lane)
        let mut rust_notes: Vec<BattleNote> = model
            .notes
            .iter()
            .map(|n| {
                let end_time_us = if n.is_long_note() && n.end_time_us > 0 {
                    Some(n.end_time_us)
                } else if n.is_long_note() && n.end_time_us == 0 {
                    Some(-1i64)
                } else {
                    None
                };
                BattleNote {
                    lane: n.lane,
                    time_us: n.time_us,
                    note_type: format!("{:?}", n.note_type),
                    wav_id: n.wav_id as u32,
                    end_time_us,
                }
            })
            .collect();
        rust_notes.sort_by_key(|n| (n.time_us, n.lane));

        let mut expected_notes = tc.output_notes.clone();
        expected_notes.sort_by_key(|n| (n.time_us, n.lane));

        if rust_notes == expected_notes {
            pass += 1;
        } else {
            fail += 1;
            eprintln!("FAIL case[{i}] name={name}", name = tc.name,);
            eprintln!("  expected ({} notes):", expected_notes.len());
            for n in &expected_notes {
                eprintln!(
                    "    lane={} time={} type={} wav={} end={:?}",
                    n.lane, n.time_us, n.note_type, n.wav_id, n.end_time_us
                );
            }
            eprintln!("  actual ({} notes):", rust_notes.len());
            for n in &rust_notes {
                eprintln!(
                    "    lane={} time={} type={} wav={} end={:?}",
                    n.lane, n.time_us, n.note_type, n.wav_id, n.end_time_us
                );
            }
        }
    }

    println!(
        "\nBattle results: {pass} passed, {fail} failed (total {})",
        fixture.test_cases.len()
    );
    assert_eq!(fail, 0, "{fail} battle test(s) failed");
}

/// Build a BmsModel from BattleNote test data.
fn build_model_from_battle_notes(
    mode: bms_model::PlayMode,
    battle_notes: &[BattleNote],
) -> bms_model::BmsModel {
    let mut notes = Vec::new();

    for bn in battle_notes {
        let note = match bn.note_type.as_str() {
            "Normal" => bms_model::Note::normal(bn.lane, bn.time_us, bn.wav_id as u16),
            "LongNote" => {
                let end_time_us = bn.end_time_us.unwrap_or(0);
                if end_time_us > 0 {
                    // LN start
                    bms_model::Note::long_note(
                        bn.lane,
                        bn.time_us,
                        end_time_us,
                        bn.wav_id as u16,
                        0,
                        bms_model::LnType::LongNote,
                    )
                } else {
                    // LN end (end_time_us == -1 or 0)
                    bms_model::Note::long_note(
                        bn.lane,
                        bn.time_us,
                        0,
                        bn.wav_id as u16,
                        0,
                        bms_model::LnType::LongNote,
                    )
                }
            }
            other => panic!("Unknown note type in fixture: {other}"),
        };
        notes.push(note);
    }

    // Build LN pair indices for 1P notes
    // Match start (end_time_us > 0) with end (end_time_us == 0) on same lane
    let starts: Vec<usize> = notes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.is_long_note() && n.end_time_us > 0)
        .map(|(i, _)| i)
        .collect();

    for &si in &starts {
        let lane = notes[si].lane;
        let end_time = notes[si].end_time_us;
        if let Some(ei) = notes.iter().enumerate().position(|(i, n)| {
            i != si
                && n.lane == lane
                && n.is_long_note()
                && n.time_us == end_time
                && n.end_time_us == 0
        }) {
            notes[si].pair_index = ei;
            notes[ei].pair_index = si;
        }
    }

    bms_model::BmsModel {
        mode,
        notes,
        ..Default::default()
    }
}

/// Build a minimal BmsModel with notes that produce the given chord patterns.
///
/// Each chord pattern is a bitmask where bit `j` means lane `j` has a note.
/// We place one timeline per pattern, with Normal notes on the active lanes.
fn build_model_from_chord_patterns(
    mode: bms_model::PlayMode,
    chord_patterns: &[u32],
) -> bms_model::BmsModel {
    let mut notes = Vec::new();
    let base_time = 1_000_000i64; // 1 second

    for (idx, &pattern) in chord_patterns.iter().enumerate() {
        let time_us = base_time + (idx as i64) * 100_000;
        for lane in 0..9 {
            if (pattern >> lane) & 1 == 1 {
                notes.push(bms_model::Note::normal(lane, time_us, 1));
            }
        }
    }

    bms_model::BmsModel {
        judge_rank: 100,
        judge_rank_raw: 100,
        judge_rank_type: bms_model::JudgeRankType::BmsonJudgeRank,
        mode,
        notes,
        total_measures: 4,
        ..Default::default()
    }
}
