// Tests for the JavaRandom fix: these assert CORRECT behavior.
// Before the fix: these tests FAIL (non-determinism due to rand::random()).
// After the fix: these tests PASS (seeded JavaRandom produces deterministic output).

use beatoraja_core::pattern::long_note_modifier::LongNoteModifier;
use beatoraja_core::pattern::mine_note_modifier::MineNoteModifier;
use beatoraja_core::pattern::pattern_modifier::PatternModifier;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;

fn make_test_model(mode: &Mode, timelines: Vec<TimeLine>) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_all_time_line(timelines);
    model.set_mode(mode.clone());
    model
}

/// MineNoteModifier.modify() in AddRandom mode MUST be deterministic
/// when the same seed is used.
#[test]
fn mine_note_modifier_deterministic_with_seed() {
    let seed: i64 = 42;
    let mode = Mode::BEAT_7K;

    let build_model = || {
        let mut timelines = Vec::new();
        for section in 0..50 {
            let mut tl = TimeLine::new(section as f64, section * 1000, 8);
            tl.set_note(0, Some(Note::new_normal(10)));
            timelines.push(tl);
        }
        make_test_model(&mode, timelines)
    };

    let extract_notes = |model: &BMSModel| -> Vec<Vec<Option<i32>>> {
        model
            .get_all_time_lines()
            .iter()
            .map(|tl| {
                (0..8)
                    .map(|lane| tl.get_note(lane).map(|n| n.get_wav()))
                    .collect()
            })
            .collect()
    };

    // Run 1
    let mut model1 = build_model();
    let mut modifier1 = MineNoteModifier::with_mode(1); // AddRandom
    modifier1.set_seed(seed);
    modifier1.modify(&mut model1);
    let notes1 = extract_notes(&model1);

    // Run 2 — same seed, same input
    let mut model2 = build_model();
    let mut modifier2 = MineNoteModifier::with_mode(1);
    modifier2.set_seed(seed);
    modifier2.modify(&mut model2);
    let notes2 = extract_notes(&model2);

    assert_eq!(
        notes1, notes2,
        "MineNoteModifier with same seed must produce identical output"
    );
}

/// LongNoteModifier.modify() with rate != 1.0 MUST be deterministic
/// when the same seed is used.
#[test]
fn long_note_modifier_deterministic_with_seed() {
    let seed: i64 = 42;
    let mode = Mode::BEAT_7K;

    let build_model = || {
        let mut timelines = Vec::new();
        for section in 0..100 {
            let mut tl = TimeLine::new(section as f64, section * 1000, 8);
            if section % 2 == 0 {
                for lane in 0..7 {
                    let mut ln = Note::new_long(10 + lane);
                    ln.set_long_note_type(1);
                    tl.set_note(lane, Some(ln));
                }
            } else {
                for lane in 0..7 {
                    let mut end = Note::new_long(-2);
                    end.set_end(true);
                    tl.set_note(lane, Some(end));
                }
            }
            timelines.push(tl);
        }
        make_test_model(&mode, timelines)
    };

    let extract_notes = |model: &BMSModel| -> Vec<Vec<Option<i32>>> {
        model
            .get_all_time_lines()
            .iter()
            .map(|tl| {
                (0..8)
                    .map(|lane| tl.get_note(lane).map(|n| n.get_wav()))
                    .collect()
            })
            .collect()
    };

    // Run 1
    let mut model1 = build_model();
    let mut modifier1 = LongNoteModifier::with_params(0, 0.5); // Remove mode, 50% rate
    modifier1.set_seed(seed);
    modifier1.modify(&mut model1);
    let notes1 = extract_notes(&model1);

    // Run 2 — same seed, same input
    let mut model2 = build_model();
    let mut modifier2 = LongNoteModifier::with_params(0, 0.5);
    modifier2.set_seed(seed);
    modifier2.modify(&mut model2);
    let notes2 = extract_notes(&model2);

    assert_eq!(
        notes1, notes2,
        "LongNoteModifier with same seed must produce identical output"
    );
}
