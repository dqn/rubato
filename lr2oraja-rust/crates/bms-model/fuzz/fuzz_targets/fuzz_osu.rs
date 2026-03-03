#![no_main]

use libfuzzer_sys::fuzz_target;

use bms_model::osu::Osu;
use bms_model::osu_decoder::OSUDecoder;
use std::io::{BufReader, Cursor, Write};

fuzz_target!(|data: &[u8]| {
    // Layer 1: Test Osu::parse with arbitrary bytes via in-memory reader.
    // The parser should never panic on malformed input.
    let lossy = String::from_utf8_lossy(data);
    let mut reader = BufReader::new(Cursor::new(lossy.as_bytes()));
    let _ = Osu::parse(&mut reader);

    // Layer 2: Exercise the full OSU decoder pipeline via a temp file.
    // This also exercises Shift_JIS decoding and hash computation.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fuzz.osu");
    if let Ok(mut f) = std::fs::File::create(&path) {
        if f.write_all(data).is_ok() {
            let mut decoder = OSUDecoder::new(0);
            let _ = decoder.decode_path(&path);
        }
    }
});
