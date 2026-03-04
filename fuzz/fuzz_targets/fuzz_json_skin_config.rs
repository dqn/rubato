#![no_main]

use libfuzzer_sys::fuzz_target;

use rubato_skin::json::json_skin::Skin;

fuzz_target!(|data: &[u8]| {
    // Only process valid UTF-8 strings (JSON requires UTF-8).
    if let Ok(s) = std::str::from_utf8(data) {
        // Attempt direct serde_json deserialization into the Skin config type.
        // This exercises all nested struct parsing, default value handling,
        // and serde(rename) / serde(default) attributes.
        let _ = serde_json::from_str::<Skin>(s);
    }
});
