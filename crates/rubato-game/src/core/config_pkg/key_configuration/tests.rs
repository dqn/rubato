use super::KeyConfiguration;
use super::gdx_key_name::gdx_key_name;
use crate::core::main_state::MainStateData;
use crate::core::timer_manager::TimerManager;
use bms::model::mode::Mode;
use rubato_types::play_mode_config::{
    KeyboardConfig, MidiConfig, MidiInput, MidiInputType, PlayModeConfig,
};

/// Creates a PlayModeConfig for 7K mode (mode index 1 in KEYSA).
fn make_pmc() -> PlayModeConfig {
    PlayModeConfig::new(Mode::BEAT_7K)
}

/// Creates a KeyConfiguration with the given mode index, without requiring MainController.
fn make_kc(mode: usize) -> KeyConfiguration {
    KeyConfiguration {
        state_data: MainStateData::new(TimerManager::new()),
        cursorpos: 0,
        _scrollpos: 0,
        keyinput: false,
        mode,
        _deletepressed: false,
    }
}

// -- Getter tests --

#[test]
fn test_get_keyboard_key_assign_positive_index() {
    let pmc = make_pmc();
    let kb = &pmc.keyboard;
    // Index 0 should return the first assigned key value
    let val = KeyConfiguration::keyboard_key_assign(kb, 0);
    assert_eq!(val, kb.keys[0]);
}

#[test]
fn test_get_keyboard_key_assign_start() {
    let pmc = make_pmc();
    let kb = &pmc.keyboard;
    assert_eq!(KeyConfiguration::keyboard_key_assign(kb, -1), kb.start);
}

#[test]
fn test_get_keyboard_key_assign_select() {
    let pmc = make_pmc();
    let kb = &pmc.keyboard;
    assert_eq!(KeyConfiguration::keyboard_key_assign(kb, -2), kb.select);
}

#[test]
fn test_get_keyboard_key_assign_other_negative() {
    let pmc = make_pmc();
    assert_eq!(KeyConfiguration::keyboard_key_assign(&pmc.keyboard, -3), 0);
}

#[test]
fn test_get_keyboard_key_assign_out_of_bounds() {
    let pmc = make_pmc();
    assert_eq!(
        KeyConfiguration::keyboard_key_assign(&pmc.keyboard, 9999),
        0
    );
}

#[test]
fn test_get_controller_key_assign_positive_index() {
    let pmc = make_pmc();
    let val = KeyConfiguration::controller_key_assign(&pmc.controller, 0, 0);
    assert_eq!(val, pmc.controller[0].keys[0]);
}

#[test]
fn test_get_controller_key_assign_start_select() {
    let pmc = make_pmc();
    assert_eq!(
        KeyConfiguration::controller_key_assign(&pmc.controller, 0, -1),
        pmc.controller[0].start
    );
    assert_eq!(
        KeyConfiguration::controller_key_assign(&pmc.controller, 0, -2),
        pmc.controller[0].select
    );
}

#[test]
fn test_get_controller_key_assign_no_device() {
    let pmc = make_pmc();
    assert_eq!(
        KeyConfiguration::controller_key_assign(&pmc.controller, 99, 0),
        0
    );
}

#[test]
fn test_get_midi_key_assign_positive_index() {
    // PlayModeConfig::new(BEAT_7K) creates MIDI with enable=false (is_midi=false),
    // so all keys are None. Create an enabled MIDI config directly.
    let midi = MidiConfig::new(Mode::BEAT_7K, true);
    let mi = KeyConfiguration::midi_key_assign(&midi, 0);
    // BEAT_7K MIDI enabled: keys[0] = Some(MidiInput { NOTE, 53 })
    assert_eq!(mi.input_type, MidiInputType::NOTE);
    assert_eq!(mi.value, 53);
}

#[test]
fn test_get_midi_key_assign_start() {
    let pmc = make_pmc();
    let mi = KeyConfiguration::midi_key_assign(&pmc.midi, -1);
    assert_eq!(mi.input_type, MidiInputType::NOTE);
    assert_eq!(mi.value, 47);
}

#[test]
fn test_get_midi_key_assign_select() {
    let pmc = make_pmc();
    let mi = KeyConfiguration::midi_key_assign(&pmc.midi, -2);
    assert_eq!(mi.input_type, MidiInputType::NOTE);
    assert_eq!(mi.value, 48);
}

#[test]
fn test_get_midi_key_assign_other_negative() {
    let mi = KeyConfiguration::midi_key_assign(&make_pmc().midi, -5);
    assert_eq!(mi.input_type, MidiInputType::NOTE);
    assert_eq!(mi.value, 0);
}

#[test]
fn test_get_mouse_scratch_key_string_no_assignment() {
    let pmc = make_pmc();
    let msc = &pmc.keyboard.mouse_scratch_config;
    // Default mouse scratch keys are all -1, so should return default
    let result = KeyConfiguration::mouse_scratch_key_string(msc, 0, Some("fallback"));
    assert_eq!(result, Some("fallback".to_string()));
}

#[test]
fn test_get_mouse_scratch_key_string_none_default() {
    let pmc = make_pmc();
    let msc = &pmc.keyboard.mouse_scratch_config;
    let result = KeyConfiguration::mouse_scratch_key_string(msc, 0, None);
    assert_eq!(result, None);
}

#[test]
fn test_get_mouse_scratch_key_string_with_assignment() {
    let mut pmc = make_pmc();
    // Assign mouse scratch key at index 0: 0 = "MOUSE RIGHT"
    pmc.keyboard.mouse_scratch_config.keys[0] = 0;
    let result = KeyConfiguration::mouse_scratch_key_string(
        &pmc.keyboard.mouse_scratch_config,
        0,
        Some("fallback"),
    );
    assert_eq!(result, Some("MOUSE RIGHT".to_string()));
}

// -- Mutation tests --

#[test]
fn test_reset_key_assign() {
    let mut pmc = make_pmc();
    // Set some values
    pmc.keyboard.keys[0] = 42;
    pmc.controller[0].keys[0] = 99;
    pmc.keyboard.mouse_scratch_config.keys[0] = 2;
    pmc.midi.keys[0] = Some(MidiInput::new(MidiInputType::NOTE, 60));

    KeyConfiguration::reset_key_assign(&mut pmc, 0);

    assert_eq!(pmc.keyboard.keys[0], -1);
    assert_eq!(pmc.controller[0].keys[0], -1);
    assert_eq!(pmc.keyboard.mouse_scratch_config.keys[0], -1);
    assert!(pmc.midi.keys[0].is_none());
}

#[test]
fn test_reset_key_assign_negative_is_noop() {
    let mut pmc = make_pmc();
    let start_before = pmc.keyboard.start;
    KeyConfiguration::reset_key_assign(&mut pmc, -1);
    // Start should not be affected
    assert_eq!(pmc.keyboard.start, start_before);
}

#[test]
fn test_delete_key_assign_positive() {
    let mut pmc = make_pmc();
    pmc.keyboard.keys[2] = 42;
    pmc.keyboard.mouse_scratch_config.keys[2] = 1;
    pmc.controller[0].keys[2] = 88;
    pmc.midi.keys[2] = Some(MidiInput::new(MidiInputType::NOTE, 60));

    KeyConfiguration::delete_key_assign(&mut pmc, 2);

    assert_eq!(pmc.keyboard.keys[2], -1);
    assert_eq!(pmc.keyboard.mouse_scratch_config.keys[2], -1);
    assert_eq!(pmc.controller[0].keys[2], -1);
    assert!(pmc.midi.keys[2].is_none());
}

#[test]
fn test_delete_key_assign_start() {
    let mut pmc = make_pmc();
    pmc.keyboard.mouse_scratch_config.start = 2;
    pmc.controller[0].start = 7;
    pmc.midi.start = Some(MidiInput::new(MidiInputType::NOTE, 47));

    KeyConfiguration::delete_key_assign(&mut pmc, -1);

    assert_eq!(pmc.keyboard.mouse_scratch_config.start, -1);
    assert_eq!(pmc.controller[0].start, -1);
    assert!(pmc.midi.start.is_none());
}

#[test]
fn test_delete_key_assign_select() {
    let mut pmc = make_pmc();
    pmc.keyboard.mouse_scratch_config.select = 3;
    pmc.controller[0].select = 8;
    pmc.midi.select = Some(MidiInput::new(MidiInputType::NOTE, 48));

    KeyConfiguration::delete_key_assign(&mut pmc, -2);

    assert_eq!(pmc.keyboard.mouse_scratch_config.select, -1);
    assert_eq!(pmc.controller[0].select, -1);
    assert!(pmc.midi.select.is_none());
}

// -- Setter tests --

#[test]
fn test_set_keyboard_key_assign_positive() {
    let mut pmc = make_pmc();
    KeyConfiguration::set_keyboard_key_assign(&mut pmc, 0, 77, false);
    assert_eq!(pmc.keyboard.keys[0], 77);
    // Other devices should be reset at index 0
    assert_eq!(pmc.controller[0].keys[0], -1);
}

#[test]
fn test_set_keyboard_key_assign_reserved() {
    let mut pmc = make_pmc();
    let original = pmc.keyboard.keys[0];
    KeyConfiguration::set_keyboard_key_assign(&mut pmc, 0, 77, true);
    // Should be unchanged because is_reserved is true
    assert_eq!(pmc.keyboard.keys[0], original);
}

#[test]
fn test_set_keyboard_key_assign_start() {
    let mut pmc = make_pmc();
    KeyConfiguration::set_keyboard_key_assign(&mut pmc, -1, 99, false);
    assert_eq!(pmc.keyboard.start, 99);
}

#[test]
fn test_set_keyboard_key_assign_select() {
    let mut pmc = make_pmc();
    KeyConfiguration::set_keyboard_key_assign(&mut pmc, -2, 88, false);
    assert_eq!(pmc.keyboard.select, 88);
}

#[test]
fn test_set_controller_key_assign() {
    let mut pmc = make_pmc();
    let name = pmc.controller[0].name.clone();
    KeyConfiguration::set_controller_key_assign(&mut pmc, 0, &name, 55);
    assert_eq!(pmc.controller[0].keys[0], 55);
}

#[test]
fn test_set_controller_key_assign_unknown_name() {
    let mut pmc = make_pmc();
    let original = pmc.controller[0].keys[0];
    KeyConfiguration::set_controller_key_assign(&mut pmc, 0, "nonexistent", 55);
    // Should be unchanged — name not found
    assert_eq!(pmc.controller[0].keys[0], original);
}

#[test]
fn test_set_midi_key_assign_positive() {
    let mut pmc = make_pmc();
    let mi = Some(MidiInput::new(MidiInputType::CONTROL_CHANGE, 64));
    KeyConfiguration::set_midi_key_assign(&mut pmc, 0, mi);
    let assigned = pmc.midi.keys[0].as_ref().unwrap();
    assert_eq!(assigned.input_type, MidiInputType::CONTROL_CHANGE);
    assert_eq!(assigned.value, 64);
}

#[test]
fn test_set_midi_key_assign_start() {
    let mut pmc = make_pmc();
    let mi = Some(MidiInput::new(MidiInputType::PITCH_BEND, 1));
    KeyConfiguration::set_midi_key_assign(&mut pmc, -1, mi);
    let assigned = pmc.midi.start.as_ref().unwrap();
    assert_eq!(assigned.input_type, MidiInputType::PITCH_BEND);
    assert_eq!(assigned.value, 1);
}

#[test]
fn test_set_mouse_scratch_key_assign_positive() {
    let mut pmc = make_pmc();
    KeyConfiguration::set_mouse_scratch_key_assign(&mut pmc, 0, 2);
    assert_eq!(pmc.keyboard.mouse_scratch_config.keys[0], 2);
}

#[test]
fn test_set_mouse_scratch_key_assign_start() {
    let mut pmc = make_pmc();
    KeyConfiguration::set_mouse_scratch_key_assign(&mut pmc, -1, 3);
    assert_eq!(pmc.keyboard.mouse_scratch_config.start, 3);
}

#[test]
fn test_set_mouse_scratch_key_assign_select() {
    let mut pmc = make_pmc();
    KeyConfiguration::set_mouse_scratch_key_assign(&mut pmc, -2, 1);
    assert_eq!(pmc.keyboard.mouse_scratch_config.select, 1);
}

// -- Validator tests --

#[test]
fn test_validate_keyboard_length_expands() {
    let kc = make_kc(1); // mode 1 = 7K, KEYSA[1] max = 8
    let mut kb = KeyboardConfig::new(Mode::BEAT_7K, true);
    kb.keys.clear(); // Empty
    kc.validate_keyboard_length(&mut kb);
    // KEYSA[1] = [0,1,2,3,4,5,6,7,8,-1,-2], max positive = 8, needed = 9
    assert!(kb.keys.len() >= 9);
}

#[test]
fn test_validate_keyboard_length_already_sufficient() {
    let kc = make_kc(1);
    let mut kb = KeyboardConfig::new(Mode::BEAT_7K, true);
    let original_len = kb.keys.len();
    kc.validate_keyboard_length(&mut kb);
    // Should not shrink
    assert_eq!(kb.keys.len(), original_len);
}

#[test]
fn test_validate_controller_length_adds_players() {
    let kc = make_kc(4); // mode 4 = 14K, has keys > 100 (2P keys)
    let mut pmc = PlayModeConfig::new(Mode::BEAT_14K);
    pmc.controller.clear();
    kc.validate_controller_length(&mut pmc);
    // Should have at least 1 controller (single-player keys are < 100)
    assert!(!pmc.controller.is_empty());
}

#[test]
fn test_validate_controller_length_expands_keys() {
    let kc = make_kc(1); // mode 1 = 7K
    let mut pmc = PlayModeConfig::new(Mode::BEAT_7K);
    for cc in pmc.controller.iter_mut() {
        cc.keys.clear();
    }
    kc.validate_controller_length(&mut pmc);
    // KEYSA[1] max key%100 = 8, so each controller needs at least 9 keys
    for cc in &pmc.controller {
        assert!(cc.keys.len() >= 9);
    }
}

#[test]
fn test_validate_midi_length_expands() {
    let kc = make_kc(1);
    let mut midi = MidiConfig::new(Mode::BEAT_7K, true);
    midi.keys.clear();
    kc.validate_midi_length(&mut midi);
    assert!(midi.keys.len() >= 9);
}

#[test]
fn test_validate_midi_length_already_sufficient() {
    let kc = make_kc(1);
    let mut midi = MidiConfig::new(Mode::BEAT_7K, true);
    let original_len = midi.keys.len();
    kc.validate_midi_length(&mut midi);
    assert_eq!(midi.keys.len(), original_len);
}

// -- gdx_key_name tests --

#[test]
fn test_gdx_key_name_letters() {
    assert_eq!(gdx_key_name(29), "A");
    assert_eq!(gdx_key_name(54), "Z");
    assert_eq!(gdx_key_name(47), "S");
}

#[test]
fn test_gdx_key_name_numbers() {
    assert_eq!(gdx_key_name(7), "0");
    assert_eq!(gdx_key_name(16), "9");
}

#[test]
fn test_gdx_key_name_special() {
    assert_eq!(gdx_key_name(66), "Enter");
    assert_eq!(gdx_key_name(111), "Escape");
    assert_eq!(gdx_key_name(244), "F1");
    assert_eq!(gdx_key_name(255), "F12");
    assert_eq!(gdx_key_name(59), "L-Shift");
}

#[test]
fn test_gdx_key_name_unknown() {
    assert_eq!(gdx_key_name(999), "Unknown");
}

// -- key_assign tests --

#[test]
fn test_get_key_assign_returns_key_name() {
    let kc = make_kc(1); // 7K mode
    // KEYSA[1][0] = 0, so it reads keyboard_keys[0]
    // Key code 54 = Z
    let keys = vec![54, 47, 52, 32, 31, 34, 50, 59, 129];
    assert_eq!(kc.key_assign(0, &keys), "Z");
    assert_eq!(kc.key_assign(1, &keys), "S");
    assert_eq!(kc.key_assign(2, &keys), "X");
}

#[test]
fn test_get_key_assign_out_of_bounds() {
    let kc = make_kc(1);
    let keys = vec![54];
    assert_eq!(kc.key_assign(999, &keys), "!!!");
}

#[test]
fn test_get_key_assign_start_select_shows_dashes() {
    let kc = make_kc(1); // 7K: KEYSA[1] last two are -1, -2 (START, SELECT)
    let keys = vec![54; 9];
    // Index 9 maps to KEYSA[1][9] = -1 (START)
    assert_eq!(kc.key_assign(9, &keys), "---");
    // Index 10 maps to KEYSA[1][10] = -2 (SELECT)
    assert_eq!(kc.key_assign(10, &keys), "---");
}

#[test]
fn test_get_key_assign_unassigned_key() {
    let kc = make_kc(1);
    let keys = vec![-1; 9]; // All unassigned
    assert_eq!(kc.key_assign(0, &keys), "---");
}
