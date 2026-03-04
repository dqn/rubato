#![no_main]

use libfuzzer_sys::fuzz_target;

use rubato_skin::lr2::lr2_skin_loader::LR2SkinLoaderState;

fuzz_target!(|data: &[u8]| {
    // LR2 skin files are Shift_JIS encoded CSV-like text.
    // Feed arbitrary bytes as UTF-8 lines through the directive processor.
    if let Ok(content) = std::str::from_utf8(data) {
        let mut state = LR2SkinLoaderState::new();

        // Process each line through the #IF/#ELSE/#ENDIF/#SETOPTION directive engine
        // and command dispatch. Pass None for MainState (no game state context).
        for line in content.lines() {
            let _ = state.process_line_directives(line, None);
        }
    }

    // Also test with raw Shift_JIS-decoded content
    let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(data);
    let mut state = LR2SkinLoaderState::new();
    for line in decoded.lines() {
        let _ = state.process_line_directives(line, None);
    }
});
