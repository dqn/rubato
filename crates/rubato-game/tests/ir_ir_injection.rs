//! GREEN tests verifying URL encoding prevents injection.
//!
//! These tests prove that `LR2IRSongData::to_url_encoded_form()` properly
//! encodes special URL characters, preventing parameter injection attacks.
//! No actual HTTP requests are made.

use rubato_game::ir::lr2_ir_connection::LR2IRSongData;

// ---------------------------------------------------------------------------
// LR2IRSongData::to_url_encoded_form — md5 field encoding
// ---------------------------------------------------------------------------

#[test]
fn to_url_encoded_form_md5_with_ampersand_is_encoded() {
    let song = LR2IRSongData::new("abc&extra=payload".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();
    assert!(
        form.contains("abc%26extra%3Dpayload"),
        "md5 ampersand and equals should be percent-encoded: {}",
        form
    );
    assert!(
        !form.contains("&extra=payload"),
        "raw injection should not be present: {}",
        form
    );
}

#[test]
fn to_url_encoded_form_md5_with_equals_sign_is_encoded() {
    let song = LR2IRSongData::new("abc=def".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();
    assert!(
        form.contains("songmd5=abc%3Ddef"),
        "equals sign should be percent-encoded: {}",
        form
    );
}

#[test]
fn to_url_encoded_form_md5_with_question_mark_is_encoded() {
    let song = LR2IRSongData::new("abc?query=1".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();
    assert!(
        form.contains("abc%3Fquery%3D1"),
        "question mark should be percent-encoded: {}",
        form
    );
}

#[test]
fn to_url_encoded_form_md5_with_hash_is_encoded() {
    let song = LR2IRSongData::new("abc#fragment".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();
    assert!(
        form.contains("abc%23fragment"),
        "hash should be percent-encoded: {}",
        form
    );
}

#[test]
fn to_url_encoded_form_md5_with_space_is_encoded() {
    let song = LR2IRSongData::new("abc def".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();
    assert!(
        form.contains("abc%20def") || form.contains("abc+def"),
        "space should be percent-encoded: {}",
        form
    );
}

// ---------------------------------------------------------------------------
// LR2IRSongData::to_url_encoded_form — id field encoding
// ---------------------------------------------------------------------------

#[test]
fn to_url_encoded_form_id_with_ampersand_is_encoded() {
    let song = LR2IRSongData::new(
        "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        "1&admin=true".to_string(),
    );
    let form = song.to_url_encoded_form();
    assert!(
        !form.contains("&admin=true"),
        "raw injection should not be present: {}",
        form
    );
    assert!(
        form.contains("1%26admin%3Dtrue"),
        "id field should be percent-encoded: {}",
        form
    );
}

// ---------------------------------------------------------------------------
// LR2IRSongData::to_url_encoded_form — last_update field encoding
// ---------------------------------------------------------------------------

#[test]
fn to_url_encoded_form_last_update_with_ampersand_is_encoded() {
    let mut song = LR2IRSongData::new(
        "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        "1".to_string(),
    );
    song.last_update = "2024-01-01&admin=true".to_string();
    let form = song.to_url_encoded_form();
    assert!(
        !form.contains("&admin=true"),
        "raw injection should not be present: {}",
        form
    );
    assert!(
        form.contains("2024-01-01%26admin%3Dtrue"),
        "last_update field should be percent-encoded: {}",
        form
    );
}

// ---------------------------------------------------------------------------
// Multiple special characters combined
// ---------------------------------------------------------------------------

#[test]
fn to_url_encoded_form_md5_with_multiple_special_chars_all_encoded() {
    let song = LR2IRSongData::new("a&b=c#d?e f".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();
    // All special characters should be encoded
    assert!(
        form.contains("%26"),
        "ampersand should be encoded: {}",
        form
    );
    assert!(form.contains("%3D"), "equals should be encoded: {}", form);
    assert!(form.contains("%23"), "hash should be encoded: {}", form);
    assert!(
        form.contains("%3F"),
        "question mark should be encoded: {}",
        form
    );
    assert!(
        form.contains("%20") || form.contains("+"),
        "space should be encoded: {}",
        form
    );
}
