//! RED tests demonstrating HTTP parameter injection via missing URL encoding.
//!
//! These tests prove that `LR2IRSongData::to_url_encoded_form()` and the URL
//! construction in `LR2IRConnection` do NOT encode special URL characters,
//! allowing parameter injection attacks. No actual HTTP requests are made.

use rubato_ir::lr2_ir_connection::LR2IRSongData;

// ---------------------------------------------------------------------------
// LR2IRSongData::to_url_encoded_form — md5 field injection
// ---------------------------------------------------------------------------

#[test]
fn to_url_encoded_form_md5_with_ampersand_injects_extra_parameter() {
    // An attacker-controlled md5 containing "&extra=payload" should be encoded
    // as "%26extra%3Dpayload" so the server sees it as part of the songmd5 value.
    // Instead, the raw "&" splits it into a separate parameter.
    let song = LR2IRSongData::new("abc&extra=payload".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();

    // BUG: The raw "&" is present, creating an injected "extra=payload" parameter.
    assert!(
        form.contains("&extra=payload"),
        "Expected injected parameter in form body, got: {}",
        form
    );
    // If properly encoded, the md5 value would contain "%26" instead of "&".
    assert!(
        !form.contains("%26"),
        "md5 ampersand should have been percent-encoded but was not: {}",
        form
    );
}

#[test]
fn to_url_encoded_form_md5_with_equals_sign_not_encoded() {
    // "=" inside the md5 value should be encoded as "%3D" so the server
    // doesn't interpret it as a key=value separator within the songmd5 field.
    let song = LR2IRSongData::new("abc=def".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();

    // BUG: Raw "=" in the value — the server may misparse the field boundary.
    assert_eq!(form, "songmd5=abc=def&id=1&lastupdate=");
    assert!(
        !form.contains("%3D"),
        "md5 equals sign should have been percent-encoded but was not: {}",
        form
    );
}

#[test]
fn to_url_encoded_form_md5_with_question_mark_not_encoded() {
    let song = LR2IRSongData::new("abc?query=1".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();

    // BUG: "?" is not encoded. In a URL context this would start a query string.
    assert!(
        form.contains("abc?query=1"),
        "Expected raw '?' in form body, got: {}",
        form
    );
    assert!(
        !form.contains("%3F"),
        "md5 question mark should have been percent-encoded but was not: {}",
        form
    );
}

#[test]
fn to_url_encoded_form_md5_with_hash_not_encoded() {
    let song = LR2IRSongData::new("abc#fragment".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();

    // BUG: "#" is not encoded. In a URL context this starts a fragment,
    // causing the server to never see anything after it.
    assert!(
        form.contains("abc#fragment"),
        "Expected raw '#' in form body, got: {}",
        form
    );
    assert!(
        !form.contains("%23"),
        "md5 hash should have been percent-encoded but was not: {}",
        form
    );
}

#[test]
fn to_url_encoded_form_md5_with_space_not_encoded() {
    let song = LR2IRSongData::new("abc def".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();

    // BUG: Space is not encoded as "+" or "%20".
    assert!(
        form.contains("abc def"),
        "Expected raw space in form body, got: {}",
        form
    );
    assert!(
        !form.contains("%20") && !form.contains("abc+def"),
        "md5 space should have been percent-encoded but was not: {}",
        form
    );
}

// ---------------------------------------------------------------------------
// LR2IRSongData::to_url_encoded_form — id field injection
// ---------------------------------------------------------------------------

#[test]
fn to_url_encoded_form_id_with_ampersand_injects_extra_parameter() {
    let song = LR2IRSongData::new(
        "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        "1&admin=true".to_string(),
    );
    let form = song.to_url_encoded_form();

    // BUG: The id field's "&admin=true" becomes a separate parameter.
    assert!(
        form.contains("&admin=true"),
        "Expected injected parameter via id field, got: {}",
        form
    );
}

// ---------------------------------------------------------------------------
// LR2IRSongData::to_url_encoded_form — last_update field injection
// ---------------------------------------------------------------------------

#[test]
fn to_url_encoded_form_last_update_with_ampersand_injects_extra_parameter() {
    let mut song = LR2IRSongData::new(
        "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        "1".to_string(),
    );
    song.last_update = "2024-01-01&admin=true".to_string();
    let form = song.to_url_encoded_form();

    // BUG: The last_update field's "&admin=true" becomes a separate parameter.
    assert!(
        form.contains("&admin=true"),
        "Expected injected parameter via last_update field, got: {}",
        form
    );
}

// ---------------------------------------------------------------------------
// Multiple special characters combined
// ---------------------------------------------------------------------------

#[test]
fn to_url_encoded_form_md5_with_multiple_special_chars() {
    // Combining multiple dangerous characters in a single value.
    let song = LR2IRSongData::new("a&b=c#d?e f".to_string(), "1".to_string());
    let form = song.to_url_encoded_form();

    // BUG: None of the special characters are encoded.
    assert!(
        form.contains("a&b=c#d?e f"),
        "Expected all special chars raw in form body, got: {}",
        form
    );
    // A correct implementation would produce something like:
    // "songmd5=a%26b%3Dc%23d%3Fe+f&id=1&lastupdate="
    assert!(
        !form.contains("%26"),
        "No percent-encoding applied: {}",
        form
    );
}
