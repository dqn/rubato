use super::BMSPlayerInputProcessor;

/// Music select key assignments
/// Translates: bms.player.beatoraja.select.MusicSelectKeyProperty
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MusicSelectKeyProperty {
    Beat7k,
    Popn9k,
    Beat14k,
}

/// Music select key actions
/// Translates: bms.player.beatoraja.select.MusicSelectKeyProperty.MusicSelectKey
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicSelectKey {
    Play,
    Auto,
    Replay,
    Up,
    Down,
    FolderOpen,
    FolderClose,
    Practice,
    Option1Up,
    Option1Down,
    GaugeUp,
    GaugeDown,
    OptiondpUp,
    OptiondpDown,
    HsfixUp,
    HsfixDown,
    Option2Up,
    Option2Down,
    TargetUp,
    TargetDown,
    JudgeArea,
    Nomine,
    BpmGuide,
    LegacyNote,
    Constant,
    JudgeWindowUp,
    JudgeWindowDown,
    MarkNote,
    BgaUp,
    BgaDown,
    GaugeAutoShiftUp,
    GaugeAutoShiftDown,
    DurationUp,
    DurationDown,
    NotesDisplayTimingUp,
    NotesDisplayTimingDown,
    NotesDisplayTimingAutoAdjust,
    NextReplay,
}

use MusicSelectKey::*;

impl MusicSelectKeyProperty {
    pub const VALUES: &'static [MusicSelectKeyProperty] = &[
        MusicSelectKeyProperty::Beat7k,
        MusicSelectKeyProperty::Popn9k,
        MusicSelectKeyProperty::Beat14k,
    ];

    fn assign(&self) -> &'static [&'static [MusicSelectKey]] {
        match self {
            MusicSelectKeyProperty::Beat7k => &BEAT_7K_ASSIGN,
            MusicSelectKeyProperty::Popn9k => &POPN_9K_ASSIGN,
            MusicSelectKeyProperty::Beat14k => &BEAT_14K_ASSIGN,
        }
    }

    pub fn analog_change(&self, input: &mut BMSPlayerInputProcessor, code: MusicSelectKey) -> i32 {
        let assign = self.assign();
        let mut d_ticks = 0;
        for (i, keys) in assign.iter().enumerate() {
            for &index in *keys {
                if code == index && input.is_analog_input(i) {
                    d_ticks += input.analog_diff_and_reset(i, 200);
                }
            }
        }
        d_ticks
    }

    pub fn is_non_analog_pressed(
        &self,
        input: &mut BMSPlayerInputProcessor,
        code: MusicSelectKey,
        reset_state: bool,
    ) -> bool {
        let assign = self.assign();
        for (i, keys) in assign.iter().enumerate() {
            for &index in *keys {
                if code == index && input.key_state(i as i32) {
                    if input.is_analog_input(i) {
                        continue;
                    }
                    if reset_state {
                        return input.reset_key_changed_time(i as i32);
                    } else {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn is_pressed(
        &self,
        input: &mut BMSPlayerInputProcessor,
        code: MusicSelectKey,
        reset_state: bool,
    ) -> bool {
        let assign = self.assign();
        for (i, keys) in assign.iter().enumerate() {
            for &index in *keys {
                if code == index && input.key_state(i as i32) {
                    if reset_state {
                        return input.reset_key_changed_time(i as i32);
                    } else {
                        return true;
                    }
                }
            }
        }
        false
    }
}

static BEAT_7K_ASSIGN: [&[MusicSelectKey]; 9] = [
    &[Play, FolderOpen, Option1Down, JudgeWindowUp, BgaDown],
    &[FolderClose, Option1Up, Constant, GaugeAutoShiftDown],
    &[
        Practice,
        FolderOpen,
        GaugeDown,
        JudgeArea,
        NotesDisplayTimingAutoAdjust,
    ],
    &[FolderClose, OptiondpDown, LegacyNote, DurationDown],
    &[
        FolderOpen,
        Auto,
        HsfixDown,
        MarkNote,
        NotesDisplayTimingDown,
    ],
    &[NextReplay, Option2Up, BpmGuide, DurationUp],
    &[
        FolderOpen,
        Replay,
        Option2Down,
        Nomine,
        NotesDisplayTimingUp,
    ],
    &[Up, TargetUp],
    &[Down, TargetDown],
];

static POPN_9K_ASSIGN: [&[MusicSelectKey]; 9] = [
    &[Auto, Option1Down, JudgeWindowUp, BgaDown],
    &[Option1Up, Constant, GaugeAutoShiftDown],
    &[
        FolderClose,
        GaugeDown,
        JudgeArea,
        NotesDisplayTimingAutoAdjust,
    ],
    &[Down, OptiondpDown, LegacyNote, DurationDown],
    &[
        Play,
        FolderOpen,
        HsfixDown,
        MarkNote,
        NotesDisplayTimingDown,
    ],
    &[Up, Option2Up, BpmGuide, DurationUp],
    &[
        Practice,
        FolderOpen,
        Option2Down,
        Nomine,
        NotesDisplayTimingUp,
    ],
    &[TargetUp, NextReplay],
    &[Replay, TargetDown],
];

static BEAT_14K_ASSIGN: [&[MusicSelectKey]; 18] = [
    &[Play, FolderOpen, Option1Down, JudgeWindowUp, BgaDown],
    &[FolderClose, Option1Up, Constant, GaugeAutoShiftDown],
    &[
        Practice,
        FolderOpen,
        GaugeDown,
        JudgeArea,
        NotesDisplayTimingAutoAdjust,
    ],
    &[FolderClose, OptiondpDown, LegacyNote, DurationDown],
    &[
        FolderOpen,
        Auto,
        HsfixDown,
        MarkNote,
        NotesDisplayTimingDown,
    ],
    &[NextReplay, Option2Up, BpmGuide, DurationUp],
    &[
        FolderOpen,
        Replay,
        Option2Down,
        Nomine,
        NotesDisplayTimingUp,
    ],
    &[Up, TargetUp],
    &[Down, TargetDown],
    &[Play, FolderOpen, Option1Down, JudgeWindowUp, BgaDown],
    &[FolderClose, Option1Up, Constant, GaugeAutoShiftDown],
    &[
        Practice,
        FolderOpen,
        GaugeDown,
        JudgeArea,
        NotesDisplayTimingAutoAdjust,
    ],
    &[FolderClose, OptiondpDown, LegacyNote, DurationDown],
    &[
        FolderOpen,
        Auto,
        HsfixDown,
        MarkNote,
        NotesDisplayTimingDown,
    ],
    &[NextReplay, Option2Up, BpmGuide, DurationUp],
    &[
        FolderOpen,
        Replay,
        Option2Down,
        Nomine,
        NotesDisplayTimingUp,
    ],
    &[Up, TargetUp],
    &[Down, TargetDown],
];
