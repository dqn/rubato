//! Type cast overflow audit tests for random seed encoding in BMSPlayer.
//!
//! These tests document the arithmetic overflow vulnerability in the seed
//! encoding formula used by BMSPlayer::encode_seed_for_score.
//!
//! Source: crates/beatoraja-play/src/bms_player.rs line 741
//! Code:   `self.playinfo.randomoption2seed * 65536 * 256 + self.playinfo.randomoptionseed`
//!
//! In Java, these fields are `long` (64-bit signed), so the multiplication
//! `long * int * int` promotes to `long` arithmetic and doesn't overflow for
//! any realistic seed value.
//!
//! However, the encoding formula packs two seeds into a single i64 using:
//!   encoded = seed2 * 16_777_216 + seed1
//! The decoding side (MusicSelector.java line 403-404) does:
//!   seed1 = encoded % 16_777_216
//!   seed2 = encoded / 16_777_216
//!
//! When seed1 >= 16_777_216 or seed2 is large enough that the product exceeds
//! the domain of the encoding scheme, the packed value cannot be correctly
//! decoded. This test documents the encoding domain limitations.
//!
//! Additionally, if the code were ported with i32 fields (as the Java `int`
//! type suggests in some call sites), the multiplication `i32 * 65536 * 256`
//! overflows for seed2 >= 128, since `128 * 16_777_216 = 2_147_483_648 > i32::MAX`.

/// Seed encoding formula extracted from BMSPlayer::encode_seed_for_score.
/// Tests the arithmetic directly since BMSPlayer requires complex construction.
fn encode_seed_i64(seed1: i64, seed2: i64) -> i64 {
    seed2 * 65536 * 256 + seed1
}

/// Same formula but using i32 arithmetic, simulating what would happen if
/// the fields were i32 (as in some Java call sites that use int).
fn encode_seed_i32(seed1: i32, seed2: i32) -> i32 {
    seed2
        .wrapping_mul(65536)
        .wrapping_mul(256)
        .wrapping_add(seed1)
}

/// Decode formula from MusicSelector.java lines 403-404.
fn decode_seed(encoded: i64) -> (i64, i64) {
    let seed1 = encoded % (65536 * 256);
    let seed2 = encoded / (65536 * 256);
    (seed1, seed2)
}

/// seed2=1: 1 * 16_777_216 = 16_777_216. Fits in both i32 and i64.
/// Round-trip should work.
#[test]
fn random_seed_encoding_seed2_1_ok() {
    let encoded = encode_seed_i64(42, 1);
    assert_eq!(encoded, 16_777_216 + 42);

    let (seed1, seed2) = decode_seed(encoded);
    assert_eq!(seed1, 42, "seed1 should round-trip");
    assert_eq!(seed2, 1, "seed2 should round-trip");
}

/// seed2=127: 127 * 16_777_216 = 2_130_706_432. Just below i32::MAX.
/// This is the last value that works with i32 arithmetic.
#[test]
fn random_seed_encoding_seed2_127_boundary() {
    let encoded_i64 = encode_seed_i64(0, 127);
    assert_eq!(encoded_i64, 127 * 16_777_216);

    let encoded_i32 = encode_seed_i32(0, 127);
    assert_eq!(
        encoded_i32 as i64,
        127 * 16_777_216,
        "seed2=127 should fit in i32"
    );

    let (seed1, seed2) = decode_seed(encoded_i64);
    assert_eq!(seed1, 0);
    assert_eq!(seed2, 127);
}

/// BUG (i32 arithmetic): seed2=128: 128 * 16_777_216 = 2_147_483_648 > i32::MAX.
/// In debug mode this would panic; in release mode it wraps to -2_147_483_648.
/// The wrapping value cannot be correctly decoded.
///
/// The current Rust code uses i64, so it doesn't overflow. But the packing
/// scheme's domain assumes seed2 fits in a range where the product is valid.
/// If any call site truncates the encoded value to i32 (e.g., storing in a
/// database column declared as INTEGER), the data is silently corrupted.
#[test]
fn random_seed_encoding_overflow() {
    // i64 arithmetic is fine
    let encoded_i64 = encode_seed_i64(0, 128);
    assert_eq!(encoded_i64, 128 * 16_777_216_i64);
    let (seed1, seed2) = decode_seed(encoded_i64);
    assert_eq!(seed1, 0, "i64 round-trip: seed1 should be 0");
    assert_eq!(seed2, 128, "i64 round-trip: seed2 should be 128");

    // i32 arithmetic overflows: 128 * 16_777_216 = 2_147_483_648 > i32::MAX
    let encoded_i32 = encode_seed_i32(0, 128);
    // wrapping_mul produces: 128 * 65536 = 8_388_608; 8_388_608 * 256 = 2_147_483_648
    // which wraps to -2_147_483_648 in i32.
    // i32 arithmetic overflows: this documents why i64 is used in production
    assert!(
        encoded_i32 < 0,
        "i32 wraps negative for seed2>=128 (expected, i64 used in production), got {}",
        encoded_i32
    );
}

/// BUG: Even with i64, if the encoded value is stored in a Java int field
/// or Rust i32 field (e.g., via database or serialization), truncation occurs.
/// This test demonstrates that encode_seed_i64 produces values that don't
/// fit in i32 for seed2 >= 128.
#[test]
fn random_seed_encoding_i64_to_i32_truncation() {
    let encoded = encode_seed_i64(100, 200);
    // 200 * 16_777_216 + 100 = 3_355_443_300
    let expected = 200_i64 * 16_777_216 + 100;
    assert_eq!(encoded, expected);

    // If truncated to i32, the packed value is corrupted.
    // This documents why ReplayData stores seeds as i64.
    let truncated = encoded as i32;
    assert!(
        truncated < 0,
        "i64→i32 truncation produces negative value for seed2>=128: {}",
        truncated
    );
    // Verify i64 roundtrip works correctly (the production path)
    let (decoded_s1, decoded_s2) = decode_seed(encoded);
    assert_eq!(decoded_s1, 100, "i64 roundtrip: seed1");
    assert_eq!(decoded_s2, 200, "i64 roundtrip: seed2");
}
