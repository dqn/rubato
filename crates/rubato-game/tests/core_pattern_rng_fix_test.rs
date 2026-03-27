// Tests for RNG behavior in pattern modifiers.
//
// LongNoteModifier, MineNoteModifier, and ScrollSpeedModifier use non-deterministic
// rand::random() to match Java's Math.random() behavior. These modifiers intentionally
// do NOT use seeded JavaRandom because Java's source uses Math.random(), not
// new Random(seed).nextDouble().
//
// Deterministic seeded JavaRandom is only used by Randomizer (lane shuffling) and
// other modifiers that use java.util.Random in the Java source.

use bms::model::bms_model::BMSModel;
use bms::model::mode::Mode;
use bms::model::note::Note;
use bms::model::time_line::TimeLine;
use rubato_game::core::pattern::long_note_modifier::LongNoteModifier;
use rubato_game::core::pattern::mine_note_modifier::MineNoteModifier;
use rubato_game::core::pattern::pattern_modifier::PatternModifier;

fn make_test_model(mode: &Mode, timelines: Vec<TimeLine>) -> BMSModel {
    let mut model = BMSModel::new();
    model.timelines = timelines;
    model.set_mode(*mode);
    model
}

/// MineNoteModifier.modify() in AddRandom mode uses non-deterministic Math.random()
/// equivalent. Verify it produces mine notes (probabilistic, not deterministic).
#[test]
fn mine_note_modifier_add_random_produces_expected_distribution() {
    let mode = Mode::BEAT_7K;

    let build_model = || {
        let mut timelines = Vec::new();
        for section in 0..200 {
            let mut tl = TimeLine::new(section as f64, section * 1000, 8);
            tl.set_note(0, Some(Note::new_normal(10)));
            timelines.push(tl);
        }
        make_test_model(&mode, timelines)
    };

    let mut model = build_model();
    let mut modifier = MineNoteModifier::with_mode(1); // AddRandom
    modifier.modify(&mut model);

    // Count mine notes placed in blank lanes (lanes 1-7 are blank)
    let mine_count: usize = model
        .timelines
        .iter()
        .map(|tl| {
            (1..8)
                .filter(|&lane| tl.note(lane).is_some_and(|n| n.is_mine()))
                .count()
        })
        .sum();

    // With 200 timelines * 7 blank lanes = 1400 opportunities at 10% probability,
    // expected ~140 mines. Verify at least some mines were placed.
    assert!(
        mine_count > 0,
        "MineNoteModifier AddRandom should place mine notes (got 0 out of 1400 opportunities)"
    );
}

/// LongNoteModifier.modify() uses non-deterministic Math.random() equivalent.
/// With rate=0.5, approximately half the LNs should be removed (not all or none).
#[test]
fn long_note_modifier_remove_mode_partial_removal() {
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

    let mut model = build_model();
    let mut modifier = LongNoteModifier::with_params(0, 0.5); // Remove mode, 50% rate
    modifier.modify(&mut model);

    // Count remaining long note starts on even timelines
    let remaining_ln: usize = model
        .timelines
        .iter()
        .map(|tl| {
            (0..7)
                .filter(|&lane| tl.note(lane).is_some_and(|n| n.is_long() && !n.is_end()))
                .count()
        })
        .sum();

    // Original: 50 timelines * 7 lanes = 350 LN starts
    // With rate=0.5 and non-deterministic RNG, some but not all should be removed.
    let total_original = 350;
    assert!(
        remaining_ln > 0 && remaining_ln < total_original,
        "With rate=0.5, some but not all LNs should be removed. Got {} remaining out of {}",
        remaining_ln,
        total_original
    );
}
