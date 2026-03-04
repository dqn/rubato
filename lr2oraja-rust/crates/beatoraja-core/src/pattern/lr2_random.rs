const N: usize = 624;
const M: usize = 397;
const MATRIX_A: i32 = 0x9908b0dfu32 as i32;
const UPPER_MASK: i32 = 0x80000000u32 as i32;
const LOWER_MASK: i32 = 0x7fffffffi32;
const TEMPERING_MASK_B: i32 = 0x9d2c5680u32 as i32;
const TEMPERING_MASK_C: i32 = 0xefc60000u32 as i32;

pub struct LR2Random {
    mti: usize,
    mt: Vec<i32>,
    mtr: Vec<i32>,
}

impl LR2Random {
    pub fn new() -> Self {
        let mut r = LR2Random {
            mti: 0,
            mt: vec![0i32; N + 1],
            mtr: vec![0i32; N],
        };
        r.set_seed(4357);
        r
    }

    pub fn with_seed(seed: i32) -> Self {
        let mut r = LR2Random {
            mti: 0,
            mt: vec![0i32; N + 1],
            mtr: vec![0i32; N],
        };
        r.set_seed(seed);
        r
    }

    pub fn set_seed(&mut self, seed: i32) {
        let mut seed = seed;
        for i in 0..N {
            self.mt[i] = seed & (0xffff0000u32 as i32);
            seed = seed.wrapping_mul(69069).wrapping_add(1);
            self.mt[i] |= ((seed as u32 & 0xffff0000) >> 16) as i32;
            seed = seed.wrapping_mul(69069).wrapping_add(1);
        }
        self.generate_mt();
    }

    pub fn next_int(&mut self, max: i32) -> i32 {
        let rand_max = max as i64;
        let r = self.rand_mt() as u32 as u64;
        ((r * rand_max as u64) >> 32) as i32
    }

    fn generate_mt(&mut self) {
        let mag01: [i32; 2] = [0, MATRIX_A];
        let mut y: i32;

        for kk in 0..(N - M) {
            y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
            self.mt[kk] = self.mt[kk + M] ^ ((y as u32 >> 1) as i32) ^ mag01[(y & 0x1) as usize];
        }

        self.mt[N] = self.mt[0];
        for kk in (N - M)..N {
            y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
            self.mt[kk] =
                self.mt[kk + M - N] ^ ((y as u32 >> 1) as i32) ^ mag01[(y & 0x1) as usize];
        }

        for kk in 0..N {
            y = self.mt[kk];
            y ^= (y as u32 >> 11) as i32;
            y ^= (y << 7) & TEMPERING_MASK_B;
            y ^= (y << 15) & TEMPERING_MASK_C;
            y ^= (y as u32 >> 18) as i32;
            self.mtr[kk] = y;
        }
        self.mti = 0;
    }

    pub fn rand_mt(&mut self) -> i32 {
        if self.mti >= N {
            self.generate_mt();
        }
        let result = self.mtr[self.mti];
        self.mti += 1;
        result
    }
}

impl Default for LR2Random {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_seed_is_4357() {
        // new() calls set_seed(4357)
        let mut rng = LR2Random::new();
        let mut rng2 = LR2Random::with_seed(4357);
        // Both should produce the same sequence
        for _ in 0..100 {
            assert_eq!(rng.rand_mt(), rng2.rand_mt());
        }
    }

    #[test]
    fn deterministic_with_same_seed() {
        let mut rng1 = LR2Random::with_seed(12345);
        let mut rng2 = LR2Random::with_seed(12345);
        for _ in 0..1000 {
            assert_eq!(rng1.rand_mt(), rng2.rand_mt());
        }
    }

    #[test]
    fn different_seeds_produce_different_sequences() {
        let mut rng1 = LR2Random::with_seed(1);
        let mut rng2 = LR2Random::with_seed(2);
        let mut differs = false;
        for _ in 0..100 {
            if rng1.rand_mt() != rng2.rand_mt() {
                differs = true;
                break;
            }
        }
        assert!(differs, "Different seeds should produce different output");
    }

    #[test]
    fn next_int_range() {
        let mut rng = LR2Random::with_seed(42);
        for _ in 0..10000 {
            let val = rng.next_int(10);
            assert!((0..10).contains(&val), "next_int(10) returned {}", val);
        }
    }

    #[test]
    fn next_int_max_1_always_returns_0() {
        let mut rng = LR2Random::with_seed(99);
        for _ in 0..100 {
            assert_eq!(rng.next_int(1), 0);
        }
    }

    #[test]
    fn mt_state_length() {
        let rng = LR2Random::new();
        assert_eq!(rng.mt.len(), N + 1);
        assert_eq!(rng.mtr.len(), N);
    }

    #[test]
    fn regeneration_after_n_calls() {
        // After N (624) calls, generate_mt should be triggered
        let mut rng = LR2Random::with_seed(100);
        for _ in 0..N {
            rng.rand_mt();
        }
        assert_eq!(rng.mti, N);
        // Next call triggers regeneration
        let _val = rng.rand_mt();
        assert_eq!(rng.mti, 1);
    }

    #[test]
    fn set_seed_resets_state() {
        let mut rng = LR2Random::with_seed(42);
        // Generate some values
        for _ in 0..50 {
            rng.rand_mt();
        }
        // Reset seed
        rng.set_seed(42);
        let mut rng2 = LR2Random::with_seed(42);
        for _ in 0..100 {
            assert_eq!(rng.rand_mt(), rng2.rand_mt());
        }
    }

    #[test]
    fn known_output_seed_4357() {
        // Record a known sequence from seed 4357 and verify it's stable
        let mut rng = LR2Random::with_seed(4357);
        let first_10: Vec<i32> = (0..10).map(|_| rng.rand_mt()).collect();
        // Verify against a second run to ensure determinism
        let mut rng2 = LR2Random::with_seed(4357);
        let first_10_again: Vec<i32> = (0..10).map(|_| rng2.rand_mt()).collect();
        assert_eq!(first_10, first_10_again);
    }

    #[test]
    fn next_int_distribution_is_roughly_uniform() {
        let mut rng = LR2Random::with_seed(777);
        let n = 10;
        let iters = 10000;
        let mut counts = vec![0usize; n as usize];
        for _ in 0..iters {
            let val = rng.next_int(n);
            counts[val as usize] += 1;
        }
        // Each bucket should have roughly iters/n = 1000 hits
        // Allow 40% deviation
        let expected = iters as f64 / n as f64;
        for (i, &count) in counts.iter().enumerate() {
            assert!(
                (count as f64 - expected).abs() < expected * 0.4,
                "Bucket {} has {} hits, expected ~{}",
                i,
                count,
                expected
            );
        }
    }

    #[test]
    fn with_seed_zero() {
        // Seed 0 should still produce valid output
        let mut rng = LR2Random::with_seed(0);
        // Just verify it doesn't panic and produces values
        for _ in 0..100 {
            let _ = rng.rand_mt();
        }
    }

    #[test]
    fn with_seed_negative() {
        // Negative seed should work via wrapping arithmetic
        let mut rng = LR2Random::with_seed(-1);
        for _ in 0..100 {
            let _ = rng.rand_mt();
        }
    }
}
