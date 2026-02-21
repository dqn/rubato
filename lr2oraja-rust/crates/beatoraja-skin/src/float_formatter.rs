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
    pub fn get_iketa(&self) -> i32 {
        self.iketa
    }

    pub fn get_fketa(&self) -> i32 {
        self.fketa
    }

    pub fn get_sign(&self) -> i32 {
        self.sign
    }

    pub fn get_zeropadding(&self) -> i32 {
        self.zeropadding
    }

    pub fn get_digits(&self) -> &[i32] {
        &self.digits
    }

    pub fn get_keta_length(&self) -> i32 {
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

        let mut fval = (value * 10.0_f64.powi(self.fketa)) as i64;
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
