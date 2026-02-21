// Decide-specific skin state synchronization.
//
// Updates SharedGameState with song metadata for the Decide screen.

use bms_model::{BmsModel, NoteType};
use bms_skin::property_id::{
    NUMBER_JUDGERANK, NUMBER_MAXBPM, NUMBER_MINBPM, NUMBER_PLAYLEVEL, NUMBER_TOTALNOTES2,
    OPTION_5KEYSONG, OPTION_7KEYSONG, OPTION_9KEYSONG, OPTION_10KEYSONG, OPTION_14KEYSONG,
    OPTION_24KEYDPSONG, OPTION_24KEYSONG, OPTION_BGA, OPTION_BPMCHANGE, OPTION_LN, OPTION_NO_BGA,
    OPTION_NO_BPMCHANGE, OPTION_NO_LN, STRING_ARTIST, STRING_FULLTITLE, STRING_GENRE,
    STRING_SUBARTIST, STRING_SUBTITLE, STRING_TITLE,
};

use crate::game_state::SharedGameState;

/// Synchronize decide-specific state into SharedGameState for skin rendering.
pub fn sync_decide_state(state: &mut SharedGameState, model: &BmsModel) {
    // Song metadata strings
    state.strings.insert(STRING_TITLE, model.title.clone());
    state
        .strings
        .insert(STRING_SUBTITLE, model.subtitle.clone());
    state.strings.insert(
        STRING_FULLTITLE,
        format!("{} {}", model.title, model.subtitle),
    );
    state.strings.insert(STRING_ARTIST, model.artist.clone());
    state
        .strings
        .insert(STRING_SUBARTIST, model.sub_artist.clone());
    state.strings.insert(STRING_GENRE, model.genre.clone());

    // BPM
    let min_bpm = model.min_bpm() as i32;
    let max_bpm = model.max_bpm() as i32;
    state.integers.insert(NUMBER_MINBPM, min_bpm);
    state.integers.insert(NUMBER_MAXBPM, max_bpm);

    // BPM change flags
    let has_bpm_change = min_bpm != max_bpm;
    state.booleans.insert(OPTION_NO_BPMCHANGE, !has_bpm_change);
    state.booleans.insert(OPTION_BPMCHANGE, has_bpm_change);

    // Total notes
    let judge_notes = model.build_judge_notes();
    let total_notes = judge_notes.iter().filter(|n| n.is_playable()).count() as i32;
    state.integers.insert(NUMBER_TOTALNOTES2, total_notes);

    // Play level and judge rank
    state.integers.insert(NUMBER_PLAYLEVEL, model.play_level);
    state.integers.insert(NUMBER_JUDGERANK, model.judge_rank);

    // Mode flags
    let mode_id = model.mode.mode_id();
    sync_mode_flags(state, mode_id);

    // LN flags: check if any notes are long notes
    let has_ln = model.notes.iter().any(|n| {
        matches!(
            n.note_type,
            NoteType::LongNote | NoteType::ChargeNote | NoteType::HellChargeNote
        )
    });
    state.booleans.insert(OPTION_LN, has_ln);
    state.booleans.insert(OPTION_NO_LN, !has_ln);

    // BGA flags: check if any BGA events exist
    let has_bga = !model.bga_events.is_empty();
    state.booleans.insert(OPTION_BGA, has_bga);
    state.booleans.insert(OPTION_NO_BGA, !has_bga);
}

/// Set mode-specific booleans from play mode ID.
fn sync_mode_flags(state: &mut SharedGameState, mode_id: i32) {
    state.booleans.insert(OPTION_7KEYSONG, false);
    state.booleans.insert(OPTION_5KEYSONG, false);
    state.booleans.insert(OPTION_14KEYSONG, false);
    state.booleans.insert(OPTION_10KEYSONG, false);
    state.booleans.insert(OPTION_9KEYSONG, false);
    state.booleans.insert(OPTION_24KEYSONG, false);
    state.booleans.insert(OPTION_24KEYDPSONG, false);

    match mode_id {
        7 => {
            state.booleans.insert(OPTION_7KEYSONG, true);
        }
        5 => {
            state.booleans.insert(OPTION_5KEYSONG, true);
        }
        14 => {
            state.booleans.insert(OPTION_14KEYSONG, true);
        }
        10 => {
            state.booleans.insert(OPTION_10KEYSONG, true);
        }
        9 => {
            state.booleans.insert(OPTION_9KEYSONG, true);
        }
        24 => {
            state.booleans.insert(OPTION_24KEYSONG, true);
        }
        48 => {
            state.booleans.insert(OPTION_24KEYDPSONG, true);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_decide_populates_title() {
        let mut state = SharedGameState::default();
        let mut model = BmsModel::default();
        model.title = "Test Song".to_string();
        model.artist = "Test Artist".to_string();

        sync_decide_state(&mut state, &model);

        assert_eq!(state.strings.get(&STRING_TITLE).unwrap(), "Test Song");
        assert_eq!(state.strings.get(&STRING_ARTIST).unwrap(), "Test Artist");
    }

    #[test]
    fn sync_decide_populates_fulltitle() {
        let mut state = SharedGameState::default();
        let mut model = BmsModel::default();
        model.title = "Title".to_string();
        model.subtitle = "Sub".to_string();

        sync_decide_state(&mut state, &model);

        assert_eq!(state.strings.get(&STRING_FULLTITLE).unwrap(), "Title Sub");
    }

    #[test]
    fn sync_decide_populates_bpm() {
        let mut state = SharedGameState::default();
        let mut model = BmsModel::default();
        model.initial_bpm = 150.0;

        sync_decide_state(&mut state, &model);

        assert_eq!(*state.integers.get(&NUMBER_MINBPM).unwrap(), 150);
        assert_eq!(*state.integers.get(&NUMBER_MAXBPM).unwrap(), 150);
    }

    #[test]
    fn sync_decide_populates_playlevel_and_judgerank() {
        let mut state = SharedGameState::default();
        let mut model = BmsModel::default();
        model.play_level = 12;
        model.judge_rank = 2;

        sync_decide_state(&mut state, &model);

        assert_eq!(*state.integers.get(&NUMBER_PLAYLEVEL).unwrap(), 12);
        assert_eq!(*state.integers.get(&NUMBER_JUDGERANK).unwrap(), 2);
    }

    #[test]
    fn sync_decide_mode_flags_7k() {
        let mut state = SharedGameState::default();
        let mut model = BmsModel::default();
        model.mode = bms_model::PlayMode::Beat7K;

        sync_decide_state(&mut state, &model);

        assert!(*state.booleans.get(&OPTION_7KEYSONG).unwrap());
        assert!(!*state.booleans.get(&OPTION_5KEYSONG).unwrap());
    }

    #[test]
    fn sync_decide_ln_flags_no_ln() {
        let mut state = SharedGameState::default();
        let model = BmsModel::default();

        sync_decide_state(&mut state, &model);

        assert!(!*state.booleans.get(&OPTION_LN).unwrap());
        assert!(*state.booleans.get(&OPTION_NO_LN).unwrap());
    }

    #[test]
    fn sync_decide_bpm_change_flags() {
        let mut state = SharedGameState::default();
        let mut model = BmsModel::default();
        model.initial_bpm = 150.0;
        model.bpm_changes.push(bms_model::BpmChange {
            time_us: 1_000_000,
            bpm: 200.0,
        });

        sync_decide_state(&mut state, &model);

        assert!(*state.booleans.get(&OPTION_BPMCHANGE).unwrap());
        assert!(!*state.booleans.get(&OPTION_NO_BPMCHANGE).unwrap());
    }
}
