/// Float number array representation formatter
///
/// Translated from FloatFormatter.java
const KETAMAX: i32 = 8;

const SIGNSYMBOL: i32 = 12;
const DECIMALPOINT: i32 = 11;
const REVERSEZERO: i32 = 10;

pub struct FloatFormatter {
    iketa: i32,
    fketa: i32,
    sign: i32,
    length: i32,
    zeropadding: i32,
    digits: Vec<i32>,
    base: i32,
}

impl FloatFormatter {
    pub fn iketa(&self) -> i32 {
        self.iketa
    }

    pub fn fketa(&self) -> i32 {
        self.fketa
    }

    pub fn sign(&self) -> i32 {
        self.sign
    }

    pub fn zeropadding(&self) -> i32 {
        self.zeropadding
    }

    pub fn digits(&self) -> &[i32] {
        &self.digits
    }

    pub fn keta_length(&self) -> i32 {
        self.length
    }

    pub fn new(iketa: i32, fketa: i32, sign: bool, zeropadding: i32) -> Self {
        let mut temp_iketa = if iketa >= 0 { iketa } else { 0 };
        let mut temp_fketa = if fketa >= 0 { fketa } else { 0 };
        let sign_val = if sign { 1 } else { 0 };
        let zeropadding_val = if zeropadding >= 2 {
            2
        } else if zeropadding >= 1 {
            1
        } else {
            0
        };

        if temp_iketa >= KETAMAX || temp_fketa >= KETAMAX || temp_iketa + temp_fketa >= KETAMAX {
            temp_fketa = if temp_fketa < KETAMAX {
                temp_fketa
            } else {
                KETAMAX
            };
            temp_iketa = KETAMAX - temp_fketa;
        }

        let length = sign_val + temp_iketa + temp_fketa + (if temp_fketa != 0 { 1 } else { 0 });
        let digits = vec![-1_i32; (length + 1) as usize];
        let base = sign_val + temp_iketa;

        Self {
            iketa: temp_iketa,
            fketa: temp_fketa,
            sign: sign_val,
            length,
            zeropadding: zeropadding_val,
            digits,
            base,
        }
    }

    pub fn calculate_and_get_digits(&mut self, value: f64) -> &[i32] {
        if self.digits.len() == 1 {
            return &self.digits;
        }
        self.digits.fill(-1);

        if self.iketa == 0 && self.fketa == 0 && self.sign == 1 {
            self.digits[1] = SIGNSYMBOL;
            return &self.digits;
        }

        let is_sign = (self.sign == 1) && (value < 10.0_f64.powi(self.iketa));

        if self.zeropadding == 0 {
            let ival = value as i32;
            self.base = (self.iketa)
                .min((if ival != 0 { ival } else { 1 } as f64).log10() as i32 + 1)
                + self.sign;
        }

        // Use abs() for digit extraction; sign is handled separately via SIGNSYMBOL.
        // Without this, negative values produce negative remainders (fval % 10 in -9..0)
        // which are invalid sprite indices.
        let mut fval = ((value * 10.0_f64.powi(self.fketa)) as i64).abs();
        let mut nowketa;
        if self.iketa == 0 {
            nowketa = self.fketa + self.sign + 1;
        } else {
            nowketa = self.base + self.fketa + (if self.fketa != 0 { 1 } else { 0 });
        }
        let mut fcnt = self.fketa;

        while nowketa > self.sign {
            if fcnt > -1 {
                self.digits[nowketa as usize] = (fval % 10) as i32;
            } else if fval == 0 && self.zeropadding == 2 {
                self.digits[nowketa as usize] = REVERSEZERO;
            } else {
                self.digits[nowketa as usize] = (fval % 10) as i32;
            }
            fcnt -= 1;
            if fcnt == 0 {
                nowketa -= 1;
                self.digits[nowketa as usize] = DECIMALPOINT;
            }
            fval /= 10;
            nowketa -= 1;
        }
        if nowketa == 1 {
            if is_sign {
                self.digits[1] = SIGNSYMBOL;
            } else {
                self.digits[1] = (fval % 10) as i32;
            }
        }

        if self.iketa == 0 && self.sign == 1 {
            self.digits[1] = SIGNSYMBOL;
        }

        &self.digits
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // output_length_invariant: The returned digits slice length always equals
    // `length + 1` (i.e., `keta_length() + 1`), regardless of the input value.
    proptest! {
        #[test]
        fn output_length_invariant(
            iketa in 0..=6i32,
            fketa in 0..=6i32,
            sign in proptest::bool::ANY,
            zeropadding in 0..=2i32,
            value in 0.0..=999999.0f64,
        ) {
            let mut formatter = FloatFormatter::new(iketa, fketa, sign, zeropadding);
            let expected_len = (formatter.keta_length() + 1) as usize;
            let digits = formatter.calculate_and_get_digits(value);
            prop_assert_eq!(
                digits.len(),
                expected_len,
                "iketa={}, fketa={}, sign={}, zeropadding={}, value={}",
                iketa, fketa, sign, zeropadding, value,
            );
        }
    }

    // digit_values_in_valid_range: Each element in the returned digits array is
    // either -1 (unused/blank) or in the range 0..=SIGNSYMBOL (0-9 digits,
    // REVERSEZERO=10, DECIMALPOINT=11, SIGNSYMBOL=12).
    proptest! {
        #[test]
        fn digit_values_in_valid_range(
            iketa in 0..=6i32,
            fketa in 0..=6i32,
            sign in proptest::bool::ANY,
            zeropadding in 0..=2i32,
            value in -999999.0..=999999.0f64,
        ) {
            let mut formatter = FloatFormatter::new(iketa, fketa, sign, zeropadding);
            let digits = formatter.calculate_and_get_digits(value);
            for (i, &d) in digits.iter().enumerate() {
                prop_assert!(
                    d == -1 || (0..=SIGNSYMBOL).contains(&d),
                    "digit[{}] = {} is out of valid range for iketa={}, fketa={}, sign={}, zeropadding={}, value={}",
                    i, d, iketa, fketa, sign, zeropadding, value,
                );
            }
        }
    }

    // decimal_point_present_when_fketa_positive: When fketa > 0 and zeropadding >= 1,
    // the digits array contains exactly one DECIMALPOINT value.
    proptest! {
        #[test]
        fn decimal_point_present_when_fketa_positive(
            iketa in 0..=6i32,
            fketa in 1..=6i32,
            sign in proptest::bool::ANY,
            zeropadding in 1..=2i32,
            value in 0.0..=999999.0f64,
        ) {
            let mut formatter = FloatFormatter::new(iketa, fketa, sign, zeropadding);
            let digits = formatter.calculate_and_get_digits(value);
            let dp_count = digits.iter().filter(|&&d| d == DECIMALPOINT).count();
            prop_assert_eq!(
                dp_count,
                1,
                "Expected exactly 1 DECIMALPOINT but found {} for iketa={}, fketa={}, sign={}, zeropadding={}, value={}",
                dp_count, iketa, fketa, sign, zeropadding, value,
            );
        }
    }
}
