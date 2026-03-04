//! Port of java.util.Random — LCG with identical seed scrambling and nextInt() behavior.
//!
//! multiplier = 0x5DEECE66D, addend = 0xB, mask = (1L << 48) - 1
//! Seeding: (seed ^ multiplier) & mask
//! next(bits): seed = (seed * multiplier + addend) & mask; return (int)(seed >>> (48 - bits))

const MULTIPLIER: i64 = 0x5DEECE66D;
const ADDEND: i64 = 0xB;
const MASK: i64 = (1i64 << 48) - 1;

pub struct JavaRandom {
    seed: i64,
}

impl JavaRandom {
    pub fn new(seed: i64) -> Self {
        JavaRandom {
            seed: (seed ^ MULTIPLIER) & MASK,
        }
    }

    fn next(&mut self, bits: i32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// Re-seed the RNG (equivalent to `java.util.Random.setSeed(seed)`).
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ MULTIPLIER) & MASK;
    }

    /// Port of `java.util.Random.nextDouble()`.
    /// Returns a uniformly distributed double in [0.0, 1.0).
    /// Formula: `(((long)(next(26)) << 27) + next(27)) / (double)(1L << 53)`
    pub fn next_double(&mut self) -> f64 {
        let high = self.next(26) as i64;
        let low = self.next(27) as i64;
        ((high << 27) + low) as f64 / (1i64 << 53) as f64
    }

    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        // Power of 2
        if (bound & (bound - 1)) == 0 {
            return ((bound as i64 * self.next(31) as i64) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let val = bits % bound;
            if bits - val + (bound - 1) >= 0 {
                return val;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_random_seed_zero_first_next_int() {
        // Verified against Java: new Random(0).nextInt(100) == 60
        let mut rng = JavaRandom::new(0);
        assert_eq!(rng.next_int_bounded(100), 60);
    }

    #[test]
    fn java_random_seed_42_sequence() {
        // Verified against Java: new Random(42).nextInt(10) sequence: 0, 3, 8, 4, 0
        let mut rng = JavaRandom::new(42);
        assert_eq!(rng.next_int_bounded(10), 0);
        assert_eq!(rng.next_int_bounded(10), 3);
        assert_eq!(rng.next_int_bounded(10), 8);
        assert_eq!(rng.next_int_bounded(10), 4);
        assert_eq!(rng.next_int_bounded(10), 0);
    }

    #[test]
    fn java_random_power_of_two_bound() {
        // Verified against Java: new Random(123).nextInt(2) == 1, nextInt(4) == 0
        let mut rng = JavaRandom::new(123);
        assert_eq!(rng.next_int_bounded(2), 1);
        assert_eq!(rng.next_int_bounded(4), 0);
    }

    #[test]
    fn java_random_negative_seed() {
        // Verified against Java: new Random(-1).nextInt(100) == 13
        let mut rng = JavaRandom::new(-1);
        assert_eq!(rng.next_int_bounded(100), 13);
    }

    #[test]
    fn java_random_next_double_seed_zero() {
        // Verified against Java: new Random(0).nextDouble() = 0.730967787376657
        let mut rng = JavaRandom::new(0);
        let val = rng.next_double();
        assert!((val - 0.730967787376657).abs() < 1e-15);
    }

    #[test]
    fn java_random_next_double_seed_42() {
        // Verified against Java: new Random(42).nextDouble() = 0.7275636800328681
        let mut rng = JavaRandom::new(42);
        let val = rng.next_double();
        assert!((val - 0.7275636800328681).abs() < 1e-15);
    }

    #[test]
    fn java_random_next_double_range() {
        let mut rng = JavaRandom::new(12345);
        for _ in 0..1000 {
            let val = rng.next_double();
            assert!(val >= 0.0 && val < 1.0);
        }
    }

    #[test]
    fn java_random_set_seed_resets_sequence() {
        let mut rng = JavaRandom::new(42);
        let first = rng.next_int_bounded(100);
        rng.set_seed(42);
        let second = rng.next_int_bounded(100);
        assert_eq!(first, second);
    }

    #[test]
    fn java_random_bound_one_always_zero() {
        let mut rng = JavaRandom::new(0);
        for _ in 0..100 {
            assert_eq!(rng.next_int_bounded(1), 0);
        }
    }
}
