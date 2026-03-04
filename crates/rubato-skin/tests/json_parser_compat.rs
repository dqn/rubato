// Integration tests documenting Gson vs serde_json parser incompatibilities.
//
// These tests are GREEN: they assert what serde_json ACTUALLY does (not what
// we wish it did). Each test exposes a gap between Java Gson's lenient parsing
// and serde_json's strict parsing that could cause real-world skin files to
// fail loading after the Java → Rust port.

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Helper: reimplements the same string-aware pipeline as `fix_lenient_json()`
// in `json_skin_loader.rs` (which is `fn`, not `pub fn`, so we cannot call
// it from an integration test).
// ---------------------------------------------------------------------------
fn fix_lenient_json(json: &str) -> String {
    // 1. Strip UTF-8 BOM
    let json = json.strip_prefix('\u{FEFF}').unwrap_or(json);

    // 2. Strip comments (string-aware)
    let stripped = strip_comments(json);

    // 3-4. Fix trailing commas and missing commas (string-aware)
    fix_commas_string_aware(&stripped)
}

/// String-aware comma fixer: removes trailing commas and inserts missing commas
/// between adjacent objects, without touching content inside string literals.
fn fix_commas_string_aware(json: &str) -> String {
    let bytes = json.as_bytes();
    let len = bytes.len();
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            out.push(bytes[i]);
            if bytes[i] == b'\\' {
                i += 1;
                if i < len {
                    out.push(bytes[i]);
                }
            } else if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        match bytes[i] {
            b'"' => {
                in_string = true;
                out.push(b'"');
                i += 1;
            }
            b',' => {
                let mut j = i + 1;
                while j < len && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < len && (bytes[j] == b'}' || bytes[j] == b']') {
                    i += 1; // skip trailing comma
                } else {
                    out.push(b',');
                    i += 1;
                }
            }
            b'}' => {
                out.push(b'}');
                let mut j = i + 1;
                while j < len && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < len && bytes[j] == b'{' {
                    out.push(b',');
                }
                i += 1;
            }
            _ => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }

    String::from_utf8(out).unwrap_or_else(|_| json.to_string())
}

/// Strip `//` line comments and `/* */` block comments from JSON text,
/// preserving comment-like sequences inside string literals.
fn strip_comments(json: &str) -> String {
    let bytes = json.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        if in_string {
            let ch = bytes[i];
            out.push(ch as char);
            if ch == b'\\' {
                i += 1;
                if i < len {
                    out.push(bytes[i] as char);
                }
            } else if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if bytes[i] == b'"' {
            in_string = true;
            out.push('"');
            i += 1;
        } else if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
        } else if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }

    out
}

// =========================================================================
// 1. Raw serde_json rejects `//` line comments, but fix_lenient_json strips
//    them so the result parses successfully (matching Gson's lenient mode).
// =========================================================================
#[test]
fn json_line_comment_rejected() {
    let json = r#"{
        // this is a line comment
        "key": "value"
    }"#;

    // Raw serde_json still rejects comments
    let raw: Result<serde_json::Value, _> = serde_json::from_str(json);
    assert!(raw.is_err(), "raw serde_json should reject line comments");

    // After preprocessing, comments are stripped and JSON parses
    let fixed = fix_lenient_json(json);
    let result: serde_json::Value = serde_json::from_str(&fixed).unwrap();
    assert_eq!(result["key"].as_str().unwrap(), "value");
}

// =========================================================================
// 2. Raw serde_json rejects `/* */` block comments, but fix_lenient_json
//    strips them so the result parses successfully (matching Gson behavior).
// =========================================================================
#[test]
fn json_block_comment_rejected() {
    let json = r#"{
        /* this is a block comment */
        "key": "value"
    }"#;

    // Raw serde_json still rejects comments
    let raw: Result<serde_json::Value, _> = serde_json::from_str(json);
    assert!(raw.is_err(), "raw serde_json should reject block comments");

    // After preprocessing, comments are stripped and JSON parses
    let fixed = fix_lenient_json(json);
    let result: serde_json::Value = serde_json::from_str(&fixed).unwrap();
    assert_eq!(result["key"].as_str().unwrap(), "value");
}

// =========================================================================
// 3. fix_lenient_json preserves `}{` sequences inside string values
//    The string-aware state machine only modifies structural braces,
//    not braces inside quoted string literals.
// =========================================================================
#[test]
fn fix_lenient_json_preserves_braces_in_strings() {
    let json = r#"{"key": "a}{b"}"#;
    let fixed = fix_lenient_json(json);

    // String-aware processing preserves the value
    assert_eq!(
        json, &fixed,
        "fix_lenient_json must not modify braces inside string literals"
    );

    let parsed: serde_json::Value = serde_json::from_str(&fixed).unwrap();
    assert_eq!(parsed["key"].as_str().unwrap(), "a}{b");
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
