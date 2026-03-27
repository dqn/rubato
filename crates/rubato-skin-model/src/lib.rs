//! Skin data model types: property constants, config offsets, resolution.

pub mod skin_config_offset;
pub mod skin_property;
pub mod skin_resolution;
pub mod skin_timer;

/// Division that returns 0.0 when the divisor is 0.0.
/// Prevents NaN/Inf from malformed skin data (e.g. zero-width src resolution).
#[inline]
pub fn safe_div_f32(a: f32, b: f32) -> f32 {
    if b == 0.0 { 0.0 } else { a / b }
}

#[cfg(test)]
mod safe_div_tests {
    use super::*;

    #[test]
    fn safe_div_f32_normal() {
        assert_eq!(safe_div_f32(10.0, 2.0), 5.0);
    }

    #[test]
    fn safe_div_f32_zero_divisor() {
        assert_eq!(safe_div_f32(10.0, 0.0), 0.0);
        assert_eq!(safe_div_f32(0.0, 0.0), 0.0);
        assert_eq!(safe_div_f32(-5.0, 0.0), 0.0);
    }

    #[test]
    fn safe_div_f32_negative_zero_divisor() {
        assert_eq!(safe_div_f32(10.0, -0.0), 0.0);
    }
}
