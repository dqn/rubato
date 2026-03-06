use std::path::Path;

use crate::bms_decoder::BMSDecoder;
use crate::bms_model::LNTYPE_LONGNOTE;
use crate::bmson_decoder::BMSONDecoder;
use crate::chart_information::ChartInformation;
use crate::decode_log::DecodeLog;
use crate::osu_decoder::OSUDecoder;
use crate::time_line::TimeLine;

pub struct TimeLineCache {
    pub time: f64,
    pub timeline: TimeLine,
}

impl TimeLineCache {
    pub fn new(time: f64, timeline: TimeLine) -> Self {
        TimeLineCache { time, timeline }
    }
}

pub enum ChartDecoderImpl {
    Bms(BMSDecoder),
    Bmson(BMSONDecoder),
    Osu(OSUDecoder),
}

impl ChartDecoderImpl {
    pub fn decode_path(&mut self, path: &Path) -> Option<crate::bms_model::BMSModel> {
        let info = ChartInformation::new(
            Some(path.to_path_buf()),
            match self {
                ChartDecoderImpl::Bms(d) => d.lntype,
                ChartDecoderImpl::Bmson(d) => d.lntype,
                ChartDecoderImpl::Osu(d) => d.lntype,
            },
            None,
        );
        self.decode(info)
    }

    pub fn decode(&mut self, info: ChartInformation) -> Option<crate::bms_model::BMSModel> {
        match self {
            ChartDecoderImpl::Bms(d) => d.decode(info),
            ChartDecoderImpl::Bmson(d) => d.decode(info),
            ChartDecoderImpl::Osu(d) => d.decode(info),
        }
    }

    pub fn decode_log(&self) -> &[DecodeLog] {
        match self {
            ChartDecoderImpl::Bms(d) => &d.log,
            ChartDecoderImpl::Bmson(d) => &d.log,
            ChartDecoderImpl::Osu(d) => &d.log,
        }
    }
}

pub fn decoder(p: &Path) -> Option<ChartDecoderImpl> {
    let s = p
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if s.ends_with(".bms") || s.ends_with(".bme") || s.ends_with(".bml") || s.ends_with(".pms") {
        return Some(ChartDecoderImpl::Bms(BMSDecoder::new_with_lntype(
            LNTYPE_LONGNOTE,
        )));
    } else if s.ends_with(".bmson") {
        return Some(ChartDecoderImpl::Bmson(BMSONDecoder::new(LNTYPE_LONGNOTE)));
    } else if s.ends_with(".osu") {
        return Some(ChartDecoderImpl::Osu(OSUDecoder::new(LNTYPE_LONGNOTE)));
    }
    None
}

#[allow(clippy::result_unit_err)]
pub fn parse_int36_str(s: &str, index: usize) -> Result<i32, ()> {
    let bytes = s.as_bytes();
    if index + 1 >= bytes.len() {
        return Err(());
    }
    let result = parse_int36(bytes[index] as char, bytes[index + 1] as char);
    if result == -1 { Err(()) } else { Ok(result) }
}

pub fn parse_int36(c1: char, c2: char) -> i32 {
    let mut result: i32;
    if c1.is_ascii_digit() {
        result = ((c1 as i32) - ('0' as i32)) * 36;
    } else if c1.is_ascii_lowercase() {
        result = (((c1 as i32) - ('a' as i32)) + 10) * 36;
    } else if c1.is_ascii_uppercase() {
        result = (((c1 as i32) - ('A' as i32)) + 10) * 36;
    } else {
        return -1;
    }

    if c2.is_ascii_digit() {
        result += (c2 as i32) - ('0' as i32);
    } else if c2.is_ascii_lowercase() {
        result += ((c2 as i32) - ('a' as i32)) + 10;
    } else if c2.is_ascii_uppercase() {
        result += ((c2 as i32) - ('A' as i32)) + 10;
    } else {
        return -1;
    }

    result
}

#[allow(clippy::result_unit_err)]
pub fn parse_int62_str(s: &str, index: usize) -> Result<i32, ()> {
    let bytes = s.as_bytes();
    if index + 1 >= bytes.len() {
        return Err(());
    }
    let result = parse_int62(bytes[index] as char, bytes[index + 1] as char);
    if result == -1 { Err(()) } else { Ok(result) }
}

pub fn parse_int62(c1: char, c2: char) -> i32 {
    let mut result: i32;
    if c1.is_ascii_digit() {
        result = ((c1 as i32) - ('0' as i32)) * 62;
    } else if c1.is_ascii_uppercase() {
        result = (((c1 as i32) - ('A' as i32)) + 10) * 62;
    } else if c1.is_ascii_lowercase() {
        result = (((c1 as i32) - ('a' as i32)) + 36) * 62;
    } else {
        return -1;
    }

    if c2.is_ascii_digit() {
        result += (c2 as i32) - ('0' as i32);
    } else if c2.is_ascii_uppercase() {
        result += ((c2 as i32) - ('A' as i32)) + 10;
    } else if c2.is_ascii_lowercase() {
        result += ((c2 as i32) - ('a' as i32)) + 36;
    } else {
        return -1;
    }

    result
}

pub fn to_base62(mut decimal: i32) -> String {
    let mut sb = Vec::with_capacity(2);
    for _ in 0..2 {
        let m = decimal % 62;
        if m < 10 {
            sb.push((b'0' + m as u8) as char);
        } else if m < 36 {
            sb.push((b'A' + (m - 10) as u8) as char);
        } else if m < 62 {
            sb.push((b'a' + (m - 36) as u8) as char);
        } else {
            sb.push('0');
        }
        decimal /= 62;
    }
    sb.reverse();
    sb.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // --- parse_int36 tests ---

    #[test]
    fn parse_int36_zero_zero() {
        assert_eq!(parse_int36('0', '0'), 0);
    }

    #[test]
    fn parse_int36_zero_one() {
        assert_eq!(parse_int36('0', '1'), 1);
    }

    #[test]
    fn parse_int36_max_value() {
        // 'Z','Z' = 35*36 + 35 = 1295
        assert_eq!(parse_int36('Z', 'Z'), 1295);
    }

    #[test]
    fn parse_int36_lowercase_letter() {
        // 'a','0' = 10*36 = 360
        assert_eq!(parse_int36('a', '0'), 360);
    }

    #[test]
    fn parse_int36_invalid_first_char() {
        assert_eq!(parse_int36('!', '0'), -1);
    }

    #[test]
    fn parse_int36_invalid_second_char() {
        assert_eq!(parse_int36('0', '!'), -1);
    }

    #[test]
    fn parse_int36_case_insensitive() {
        // 'a' and 'A' should produce same result
        assert_eq!(parse_int36('a', '0'), parse_int36('A', '0'));
        assert_eq!(parse_int36('0', 'f'), parse_int36('0', 'F'));
    }

    // --- parse_int62 tests ---

    #[test]
    fn parse_int62_zero_zero() {
        assert_eq!(parse_int62('0', '0'), 0);
    }

    #[test]
    fn parse_int62_uppercase_max() {
        // 'Z','Z' = 35*62 + 35 = 2205
        assert_eq!(parse_int62('Z', 'Z'), 2205);
    }

    #[test]
    fn parse_int62_lowercase_aa() {
        // 'a','a' = 36*62 + 36 = 2268
        assert_eq!(parse_int62('a', 'a'), 2268);
    }

    #[test]
    fn parse_int62_lowercase_max() {
        // 'z','z' = 61*62 + 61 = 3843
        assert_eq!(parse_int62('z', 'z'), 3843);
    }

    #[test]
    fn parse_int62_uppercase_a0() {
        // 'A','0' = 10*62 = 620
        assert_eq!(parse_int62('A', '0'), 620);
    }

    #[test]
    fn parse_int62_invalid_char() {
        assert_eq!(parse_int62('!', '0'), -1);
        assert_eq!(parse_int62('0', '!'), -1);
    }

    // --- to_base62 tests ---

    #[test]
    fn to_base62_zero() {
        assert_eq!(to_base62(0), "00");
    }

    #[test]
    fn to_base62_max() {
        assert_eq!(to_base62(3843), "zz");
    }

    #[test]
    fn to_base62_620() {
        assert_eq!(to_base62(620), "A0");
    }

    // --- roundtrip tests ---

    #[test]
    fn roundtrip_parse_int62_to_base62() {
        // For all valid 2-digit base-62 values, roundtrip should work
        for val in [0, 1, 9, 10, 35, 36, 61, 100, 620, 2205, 2268, 3843] {
            let s = to_base62(val);
            let chars: Vec<char> = s.chars().collect();
            assert_eq!(
                parse_int62(chars[0], chars[1]),
                val,
                "roundtrip failed for {}",
                val
            );
        }
    }

    // --- parse_int36_str tests ---

    #[test]
    fn parse_int36_str_valid() {
        // "AB" at index 0 => A=10, B=11 => 10*36 + 11 = 371
        assert_eq!(parse_int36_str("AB", 0), Ok(371));
    }

    #[test]
    fn parse_int36_str_too_short() {
        assert_eq!(parse_int36_str("A", 0), Err(()));
    }

    #[test]
    fn parse_int36_str_invalid_char() {
        assert_eq!(parse_int36_str("!B", 0), Err(()));
    }

    #[test]
    fn parse_int36_str_index_offset() {
        // "xxAB" at index 2 => same as "AB" at 0
        assert_eq!(parse_int36_str("xxAB", 2), Ok(371));
    }

    #[test]
    fn parse_int36_str_index_out_of_bounds() {
        assert_eq!(parse_int36_str("AB", 1), Err(()));
    }

    // --- parse_int62_str tests ---

    #[test]
    fn parse_int62_str_valid() {
        // "AB" at index 0 => A=10, B=11 => 10*62 + 11 = 631
        assert_eq!(parse_int62_str("AB", 0), Ok(631));
    }

    #[test]
    fn parse_int62_str_too_short() {
        assert_eq!(parse_int62_str("A", 0), Err(()));
    }

    // --- decoder tests ---

    #[test]
    fn get_decoder_bms() {
        let dec = decoder(Path::new("test.bms"));
        assert!(matches!(dec, Some(ChartDecoderImpl::Bms(_))));
    }

    #[test]
    fn get_decoder_bme() {
        let dec = decoder(Path::new("test.bme"));
        assert!(matches!(dec, Some(ChartDecoderImpl::Bms(_))));
    }

    #[test]
    fn get_decoder_bml() {
        let dec = decoder(Path::new("test.bml"));
        assert!(matches!(dec, Some(ChartDecoderImpl::Bms(_))));
    }

    #[test]
    fn get_decoder_pms() {
        let dec = decoder(Path::new("test.pms"));
        assert!(matches!(dec, Some(ChartDecoderImpl::Bms(_))));
    }

    #[test]
    fn get_decoder_bmson() {
        let dec = decoder(Path::new("test.bmson"));
        assert!(matches!(dec, Some(ChartDecoderImpl::Bmson(_))));
    }

    #[test]
    fn get_decoder_osu() {
        let dec = decoder(Path::new("test.osu"));
        assert!(matches!(dec, Some(ChartDecoderImpl::Osu(_))));
    }

    #[test]
    fn get_decoder_unknown_extension() {
        assert!(decoder(Path::new("test.mp3")).is_none());
    }

    #[test]
    fn get_decoder_case_insensitive() {
        let dec = decoder(Path::new("test.BMS"));
        assert!(matches!(dec, Some(ChartDecoderImpl::Bms(_))));

        let dec = decoder(Path::new("test.Bme"));
        assert!(matches!(dec, Some(ChartDecoderImpl::Bms(_))));

        let dec = decoder(Path::new("test.BMSON"));
        assert!(matches!(dec, Some(ChartDecoderImpl::Bmson(_))));
    }
}
