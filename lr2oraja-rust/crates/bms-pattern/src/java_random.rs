// java.util.Random LCG reproduction
//
// Implements the exact same algorithm as java.util.Random
// so that pattern shuffle results are bit-identical.

const MULTIPLIER: i64 = 0x5DEECE66D;
const ADDEND: i64 = 0xB;
const MASK: i64 = (1 << 48) - 1;

/// A faithful reproduction of `java.util.Random`.
///
/// Uses the same 48-bit Linear Congruential Generator (LCG):
///   seed = (seed * 0x5DEECE66D + 0xB) & ((1 << 48) - 1)
#[derive(Debug, Clone)]
pub struct JavaRandom {
    seed: i64,
}

impl JavaRandom {
    /// Create a new JavaRandom with the given seed.
    ///
    /// Matches `new java.util.Random(seed)`:
    ///   internal_seed = (seed ^ 0x5DEECE66D) & ((1 << 48) - 1)
    pub fn new(seed: i64) -> Self {
        Self {
            seed: (seed ^ MULTIPLIER) & MASK,
        }
    }

    /// Generate next `bits` random bits (1..=32).
    ///
    /// Matches `java.util.Random.next(int bits)`.
    fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// Returns a pseudorandom int in [0, bound).
    ///
    /// Matches `java.util.Random.nextInt(int bound)`.
    ///
    /// # Panics
    /// Panics if `bound <= 0`.
    pub fn next_int(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive, got {bound}");

        // Power-of-2 optimization
        if bound & (bound - 1) == 0 {
            return ((bound as i64 * self.next(31) as i64) >> 31) as i32;
        }

        // Rejection sampling to avoid modulo bias
        loop {
            let bits = self.next(31);
            let val = bits % bound;
            if bits - val + (bound - 1) >= 0 {
                return val;
            }
        }
    }

    /// Returns the next pseudorandom i64.
    ///
    /// Matches `java.util.Random.nextLong()`.
    #[allow(dead_code)] // Parsed for completeness (java.util.Random API)
    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32) + self.next(32) as i64
    }

    /// Re-seeds this generator.
    ///
    /// Matches `java.util.Random.setSeed(long seed)`.
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ MULTIPLIER) & MASK;
    }

    /// Returns the next pseudorandom f64 in [0.0, 1.0).
    ///
    /// Matches `java.util.Random.nextDouble()`.
    #[allow(dead_code)] // Parsed for completeness (java.util.Random API)
    pub fn next_double(&mut self) -> f64 {
        let high = self.next(26) as i64;
        let low = self.next(27) as i64;
        ((high << 27) + low) as f64 / ((1_i64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_initialization() {
        // Verify internal seed matches Java's constructor
        let rng = JavaRandom::new(0);
        assert_eq!(rng.seed, MULTIPLIER & MASK);

        let rng = JavaRandom::new(12345);
        assert_eq!(rng.seed, (12345 ^ MULTIPLIER) & MASK);
    }

    #[test]
    fn test_next_int_sequence_seed_0() {
        // Generated from Java: new Random(0), calling nextInt(100) 10 times
        let mut rng = JavaRandom::new(0);
        let expected = [60, 48, 29, 47, 15, 53, 91, 61, 19, 54];
        for &e in &expected {
            assert_eq!(rng.next_int(100), e);
        }
    }

    #[test]
    fn test_next_int_sequence_seed_42() {
        // Generated from Java: new Random(42), calling nextInt(100) 10 times
        let mut rng = JavaRandom::new(42);
        let expected = [30, 63, 48, 84, 70, 25, 5, 18, 19, 93];
        for &e in &expected {
            assert_eq!(rng.next_int(100), e);
        }
    }

    #[test]
    fn test_next_int_power_of_two() {
        // Generated from Java: new Random(12345), calling nextInt(8) 10 times
        let mut rng = JavaRandom::new(12345);
        let expected = [2, 4, 7, 7, 6, 0, 2, 0, 1, 0];
        for &e in &expected {
            assert_eq!(rng.next_int(8), e);
        }
    }

    #[test]
    fn test_next_int_bound_2() {
        // Generated from Java: new Random(100), calling nextInt(2) 10 times
        let mut rng = JavaRandom::new(100);
        let expected = [1, 1, 0, 1, 1, 0, 1, 0, 1, 1];
        for &e in &expected {
            assert_eq!(rng.next_int(2), e);
        }
    }

    #[test]
    fn test_next_int_small_bounds() {
        // Generated from Java: new Random(7777), calling nextInt(7) 10 times
        let mut rng = JavaRandom::new(7777);
        let expected = [3, 2, 4, 3, 3, 1, 6, 6, 4, 6];
        for &e in &expected {
            assert_eq!(rng.next_int(7), e);
        }
    }

    #[test]
    fn test_next_int_large_bound() {
        // Generated from Java: new Random(99999), calling nextInt(1000000) 5 times
        let mut rng = JavaRandom::new(99999);
        let expected = [115041, 665290, 967208, 135309, 753130];
        for &e in &expected {
            assert_eq!(rng.next_int(1000000), e);
        }
    }

    #[test]
    fn test_next_long() {
        // Generated from Java: new Random(0), calling nextLong() 5 times
        let mut rng = JavaRandom::new(0);
        let expected: [i64; 5] = [
            -4962768465676381896,
            4437113781045784766,
            -6688467811848818630,
            -8292973307042192125,
            -7423979211207825555,
        ];
        for &e in &expected {
            assert_eq!(rng.next_long(), e);
        }
    }

    #[test]
    fn test_next_double() {
        // Generated from Java: new Random(0), calling nextDouble() 5 times
        let mut rng = JavaRandom::new(0);
        let expected = [
            0.730967787376657,
            0.24053641567148587,
            0.6374174253501083,
            0.5504370051176339,
            0.5975452777972018,
        ];
        for (i, &e) in expected.iter().enumerate() {
            let actual = rng.next_double();
            assert!(
                (actual - e).abs() < 1e-15,
                "next_double()[{i}]: expected {e}, got {actual}"
            );
        }
    }

    #[test]
    fn test_negative_seed() {
        // Generated from Java: new Random(-1), calling nextInt(100) 5 times
        let mut rng = JavaRandom::new(-1);
        let expected = [13, 25, 79, 39, 4];
        for &e in &expected {
            assert_eq!(rng.next_int(100), e);
        }
    }

    #[test]
    #[should_panic(expected = "bound must be positive")]
    fn test_next_int_zero_bound_panics() {
        let mut rng = JavaRandom::new(0);
        rng.next_int(0);
    }

    #[test]
    #[should_panic(expected = "bound must be positive")]
    fn test_next_int_negative_bound_panics() {
        let mut rng = JavaRandom::new(0);
        rng.next_int(-1);
    }
}
