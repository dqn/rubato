// Phase 49: Randomizer RNG divergence tests
//
// Demonstrates that StdRng and JavaRandom produce completely different
// sequences from the same seed, and verifies JavaRandom determinism.

use beatoraja_pattern::java_random::JavaRandom;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;

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
