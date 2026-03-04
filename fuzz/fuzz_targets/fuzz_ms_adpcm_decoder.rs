#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use rubato_audio::ms_adpcm_decoder::MSADPCMDecoder;

/// Structured input for MS-ADPCM fuzzing: decoder parameters + raw audio data.
#[derive(Arbitrary, Debug)]
struct AdpcmInput {
    channels: u8,
    sample_rate: u16,
    block_align: u16,
    data: Vec<u8>,
}

fuzz_target!(|input: AdpcmInput| {
    // Clamp channels to a reasonable range to avoid massive allocations.
    // 0 is rejected by the constructor; 1-8 covers mono through 7.1 surround.
    let channels = (input.channels % 8) as i32 + 1;
    let sample_rate = input.sample_rate.max(1) as i32;
    // block_align must be > channels * 6 to get positive samples_per_block
    let block_align = input.block_align.max(1) as i32;

    if let Ok(mut decoder) = MSADPCMDecoder::new(channels, sample_rate, block_align) {
        // Ignore decode errors -- we care about panics and OOB access
        let _ = decoder.decode(&input.data);
    }
});
