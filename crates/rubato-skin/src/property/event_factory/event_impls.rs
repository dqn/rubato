use super::super::event::Event;
use crate::reexports::MainState;

use rubato_core::bms_player_mode::BMSPlayerMode;
use rubato_play::judge_algorithm::DEFAULT_ALGORITHM;
use rubato_play::target_property::TargetProperty;
use rubato_types::event_id::EventId;
use rubato_types::main_state_type::MainStateType;
use rubato_types::play_config;

// ============================================================
// Delegate Event: forwards to state.execute_event()
// Used for events that require types not available in beatoraja-skin
// (e.g., SongDatabase, BarManager, Desktop, IRConnection, etc.)
// ============================================================

pub(super) struct DelegateEvent {
    pub(super) event_id: EventId,
}

impl Event for DelegateEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, arg2: i32) {
        state.execute_event(self.event_id.as_i32(), arg1, arg2);
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// State change events (keyconfig, skinconfig)
// ============================================================

pub(super) struct StateChangeEvent(pub(super) MainStateType);

impl Event for StateChangeEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if state.is_music_selector() {
            state.change_state(self.0);
        }
    }

    fn get_event_id(&self) -> EventId {
        match self.0 {
            MainStateType::Config => EventId(13),
            MainStateType::SkinConfig => EventId(14),
            _ => EventId::UNDEFINED,
        }
    }
}

// ============================================================
// Select song events (play, autoplay, practice)
// ============================================================

pub(super) struct SelectSongEvent(pub(super) BMSPlayerMode);

impl Event for SelectSongEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if state.is_music_selector() {
            state.select_song(self.0);
        }
    }

    fn get_event_id(&self) -> EventId {
        match &self.0 {
            m if *m == BMSPlayerMode::PLAY => EventId(15),
            m if *m == BMSPlayerMode::AUTOPLAY => EventId(16),
            m if *m == BMSPlayerMode::PRACTICE => EventId(315),
            _ => EventId::UNDEFINED,
        }
    }
}

// ============================================================
// Replay events
// ============================================================

pub(super) struct ReplayEvent(pub(super) i32);

impl Event for ReplayEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if state.is_music_selector()
            && let Some(mode) = BMSPlayerMode::replay_mode(self.0)
        {
            state.select_song(*mode);
        }
        // MusicResult/CourseResult replay saving is handled by execute_event delegation
        // because those types need cross-crate access
        if !state.is_music_selector() {
            state.execute_event(self.get_event_id().as_i32(), 0, 0);
        }
    }

    fn get_event_id(&self) -> EventId {
        match self.0 {
            0 => EventId(19),
            1 => EventId(316),
            2 => EventId(317),
            3 => EventId(318),
            _ => EventId::UNDEFINED,
        }
    }
}

// ============================================================
// Mode event: cycle through MODE filter array
// ============================================================

/// Mode filter array (same as MusicSelector.MODE in Java)
static MODE_FILTER: [Option<bms_model::mode::Mode>; 8] = [
    None,
    Some(bms_model::mode::Mode::BEAT_7K),
    Some(bms_model::mode::Mode::BEAT_14K),
    Some(bms_model::mode::Mode::POPN_9K),
    Some(bms_model::mode::Mode::BEAT_5K),
    Some(bms_model::mode::Mode::BEAT_10K),
    Some(bms_model::mode::Mode::KEYBOARD_24K),
    Some(bms_model::mode::Mode::KEYBOARD_24K_DOUBLE),
];

pub(super) struct ModeEvent;

impl Event for ModeEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let current_mode = config.mode;
        let mut mode_idx = 0;
        for (i, m) in MODE_FILTER.iter().enumerate() {
            if *m == current_mode {
                mode_idx = i;
                break;
            }
        }
        let len = MODE_FILTER.len();
        let next_idx = if arg1 >= 0 {
            (mode_idx + 1) % len
        } else {
            (mode_idx + len - 1) % len
        };
        config.mode = MODE_FILTER[next_idx];
        state.update_bar_after_change();
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        EventId(11)
    }
}

// ============================================================
// Sort event: cycle through default sorters
// ============================================================

pub(super) struct SortEvent;

impl Event for SortEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let len = rubato_types::bar_sorter::BarSorter::DEFAULT_SORTER.len() as i32;
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let current = config.select_settings.sort;
        let next = if arg1 >= 0 {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        config.select_settings.sort = next;
        config.select_settings.sortid = Some(
            rubato_types::bar_sorter::BarSorter::DEFAULT_SORTER[next as usize]
                .name()
                .to_string(),
        );
        state.update_bar_after_change();
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        EventId(12)
    }
}

// ============================================================
// Songbar sort event: cycle through ALL sorters by sortid
// ============================================================

pub(super) struct SongbarSortEvent;

impl Event for SongbarSortEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let all = &rubato_types::bar_sorter::BarSorter::ALL_SORTER;
        let len = all.len();
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let current_sortid = config.select_settings.sortid.clone().unwrap_or_default();
        let mut found_idx = None;
        for (i, s) in all.iter().enumerate() {
            if s.name() == current_sortid {
                found_idx = Some(i);
                break;
            }
        }
        if let Some(idx) = found_idx {
            let next_idx = if arg1 >= 0 {
                (idx + 1) % len
            } else {
                (idx + len - 1) % len
            };
            config.select_settings.sortid = Some(all[next_idx].name().to_string());
            state.update_bar_after_change();
            state.play_option_change_sound();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(312)
    }
}

// ============================================================
// PlayerConfig cycle event (generic)
// Cycles a PlayerConfig integer field through [0..count)
// ============================================================

pub(super) struct PlayerConfigCycleEvent {
    pub(super) event_id: EventId,
    pub(super) get: fn(&rubato_types::player_config::PlayerConfig) -> i32,
    pub(super) set: fn(&mut rubato_types::player_config::PlayerConfig, i32),
    pub(super) count: i32,
    pub(super) music_selector_only: bool,
}

impl Event for PlayerConfigCycleEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if self.music_selector_only && !state.is_music_selector() {
            return;
        }
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let current = (self.get)(config);
        let next = if arg1 >= 0 {
            (current + 1) % self.count
        } else {
            (current + self.count - 1) % self.count
        };
        (self.set)(config, next);
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// PlayConfig cycle event (generic)
// Cycles a PlayConfig integer field through [0..count)
// Only available for MusicSelector (needs getSelectedBarPlayConfig)
// ============================================================

pub(super) struct PlayConfigCycleEvent {
    pub(super) event_id: EventId,
    pub(super) get: fn(&play_config::PlayConfig) -> i32,
    pub(super) set: fn(&mut play_config::PlayConfig, i32),
    pub(super) count: i32,
}

impl Event for PlayConfigCycleEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(pc) = state.selected_play_config_mut() else {
            return;
        };
        let current = (self.get)(pc);
        let next = if arg1 >= 0 {
            (current + 1) % self.count
        } else {
            (current + self.count - 1) % self.count
        };
        (self.set)(pc, next);
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// PlayConfig toggle event (generic)
// Toggles a PlayConfig boolean field
// ============================================================

pub(super) struct PlayConfigToggleEvent {
    pub(super) event_id: EventId,
    pub(super) get: fn(&play_config::PlayConfig) -> bool,
    pub(super) set: fn(&mut play_config::PlayConfig, bool),
}

impl Event for PlayConfigToggleEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(pc) = state.selected_play_config_mut() else {
            return;
        };
        let current = (self.get)(pc);
        (self.set)(pc, !current);
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// Config cycle event (for bga, bgaexpand)
// Cycles a Config integer field through [0..count)
// ============================================================

pub(super) struct ConfigCycleEvent {
    pub(super) event_id: EventId,
    pub(super) get: fn(&rubato_types::config::Config) -> i32,
    pub(super) set: fn(&mut rubato_types::config::Config, i32),
    pub(super) count: i32,
}

impl Event for ConfigCycleEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(config) = state.config_mut() else {
            return;
        };
        let current = (self.get)(config);
        let next = if arg1 >= 0 {
            (current + 1) % self.count
        } else {
            (current + self.count - 1) % self.count
        };
        (self.set)(config, next);
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// Hispeed event
// ============================================================

pub(super) struct HispeedEvent;

impl Event for HispeedEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(pc) = state.selected_play_config_mut() else {
            return;
        };
        let margin = pc.hispeedmargin;
        let delta = if arg1 >= 0 { margin } else { -margin };
        let new_hispeed =
            (pc.hispeed + delta).clamp(play_config::HISPEED_MIN, play_config::HISPEED_MAX);
        if (new_hispeed - pc.hispeed).abs() > f32::EPSILON {
            pc.hispeed = new_hispeed;
            state.play_option_change_sound();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(57)
    }
}

// ============================================================
// Duration event
// ============================================================

pub(super) struct DurationEvent;

impl Event for DurationEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(pc) = state.selected_play_config_mut() else {
            return;
        };
        let inc = if arg2 > 0 { arg2 } else { 1 };
        let delta = if arg1 >= 0 { inc } else { -inc };
        let new_duration =
            (pc.duration + delta).clamp(play_config::DURATION_MIN, play_config::DURATION_MAX);
        if new_duration != pc.duration {
            pc.duration = new_duration;
            state.play_option_change_sound();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(59)
    }
}

// ============================================================
// Hispeed auto-adjust toggle
// ============================================================

pub(super) struct HispeedAutoAdjustEvent;

impl Event for HispeedAutoAdjustEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(pc) = state.selected_play_config_mut() else {
            return;
        };
        pc.hispeedautoadjust = !pc.hispeedautoadjust;
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        EventId(342)
    }
}

// ============================================================
// Notes display timing event
// ============================================================

pub(super) struct NotesDisplayTimingEvent;

impl Event for NotesDisplayTimingEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let max = rubato_types::player_config::JUDGETIMING_MAX;
        let min = rubato_types::player_config::JUDGETIMING_MIN;
        let inc = if arg1 >= 0 {
            if config.judge_settings.judgetiming < max {
                1
            } else {
                0
            }
        } else if config.judge_settings.judgetiming > min {
            -1
        } else {
            0
        };
        if inc != 0 {
            config.judge_settings.judgetiming += inc;
            if state.is_music_selector() {
                state.play_option_change_sound();
            }
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(74)
    }
}

// ============================================================
// Notes display timing auto-adjust toggle
// ============================================================

pub(super) struct NotesDisplayTimingAutoAdjustEvent;

impl Event for NotesDisplayTimingAutoAdjustEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        let Some(config) = state.player_config_mut() else {
            return;
        };
        config.judge_settings.notes_display_timing_auto_adjust =
            !config.judge_settings.notes_display_timing_auto_adjust;
        if state.is_music_selector() {
            state.play_option_change_sound();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(75)
    }
}

// ============================================================
// Target event: cycle through target IDs
// ============================================================

pub(super) struct TargetEvent;

impl Event for TargetEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let targets = {
            let targets = TargetProperty::targets();
            if targets.is_empty() {
                config.select_settings.targetlist.clone()
            } else {
                targets
            }
        };
        if targets.is_empty() {
            return;
        }
        let mut index = 0;
        for (i, t) in targets.iter().enumerate() {
            if *t == config.select_settings.targetid {
                index = i;
                break;
            }
        }
        let len = targets.len();
        let next = if arg1 >= 0 {
            (index + 1) % len
        } else {
            (index + len - 1) % len
        };
        config.select_settings.targetid = targets[next].clone();
        state.play_option_change_sound();
        if state.is_music_selector() {
            state.update_bar_after_change();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(77)
    }
}

// ============================================================
// Key assign event (no-op, matches Java behavior)
// ============================================================

pub(super) struct KeyAssignEvent(pub(super) i32);

impl Event for KeyAssignEvent {
    fn exec(&self, _state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        // In Java, changeKeyAssign only checks `state instanceof KeyConfiguration`
        // and does nothing inside the body. Preserved as no-op.
    }

    fn get_event_id(&self) -> EventId {
        // keyassign1..39 = 101..139, keyassign40..54 = 150..164
        if self.0 < 39 {
            EventId(101 + self.0)
        } else {
            EventId(150 + (self.0 - 39))
        }
    }
}

// ============================================================
// LN mode event (disabled in this fork)
// ============================================================

pub(super) struct LnModeEvent;

impl Event for LnModeEvent {
    fn exec(&self, _state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        // LN mode switching is disabled in this fork (endless dream).
        // Java code has the logic commented out with `return;` at the top.
    }

    fn get_event_id(&self) -> EventId {
        EventId(308)
    }
}

// ============================================================
// Auto save replay event
// ============================================================

pub(super) struct AutoSaveReplayEvent {
    pub(super) index: usize,
    pub(super) event_id: EventId,
}

impl Event for AutoSaveReplayEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        // ReplayAutoSaveConstraint::values().len() = 11
        let length = 11;
        let Some(config) = state.player_config_mut() else {
            return;
        };
        if self.index >= config.misc_settings.autosavereplay.len() {
            return;
        }
        let current = config.misc_settings.autosavereplay[self.index];
        let next = if arg1 >= 0 {
            (current + 1) % length
        } else {
            (current + length - 1) % length
        };
        config.misc_settings.autosavereplay[self.index] = next;
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        self.event_id
    }
}

// ============================================================
// Judge algorithm event
// ============================================================

pub(super) struct JudgeAlgorithmEvent;

impl Event for JudgeAlgorithmEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let algorithms = DEFAULT_ALGORITHM;
        let alg_len = algorithms.len();
        let Some(pc) = state.selected_play_config_mut() else {
            return;
        };
        let jt = pc.judgetype.clone();
        for (i, alg) in algorithms.iter().enumerate() {
            if jt == alg.name() {
                let next = if arg1 >= 0 {
                    (i + 1) % alg_len
                } else {
                    (i + alg_len - 1) % alg_len
                };
                pc.judgetype = algorithms[next].name().to_string();
                // Need to play sound after releasing borrow on pc
                break;
            }
        }
        // Check if judgetype actually changed
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        EventId(340)
    }
}

// ============================================================
// Guide SE toggle
// ============================================================

pub(super) struct GuideSeEvent;

impl Event for GuideSeEvent {
    fn exec(&self, state: &mut dyn MainState, _arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        let Some(config) = state.player_config_mut() else {
            return;
        };
        config.display_settings.is_guide_se = !config.display_settings.is_guide_se;
        state.play_option_change_sound();
    }

    fn get_event_id(&self) -> EventId {
        EventId(343)
    }
}

// ============================================================
// Chart replication mode event
// ============================================================

pub(super) struct ChartReplicationModeEvent;

impl Event for ChartReplicationModeEvent {
    fn exec(&self, state: &mut dyn MainState, arg1: i32, _arg2: i32) {
        if !state.is_music_selector() {
            return;
        }
        // ChartReplicationMode.values() = [NONE, RIVALCHART, RIVALOPTION]
        let values = ["NONE", "RIVALCHART", "RIVALOPTION"];
        let len = values.len();
        let Some(config) = state.player_config_mut() else {
            return;
        };
        let current_id = config.play_settings.chart_replication_mode.clone();
        let mut found = false;
        for (i, name) in values.iter().enumerate() {
            if *name == current_id {
                let next = if arg1 >= 0 {
                    (i + 1) % len
                } else {
                    (i + len - 1) % len
                };
                config.play_settings.chart_replication_mode = values[next].to_string();
                found = true;
                break;
            }
        }
        if found {
            state.play_option_change_sound();
        }
    }

    fn get_event_id(&self) -> EventId {
        EventId(344)
    }
}
