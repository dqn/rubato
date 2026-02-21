// Select-specific skin state synchronization.
//
// Updates SharedGameState with song metadata, bar type, and mode flags
// from the current selection in MusicSelect state.

use std::collections::HashMap;

use bms_database::CourseDataConstraint;
use bms_database::SongInformation;
use bms_render::draw::bar::{BarScrollState, BarSlotData, BarType};
use bms_rule::ClearType;
use bms_rule::ScoreData;
use bms_skin::property_id::{
    FLOAT_BEST_RATE, FLOAT_CHART_AVERAGEDENSITY, FLOAT_CHART_ENDDENSITY, FLOAT_CHART_PEAKDENSITY,
    FLOAT_CHART_TOTALGAUGE, FLOAT_SCORE_RATE2, NUMBER_BAD_RATE, NUMBER_BAD2, NUMBER_BEST_RATE,
    NUMBER_BEST_RATE_AFTERDOT, NUMBER_CLEARCOUNT, NUMBER_DENSITY_AVERAGE,
    NUMBER_DENSITY_AVERAGE_AFTERDOT, NUMBER_DENSITY_END, NUMBER_DENSITY_END_AFTERDOT,
    NUMBER_DENSITY_PEAK, NUMBER_DENSITY_PEAK_AFTERDOT, NUMBER_FAILCOUNT, NUMBER_FOLDER_TOTALSONGS,
    NUMBER_GOOD_RATE, NUMBER_GOOD2, NUMBER_GREAT_RATE, NUMBER_GREAT2, NUMBER_HIGHSCORE2,
    NUMBER_MAINBPM, NUMBER_MAXBPM, NUMBER_MAXCOMBO2, NUMBER_MINBPM, NUMBER_MISSCOUNT2,
    NUMBER_PERFECT_RATE, NUMBER_PERFECT2, NUMBER_PLAYCOUNT, NUMBER_PLAYLEVEL, NUMBER_POOR_RATE,
    NUMBER_POOR2, NUMBER_RIVAL_CLEARCOUNT, NUMBER_RIVAL_FAILCOUNT, NUMBER_RIVAL_MAXCOMBO,
    NUMBER_RIVAL_MISSCOUNT, NUMBER_RIVAL_PLAYCOUNT, NUMBER_RIVAL_SCORE, NUMBER_SCORE_RATE,
    NUMBER_SCORE_RATE_AFTERDOT, NUMBER_SCORE2, NUMBER_SONGGAUGE_TOTAL, NUMBER_TOTALNOTE_BSS,
    NUMBER_TOTALNOTE_LN, NUMBER_TOTALNOTE_NORMAL, NUMBER_TOTALNOTE_SCRATCH, NUMBER_TOTALNOTES2,
    OPTION_5KEYSONG, OPTION_7KEYSONG, OPTION_9KEYSONG, OPTION_10KEYSONG, OPTION_14KEYSONG,
    OPTION_24KEYDPSONG, OPTION_24KEYSONG, OPTION_BGA, OPTION_BPMCHANGE, OPTION_COMPARE_RIVAL,
    OPTION_DIFFICULTY0, OPTION_DIFFICULTY1, OPTION_DIFFICULTY2, OPTION_DIFFICULTY3,
    OPTION_DIFFICULTY4, OPTION_DIFFICULTY5, OPTION_FOLDERBAR, OPTION_GRADEBAR,
    OPTION_GRADEBAR_CLASS, OPTION_GRADEBAR_CN, OPTION_GRADEBAR_GAUGE_5KEYS,
    OPTION_GRADEBAR_GAUGE_7KEYS, OPTION_GRADEBAR_GAUGE_9KEYS, OPTION_GRADEBAR_GAUGE_24KEYS,
    OPTION_GRADEBAR_GAUGE_LR2, OPTION_GRADEBAR_HCN, OPTION_GRADEBAR_LN, OPTION_GRADEBAR_MIRROR,
    OPTION_GRADEBAR_NOGOOD, OPTION_GRADEBAR_NOGREAT, OPTION_GRADEBAR_NOSPEED,
    OPTION_GRADEBAR_RANDOM, OPTION_LN, OPTION_NO_BGA, OPTION_NO_BPMCHANGE, OPTION_NO_LN,
    OPTION_NOT_COMPARE_RIVAL, OPTION_PANEL1, OPTION_PANEL2, OPTION_PANEL3, OPTION_PLAYABLEBAR,
    OPTION_RANDOMCOURSEBAR, OPTION_RANDOMSELECTBAR, OPTION_SELECT_BAR_ASSIST_EASY_CLEARED,
    OPTION_SELECT_BAR_EASY_CLEARED, OPTION_SELECT_BAR_EXHARD_CLEARED, OPTION_SELECT_BAR_FAILED,
    OPTION_SELECT_BAR_FULL_COMBO_CLEARED, OPTION_SELECT_BAR_HARD_CLEARED,
    OPTION_SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED, OPTION_SELECT_BAR_MAX_CLEARED,
    OPTION_SELECT_BAR_NORMAL_CLEARED, OPTION_SELECT_BAR_NOT_PLAYED,
    OPTION_SELECT_BAR_PERFECT_CLEARED, OPTION_SELECT_REPLAYDATA, OPTION_SELECT_REPLAYDATA2,
    OPTION_SELECT_REPLAYDATA3, OPTION_SELECT_REPLAYDATA4, OPTION_SONGBAR,
    RATE_MUSICSELECT_POSITION, STRING_ARTIST, STRING_FULLTITLE, STRING_GENRE, STRING_RIVAL,
    STRING_SUBARTIST, STRING_SUBTITLE, STRING_TITLE,
};

use crate::game_state::SharedGameState;
use crate::state::select::bar_manager::{Bar, BarManager};

/// Synchronize select-specific state into SharedGameState for skin rendering.
///
/// Called once per frame during the MusicSelect state.
/// Rival data for skin state synchronization.
pub struct RivalSkinData<'a> {
    /// Name of the currently selected rival (empty if no rival selected).
    pub name: &'a str,
    /// Rival's score for the currently selected song (None if no score).
    pub score: Option<&'a ScoreData>,
}

#[allow(clippy::too_many_arguments)]
pub fn sync_select_state(
    state: &mut SharedGameState,
    bar_manager: &BarManager,
    has_ln: bool,
    bga_on: bool,
    _is_preview_playing: bool,
    selected_replay: i32,
    rival: Option<&RivalSkinData<'_>>,
    selected_score: Option<&ScoreData>,
) {
    // Bar type booleans (clear previous)
    state.booleans.insert(OPTION_SONGBAR, false);
    state.booleans.insert(OPTION_FOLDERBAR, false);
    state.booleans.insert(OPTION_PLAYABLEBAR, false);

    // Grade bar flag (clear)
    state.booleans.insert(OPTION_GRADEBAR, false);
    state.booleans.insert(OPTION_RANDOMSELECTBAR, false);
    state.booleans.insert(OPTION_RANDOMCOURSEBAR, false);

    match bar_manager.current() {
        Some(Bar::Song(song_data)) => {
            state.booleans.insert(OPTION_SONGBAR, true);
            state.booleans.insert(OPTION_PLAYABLEBAR, true);

            // Song metadata strings
            state.strings.insert(STRING_TITLE, song_data.title.clone());
            state
                .strings
                .insert(STRING_SUBTITLE, song_data.subtitle.clone());
            state.strings.insert(
                STRING_FULLTITLE,
                format!("{} {}", song_data.title, song_data.subtitle),
            );
            state
                .strings
                .insert(STRING_ARTIST, song_data.artist.clone());
            state
                .strings
                .insert(STRING_SUBARTIST, song_data.subartist.clone());
            state.strings.insert(STRING_GENRE, song_data.genre.clone());

            // BPM
            state.integers.insert(NUMBER_MINBPM, song_data.minbpm);
            state.integers.insert(NUMBER_MAXBPM, song_data.maxbpm);

            // Total notes
            state.integers.insert(NUMBER_TOTALNOTES2, song_data.notes);

            // Play level (Java: NUMBER_PLAYLEVEL)
            state.integers.insert(NUMBER_PLAYLEVEL, song_data.level);

            // Mode flags
            let mode_id = song_data.mode;
            sync_mode_flags(state, mode_id);

            // Difficulty flags (H8)
            sync_difficulty_flags(state, song_data.difficulty);

            // BPM change flags (H8)
            sync_bpm_flags(state, song_data.minbpm, song_data.maxbpm);
        }
        Some(Bar::Folder { .. })
        | Some(Bar::TableRoot { .. })
        | Some(Bar::HashFolder { .. })
        | Some(Bar::Container { .. })
        | Some(Bar::SameFolder { .. }) => {
            state.booleans.insert(OPTION_FOLDERBAR, true);
            clear_song_metadata(state);
        }
        Some(Bar::Grade(grade_data)) => {
            state.booleans.insert(OPTION_GRADEBAR, true);
            state.booleans.insert(OPTION_PLAYABLEBAR, true);
            sync_grade_bar_constraints(state, &grade_data.constraints);
            clear_song_metadata(state);
        }
        Some(Bar::Course(course_data)) => {
            state.booleans.insert(OPTION_GRADEBAR, true);
            state.booleans.insert(OPTION_PLAYABLEBAR, true);
            sync_grade_bar_constraints(state, &course_data.constraint);
            clear_song_metadata(state);
        }
        Some(Bar::RandomCourse(_)) => {
            state.booleans.insert(OPTION_RANDOMCOURSEBAR, true);
            state.booleans.insert(OPTION_PLAYABLEBAR, true);
            clear_song_metadata(state);
        }
        Some(Bar::Executable { .. }) => {
            state.booleans.insert(OPTION_RANDOMSELECTBAR, true);
            state.booleans.insert(OPTION_PLAYABLEBAR, true);
            clear_song_metadata(state);
        }
        Some(Bar::Function { .. })
        | Some(Bar::Command { .. })
        | Some(Bar::SearchWord { .. })
        | Some(Bar::ContextMenu(_)) => {
            clear_song_metadata(state);
        }
        None => {
            clear_song_metadata(state);
        }
    }

    // Select position (fraction of cursor within bar list)
    let total = bar_manager.bar_count();
    let cursor = bar_manager.cursor_pos();
    state
        .integers
        .insert(NUMBER_FOLDER_TOTALSONGS, total as i32);
    if total > 0 {
        state
            .floats
            .insert(RATE_MUSICSELECT_POSITION, cursor as f32 / total as f32);
    }

    // LN / BGA feature flags
    state.booleans.insert(OPTION_LN, has_ln);
    state.booleans.insert(OPTION_NO_LN, !has_ln);
    state.booleans.insert(OPTION_BGA, bga_on);
    state.booleans.insert(OPTION_NO_BGA, !bga_on);

    // Replay slot selection (OPTION_SELECT_REPLAYDATA 1205-1208)
    state
        .booleans
        .insert(OPTION_SELECT_REPLAYDATA, selected_replay == 0);
    state
        .booleans
        .insert(OPTION_SELECT_REPLAYDATA2, selected_replay == 1);
    state
        .booleans
        .insert(OPTION_SELECT_REPLAYDATA3, selected_replay == 2);
    state
        .booleans
        .insert(OPTION_SELECT_REPLAYDATA4, selected_replay == 3);

    // Rival score data (Java parity: MusicSelector.selectedRivalScoreData)
    let has_rival = rival.is_some_and(|r| !r.name.is_empty());
    state.booleans.insert(OPTION_COMPARE_RIVAL, has_rival);
    state.booleans.insert(OPTION_NOT_COMPARE_RIVAL, !has_rival);

    if let Some(rival) = rival {
        state.strings.insert(STRING_RIVAL, rival.name.to_string());
        if let Some(sd) = rival.score {
            state.integers.insert(NUMBER_RIVAL_SCORE, sd.exscore());
            state.integers.insert(NUMBER_RIVAL_MAXCOMBO, sd.maxcombo);
            state.integers.insert(NUMBER_RIVAL_MISSCOUNT, sd.minbp);
            state.integers.insert(NUMBER_RIVAL_PLAYCOUNT, sd.playcount);
            state
                .integers
                .insert(NUMBER_RIVAL_CLEARCOUNT, sd.clearcount);
            state
                .integers
                .insert(NUMBER_RIVAL_FAILCOUNT, sd.playcount - sd.clearcount);
        } else {
            clear_rival_scores(state);
        }
    } else {
        state.strings.insert(STRING_RIVAL, String::new());
        clear_rival_scores(state);
    }

    // Score data properties for the selected song bar (Java: IntegerPropertyFactory)
    sync_selected_score(state, selected_score);
}

fn clear_rival_scores(state: &mut SharedGameState) {
    state.integers.remove(&NUMBER_RIVAL_SCORE);
    state.integers.remove(&NUMBER_RIVAL_MAXCOMBO);
    state.integers.remove(&NUMBER_RIVAL_MISSCOUNT);
    state.integers.remove(&NUMBER_RIVAL_PLAYCOUNT);
    state.integers.remove(&NUMBER_RIVAL_CLEARCOUNT);
    state.integers.remove(&NUMBER_RIVAL_FAILCOUNT);
}

/// Synchronize bar scroll state for skin bar rendering.
///
/// Builds a BarScrollState from the BarManager and stores it in SharedGameState
/// for the skin renderer to pick up.
pub fn sync_bar_scroll_state(
    state: &mut SharedGameState,
    bar_manager: &BarManager,
    center_bar: usize,
    angle_lerp: f32,
    angle: i32,
    score_lamp_cache: &HashMap<String, i32>,
) {
    let total = bar_manager.bar_count();
    if total == 0 {
        state.bar_scroll_state = None;
        return;
    }

    let mut slots = Vec::with_capacity(total);
    for bar in bar_manager.bars() {
        let slot = match bar {
            Bar::Song(song_data) => {
                use bms_database::song_data::{
                    FEATURE_CHARGENOTE, FEATURE_HELLCHARGENOTE, FEATURE_MINENOTE, FEATURE_RANDOM,
                };

                // Map song feature flags to bar feature flags
                let mut features = 0u32;
                if song_data.has_any_long_note() {
                    features |= bms_render::draw::bar::FEATURE_LN;
                }
                if song_data.feature & FEATURE_MINENOTE != 0 {
                    features |= bms_render::draw::bar::FEATURE_MINE;
                }
                if song_data.feature & FEATURE_RANDOM != 0 {
                    features |= bms_render::draw::bar::FEATURE_RANDOM;
                }
                if song_data.feature & FEATURE_CHARGENOTE != 0 {
                    features |= bms_render::draw::bar::FEATURE_CHARGENOTE;
                }
                if song_data.feature & FEATURE_HELLCHARGENOTE != 0 {
                    features |= bms_render::draw::bar::FEATURE_HELL_CHARGENOTE;
                }

                BarSlotData {
                    bar_type: BarType::Song {
                        exists: !song_data.path.is_empty(),
                    },
                    lamp_id: score_lamp_cache
                        .get(&song_data.sha256)
                        .copied()
                        .unwrap_or(0),
                    trophy_id: None,
                    level: song_data.level,
                    difficulty: song_data.difficulty,
                    title: song_data.title.clone(),
                    subtitle: None,
                    text_type: 0, // Song type
                    features,
                }
            }
            Bar::Folder { name, .. } => BarSlotData {
                bar_type: BarType::Folder,
                title: name.clone(),
                text_type: 1, // Folder type
                ..Default::default()
            },
            Bar::Course(course_data) => BarSlotData {
                bar_type: BarType::Grade { all_songs: true },
                title: course_data.name.clone(),
                text_type: 2, // Grade type
                ..Default::default()
            },
            Bar::TableRoot { name, .. } => BarSlotData {
                bar_type: BarType::Table,
                title: name.clone(),
                text_type: 1, // Folder type
                ..Default::default()
            },
            Bar::HashFolder { name, .. } => BarSlotData {
                bar_type: BarType::Folder,
                title: name.clone(),
                text_type: 1, // Folder type
                ..Default::default()
            },
            Bar::Executable { name, .. } => BarSlotData {
                bar_type: BarType::Song { exists: true },
                title: name.clone(),
                text_type: 0, // Song type
                ..Default::default()
            },
            Bar::Function {
                title,
                subtitle,
                display_bar_type,
                lamp,
                ..
            } => BarSlotData {
                bar_type: BarType::Function {
                    display_bar_type: *display_bar_type,
                    display_text_type: 5,
                },
                lamp_id: *lamp,
                title: title.clone(),
                subtitle: subtitle.clone(),
                text_type: 5, // Function type
                ..Default::default()
            },
            Bar::Grade(grade_data) => BarSlotData {
                bar_type: BarType::Grade { all_songs: true },
                title: grade_data.name.clone(),
                text_type: 2, // Grade type
                ..Default::default()
            },
            Bar::RandomCourse(rc) => BarSlotData {
                bar_type: BarType::Grade { all_songs: true },
                title: rc.name.clone(),
                text_type: 2, // Grade type
                ..Default::default()
            },
            Bar::Command { name, .. } => BarSlotData {
                bar_type: BarType::Command,
                title: name.clone(),
                text_type: 3, // Command type
                ..Default::default()
            },
            Bar::Container { name, .. } => BarSlotData {
                bar_type: BarType::Folder,
                title: name.clone(),
                text_type: 1, // Folder type
                ..Default::default()
            },
            Bar::SameFolder { name, .. } => BarSlotData {
                bar_type: BarType::Folder,
                title: name.clone(),
                text_type: 1, // Folder type
                ..Default::default()
            },
            Bar::SearchWord { query } => BarSlotData {
                bar_type: BarType::Search,
                title: query.clone(),
                text_type: 4, // Search type
                ..Default::default()
            },
            Bar::ContextMenu(cm) => BarSlotData {
                bar_type: BarType::Command,
                title: cm.source_bar.bar_name().to_string(),
                text_type: 3, // Command type
                ..Default::default()
            },
        };
        slots.push(slot);
    }

    state.bar_scroll_state = Some(BarScrollState {
        center_bar,
        selected_index: bar_manager.cursor_pos(),
        total_bars: total,
        angle_lerp,
        angle,
        slots,
    });
}

/// Set mode-specific booleans from song mode ID.
fn sync_mode_flags(state: &mut SharedGameState, mode_id: i32) {
    state.booleans.insert(OPTION_7KEYSONG, false);
    state.booleans.insert(OPTION_5KEYSONG, false);
    state.booleans.insert(OPTION_14KEYSONG, false);
    state.booleans.insert(OPTION_10KEYSONG, false);
    state.booleans.insert(OPTION_9KEYSONG, false);
    state.booleans.insert(OPTION_24KEYSONG, false);
    state.booleans.insert(OPTION_24KEYDPSONG, false);

    // mode_id matches PlayMode::mode_id() values
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

/// Synchronize song information properties into SharedGameState.
///
/// When `info` is Some, populates note counts, density metrics, main BPM, and TOTAL.
/// When `info` is None, removes all song information properties.
pub fn sync_song_information(state: &mut SharedGameState, info: Option<&SongInformation>) {
    const INTEGER_IDS: &[i32] = &[
        NUMBER_TOTALNOTE_NORMAL,
        NUMBER_TOTALNOTE_LN,
        NUMBER_TOTALNOTE_SCRATCH,
        NUMBER_TOTALNOTE_BSS,
        NUMBER_DENSITY_PEAK,
        NUMBER_DENSITY_PEAK_AFTERDOT,
        NUMBER_DENSITY_END,
        NUMBER_DENSITY_END_AFTERDOT,
        NUMBER_DENSITY_AVERAGE,
        NUMBER_DENSITY_AVERAGE_AFTERDOT,
        NUMBER_SONGGAUGE_TOTAL,
        NUMBER_MAINBPM,
    ];
    const FLOAT_IDS: &[i32] = &[
        FLOAT_CHART_AVERAGEDENSITY,
        FLOAT_CHART_ENDDENSITY,
        FLOAT_CHART_PEAKDENSITY,
        FLOAT_CHART_TOTALGAUGE,
    ];

    match info {
        Some(info) => {
            state.integers.insert(NUMBER_TOTALNOTE_NORMAL, info.n);
            state.integers.insert(NUMBER_TOTALNOTE_LN, info.ln);
            state.integers.insert(NUMBER_TOTALNOTE_SCRATCH, info.s);
            state.integers.insert(NUMBER_TOTALNOTE_BSS, info.ls);
            state
                .integers
                .insert(NUMBER_DENSITY_PEAK, info.peakdensity as i32);
            state.integers.insert(
                NUMBER_DENSITY_PEAK_AFTERDOT,
                ((info.peakdensity * 100.0) as i32) % 100,
            );
            state
                .integers
                .insert(NUMBER_DENSITY_END, info.enddensity as i32);
            state.integers.insert(
                NUMBER_DENSITY_END_AFTERDOT,
                ((info.enddensity * 100.0) as i32) % 100,
            );
            state
                .integers
                .insert(NUMBER_DENSITY_AVERAGE, info.density as i32);
            state.integers.insert(
                NUMBER_DENSITY_AVERAGE_AFTERDOT,
                ((info.density * 100.0) as i32) % 100,
            );
            state
                .integers
                .insert(NUMBER_SONGGAUGE_TOTAL, info.total as i32);
            state.integers.insert(NUMBER_MAINBPM, info.mainbpm as i32);

            state
                .floats
                .insert(FLOAT_CHART_AVERAGEDENSITY, info.density as f32);
            state
                .floats
                .insert(FLOAT_CHART_ENDDENSITY, info.enddensity as f32);
            state
                .floats
                .insert(FLOAT_CHART_PEAKDENSITY, info.peakdensity as f32);
            state
                .floats
                .insert(FLOAT_CHART_TOTALGAUGE, info.total as f32);

            // Graph data: BPM events (speedchange → (time_us, bpm))
            state.bpm_events.clear();
            for pair in info.speedchange_values() {
                // pair = [speed, time_ms]
                let time_us = (pair[1] * 1000.0) as i64;
                let bpm = pair[0];
                state.bpm_events.push((time_us, bpm));
            }

            // Graph data: note distribution (sum 7 lane columns per bucket)
            state.note_distribution.clear();
            for bucket in info.distribution_values() {
                let total: i32 = bucket.iter().sum();
                state.note_distribution.push(total as u32);
            }
        }
        None => {
            for &id in INTEGER_IDS {
                state.integers.remove(&id);
            }
            for &id in FLOAT_IDS {
                state.floats.remove(&id);
            }
            state.bpm_events.clear();
            state.note_distribution.clear();
        }
    }
}

/// Synchronize difficulty flags for the selected song.
pub fn sync_difficulty_flags(state: &mut SharedGameState, difficulty: i32) {
    let ids = [
        OPTION_DIFFICULTY0,
        OPTION_DIFFICULTY1,
        OPTION_DIFFICULTY2,
        OPTION_DIFFICULTY3,
        OPTION_DIFFICULTY4,
        OPTION_DIFFICULTY5,
    ];
    for &id in &ids {
        state.booleans.insert(id, false);
    }
    let idx = if (1..=5).contains(&difficulty) {
        difficulty as usize
    } else {
        0 // undefined/out-of-range maps to DIFFICULTY0
    };
    state.booleans.insert(ids[idx], true);
}

/// Synchronize BPM change flags for the selected song.
pub fn sync_bpm_flags(state: &mut SharedGameState, minbpm: i32, maxbpm: i32) {
    let has_change = minbpm != maxbpm;
    state.booleans.insert(OPTION_NO_BPMCHANGE, !has_change);
    state.booleans.insert(OPTION_BPMCHANGE, has_change);
}

/// Synchronize select bar clear status flags.
///
/// Java: BooleanPropertyFactory IDs 100-105, 1100-1104.
pub fn sync_bar_clear_status(state: &mut SharedGameState, clear: Option<ClearType>) {
    let all_ids = [
        OPTION_SELECT_BAR_NOT_PLAYED,
        OPTION_SELECT_BAR_FAILED,
        OPTION_SELECT_BAR_ASSIST_EASY_CLEARED,
        OPTION_SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED,
        OPTION_SELECT_BAR_EASY_CLEARED,
        OPTION_SELECT_BAR_NORMAL_CLEARED,
        OPTION_SELECT_BAR_HARD_CLEARED,
        OPTION_SELECT_BAR_EXHARD_CLEARED,
        OPTION_SELECT_BAR_FULL_COMBO_CLEARED,
        OPTION_SELECT_BAR_PERFECT_CLEARED,
        OPTION_SELECT_BAR_MAX_CLEARED,
    ];
    for &id in &all_ids {
        state.booleans.insert(id, false);
    }
    match clear {
        None => state.booleans.insert(OPTION_SELECT_BAR_NOT_PLAYED, true),
        Some(ClearType::NoPlay) => state.booleans.insert(OPTION_SELECT_BAR_NOT_PLAYED, true),
        Some(ClearType::Failed) => state.booleans.insert(OPTION_SELECT_BAR_FAILED, true),
        Some(ClearType::AssistEasy) => state
            .booleans
            .insert(OPTION_SELECT_BAR_ASSIST_EASY_CLEARED, true),
        Some(ClearType::LightAssistEasy) => state
            .booleans
            .insert(OPTION_SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED, true),
        Some(ClearType::Easy) => state.booleans.insert(OPTION_SELECT_BAR_EASY_CLEARED, true),
        Some(ClearType::Normal) => state
            .booleans
            .insert(OPTION_SELECT_BAR_NORMAL_CLEARED, true),
        Some(ClearType::Hard) => state.booleans.insert(OPTION_SELECT_BAR_HARD_CLEARED, true),
        Some(ClearType::ExHard) => state
            .booleans
            .insert(OPTION_SELECT_BAR_EXHARD_CLEARED, true),
        Some(ClearType::FullCombo) => state
            .booleans
            .insert(OPTION_SELECT_BAR_FULL_COMBO_CLEARED, true),
        Some(ClearType::Perfect) => state
            .booleans
            .insert(OPTION_SELECT_BAR_PERFECT_CLEARED, true),
        Some(ClearType::Max) => state.booleans.insert(OPTION_SELECT_BAR_MAX_CLEARED, true),
    };
}

/// Synchronize panel state booleans.
pub fn sync_panel_state(state: &mut SharedGameState, panel: i32) {
    state.booleans.insert(OPTION_PANEL1, panel == 1);
    state.booleans.insert(OPTION_PANEL2, panel == 2);
    state.booleans.insert(OPTION_PANEL3, panel == 3);
}

/// Synchronize grade bar constraint flags.
///
/// Called when a Grade or Course bar is selected.
pub fn sync_grade_bar_constraints(
    state: &mut SharedGameState,
    constraints: &[CourseDataConstraint],
) {
    use CourseDataConstraint::*;
    state
        .booleans
        .insert(OPTION_GRADEBAR_CLASS, constraints.contains(&Class));
    state
        .booleans
        .insert(OPTION_GRADEBAR_MIRROR, constraints.contains(&GradeMirror));
    state
        .booleans
        .insert(OPTION_GRADEBAR_RANDOM, constraints.contains(&GradeRandom));
    state
        .booleans
        .insert(OPTION_GRADEBAR_NOSPEED, constraints.contains(&NoSpeed));
    state
        .booleans
        .insert(OPTION_GRADEBAR_NOGOOD, constraints.contains(&NoGood));
    state
        .booleans
        .insert(OPTION_GRADEBAR_NOGREAT, constraints.contains(&NoGreat));
    state
        .booleans
        .insert(OPTION_GRADEBAR_GAUGE_LR2, constraints.contains(&GaugeLr2));
    state.booleans.insert(
        OPTION_GRADEBAR_GAUGE_5KEYS,
        constraints.contains(&Gauge5Keys),
    );
    state.booleans.insert(
        OPTION_GRADEBAR_GAUGE_7KEYS,
        constraints.contains(&Gauge7Keys),
    );
    state.booleans.insert(
        OPTION_GRADEBAR_GAUGE_9KEYS,
        constraints.contains(&Gauge9Keys),
    );
    state.booleans.insert(
        OPTION_GRADEBAR_GAUGE_24KEYS,
        constraints.contains(&Gauge24Keys),
    );
    state
        .booleans
        .insert(OPTION_GRADEBAR_LN, constraints.contains(&Ln));
    state
        .booleans
        .insert(OPTION_GRADEBAR_CN, constraints.contains(&Cn));
    state
        .booleans
        .insert(OPTION_GRADEBAR_HCN, constraints.contains(&Hcn));
}

/// Synchronize score data properties for the selected song bar.
///
/// Java: IntegerPropertyFactory NUMBER_PLAYCOUNT(77), NUMBER_CLEARCOUNT(78),
/// NUMBER_FAILCOUNT(79), NUMBER_SCORE2(101), NUMBER_SCORE_RATE(102),
/// NUMBER_SCORE_RATE_AFTERDOT(103), NUMBER_MAXCOMBO2(105), NUMBER_HIGHSCORE2(170),
/// NUMBER_MISSCOUNT2(177), NUMBER_PERFECT2(80)-NUMBER_POOR2(84),
/// NUMBER_PERFECT_RATE(85)-NUMBER_POOR_RATE(89), NUMBER_BEST_RATE(183),
/// NUMBER_BEST_RATE_AFTERDOT(184).
fn sync_selected_score(state: &mut SharedGameState, score: Option<&ScoreData>) {
    const SCORE_INTEGER_IDS: &[i32] = &[
        NUMBER_PLAYCOUNT,
        NUMBER_CLEARCOUNT,
        NUMBER_FAILCOUNT,
        NUMBER_SCORE2,
        NUMBER_SCORE_RATE,
        NUMBER_SCORE_RATE_AFTERDOT,
        NUMBER_MAXCOMBO2,
        NUMBER_HIGHSCORE2,
        NUMBER_MISSCOUNT2,
        NUMBER_PERFECT2,
        NUMBER_GREAT2,
        NUMBER_GOOD2,
        NUMBER_BAD2,
        NUMBER_POOR2,
        NUMBER_PERFECT_RATE,
        NUMBER_GREAT_RATE,
        NUMBER_GOOD_RATE,
        NUMBER_BAD_RATE,
        NUMBER_POOR_RATE,
        NUMBER_BEST_RATE,
        NUMBER_BEST_RATE_AFTERDOT,
    ];
    const SCORE_FLOAT_IDS: &[i32] = &[FLOAT_SCORE_RATE2, FLOAT_BEST_RATE];

    match score {
        Some(sd) => {
            state.integers.insert(NUMBER_PLAYCOUNT, sd.playcount);
            state.integers.insert(NUMBER_CLEARCOUNT, sd.clearcount);
            state
                .integers
                .insert(NUMBER_FAILCOUNT, sd.playcount - sd.clearcount);

            let ex = sd.exscore();
            state.integers.insert(NUMBER_SCORE2, ex);
            state.integers.insert(NUMBER_HIGHSCORE2, ex);
            state.integers.insert(NUMBER_MAXCOMBO2, sd.maxcombo);
            state.integers.insert(NUMBER_MISSCOUNT2, sd.minbp);

            // Score rate (% of max EX score)
            let max_ex = sd.notes * 2;
            if max_ex > 0 {
                let rate_100 = ex as f64 * 100.0 / max_ex as f64;
                state.integers.insert(NUMBER_SCORE_RATE, rate_100 as i32);
                state.integers.insert(
                    NUMBER_SCORE_RATE_AFTERDOT,
                    ((rate_100 * 100.0) as i32) % 100,
                );
                state.floats.insert(FLOAT_SCORE_RATE2, rate_100 as f32);

                // Best rate (same as score rate on select screen)
                state.integers.insert(NUMBER_BEST_RATE, rate_100 as i32);
                state
                    .integers
                    .insert(NUMBER_BEST_RATE_AFTERDOT, ((rate_100 * 100.0) as i32) % 100);
                state.floats.insert(FLOAT_BEST_RATE, rate_100 as f32);
            } else {
                state.integers.insert(NUMBER_SCORE_RATE, 0);
                state.integers.insert(NUMBER_SCORE_RATE_AFTERDOT, 0);
                state.floats.insert(FLOAT_SCORE_RATE2, 0.0);
                state.integers.insert(NUMBER_BEST_RATE, 0);
                state.integers.insert(NUMBER_BEST_RATE_AFTERDOT, 0);
                state.floats.insert(FLOAT_BEST_RATE, 0.0);
            }

            // Per-judge counts (Java: NUMBER_PERFECT2(80)-NUMBER_POOR2(84))
            state
                .integers
                .insert(NUMBER_PERFECT2, sd.judge_count(bms_rule::JUDGE_PG));
            state
                .integers
                .insert(NUMBER_GREAT2, sd.judge_count(bms_rule::JUDGE_GR));
            state
                .integers
                .insert(NUMBER_GOOD2, sd.judge_count(bms_rule::JUDGE_GD));
            state
                .integers
                .insert(NUMBER_BAD2, sd.judge_count(bms_rule::JUDGE_BD));
            state
                .integers
                .insert(NUMBER_POOR2, sd.judge_count(bms_rule::JUDGE_PR));

            // Per-judge rates (Java: score.getJudgeCount(i) * 100 / score.getNotes())
            if sd.notes > 0 {
                state.integers.insert(
                    NUMBER_PERFECT_RATE,
                    sd.judge_count(bms_rule::JUDGE_PG) * 100 / sd.notes,
                );
                state.integers.insert(
                    NUMBER_GREAT_RATE,
                    sd.judge_count(bms_rule::JUDGE_GR) * 100 / sd.notes,
                );
                state.integers.insert(
                    NUMBER_GOOD_RATE,
                    sd.judge_count(bms_rule::JUDGE_GD) * 100 / sd.notes,
                );
                state.integers.insert(
                    NUMBER_BAD_RATE,
                    sd.judge_count(bms_rule::JUDGE_BD) * 100 / sd.notes,
                );
                state.integers.insert(
                    NUMBER_POOR_RATE,
                    sd.judge_count(bms_rule::JUDGE_PR) * 100 / sd.notes,
                );
            } else {
                state.integers.insert(NUMBER_PERFECT_RATE, 0);
                state.integers.insert(NUMBER_GREAT_RATE, 0);
                state.integers.insert(NUMBER_GOOD_RATE, 0);
                state.integers.insert(NUMBER_BAD_RATE, 0);
                state.integers.insert(NUMBER_POOR_RATE, 0);
            }
        }
        None => {
            for &id in SCORE_INTEGER_IDS {
                state.integers.remove(&id);
            }
            for &id in SCORE_FLOAT_IDS {
                state.floats.remove(&id);
            }
        }
    }
}

fn clear_song_metadata(state: &mut SharedGameState) {
    state.strings.insert(STRING_TITLE, String::new());
    state.strings.insert(STRING_SUBTITLE, String::new());
    state.strings.insert(STRING_FULLTITLE, String::new());
    state.strings.insert(STRING_ARTIST, String::new());
    state.strings.insert(STRING_SUBARTIST, String::new());
    state.strings.insert(STRING_GENRE, String::new());
    state.integers.insert(NUMBER_MINBPM, 0);
    state.integers.insert(NUMBER_MAXBPM, 0);
    state.integers.insert(NUMBER_TOTALNOTES2, 0);
    sync_song_information(state, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_mode_flags_7k() {
        let mut state = SharedGameState::default();
        sync_mode_flags(&mut state, 7);
        assert!(*state.booleans.get(&OPTION_7KEYSONG).unwrap());
        assert!(!*state.booleans.get(&OPTION_5KEYSONG).unwrap());
    }

    #[test]
    fn sync_mode_flags_14k() {
        let mut state = SharedGameState::default();
        sync_mode_flags(&mut state, 14);
        assert!(*state.booleans.get(&OPTION_14KEYSONG).unwrap());
        assert!(!*state.booleans.get(&OPTION_7KEYSONG).unwrap());
    }

    #[test]
    fn clear_song_metadata_empties_strings() {
        let mut state = SharedGameState::default();
        state.strings.insert(STRING_TITLE, "test".to_string());
        clear_song_metadata(&mut state);
        assert_eq!(state.strings.get(&STRING_TITLE).unwrap(), "");
    }

    #[test]
    fn sync_select_no_bar_clears_metadata() {
        let mut state = SharedGameState::default();
        let bm = BarManager::new();
        sync_select_state(&mut state, &bm, false, true, false, 0, None, None);
        assert!(!*state.booleans.get(&OPTION_SONGBAR).unwrap());
        assert!(!*state.booleans.get(&OPTION_FOLDERBAR).unwrap());
    }

    #[test]
    fn sync_select_feature_flags() {
        let mut state = SharedGameState::default();
        let bm = BarManager::new();
        sync_select_state(&mut state, &bm, true, false, false, 0, None, None);
        assert!(*state.booleans.get(&OPTION_LN).unwrap());
        assert!(!*state.booleans.get(&OPTION_NO_LN).unwrap());
        assert!(!*state.booleans.get(&OPTION_BGA).unwrap());
        assert!(*state.booleans.get(&OPTION_NO_BGA).unwrap());
    }

    fn make_test_info() -> SongInformation {
        SongInformation {
            sha256: "test_sha".to_string(),
            n: 100,
            ln: 20,
            s: 15,
            ls: 5,
            total: 300.0,
            density: 12.75,
            peakdensity: 25.50,
            enddensity: 8.33,
            mainbpm: 150.0,
            distribution: String::new(),
            speedchange: String::new(),
            lanenotes: String::new(),
        }
    }

    #[test]
    fn sync_song_information_sets_integers() {
        let mut state = SharedGameState::default();
        let info = make_test_info();
        sync_song_information(&mut state, Some(&info));

        assert_eq!(*state.integers.get(&NUMBER_TOTALNOTE_NORMAL).unwrap(), 100);
        assert_eq!(*state.integers.get(&NUMBER_TOTALNOTE_LN).unwrap(), 20);
        assert_eq!(*state.integers.get(&NUMBER_TOTALNOTE_SCRATCH).unwrap(), 15);
        assert_eq!(*state.integers.get(&NUMBER_TOTALNOTE_BSS).unwrap(), 5);
        assert_eq!(*state.integers.get(&NUMBER_MAINBPM).unwrap(), 150);
        assert_eq!(*state.integers.get(&NUMBER_SONGGAUGE_TOTAL).unwrap(), 300);
    }

    #[test]
    fn sync_song_information_sets_density_afterdot() {
        let mut state = SharedGameState::default();
        let info = make_test_info();
        sync_song_information(&mut state, Some(&info));

        // density=12.75 → INTEGER=12, AFTERDOT=75
        assert_eq!(*state.integers.get(&NUMBER_DENSITY_AVERAGE).unwrap(), 12);
        assert_eq!(
            *state
                .integers
                .get(&NUMBER_DENSITY_AVERAGE_AFTERDOT)
                .unwrap(),
            75
        );

        // peakdensity=25.50 → INTEGER=25, AFTERDOT=50
        assert_eq!(*state.integers.get(&NUMBER_DENSITY_PEAK).unwrap(), 25);
        assert_eq!(
            *state.integers.get(&NUMBER_DENSITY_PEAK_AFTERDOT).unwrap(),
            50
        );

        // enddensity=8.33 → INTEGER=8, AFTERDOT=33
        assert_eq!(*state.integers.get(&NUMBER_DENSITY_END).unwrap(), 8);
        assert_eq!(
            *state.integers.get(&NUMBER_DENSITY_END_AFTERDOT).unwrap(),
            33
        );
    }

    #[test]
    fn sync_song_information_sets_floats() {
        let mut state = SharedGameState::default();
        let info = make_test_info();
        sync_song_information(&mut state, Some(&info));

        let eps = 0.001;
        assert!((*state.floats.get(&FLOAT_CHART_AVERAGEDENSITY).unwrap() - 12.75).abs() < eps);
        assert!((*state.floats.get(&FLOAT_CHART_ENDDENSITY).unwrap() - 8.33).abs() < eps);
        assert!((*state.floats.get(&FLOAT_CHART_PEAKDENSITY).unwrap() - 25.50).abs() < eps);
        assert!((*state.floats.get(&FLOAT_CHART_TOTALGAUGE).unwrap() - 300.0).abs() < eps);
    }

    #[test]
    fn sync_song_information_none_removes_all() {
        let mut state = SharedGameState::default();
        let info = make_test_info();
        sync_song_information(&mut state, Some(&info));

        // Verify some properties are set
        assert!(state.integers.contains_key(&NUMBER_TOTALNOTE_NORMAL));
        assert!(state.floats.contains_key(&FLOAT_CHART_PEAKDENSITY));

        // Clear
        sync_song_information(&mut state, None);

        assert!(!state.integers.contains_key(&NUMBER_TOTALNOTE_NORMAL));
        assert!(!state.integers.contains_key(&NUMBER_TOTALNOTE_LN));
        assert!(!state.integers.contains_key(&NUMBER_TOTALNOTE_SCRATCH));
        assert!(!state.integers.contains_key(&NUMBER_TOTALNOTE_BSS));
        assert!(!state.integers.contains_key(&NUMBER_DENSITY_PEAK));
        assert!(!state.integers.contains_key(&NUMBER_DENSITY_PEAK_AFTERDOT));
        assert!(!state.integers.contains_key(&NUMBER_DENSITY_END));
        assert!(!state.integers.contains_key(&NUMBER_DENSITY_END_AFTERDOT));
        assert!(!state.integers.contains_key(&NUMBER_DENSITY_AVERAGE));
        assert!(
            !state
                .integers
                .contains_key(&NUMBER_DENSITY_AVERAGE_AFTERDOT)
        );
        assert!(!state.integers.contains_key(&NUMBER_SONGGAUGE_TOTAL));
        assert!(!state.integers.contains_key(&NUMBER_MAINBPM));
        assert!(!state.floats.contains_key(&FLOAT_CHART_AVERAGEDENSITY));
        assert!(!state.floats.contains_key(&FLOAT_CHART_ENDDENSITY));
        assert!(!state.floats.contains_key(&FLOAT_CHART_PEAKDENSITY));
        assert!(!state.floats.contains_key(&FLOAT_CHART_TOTALGAUGE));
    }

    #[test]
    fn clear_song_metadata_also_clears_song_information() {
        let mut state = SharedGameState::default();
        let info = make_test_info();
        sync_song_information(&mut state, Some(&info));
        assert!(state.integers.contains_key(&NUMBER_TOTALNOTE_NORMAL));

        clear_song_metadata(&mut state);
        assert!(!state.integers.contains_key(&NUMBER_TOTALNOTE_NORMAL));
        assert!(!state.floats.contains_key(&FLOAT_CHART_PEAKDENSITY));
    }

    #[test]
    fn sync_bar_scroll_state_uses_score_lamp_cache() {
        use bms_database::SongData;

        // Use set_bars_for_test to place a Song bar directly (load_root now produces
        // Folder bars grouped by folder CRC, not Song bars).
        let mut bm = BarManager::new();
        bm.set_bars_for_test(vec![Bar::Song(Box::new(SongData {
            md5: "md5_a".to_string(),
            sha256: "sha_a".to_string(),
            title: "Song A".to_string(),
            path: "a.bms".to_string(),
            ..Default::default()
        }))]);
        assert_eq!(bm.bar_count(), 1);

        let mut cache = HashMap::new();
        cache.insert("sha_a".to_string(), 6); // Hard clear

        let mut state = SharedGameState::default();
        sync_bar_scroll_state(&mut state, &bm, 0, 0.0, 0, &cache);

        let scroll = state.bar_scroll_state.as_ref().unwrap();
        assert_eq!(scroll.slots[0].lamp_id, 6);
    }

    #[test]
    fn sync_bar_scroll_state_missing_cache_defaults_to_zero() {
        use bms_database::{SongData, SongDatabase};

        let song_db = SongDatabase::open_in_memory().unwrap();
        let song = SongData {
            md5: "md5_b".to_string(),
            sha256: "sha_b".to_string(),
            title: "Song B".to_string(),
            path: "b.bms".to_string(),
            ..Default::default()
        };
        song_db.set_song_datas(&[song]).unwrap();

        let mut bm = BarManager::new();
        bm.load_root(&song_db);

        let cache = HashMap::new(); // empty cache

        let mut state = SharedGameState::default();
        sync_bar_scroll_state(&mut state, &bm, 0, 0.0, 0, &cache);

        let scroll = state.bar_scroll_state.as_ref().unwrap();
        assert_eq!(scroll.slots[0].lamp_id, 0);
    }

    #[test]
    fn sync_song_information_populates_graph_data() {
        let mut state = SharedGameState::default();
        let info = SongInformation {
            // speedchange: "150.0,0.0,180.0,5000.0" → [(0, 150.0), (5_000_000, 180.0)]
            speedchange: "150.0,0.0,180.0,5000.0".to_string(),
            // distribution: "#" + base36-encoded values; use simple single bucket
            // 7 columns, each value 1 → base36 "01" × 7 = "01010101010101"
            distribution: "#01010101010101".to_string(),
            ..make_test_info()
        };
        sync_song_information(&mut state, Some(&info));

        // BPM events: [150.0, 0.0] → (0, 150.0); [180.0, 5000.0] → (5_000_000, 180.0)
        assert_eq!(state.bpm_events.len(), 2);
        assert_eq!(state.bpm_events[0].0, 0); // 0.0 * 1000
        assert!((state.bpm_events[0].1 - 150.0).abs() < 0.001);
        assert_eq!(state.bpm_events[1].0, 5_000_000); // 5000.0 * 1000
        assert!((state.bpm_events[1].1 - 180.0).abs() < 0.001);

        // Note distribution: 1 bucket, sum of 7 × 1 = 7
        assert_eq!(state.note_distribution.len(), 1);
        assert_eq!(state.note_distribution[0], 7);
    }

    #[test]
    fn sync_song_information_none_clears_graph_data() {
        let mut state = SharedGameState::default();
        let info = SongInformation {
            speedchange: "150.0,0.0".to_string(),
            distribution: "#01010101010101".to_string(),
            ..make_test_info()
        };
        sync_song_information(&mut state, Some(&info));
        assert!(!state.bpm_events.is_empty());
        assert!(!state.note_distribution.is_empty());

        sync_song_information(&mut state, None);
        assert!(state.bpm_events.is_empty());
        assert!(state.note_distribution.is_empty());
    }

    #[test]
    fn sync_song_information_empty_graph_strings() {
        let mut state = SharedGameState::default();
        let info = make_test_info(); // distribution and speedchange are empty
        sync_song_information(&mut state, Some(&info));

        assert!(state.bpm_events.is_empty());
        assert!(state.note_distribution.is_empty());
    }

    #[test]
    fn sync_rival_data_populates_score() {
        let mut state = SharedGameState::default();
        let bm = BarManager::new();
        let rival_score = ScoreData {
            epg: 100,
            lpg: 50,
            egr: 30,
            lgr: 20,
            maxcombo: 200,
            minbp: 5,
            playcount: 10,
            clearcount: 8,
            ..Default::default()
        };
        let rival = RivalSkinData {
            name: "TestRival",
            score: Some(&rival_score),
        };
        sync_select_state(&mut state, &bm, false, true, false, 0, Some(&rival), None);
        assert!(*state.booleans.get(&OPTION_COMPARE_RIVAL).unwrap());
        assert!(!*state.booleans.get(&OPTION_NOT_COMPARE_RIVAL).unwrap());
        assert_eq!(state.strings.get(&STRING_RIVAL).unwrap(), "TestRival");
        assert_eq!(*state.integers.get(&NUMBER_RIVAL_SCORE).unwrap(), 350); // (100+50)*2 + 30+20
        assert_eq!(*state.integers.get(&NUMBER_RIVAL_MAXCOMBO).unwrap(), 200);
        assert_eq!(*state.integers.get(&NUMBER_RIVAL_MISSCOUNT).unwrap(), 5);
        assert_eq!(*state.integers.get(&NUMBER_RIVAL_PLAYCOUNT).unwrap(), 10);
        assert_eq!(*state.integers.get(&NUMBER_RIVAL_CLEARCOUNT).unwrap(), 8);
        assert_eq!(*state.integers.get(&NUMBER_RIVAL_FAILCOUNT).unwrap(), 2);
    }

    #[test]
    fn sync_rival_none_clears_rival_state() {
        let mut state = SharedGameState::default();
        let bm = BarManager::new();
        // First set rival data
        let rival_score = ScoreData {
            epg: 50,
            lpg: 50,
            ..Default::default()
        };
        let rival = RivalSkinData {
            name: "Rival",
            score: Some(&rival_score),
        };
        sync_select_state(&mut state, &bm, false, true, false, 0, Some(&rival), None);
        assert!(state.integers.contains_key(&NUMBER_RIVAL_SCORE));

        // Clear rival
        sync_select_state(&mut state, &bm, false, true, false, 0, None, None);
        assert!(!*state.booleans.get(&OPTION_COMPARE_RIVAL).unwrap());
        assert!(*state.booleans.get(&OPTION_NOT_COMPARE_RIVAL).unwrap());
        assert_eq!(state.strings.get(&STRING_RIVAL).unwrap(), "");
        assert!(!state.integers.contains_key(&NUMBER_RIVAL_SCORE));
    }

    #[test]
    fn sync_rival_with_no_score() {
        let mut state = SharedGameState::default();
        let bm = BarManager::new();
        let rival = RivalSkinData {
            name: "RivalNoScore",
            score: None,
        };
        sync_select_state(&mut state, &bm, false, true, false, 0, Some(&rival), None);
        assert!(*state.booleans.get(&OPTION_COMPARE_RIVAL).unwrap());
        assert_eq!(state.strings.get(&STRING_RIVAL).unwrap(), "RivalNoScore");
        assert!(!state.integers.contains_key(&NUMBER_RIVAL_SCORE));
    }
}
