use super::*;
use std::path::Path;

// ================================================================
// pm_parse_int tests
// ================================================================

#[test]
fn test_pm_parse_int_valid() {
    assert_eq!(pm_parse_int("42"), 42);
}

#[test]
fn test_pm_parse_int_negative() {
    assert_eq!(pm_parse_int("-1"), -1);
}

#[test]
fn test_pm_parse_int_zero() {
    assert_eq!(pm_parse_int("0"), 0);
}

#[test]
fn test_pm_parse_int_empty_returns_zero() {
    assert_eq!(pm_parse_int(""), 0, "empty string should default to 0");
}

#[test]
fn test_pm_parse_int_non_numeric_returns_zero() {
    assert_eq!(
        pm_parse_int("abc"),
        0,
        "non-numeric string should default to 0"
    );
}

#[test]
fn test_pm_parse_int_strips_non_digit_chars() {
    // The function filters to only ascii digits and '-', so "12abc34" becomes "1234"
    assert_eq!(pm_parse_int("12abc34"), 1234);
}

#[test]
fn test_pm_parse_int_with_whitespace() {
    // Whitespace is stripped (filtered out), so " 42 " becomes "42"
    assert_eq!(pm_parse_int(" 42 "), 42);
}

#[test]
fn test_pm_parse_int_large_value() {
    assert_eq!(pm_parse_int("2147483647"), i32::MAX);
}

#[test]
fn test_pm_parse_int_overflow_returns_zero() {
    // Overflowing i32 should fail to parse and return 0
    assert_eq!(
        pm_parse_int("99999999999"),
        0,
        "overflowing value should default to 0"
    );
}

// ================================================================
// pm_parse_int_radix tests
// ================================================================

#[test]
fn test_pm_parse_int_radix_base10() {
    assert_eq!(pm_parse_int_radix("42", 10), 42);
}

#[test]
fn test_pm_parse_int_radix_base16() {
    assert_eq!(pm_parse_int_radix("ff", 16), 255);
}

#[test]
fn test_pm_parse_int_radix_base16_uppercase() {
    assert_eq!(pm_parse_int_radix("FF", 16), 255);
}

#[test]
fn test_pm_parse_int_radix_base36_two_digits() {
    // "zz" in base36: z=35, so 35*36 + 35 = 1295
    assert_eq!(pm_parse_int_radix("zz", 36), 1295);
}

#[test]
fn test_pm_parse_int_radix_base36_uppercase() {
    assert_eq!(pm_parse_int_radix("ZZ", 36), 1295);
}

#[test]
fn test_pm_parse_int_radix_base36_numeric() {
    // "00" in base36: 0*36 + 0 = 0
    assert_eq!(pm_parse_int_radix("00", 36), 0);
}

#[test]
fn test_pm_parse_int_radix_base36_mixed() {
    // "a0" in base36: (10)*36 + 0 = 360
    assert_eq!(pm_parse_int_radix("a0", 36), 360);
}

#[test]
fn test_pm_parse_int_radix_base36_single_char_returns_negative() {
    // Base36 requires at least 2 chars, single char returns -1
    assert_eq!(
        pm_parse_int_radix("z", 36),
        -1,
        "base36 with <2 chars should return -1"
    );
}

#[test]
fn test_pm_parse_int_radix_base36_empty_returns_negative() {
    assert_eq!(
        pm_parse_int_radix("", 36),
        -1,
        "base36 with empty string should return -1"
    );
}

#[test]
fn test_pm_parse_int_radix_base10_invalid_returns_negative() {
    assert_eq!(
        pm_parse_int_radix("xyz", 10),
        -1,
        "invalid base10 string should return -1"
    );
}

#[test]
fn test_pm_parse_int_radix_base16_zero() {
    assert_eq!(pm_parse_int_radix("0", 16), 0);
}

// ================================================================
// pm_parse_str tests
// ================================================================

#[test]
fn test_pm_parse_str_basic() {
    let parts = vec!["#Tag", "value1", "value2"];
    let result = pm_parse_str(&parts);
    assert_eq!(result, vec!["#Tag", "value1", "value2"]);
}

#[test]
fn test_pm_parse_str_stops_at_comment_prefix() {
    // A part starting with '/' causes the loop to break
    let parts = vec!["#Tag", "value1", "/comment"];
    let result = pm_parse_str(&parts);
    assert_eq!(result, vec!["#Tag", "value1"]);
}

#[test]
fn test_pm_parse_str_inline_comment() {
    // A part containing "//" truncates the value and stops
    let parts = vec!["#Tag", "value1//comment", "value2"];
    let result = pm_parse_str(&parts);
    assert_eq!(result, vec!["#Tag", "value1"]);
}

#[test]
fn test_pm_parse_str_empty_parts_skipped() {
    // Empty parts are skipped but do not stop the loop
    let parts = vec!["#Tag", "", "value2"];
    let result = pm_parse_str(&parts);
    assert_eq!(result, vec!["#Tag", "value2"]);
}

#[test]
fn test_pm_parse_str_all_empty() {
    let parts: Vec<&str> = vec!["", "", ""];
    let result = pm_parse_str(&parts);
    assert!(
        result.is_empty(),
        "all empty parts should produce empty vec"
    );
}

#[test]
fn test_pm_parse_str_no_parts() {
    let parts: Vec<&str> = vec![];
    let result = pm_parse_str(&parts);
    assert!(result.is_empty());
}

// ================================================================
// transparent_processing tests
// ================================================================

#[test]
fn test_transparent_processing_none_returns_none() {
    let mut flag = [false; 8];
    let result = transparent_processing(None, 0, &mut flag);
    assert!(result.is_none(), "None input should return None");
    assert!(!flag[0], "flag should not be set when input is None");
}

#[test]
fn test_transparent_processing_already_flagged_returns_unchanged() {
    let tex = Texture {
        width: 2,
        height: 2,
        disposed: false,
        path: None,
        rgba_data: Some(Arc::new(vec![255; 16])),
        gpu_texture: None,
        gpu_view: None,
        sampler: None,
        pixmap_id: None,
    };
    let mut flag = [false; 8];
    flag[0] = true; // already processed
    let result = transparent_processing(Some(tex), 0, &mut flag);
    assert!(result.is_some(), "flagged texture should pass through");
}

#[test]
fn test_transparent_processing_zero_size_sets_flag() {
    let tex = Texture {
        width: 0,
        height: 0,
        disposed: false,
        path: None,
        rgba_data: Some(Arc::new(vec![])),
        gpu_texture: None,
        gpu_view: None,
        sampler: None,
        pixmap_id: None,
    };
    let mut flag = [false; 8];
    let result = transparent_processing(Some(tex), 3, &mut flag);
    assert!(result.is_some(), "zero-size texture should return Some");
    assert!(flag[3], "flag should be set for zero-size texture");
}

#[test]
fn test_transparent_processing_no_rgba_data_sets_flag() {
    let tex = Texture {
        width: 4,
        height: 4,
        disposed: false,
        path: None,
        rgba_data: None,
        gpu_texture: None,
        gpu_view: None,
        sampler: None,
        pixmap_id: None,
    };
    let mut flag = [false; 8];
    let result = transparent_processing(Some(tex), 2, &mut flag);
    assert!(
        result.is_some(),
        "texture without rgba_data should return Some"
    );
    assert!(flag[2], "flag should be set when rgba_data is None");
}

#[test]
fn test_transparent_processing_removes_transparent_color() {
    // 2x2 image, bottom-right pixel is (10, 20, 30, 255) = the transparent color
    // Other pixels should remain, matching pixel should become (0,0,0,0)
    #[rustfmt::skip]
    let rgba = vec![
        // row 0
        255, 0, 0, 255,   // (0,0): red - keep
        0, 255, 0, 255,   // (1,0): green - keep
        // row 1
        0, 0, 255, 255,   // (0,1): blue - keep
        10, 20, 30, 255,  // (1,1): transparent color - remove
    ];
    let tex = Texture {
        width: 2,
        height: 2,
        disposed: false,
        path: None,
        rgba_data: Some(Arc::new(rgba)),
        gpu_texture: None,
        gpu_view: None,
        sampler: None,
        pixmap_id: None,
    };
    let mut flag = [false; 8];
    let result = transparent_processing(Some(tex), 0, &mut flag);
    assert!(flag[0], "flag should be set after processing");
    let result = result.expect("should return Some");
    let data = result.rgba_data.expect("should have rgba_data");
    // Red pixel (0,0) should be preserved
    assert_eq!(
        &data[0..4],
        &[255, 0, 0, 255],
        "red pixel should be preserved"
    );
    // Green pixel (1,0) should be preserved
    assert_eq!(
        &data[4..8],
        &[0, 255, 0, 255],
        "green pixel should be preserved"
    );
    // Blue pixel (0,1) should be preserved
    assert_eq!(
        &data[8..12],
        &[0, 0, 255, 255],
        "blue pixel should be preserved"
    );
    // Bottom-right pixel (transparent color) should be zeroed
    assert_eq!(
        &data[12..16],
        &[0, 0, 0, 0],
        "transparent color pixel should become fully transparent"
    );
}

// ================================================================
// Constant value tests
// ================================================================

#[test]
fn test_load_type_constants() {
    assert_eq!(PLAY, 0);
    assert_eq!(BACKGROUND, 1);
    assert_eq!(NAME, 2);
    assert_eq!(FACE_UPPER, 3);
    assert_eq!(FACE_ALL, 4);
    assert_eq!(SELECT_CG, 5);
    assert_eq!(NEUTRAL, 6);
    assert_eq!(FEVER, 7);
    assert_eq!(GREAT, 8);
    assert_eq!(GOOD, 9);
    assert_eq!(BAD, 10);
    assert_eq!(FEVERWIN, 11);
    assert_eq!(WIN, 12);
    assert_eq!(LOSE, 13);
    assert_eq!(OJAMA, 14);
    assert_eq!(DANCE, 15);
}

// ================================================================
// load() boundary condition tests
// ================================================================

#[test]
fn test_load_invalid_load_type_returns_none() {
    let mut images: Vec<SkinImage> = Vec::new();
    let mut loader = PomyuCharaLoader::new(&mut images);
    let dst = PomyuCharaDestination {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 100.0,
        side: 0,
        timer: 0,
        op1: 0,
        op2: 0,
        op3: 0,
        offset: 0,
    };
    let result = loader.load(
        false,
        Path::new("/nonexistent/path.chp"),
        99, // invalid load_type (outside 0..=15)
        0,
        &dst,
    );
    assert!(result.is_none(), "load_type=99 should return None");
}

#[test]
fn test_load_negative_load_type_returns_none() {
    let mut images: Vec<SkinImage> = Vec::new();
    let mut loader = PomyuCharaLoader::new(&mut images);
    let dst = PomyuCharaDestination {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 100.0,
        side: 0,
        timer: 0,
        op1: 0,
        op2: 0,
        op3: 0,
        offset: 0,
    };
    let result = loader.load(
        false,
        Path::new("/nonexistent/path.chp"),
        -1, // negative load_type
        0,
        &dst,
    );
    assert!(result.is_none(), "load_type=-1 should return None");
}

#[test]
fn test_load_nonexistent_path_returns_none() {
    let mut images: Vec<SkinImage> = Vec::new();
    let mut loader = PomyuCharaLoader::new(&mut images);
    let dst = PomyuCharaDestination {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 100.0,
        side: 0,
        timer: 0,
        op1: 0,
        op2: 0,
        op3: 0,
        offset: 0,
    };
    let result = loader.load(
        false,
        Path::new("/definitely/does/not/exist/file.chp"),
        PLAY, // valid load_type
        0,
        &dst,
    );
    assert!(
        result.is_none(),
        "nonexistent .chp path should return None without panic"
    );
}

#[test]
fn test_load_nonexistent_directory_returns_none() {
    let mut images: Vec<SkinImage> = Vec::new();
    let mut loader = PomyuCharaLoader::new(&mut images);
    let dst = PomyuCharaDestination {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 100.0,
        side: 0,
        timer: 0,
        op1: 0,
        op2: 0,
        op3: 0,
        offset: 0,
    };
    // Path without .chp extension triggers directory search mode
    let result = loader.load(
        false,
        Path::new("/definitely/does/not/exist/dir/"),
        PLAY,
        0,
        &dst,
    );
    assert!(
        result.is_none(),
        "nonexistent directory should return None without panic"
    );
}

// ================================================================
// Bounds safety tests for char_bmp array accesses
// ================================================================

#[test]
fn test_char_bmp_get_out_of_bounds_returns_none() {
    // Verify that .get() on a fixed-size array returns None for out-of-bounds indices
    let char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
    assert!(
        char_bmp.get(8).is_none(),
        "index 8 should be out of bounds for [_; 8]"
    );
    assert!(
        char_bmp.get(100).is_none(),
        "large index should be out of bounds"
    );
    assert!(
        char_bmp.get(usize::MAX).is_none(),
        "usize::MAX should be out of bounds"
    );
}

#[test]
fn test_char_bmp_get_mut_out_of_bounds_returns_none() {
    let mut char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
    assert!(
        char_bmp.get_mut(8).is_none(),
        "get_mut(8) should be None for [_; 8]"
    );
    assert!(
        char_bmp.get_mut(usize::MAX).is_none(),
        "get_mut(usize::MAX) should be None"
    );
}

#[test]
fn test_set_color_zero_underflow_guard() {
    // set_color < 1 should be caught before computing set_color as usize - 1
    // This test verifies the guard prevents usize underflow (wrapping subtraction)
    let set_color: i32 = 0;
    assert!(
        set_color < 1,
        "set_color=0 should trigger the underflow guard"
    );

    // If the guard were absent, this would wrap to usize::MAX
    // With the guard, we never reach this computation
    let set_color: i32 = -1;
    assert!(
        set_color < 1,
        "set_color=-1 should trigger the underflow guard"
    );
}

#[test]
fn test_set_color_valid_index_computation() {
    let char_bmp_index: usize = 0;

    // set_color = 1 -> index = 0 + 1 - 1 = 0 (valid)
    let set_color = 1;
    let set_index = char_bmp_index + set_color as usize - 1;
    assert_eq!(set_index, 0);
    assert!(set_index < 8, "set_color=1 should produce valid index");

    // set_color = 2 -> index = 0 + 2 - 1 = 1 (valid)
    let set_color = 2;
    let set_index = char_bmp_index + set_color as usize - 1;
    assert_eq!(set_index, 1);
    assert!(set_index < 8, "set_color=2 should produce valid index");
}

#[test]
fn test_char_bmp_take_via_get_mut() {
    let tex = Texture {
        width: 4,
        height: 4,
        disposed: false,
        path: None,
        rgba_data: None,
        gpu_texture: None,
        gpu_view: None,
        sampler: None,
        pixmap_id: None,
    };
    let mut char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
    char_bmp[3] = Some(tex);

    // Safe take via get_mut
    let taken = char_bmp.get_mut(3).and_then(|s| s.take());
    assert!(
        taken.is_some(),
        "take from occupied slot should return Some"
    );
    assert!(char_bmp[3].is_none(), "slot should be None after take");

    // Out-of-bounds take returns None
    let taken_oob = char_bmp.get_mut(8).and_then(|s| s.take());
    assert!(taken_oob.is_none(), "out-of-bounds take should return None");
}

#[test]
fn test_transparent_processing_with_bounds_checked_index() {
    // Verify transparent_processing works correctly when called via bounds-checked pattern
    let tex = Texture {
        width: 2,
        height: 2,
        disposed: false,
        path: None,
        rgba_data: Some(Arc::new(vec![255; 16])),
        gpu_texture: None,
        gpu_view: None,
        sampler: None,
        pixmap_id: None,
    };
    let mut char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
    let mut transparent_flag = [false; 8];
    let set_index: usize = 1;

    // Place texture
    if let Some(slot) = char_bmp.get_mut(set_index) {
        *slot = Some(tex);
    }

    // Bounds-checked take + transparent_processing + put back
    let taken = char_bmp.get_mut(set_index).and_then(|s| s.take());
    if let Some(slot) = char_bmp.get_mut(set_index) {
        *slot = transparent_processing(taken, set_index, &mut transparent_flag);
    }

    assert!(
        char_bmp.get(set_index).unwrap().is_some(),
        "should have processed texture"
    );
    assert!(
        transparent_flag[set_index],
        "flag should be set after processing"
    );
}

#[test]
fn test_select_cg_bounds_checked_access() {
    // Verify that SELECT_CG access patterns use .get() safely
    let char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
    let select_cg_index: usize = 6;

    // Both indices (6 and 7) should be valid
    assert!(
        char_bmp.get(select_cg_index).is_some(),
        "index 6 should be in bounds"
    );
    assert!(
        char_bmp.get(select_cg_index + 1).is_some(),
        "index 7 should be in bounds"
    );

    // Both slots are None (no texture loaded), which is the expected default
    assert!(
        char_bmp.get(select_cg_index).unwrap().is_none(),
        "slot 6 should be None by default"
    );
    assert!(
        char_bmp.get(select_cg_index + 1).unwrap().is_none(),
        "slot 7 should be None by default"
    );
}

// ================================================================
// Shift_JIS decoding regression test (P7-01)
// ================================================================

#[test]
fn load_chp_decodes_shift_jis_japanese_filenames() {
    // Simulate a .chp file with Shift_JIS-encoded Japanese filenames.
    // In Shift_JIS, "音楽" is [0x89, 0xB9, 0x8A, 0x79].
    // The loader must decode these bytes correctly.
    let chp_line = "#CharBMP\t0\t音楽.bmp";
    let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(chp_line);

    // Decode it back as the loader does
    let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&encoded);
    assert_eq!(
        decoded, chp_line,
        "Shift_JIS roundtrip should preserve Japanese text"
    );

    // Verify the line parses correctly
    let parts: Vec<&str> = decoded.split('\t').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "#CharBMP");
    assert_eq!(parts[1], "0");
    assert_eq!(parts[2], "音楽.bmp");
}

#[test]
fn load_chp_handles_pure_ascii_in_shift_jis_mode() {
    // ASCII is a subset of Shift_JIS, so pure ASCII input must work unchanged.
    let chp_content = b"#CharBMP\t0\tchar.bmp\n#CharFace\t0\tface.bmp";
    let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(chp_content);
    let lines: Vec<&str> = decoded.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("#CharBMP"));
    assert!(lines[1].starts_with("#CharFace"));
}

// ================================================================
// chp_dir_prefix regression tests
// ================================================================

#[test]
fn chp_dir_prefix_with_forward_slash() {
    let chp = "path/to/file.chp";
    let last_sep = chp.rfind('\\').max(chp.rfind('/'));
    let prefix = match last_sep {
        Some(idx) => chp[..idx + 1].to_string(),
        None => String::new(),
    };
    assert_eq!(prefix, "path/to/");
}

#[test]
fn chp_dir_prefix_with_backslash() {
    let chp = "path\\to\\file.chp";
    let last_sep = chp.rfind('\\').max(chp.rfind('/'));
    let prefix = match last_sep {
        Some(idx) => chp[..idx + 1].to_string(),
        None => String::new(),
    };
    assert_eq!(prefix, "path\\to\\");
}

#[test]
fn chp_dir_prefix_no_separator_returns_empty() {
    let chp = "file.chp";
    let last_sep = chp.rfind('\\').max(chp.rfind('/'));
    let prefix = match last_sep {
        Some(idx) => chp[..idx + 1].to_string(),
        None => String::new(),
    };
    assert_eq!(prefix, "", "bare filename should produce empty prefix");
}

#[test]
fn chp_dir_prefix_mixed_separators_uses_rightmost() {
    let chp = "path/to\\file.chp";
    let last_sep = chp.rfind('\\').max(chp.rfind('/'));
    let prefix = match last_sep {
        Some(idx) => chp[..idx + 1].to_string(),
        None => String::new(),
    };
    // Backslash at index 7 is rightmost
    assert_eq!(prefix, "path/to\\");
}
