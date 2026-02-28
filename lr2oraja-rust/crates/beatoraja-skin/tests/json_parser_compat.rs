// Integration tests documenting Gson vs serde_json parser incompatibilities.
//
// These tests are GREEN: they assert what serde_json ACTUALLY does (not what
// we wish it did). Each test exposes a gap between Java Gson's lenient parsing
// and serde_json's strict parsing that could cause real-world skin files to
// fail loading after the Java → Rust port.

use regex::Regex;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Helper: reimplements the same regex pipeline as `fix_lenient_json()` in
// `json_skin_loader.rs` (which is `fn`, not `pub fn`, so we cannot call it
// from an integration test).
// ---------------------------------------------------------------------------
fn fix_lenient_json(json: &str) -> String {
    let trailing_comma = Regex::new(r",(\s*[}\]])").unwrap();
    let missing_comma = Regex::new(r"\}(\s*)\{").unwrap();
    let fixed = trailing_comma.replace_all(json, "$1");
    missing_comma.replace_all(&fixed, "},$1{").into_owned()
}

// =========================================================================
// 1. serde_json rejects `//` line comments (Gson accepts them in lenient mode)
// =========================================================================
#[test]
fn json_line_comment_rejected() {
    let json = r#"{
        // this is a line comment
        "key": "value"
    }"#;

    let result: Result<serde_json::Value, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "serde_json should reject line comments, but it accepted the input"
    );
}

// =========================================================================
// 2. serde_json rejects `/* */` block comments (Gson accepts them)
// =========================================================================
#[test]
fn json_block_comment_rejected() {
    let json = r#"{
        /* this is a block comment */
        "key": "value"
    }"#;

    let result: Result<serde_json::Value, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "serde_json should reject block comments, but it accepted the input"
    );
}

// =========================================================================
// 3. fix_lenient_json corrupts `}{` sequences inside string values
//    The MISSING_COMMA regex `}\s*{` does not distinguish between braces
//    inside strings vs structural braces, so it inserts a spurious comma.
//
//    Input:  `{"key": "a}{b"}`  (value is the 4-char string `a}{b`)
//    Output: `{"key": "a},{b"}` (value becomes the 5-char string `a},{b`)
//
//    The JSON still parses — but the string data is silently corrupted.
//    Gson would preserve the original value; fix_lenient_json mutates it.
// =========================================================================
#[test]
fn fix_lenient_json_corrupts_braces_in_strings() {
    let json = r#"{"key": "a}{b"}"#;
    let fixed = fix_lenient_json(json);

    // The regex sees `}{` inside the string value and inserts a comma,
    // corrupting the JSON content.
    assert_ne!(
        json, &fixed,
        "fix_lenient_json should have (incorrectly) modified the string — \
         the regex does not skip string interiors"
    );

    // The corrupted output still parses as valid JSON, but the string value
    // has been silently changed from "a}{b" to "a},{b".
    let original: serde_json::Value = serde_json::from_str(json).unwrap();
    let corrupted: serde_json::Value = serde_json::from_str(&fixed).unwrap();

    let original_val = original["key"].as_str().unwrap();
    let corrupted_val = corrupted["key"].as_str().unwrap();

    assert_eq!(original_val, "a}{b");
    assert_eq!(corrupted_val, "a},{b");
    assert_ne!(
        original_val, corrupted_val,
        "fix_lenient_json silently corrupted the string value"
    );
}

// =========================================================================
// 4. serde_json rejects numeric values for String-typed fields
//    Gson silently coerces `123` → `"123"` when the target field is String.
//    serde_json requires the JSON value to already be a string.
// =========================================================================
#[test]
fn json_numeric_to_string_rejected() {
    #[derive(Deserialize)]
    struct WithStringPath {
        #[allow(dead_code)]
        path: String,
    }

    let json = r#"{"path": 123}"#;
    let result: Result<WithStringPath, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "serde_json should reject numeric value for a String field, \
         but it accepted the input"
    );
}

// =========================================================================
// 5. serde_json rejects single-quoted strings (Gson accepts them)
// =========================================================================
#[test]
fn json_single_quoted_string_rejected() {
    let json = "{'key': 'value'}";

    let result: Result<serde_json::Value, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "serde_json should reject single-quoted strings, but it accepted the input"
    );
}

// =========================================================================
// 6. serde_json rejects unquoted keys (Gson accepts them in lenient mode)
// =========================================================================
#[test]
fn json_unquoted_key_rejected() {
    let json = r#"{key: "value"}"#;

    let result: Result<serde_json::Value, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "serde_json should reject unquoted keys, but it accepted the input"
    );
}

// =========================================================================
// 7. fix_lenient_json does NOT handle a trailing comma after a plain value
//    The regex `,(\s*[}\]])` DOES strip this pattern. This test verifies
//    that the fix actually works for simple trailing commas.
// =========================================================================
#[test]
fn fix_lenient_json_trailing_comma_after_value() {
    // `{"a": 1,}` — trailing comma after a numeric value
    let json = r#"{"a": 1,}"#;
    let fixed = fix_lenient_json(json);

    // The TRAILING_COMMA regex `,(\s*[}\]])` matches `,}` and replaces it
    // with just `}`, so this IS handled.
    let result: Result<serde_json::Value, _> = serde_json::from_str(&fixed);
    assert!(
        result.is_ok(),
        "fix_lenient_json should strip trailing comma after a plain value: \
         fixed={fixed:?}, err={:?}",
        result.err()
    );
}

// =========================================================================
// 8. fix_lenient_json handles `},}` (nested object with trailing comma)
//    This is the primary pattern the regex was designed for.
// =========================================================================
#[test]
fn fix_lenient_json_nested_trailing_comma_handled() {
    let json = r#"{"a": {"b": 1},}"#;
    let fixed = fix_lenient_json(json);

    let result: Result<serde_json::Value, _> = serde_json::from_str(&fixed);
    assert!(
        result.is_ok(),
        "fix_lenient_json should handle nested object with trailing comma: \
         fixed={fixed:?}, err={:?}",
        result.err()
    );
}
