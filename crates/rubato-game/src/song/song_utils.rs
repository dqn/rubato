use std::path::Path;

const POLYNOMIAL: u32 = 0xEDB88320;

pub fn crc32(path: &str, rootdirs: &[String], bmspath: &str) -> String {
    let mut path = path.to_string();

    for s in rootdirs {
        if let Some(parent) = Path::new(s).parent()
            && parent.to_string_lossy() == path
        {
            return "e2977170".to_string();
        }
    }

    if path.starts_with(bmspath)
        && path.len() > bmspath.len() + 1
        && path
            .as_bytes()
            .get(bmspath.len())
            .is_some_and(|&b| b == b'/' || b == b'\\')
    {
        path = path[bmspath.len() + 1..].to_string();
    }

    let previous_crc32: u32 = 0;
    let mut crc: u32 = !previous_crc32; // same as previousCrc32 ^ 0xFFFFFFFF

    // Java's getBytes() on Windows uses MS932 (Shift_JIS). Encode the path
    // as Shift_JIS bytes to match Java's CRC computation for Japanese paths.
    let bytes_str = format!("{}\\\0", path);
    let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(&bytes_str);
    for b in encoded.iter() {
        crc ^= *b as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ POLYNOMIAL;
            } else {
                crc >>= 1;
            }
        }
    }
    format!("{:x}", !crc) // same as crc ^ 0xFFFFFFFF
}

pub static ILLEGAL_SONGS: &[&str] = &["notme"];

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: compute CRC without any rootdir or bmspath effects.
    /// Uses a bmspath that cannot match any realistic path to avoid
    /// the empty-string `starts_with("")` always-true trap.
    fn raw_crc32(path: &str) -> String {
        crc32(path, &[], "\x00\x00\x00NOMATCH")
    }

    #[test]
    fn crc32_path_matching_rootdir_parent() {
        // When path matches the parent of a rootdir entry, return the fixed sentinel value.
        let rootdirs = vec!["/music/bms/songs".to_string()];
        let result = crc32("/music/bms", &rootdirs, "\x00NOMATCH");
        assert_eq!(result, "e2977170");
    }

    #[test]
    fn crc32_bmspath_prefix_stripping() {
        // When path starts with bmspath, the prefix (plus one separator byte) is stripped
        // before hashing. So stripping "/music/bms" + "/" from "/music/bms/song.bme"
        // leaves "song.bme", which should hash identically to raw "song.bme".
        let rootdirs: Vec<String> = vec![];
        let with_prefix = crc32("/music/bms/song.bme", &rootdirs, "/music/bms");
        let without_prefix = raw_crc32("song.bme");
        assert_eq!(with_prefix, without_prefix);
    }

    #[test]
    fn crc32_empty_string() {
        // Empty path with no rootdir match and no bmspath stripping.
        let result = raw_crc32("");
        // Should not panic; produces a valid hex CRC for the bytes "\\\0".
        assert!(!result.is_empty());
        // Deterministic: same input produces same output.
        assert_eq!(result, raw_crc32(""));
    }

    #[test]
    fn crc32_non_ascii_japanese_path() {
        // Non-ASCII path encoded as Shift_JIS bytes before hashing.
        let result = raw_crc32("音楽/曲データ");
        assert!(!result.is_empty());
        // Deterministic.
        assert_eq!(result, raw_crc32("音楽/曲データ"));
        // Different from ASCII path.
        assert_ne!(result, raw_crc32("ascii/path"));
    }

    #[test]
    fn crc32_japanese_path_uses_shift_jis_encoding() {
        // Verify that CRC is computed over Shift_JIS bytes, not UTF-8.
        // "音" is 0xE9 0x9F 0xB3 in UTF-8 but 0x89 0xB9 in Shift_JIS.
        // If encoding were UTF-8, the hash would be different.
        let path = "音";
        let result = raw_crc32(path);

        // Manually compute CRC over Shift_JIS-encoded bytes to verify.
        let bytes_str = format!("{}\\\0", path);
        let (sjis_bytes, _, _) = encoding_rs::SHIFT_JIS.encode(&bytes_str);

        let polynomial: u32 = 0xEDB88320;
        let mut crc: u32 = 0xFFFFFFFF;
        for b in sjis_bytes.iter() {
            crc ^= *b as u32;
            for _ in 0..8 {
                if (crc & 1) != 0 {
                    crc = (crc >> 1) ^ polynomial;
                } else {
                    crc >>= 1;
                }
            }
        }
        let expected = format!("{:x}", !crc);
        assert_eq!(
            result, expected,
            "CRC should be computed over Shift_JIS bytes"
        );

        // Also verify it differs from a naive UTF-8 computation.
        let utf8_bytes = bytes_str.as_bytes();
        let mut crc_utf8: u32 = 0xFFFFFFFF;
        for b in utf8_bytes {
            crc_utf8 ^= *b as u32;
            for _ in 0..8 {
                if (crc_utf8 & 1) != 0 {
                    crc_utf8 = (crc_utf8 >> 1) ^ polynomial;
                } else {
                    crc_utf8 >>= 1;
                }
            }
        }
        let utf8_result = format!("{:x}", !crc_utf8);
        assert_ne!(
            result, utf8_result,
            "Shift_JIS CRC should differ from UTF-8 CRC for Japanese text"
        );
    }

    #[test]
    fn crc32_bmspath_longer_than_path() {
        // When bmspath is longer than path, starts_with returns false, so no stripping.
        let rootdirs: Vec<String> = vec![];
        let result = crc32("short", &rootdirs, "this/is/a/much/longer/bmspath");
        let expected = raw_crc32("short");
        assert_eq!(result, expected);
    }

    #[test]
    fn crc32_multiple_rootdirs_one_matches() {
        // The first rootdir whose parent matches the path triggers the sentinel.
        let rootdirs = vec![
            "/other/dir/songs".to_string(),
            "/music/bms/songs".to_string(),
            "/yet/another/path".to_string(),
        ];
        let result = crc32("/music/bms", &rootdirs, "\x00NOMATCH");
        assert_eq!(result, "e2977170");
    }

    #[test]
    fn crc32_no_rootdir_matches() {
        // No rootdir parent matches the path, so normal CRC is computed.
        let rootdirs = vec![
            "/other/dir/songs".to_string(),
            "/another/path/songs".to_string(),
        ];
        let result = crc32("/music/bms", &rootdirs, "\x00NOMATCH");
        assert_ne!(result, "e2977170");
        assert!(!result.is_empty());
    }

    #[test]
    fn crc32_bmspath_prefix_not_stripped_when_equal_length() {
        // path.len() must be > bmspath.len() + 1 for stripping to occur.
        // When path == bmspath, the length condition fails, so no stripping.
        let rootdirs: Vec<String> = vec![];
        let result = crc32("/music/bms", &rootdirs, "/music/bms");
        let no_strip = raw_crc32("/music/bms");
        assert_eq!(result, no_strip);
    }

    #[test]
    fn crc32_bmspath_prefix_not_stripped_when_one_char_longer() {
        // path.len() == bmspath.len() + 1 is NOT enough; needs > bmspath.len() + 1.
        // "/music/bms/".len() == 11, "/music/bms".len() + 1 == 11, 11 > 11 is false.
        let rootdirs: Vec<String> = vec![];
        let result = crc32("/music/bms/", &rootdirs, "/music/bms");
        let no_strip = raw_crc32("/music/bms/");
        assert_eq!(result, no_strip);
    }

    #[test]
    fn crc32_bmspath_prefix_stripped_when_two_chars_longer() {
        // path.len() == bmspath.len() + 2 satisfies > bmspath.len() + 1.
        // "/music/bms/x".len() == 12, "/music/bms".len() + 1 == 11, 12 > 11 is true.
        // Stripped path: "x".
        let rootdirs: Vec<String> = vec![];
        let result = crc32("/music/bms/x", &rootdirs, "/music/bms");
        let stripped = raw_crc32("x");
        assert_eq!(result, stripped);
    }

    #[test]
    fn crc32_bmspath_not_stripped_for_sibling_directory() {
        // /music/bms should NOT strip from /music/bms2/song.bme because
        // the character after the prefix is '2', not a separator.
        let rootdirs: Vec<String> = vec![];
        let result = crc32("/music/bms2/song.bme", &rootdirs, "/music/bms");
        let no_strip = raw_crc32("/music/bms2/song.bme");
        assert_eq!(result, no_strip, "sibling dir should not be stripped");
    }

    #[test]
    fn crc32_different_paths_produce_different_hashes() {
        let a = raw_crc32("path/a.bms");
        let b = raw_crc32("path/b.bms");
        assert_ne!(a, b);
    }

    #[test]
    fn crc32_rootdir_check_takes_priority_over_bmspath_stripping() {
        // Even if bmspath would match, rootdir sentinel check happens first.
        let rootdirs = vec!["/music/bms/songs".to_string()];
        let result = crc32("/music/bms", &rootdirs, "/music");
        assert_eq!(result, "e2977170");
    }

    #[test]
    fn crc32_empty_bmspath_no_stripping_without_separator() {
        // Empty bmspath: starts_with("") is always true, but the separator check
        // requires path[0] to be '/' or '\\'. Since "abcde" starts with 'a',
        // no stripping occurs.
        let rootdirs: Vec<String> = vec![];
        let result = crc32("abcde", &rootdirs, "");
        let expected = raw_crc32("abcde");
        assert_eq!(result, expected);
    }

    #[test]
    fn crc32_empty_bmspath_strips_when_path_starts_with_separator() {
        // Empty bmspath with a path starting with '/': the separator check passes,
        // so stripping removes the leading '/'.
        let rootdirs: Vec<String> = vec![];
        let result = crc32("/abcde", &rootdirs, "");
        let expected = raw_crc32("abcde");
        assert_eq!(result, expected);
    }
}
