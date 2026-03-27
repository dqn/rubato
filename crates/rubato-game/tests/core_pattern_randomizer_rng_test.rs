// Phase 49: Randomizer RNG divergence tests
//
// Demonstrates that StdRng and JavaRandom produce completely different
// sequences from the same seed, and verifies JavaRandom determinism.
//
// Phase 51: Tests verifying RNG correctness after the StdRng -> JavaRandom fix.

use bms::model::bms_model::BMSModel;
use bms::model::mode::Mode;
use bms::model::note::Note;
use bms::model::time_line::TimeLine;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rubato_game::core::pattern::java_random::JavaRandom;
use rubato_game::core::pattern::long_note_modifier::LongNoteModifier;
use rubato_game::core::pattern::mine_note_modifier::MineNoteModifier;
use rubato_game::core::pattern::note_shuffle_modifier::NoteShuffleModifier;
use rubato_game::core::pattern::pattern_modifier::PatternModifier;
use rubato_game::core::pattern::random::Random;
use rubato_game::core::pattern::randomizer::RandomizerBase;
use rubato_game::core::player_config::PlayerConfig;

/// Phase 49a: StdRng vs JavaRandom diverge from the same seed.
/// This is the core invariant: beatoraja MUST use JavaRandom (LCG),
/// never StdRng, to match Java replay/pattern determinism.
#[test]
fn std_rng_vs_java_random_diverge() {
    let seed: u64 = 42;

    let mut std_rng = StdRng::seed_from_u64(seed);
    let mut java_rng = JavaRandom::new(seed as i64);

    let mut std_results = Vec::with_capacity(100);
    let mut java_results = Vec::with_capacity(100);

    for _ in 0..100 {
        std_results.push(std_rng.gen_range(0..7));
        java_results.push(java_rng.next_int_bounded(7));
    }

    // They MUST differ — if they don't, something is very wrong
    assert_ne!(
        std_results, java_results,
        "StdRng and JavaRandom must produce different sequences from the same seed"
    );

    // Verify they diverge from the very first value
    assert_ne!(
        std_results[0], java_results[0],
        "Even the first generated value should differ between StdRng and JavaRandom"
    );
}

/// Phase 49b: JavaRandom is deterministic — same seed always produces
/// the same sequence. This is the baseline for replay/pattern reproducibility.
#[test]
fn java_random_deterministic_same_seed() {
    let seed: i64 = 42;

    let mut rng1 = JavaRandom::new(seed);
    let mut rng2 = JavaRandom::new(seed);

    let results1: Vec<i32> = (0..100).map(|_| rng1.next_int_bounded(7)).collect();
    let results2: Vec<i32> = (0..100).map(|_| rng2.next_int_bounded(7)).collect();

    assert_eq!(
        results1, results2,
        "JavaRandom with the same seed must produce identical sequences"
    );
}

/// Verify that different seeds produce different sequences (sanity check).
#[test]
fn java_random_different_seeds_diverge() {
    let mut rng1 = JavaRandom::new(42);
    let mut rng2 = JavaRandom::new(99);

    let results1: Vec<i32> = (0..100).map(|_| rng1.next_int_bounded(7)).collect();
    let results2: Vec<i32> = (0..100).map(|_| rng2.next_int_bounded(7)).collect();

    assert_ne!(
        results1, results2,
        "JavaRandom with different seeds should produce different sequences"
    );
}

// ---- Phase 51: Tests verifying RNG correctness (post-fix) ----

/// Helper: create a BMSModel with the given mode and timelines.
fn make_test_model(mode: &Mode, timelines: Vec<TimeLine>) -> BMSModel {
    let mut model = BMSModel::new();
    model.timelines = timelines;
    model.set_mode(*mode);
    model
}

/// 4-2b: SRandomizer determinism baseline — same seed produces identical output.
///
/// Creates an SRandomizer (via NoteShuffleModifier) with the same seed twice,
/// applies it to identical note data, and verifies the outputs match.
/// Now uses JavaRandom internally, which is both deterministic and correct.
#[test]
fn s_randomizer_determinism_same_seed() {
    let seed: i64 = 12345;
    let mode = Mode::BEAT_7K;
    let config = PlayerConfig::default();

    // Build two identical models with notes spread across lanes
    let build_model = || {
        let mut timelines = Vec::new();
        for section in 0i32..4 {
            let mut tl = TimeLine::new(section as f64, section as i64 * 1000, 8);
            // Place notes in lanes 0, 2, 4 to give the randomizer something to shuffle
            tl.set_note(0, Some(Note::new_normal(10 + section)));
            tl.set_note(2, Some(Note::new_normal(20 + section)));
            tl.set_note(4, Some(Note::new_normal(30 + section)));
            timelines.push(tl);
        }
        make_test_model(&mode, timelines)
    };

    // First run
    let mut model1 = build_model();
    let mut modifier1 = NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
    modifier1.set_seed(seed);
    modifier1.modify(&mut model1);

    // Second run — same seed, same input
    let mut model2 = build_model();
    let mut modifier2 = NoteShuffleModifier::new(Random::SRandom, 0, &mode, &config);
    modifier2.set_seed(seed);
    modifier2.modify(&mut model2);

    // Extract note positions from both runs
    let extract_notes = |model: &BMSModel| -> Vec<Vec<Option<i32>>> {
        model
            .timelines
            .iter()
            .map(|tl| (0..8).map(|lane| tl.note(lane).map(|n| n.wav())).collect())
            .collect()
    };

    let notes1 = extract_notes(&model1);
    let notes2 = extract_notes(&model2);

    // Same seed -> same shuffle. JavaRandom is deterministic.
    assert_eq!(
        notes1, notes2,
        "SRandomizer with the same seed must produce identical note layouts"
    );
}

/// 4-3a: RandomizerBase now uses JavaRandom, not StdRng.
///
/// Seeds RandomizerBase with seed=42 and extracts random values via JavaRandom.
/// Compares them against an independently-seeded JavaRandom(42). They match,
/// proving the AGENTS.md invariant "JavaRandom LCG in beatoraja-pattern" holds.
#[test]
fn randomizer_base_uses_java_random() {
    let seed: i64 = 42;

    // RandomizerBase.set_random_seed() creates JavaRandom::new(seed)
    let mut base = RandomizerBase::new();
    base.set_random_seed(seed);

    // Extract 20 values from the JavaRandom inside RandomizerBase
    let base_values: Vec<i32> = (0..20).map(|_| base.random.next_int_bounded(7)).collect();

    // Now get 20 values from an independently-seeded JavaRandom
    let mut java_rng = JavaRandom::new(seed);
    let java_values: Vec<i32> = (0..20).map(|_| java_rng.next_int_bounded(7)).collect();

    // These MUST match — RandomizerBase now uses JavaRandom.
    assert_eq!(
        base_values, java_values,
        "RandomizerBase.random must produce JavaRandom sequences — \
         the JavaRandom LCG invariant from AGENTS.md is now satisfied"
    );
}

/// 4-3b: MineNoteModifier.modify() uses non-deterministic Math.random() equivalent.
///
/// In AddRandom mode (mode=1), MineNoteModifier uses rand::random() to match
/// Java's Math.random() behavior. The seed field is NOT used for this RNG.
/// This test verifies the modifier runs without errors and produces mine notes.
#[test]
fn mine_note_modifier_add_random_produces_mines() {
    let mode = Mode::BEAT_7K;

    let mut timelines = Vec::new();
    // Use enough timelines to statistically guarantee at least one mine
    // with the 10% probability (Math.random() > 0.9).
    for section in 0..200 {
        let mut tl = TimeLine::new(section as f64, section * 1000, 8);
        tl.set_note(0, Some(Note::new_normal(10)));
        timelines.push(tl);
    }
    let mut model = make_test_model(&mode, timelines);

    let mut modifier = MineNoteModifier::with_mode(1); // AddRandom
    modifier.modify(&mut model);

    // With 200 timelines and 7 blank lanes each (lanes 1-7), at 10% probability,
    // we expect ~140 mines. Verify at least one mine was placed.
    let mine_count: usize = model
        .timelines
        .iter()
        .map(|tl| {
            (0..8)
                .filter(|&lane| tl.note(lane).is_some_and(|n| n.is_mine()))
                .count()
        })
        .sum();

    assert!(
        mine_count > 0,
        "MineNoteModifier AddRandom should place at least one mine note across 200 timelines"
    );
}

/// 4-3b: LongNoteModifier.modify() uses non-deterministic Math.random() equivalent.
///
/// In Remove mode with rate=0.5, LongNoteModifier uses rand::random() to match
/// Java's Math.random() behavior. The seed field is NOT used for this RNG.
/// This test verifies that with rate=0.5, approximately half the LNs are removed.
#[test]
fn long_note_modifier_remove_mode_uses_nondeterministic_rng() {
    let mode = Mode::BEAT_7K;

    let build_model = || {
        let mut timelines = Vec::new();
        for section in 0..100 {
            let mut tl = TimeLine::new(section as f64, section * 1000, 8);
            if section % 2 == 0 {
                for lane in 0..7 {
                    let mut ln = Note::new_long(10 + lane);
                    ln.set_long_note_type(1); // TYPE_LONGNOTE
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

    // Count how many LN starts remain after Remove mode with rate=0.5
    let mut model = build_model();
    let mut modifier = LongNoteModifier::with_params(0, 0.5); // Remove mode, 50% rate
    modifier.modify(&mut model);

    // Count remaining long notes (starts only, on even timelines)
    let remaining_ln_count: usize = model
        .timelines
        .iter()
        .map(|tl| {
            (0..7)
                .filter(|&lane| tl.note(lane).is_some_and(|n| n.is_long() && !n.is_end()))
                .count()
        })
        .sum();

    // Original: 50 even timelines * 7 lanes = 350 LN starts.
    // With rate=0.5, roughly half should be removed. Allow wide margin for randomness.
    // We just verify it's not all-or-nothing (which would indicate broken RNG).
    let total_original = 350;
    assert!(
        remaining_ln_count > 0 && remaining_ln_count < total_original,
        "With rate=0.5 and non-deterministic RNG, some but not all LNs should be removed. \
         Got {} remaining out of {}",
        remaining_ln_count,
        total_original
    );
}

// ---- Worktree tests: Focused Java sequence verification ----

/// After set_random_seed(42), the internal JavaRandom must produce
/// the same sequence as `new java.util.Random(42)`.
/// Java verification: new Random(42).nextInt(10) -> 0, 3, 8, 4, 0
#[test]
fn randomizer_base_uses_java_random_sequence() {
    let mut base = RandomizerBase::new();
    base.set_random_seed(42);

    // The random field must be JavaRandom, and calling next_int_bounded
    // must match the Java LCG sequence.
    assert_eq!(base.random.next_int_bounded(10), 0);
    assert_eq!(base.random.next_int_bounded(10), 3);
    assert_eq!(base.random.next_int_bounded(10), 8);
    assert_eq!(base.random.next_int_bounded(10), 4);
    assert_eq!(base.random.next_int_bounded(10), 0);
}

/// Two RandomizerBase instances seeded with the same value must produce
/// identical sequences (determinism).
#[test]
fn randomizer_base_same_seed_same_sequence() {
    let mut base1 = RandomizerBase::new();
    let mut base2 = RandomizerBase::new();
    base1.set_random_seed(123);
    base2.set_random_seed(123);

    for _ in 0..20 {
        assert_eq!(
            base1.random.next_int_bounded(100),
            base2.random.next_int_bounded(100),
        );
    }
}

/// JavaRandom must have next_double() (port of java.util.Random.nextDouble).
/// Java verification: new Random(0).nextDouble() == 0.730967787376657
#[test]
fn java_random_next_double_exists_and_matches_java() {
    let mut rng = JavaRandom::new(0);
    let val = rng.next_double();
    // Java: new Random(0).nextDouble() = 0.730967787376657
    let expected = 0.730967787376657f64;
    assert!(
        (val - expected).abs() < 1e-15,
        "next_double() mismatch: got {}, expected {}",
        val,
        expected
    );
}

/// Verify next_double() sequence for seed 42.
/// Java verification:
///   Random r = new Random(42);
///   r.nextDouble() -> 0.7275636800328681
///   r.nextDouble() -> 0.6832234717598454
#[test]
fn java_random_next_double_sequence() {
    let mut rng = JavaRandom::new(42);
    let v1 = rng.next_double();
    let v2 = rng.next_double();

    assert!(
        (v1 - 0.7275636800328681f64).abs() < 1e-15,
        "1st next_double() mismatch: got {}",
        v1
    );
    assert!(
        (v2 - 0.6832234717598454f64).abs() < 1e-15,
        "2nd next_double() mismatch: got {}",
        v2
    );
}

/// set_random_seed with negative value should be ignored (no-op).
#[test]
fn randomizer_base_negative_seed_ignored() {
    let mut base = RandomizerBase::new();
    base.set_random_seed(42);
    // Consume one value to advance the state
    let _v1 = base.random.next_int_bounded(100);

    // Negative seed should not reset the RNG
    base.set_random_seed(-1);

    // The next value should continue from where seed=42 left off
    // (not reset to any other state)
    let mut reference = JavaRandom::new(42);
    let _ref_v1 = reference.next_int_bounded(100); // skip first
    let ref_v2 = reference.next_int_bounded(100);

    assert_eq!(base.random.next_int_bounded(100), ref_v2);
}
