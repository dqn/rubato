/// BM controller key constants.
///
/// Ported from Java `BMControllerInputProcessor.BMKeys`.
#[allow(dead_code)] // Parsed for completeness (BM controller key constants)
pub mod bm_keys {
    pub const BUTTON_1: i32 = 0;
    pub const BUTTON_2: i32 = 1;
    pub const BUTTON_3: i32 = 2;
    pub const BUTTON_4: i32 = 3;
    pub const BUTTON_5: i32 = 4;
    pub const BUTTON_6: i32 = 5;
    pub const BUTTON_7: i32 = 6;
    pub const BUTTON_8: i32 = 7;
    pub const BUTTON_9: i32 = 8;
    pub const BUTTON_10: i32 = 9;
    pub const BUTTON_11: i32 = 10;
    pub const BUTTON_12: i32 = 11;
    pub const BUTTON_13: i32 = 12;
    pub const BUTTON_14: i32 = 13;
    pub const BUTTON_15: i32 = 14;
    pub const BUTTON_16: i32 = 15;
    pub const BUTTON_17: i32 = 16;
    pub const BUTTON_18: i32 = 17;
    pub const BUTTON_19: i32 = 18;
    pub const BUTTON_20: i32 = 19;
    pub const BUTTON_21: i32 = 20;
    pub const BUTTON_22: i32 = 21;
    pub const BUTTON_23: i32 = 22;
    pub const BUTTON_24: i32 = 23;
    pub const BUTTON_25: i32 = 24;
    pub const BUTTON_26: i32 = 25;
    pub const BUTTON_27: i32 = 26;
    pub const BUTTON_28: i32 = 27;
    pub const BUTTON_29: i32 = 28;
    pub const BUTTON_30: i32 = 29;
    pub const BUTTON_31: i32 = 30;
    pub const BUTTON_32: i32 = 31;
    pub const AXIS1_PLUS: i32 = 32;
    pub const AXIS1_MINUS: i32 = 33;
    pub const AXIS2_PLUS: i32 = 34;
    pub const AXIS2_MINUS: i32 = 35;
    pub const AXIS3_PLUS: i32 = 36;
    pub const AXIS3_MINUS: i32 = 37;
    pub const AXIS4_PLUS: i32 = 38;
    pub const AXIS4_MINUS: i32 = 39;
    pub const AXIS5_PLUS: i32 = 40;
    pub const AXIS5_MINUS: i32 = 41;
    pub const AXIS6_PLUS: i32 = 42;
    pub const AXIS6_MINUS: i32 = 43;
    pub const AXIS7_PLUS: i32 = 44;
    pub const AXIS7_MINUS: i32 = 45;
    pub const AXIS8_PLUS: i32 = 46;
    pub const AXIS8_MINUS: i32 = 47;

    pub const MAXID: usize = 48;

    /// Human-readable names for BM controller key codes.
    const BMCODE: [&str; MAXID] = [
        "BUTTON 1",
        "BUTTON 2",
        "BUTTON 3",
        "BUTTON 4",
        "BUTTON 5",
        "BUTTON 6",
        "BUTTON 7",
        "BUTTON 8",
        "BUTTON 9",
        "BUTTON 10",
        "BUTTON 11",
        "BUTTON 12",
        "BUTTON 13",
        "BUTTON 14",
        "BUTTON 15",
        "BUTTON 16",
        "BUTTON 17",
        "BUTTON 18",
        "BUTTON 19",
        "BUTTON 20",
        "BUTTON 21",
        "BUTTON 22",
        "BUTTON 23",
        "BUTTON 24",
        "BUTTON 25",
        "BUTTON 26",
        "BUTTON 27",
        "BUTTON 28",
        "BUTTON 29",
        "BUTTON 30",
        "BUTTON 31",
        "BUTTON 32",
        "UP (AXIS 1 +)",
        "DOWN (AXIS 1 -)",
        "RIGHT (AXIS 2 +)",
        "LEFT (AXIS 2 -)",
        "AXIS 3 +",
        "AXIS 3 -",
        "AXIS 4 +",
        "AXIS 4 -",
        "AXIS 5 +",
        "AXIS 5 -",
        "AXIS 6 +",
        "AXIS 6 -",
        "AXIS 7 +",
        "AXIS 7 -",
        "AXIS 8 +",
        "AXIS 8 -",
    ];

    /// Returns the human-readable name for a BM controller key code.
    pub fn to_string(keycode: i32) -> &'static str {
        if keycode >= 0 && (keycode as usize) < BMCODE.len() {
            BMCODE[keycode as usize]
        } else {
            "Unknown"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::bm_keys;

    #[test]
    fn test_button_constants() {
        assert_eq!(bm_keys::BUTTON_1, 0);
        assert_eq!(bm_keys::BUTTON_32, 31);
        assert_eq!(bm_keys::AXIS1_PLUS, 32);
        assert_eq!(bm_keys::AXIS8_MINUS, 47);
        assert_eq!(bm_keys::MAXID, 48);
    }

    #[test]
    fn test_to_string() {
        assert_eq!(bm_keys::to_string(0), "BUTTON 1");
        assert_eq!(bm_keys::to_string(31), "BUTTON 32");
        assert_eq!(bm_keys::to_string(32), "UP (AXIS 1 +)");
        assert_eq!(bm_keys::to_string(47), "AXIS 8 -");
        assert_eq!(bm_keys::to_string(-1), "Unknown");
        assert_eq!(bm_keys::to_string(48), "Unknown");
    }
}
