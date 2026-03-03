#![no_main]

use libfuzzer_sys::fuzz_target;

use bms_model::bms_decoder::BMSDecoder;

fuzz_target!(|data: &[u8]| {
    // Test BMS decoder with arbitrary bytes.
    // The decoder should never panic - returning None is acceptable.
    let mut decoder = BMSDecoder::new();
    let _ = decoder.decode_bytes(data, false, None);

    // Also test PMS mode (popn 9k) parsing path
    let mut decoder_pms = BMSDecoder::new();
    let _ = decoder_pms.decode_bytes(data, true, None);
});
