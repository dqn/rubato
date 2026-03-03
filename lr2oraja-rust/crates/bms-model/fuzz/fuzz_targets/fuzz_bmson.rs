#![no_main]

use libfuzzer_sys::fuzz_target;

use bms_model::bmson::Bmson;
use bms_model::bmson_decoder::BMSONDecoder;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    // Layer 1: Test serde_json deserialization of Bmson struct.
    // Invalid JSON or schema mismatches should return Err, never panic.
    let _: Result<Bmson, _> = serde_json::from_slice(data);

    // Layer 2: Exercise the full decoder pipeline via a temp file.
    // The decoder reads the file, runs serde_json::from_slice, then processes
    // the result. For invalid JSON it returns None; for valid JSON with
    // extreme/unexpected values we want to verify no panics occur.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fuzz.bmson");
    if let Ok(mut f) = std::fs::File::create(&path) {
        if f.write_all(data).is_ok() {
            let mut decoder = BMSONDecoder::new(0);
            let _ = decoder.decode_path(&path);
        }
    }
});
