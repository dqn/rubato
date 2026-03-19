use super::bar::bar::Bar;
use super::music_select_command::MusicSelectCommand;
use super::music_select_key_property::{MusicSelectKey, MusicSelectKeyProperty};
use super::*;

/// Music select input processor
/// Translates: bms.player.beatoraja.select.MusicSelectInputProcessor
pub struct MusicSelectInputProcessor {
    /// Bar movement counter
    pub duration: i64,
    /// Bar movement direction
    pub angle: i32,

    pub durationlow: i32,
    pub durationhigh: i32,

    /// Analog scroll buffer
    pub analog_scroll_buffer: i32,
    pub analog_ticks_per_scroll: i32,

    pub is_option_key_pressed: bool,
    pub is_option_key_released: bool,

    // Duration change counter for notes display timing
    pub time_change_duration: i64,
    pub count_change_duration: i32,
}

impl MusicSelectInputProcessor {
    pub fn new(durationlow: i32, durationhigh: i32, analog_ticks_per_scroll: i32) -> Self {
        Self {
            duration: 0,
            angle: 0,
            durationlow,
            durationhigh,
            analog_scroll_buffer: 0,
            analog_ticks_per_scroll,
            is_option_key_pressed: false,
            is_option_key_released: false,
            time_change_duration: 0,
            count_change_duration: 0,
        }
    }

    /// Process input.
    /// Translates: Java MusicSelectInputProcessor.input()
    ///
    /// In Java, this method holds a reference to MusicSelector (`select`).
    /// In Rust, we pass the needed context as a mutable reference.
    pub fn input(&mut self, ctx: &mut InputContext<'_>) {
        let input = &mut ctx.input;
        let config = &mut ctx.config;

        // NUM0: search popup
        // In Java, this opens a TextInputDialog for song search.
        // In Rust, the search UI is rendered by the launcher's egui overlay.
        // The search_text event is handled by MusicSelector::search().
        if input.is_control_key_pressed(ControlKeys::Num0) {
            ctx.events.push(InputEvent::SearchRequested);
        }

        // NUM1: KEY filter switch
        if input.is_control_key_pressed(ControlKeys::Num1) {
            ctx.events.push(InputEvent::ExecuteEvent(EventType::Mode));
        }

        // NUM2: sort switch
        if input.is_control_key_pressed(ControlKeys::Num2) {
            rubato_types::last_played_sort::force_disable();
            ctx.events.push(InputEvent::ExecuteEvent(EventType::Sort));
        }

        // NUM3: LN mode switch
        if input.is_control_key_pressed(ControlKeys::Num3) {
            ctx.events.push(InputEvent::ExecuteEvent(EventType::Lnmode));
        }

        let property = &MusicSelectKeyProperty::VALUES[config.select_settings.musicselectinput
            as usize
            % MusicSelectKeyProperty::VALUES.len()];

        if !input.start_pressed()
            && !input.is_select_pressed()
            && !input.control_key_state(ControlKeys::Num5)
        {
            // No option key input
            self.is_option_key_released = true;
            if self.is_option_key_pressed {
                self.is_option_key_pressed = false;
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionClose));
            }
        }

        // NUM4 or configured NEXT_REPLAY key (when no option key held): change replay
        if input.is_control_key_pressed(ControlKeys::Num4)
            || (!input.start_pressed()
                && !input.is_select_pressed()
                && !input.control_key_state(ControlKeys::Num5)
                && property.is_pressed(input, MusicSelectKey::NextReplay, true))
        {
            ctx.events
                .push(InputEvent::Execute(MusicSelectCommand::NextReplay));
        }

        if input.start_pressed() && !input.is_select_pressed() {
            // START pressed: show play option panel
            ctx.bar_renderer_reset_input = true;
            ctx.panel_state = Some(1);

            if self.is_option_key_released {
                self.is_option_key_pressed = true;
                self.is_option_key_released = false;
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionOpen));
            }

            if property.is_pressed(input, MusicSelectKey::Option1Down, true) {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Option1p, 1));
            }
            if property.is_pressed(input, MusicSelectKey::Option1Up, true) {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Option1p, -1));
            }
            if property.is_pressed(input, MusicSelectKey::GaugeDown, true) {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Gauge1p, 1));
            }
            if property.is_pressed(input, MusicSelectKey::GaugeUp, true) {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Gauge1p, -1));
            }
            if property.is_pressed(input, MusicSelectKey::OptiondpDown, true) {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Optiondp, 1));
            }
            if property.is_pressed(input, MusicSelectKey::OptiondpUp, true) {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Optiondp, -1));
            }
            if property.is_pressed(input, MusicSelectKey::Option2Down, true) {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Option2p, 1));
            }
            if property.is_pressed(input, MusicSelectKey::Option2Up, true) {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Option2p, -1));
            }
            if property.is_pressed(input, MusicSelectKey::HsfixDown, true) {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Hsfix, 1));
            }
            if property.is_pressed(input, MusicSelectKey::HsfixUp, true) {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Hsfix, -1));
            }

            // Mouse wheel scroll for target
            let mut mov = -(input.scroll());
            input.reset_scroll();

            self.analog_scroll_buffer += property.analog_change(input, MusicSelectKey::TargetUp)
                - property.analog_change(input, MusicSelectKey::TargetDown);
            mov += self.analog_scroll_buffer / self.analog_ticks_per_scroll;
            self.analog_scroll_buffer %= self.analog_ticks_per_scroll;

            // Target scroll via keys
            let l = now_millis();
            if property.is_non_analog_pressed(input, MusicSelectKey::TargetUp, false)
                || input.control_key_state(ControlKeys::Down)
            {
                if self.duration == 0 {
                    mov = 1;
                    self.duration = l + self.durationlow as i64;
                    self.angle = self.durationlow;
                }
                if l > self.duration {
                    self.duration = l + self.durationhigh as i64;
                    mov = 1;
                    self.angle = self.durationhigh;
                }
            } else if property.is_non_analog_pressed(input, MusicSelectKey::TargetDown, false)
                || input.control_key_state(ControlKeys::Up)
            {
                if self.duration == 0 {
                    mov = -1;
                    self.duration = l + self.durationlow as i64;
                    self.angle = -self.durationlow;
                }
                if l > self.duration {
                    self.duration = l + self.durationhigh as i64;
                    mov = -1;
                    self.angle = -self.durationhigh;
                }
            } else {
                let l = now_millis();
                if l > self.duration {
                    self.duration = 0;
                }
            }

            while mov > 0 {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Target, -1));
                ctx.events.push(InputEvent::StopSound(SoundType::Scratch));
                ctx.events.push(InputEvent::PlaySound(SoundType::Scratch));
                mov -= 1;
            }
            while mov < 0 {
                ctx.events
                    .push(InputEvent::ExecuteEventArg(EventType::Target, 1));
                ctx.events.push(InputEvent::StopSound(SoundType::Scratch));
                ctx.events.push(InputEvent::PlaySound(SoundType::Scratch));
                mov += 1;
            }
        } else if input.is_select_pressed() && !input.start_pressed() {
            // SELECT pressed: show assist option panel
            ctx.bar_renderer_reset_input = true;
            ctx.panel_state = Some(2);

            if self.is_option_key_released {
                self.is_option_key_pressed = true;
                self.is_option_key_released = false;
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionOpen));
            }

            if property.is_pressed(input, MusicSelectKey::JudgeWindowUp, true) {
                config.judge_settings.custom_judge = !config.judge_settings.custom_judge;
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionChange));
            }
            if property.is_pressed(input, MusicSelectKey::Constant, true) {
                config.display_settings.scroll_mode = if config.display_settings.scroll_mode == 1 {
                    0
                } else {
                    1
                };
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionChange));
            }
            if property.is_pressed(input, MusicSelectKey::JudgeArea, true) {
                config.display_settings.showjudgearea = !config.display_settings.showjudgearea;
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionChange));
            }
            if property.is_pressed(input, MusicSelectKey::LegacyNote, true) {
                config.note_modifier_settings.longnote_mode =
                    if config.note_modifier_settings.longnote_mode == 1 {
                        0
                    } else {
                        1
                    };
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionChange));
            }
            if property.is_pressed(input, MusicSelectKey::MarkNote, true) {
                config.display_settings.markprocessednote =
                    !config.display_settings.markprocessednote;
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionChange));
            }
            if property.is_pressed(input, MusicSelectKey::BpmGuide, true) {
                config.display_settings.bpmguide = !config.display_settings.bpmguide;
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionChange));
            }
            if property.is_pressed(input, MusicSelectKey::Nomine, true) {
                config.play_settings.mine_mode = if config.play_settings.mine_mode == 1 {
                    0
                } else {
                    1
                };
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionChange));
            }
        } else if input.control_key_state(ControlKeys::Num5)
            || (input.start_pressed() && input.is_select_pressed())
        {
            // START+SELECT or NUM5: show detail option panel
            ctx.bar_renderer_reset_input = true;
            ctx.panel_state = Some(3);

            if self.is_option_key_released {
                self.is_option_key_pressed = true;
                self.is_option_key_released = false;
                ctx.events
                    .push(InputEvent::PlaySound(SoundType::OptionOpen));
            }

            if property.is_pressed(input, MusicSelectKey::BgaDown, true) {
                ctx.events.push(InputEvent::ExecuteEvent(EventType::Bga));
            }
            if property.is_pressed(input, MusicSelectKey::GaugeAutoShiftDown, true) {
                ctx.events
                    .push(InputEvent::ExecuteEvent(EventType::GaugeAutoShift));
            }
            if property.is_pressed(input, MusicSelectKey::NotesDisplayTimingDown, true) {
                ctx.events.push(InputEvent::ExecuteEventArg(
                    EventType::NotesDisplayTiming,
                    -1,
                ));
            }
            if property.is_pressed(input, MusicSelectKey::DurationDown, false) {
                let l = now_millis();
                if self.time_change_duration == 0 {
                    self.time_change_duration = l + self.durationlow as i64;
                    ctx.events
                        .push(InputEvent::ExecuteEventArg(EventType::Duration1p, -1));
                } else if l > self.time_change_duration {
                    self.count_change_duration += 1;
                    self.time_change_duration = l + self.durationhigh as i64;
                    ctx.events.push(InputEvent::ExecuteEventArgs(
                        EventType::Duration1p,
                        -1,
                        if self.count_change_duration > 50 {
                            10
                        } else {
                            0
                        },
                    ));
                }
            } else if property.is_pressed(input, MusicSelectKey::DurationUp, false) {
                let l = now_millis();
                if self.time_change_duration == 0 {
                    self.time_change_duration = l + self.durationlow as i64;
                    ctx.events
                        .push(InputEvent::ExecuteEventArg(EventType::Duration1p, 1));
                } else if l > self.time_change_duration {
                    self.count_change_duration += 1;
                    self.time_change_duration = l + self.durationhigh as i64;
                    ctx.events.push(InputEvent::ExecuteEventArgs(
                        EventType::Duration1p,
                        1,
                        if self.count_change_duration > 50 {
                            10
                        } else {
                            0
                        },
                    ));
                }
            } else {
                self.time_change_duration = 0;
                self.count_change_duration = 0;
            }
            if property.is_pressed(input, MusicSelectKey::NotesDisplayTimingUp, true) {
                ctx.events
                    .push(InputEvent::ExecuteEvent(EventType::NotesDisplayTiming));
            }
            if property.is_pressed(input, MusicSelectKey::NotesDisplayTimingAutoAdjust, true) {
                ctx.events.push(InputEvent::ExecuteEvent(
                    EventType::NotesDisplayTimingAutoAdjust,
                ));
            }
        } else {
            // No option keys: normal bar input mode
            ctx.bar_renderer_do_input = true;
            ctx.panel_state = Some(0);

            // Determine current bar type for dispatch
            let current_bar_type = ctx.selected_bar_type;

            if current_bar_type == BarType::Function
                && (property.is_pressed(input, MusicSelectKey::Practice, true)
                    || property.is_pressed(input, MusicSelectKey::Auto, true)
                    || property.is_pressed(input, MusicSelectKey::Replay, true))
            {
                ctx.events.push(InputEvent::SelectSong(BMSPlayerMode::PLAY));
            } else if matches!(
                current_bar_type,
                BarType::Song | BarType::Table | BarType::Hash
            ) && (property.is_pressed(input, MusicSelectKey::Practice, true)
                || property.is_pressed(input, MusicSelectKey::Auto, true))
            {
                ctx.events
                    .push(InputEvent::Execute(MusicSelectCommand::ShowContextMenu));
            } else if matches!(
                current_bar_type,
                BarType::Selectable | BarType::Song | BarType::Table | BarType::Hash
            ) {
                if property.is_pressed(input, MusicSelectKey::Play, true)
                    || input.is_control_key_pressed(ControlKeys::Right)
                    || input.is_control_key_pressed(ControlKeys::Enter)
                {
                    ctx.events.push(InputEvent::SelectSong(BMSPlayerMode::PLAY));
                } else if property.is_pressed(input, MusicSelectKey::Practice, true) {
                    let mode = if config.select_settings.event_mode {
                        BMSPlayerMode::PLAY
                    } else {
                        BMSPlayerMode::PRACTICE
                    };
                    ctx.events.push(InputEvent::SelectSong(mode));
                } else if property.is_pressed(input, MusicSelectKey::Auto, true) {
                    let mode = if config.select_settings.event_mode {
                        BMSPlayerMode::PLAY
                    } else {
                        BMSPlayerMode::AUTOPLAY
                    };
                    ctx.events.push(InputEvent::SelectSong(mode));
                } else if property.is_pressed(input, MusicSelectKey::Replay, true) {
                    let mode = if config.select_settings.event_mode {
                        BMSPlayerMode::PLAY
                    } else if ctx.selected_replay >= 0 {
                        BMSPlayerMode::replay_mode(ctx.selected_replay)
                            .cloned()
                            .unwrap_or(BMSPlayerMode::PLAY)
                    } else {
                        BMSPlayerMode::PLAY
                    };
                    ctx.events.push(InputEvent::SelectSong(mode));
                } else if property.is_pressed(input, MusicSelectKey::NextReplay, true) {
                    if current_bar_type == BarType::Function {
                        input.reset_key_changed_time(1);
                        ctx.events.push(InputEvent::BarManagerClose);
                    } else {
                        ctx.events
                            .push(InputEvent::Execute(MusicSelectCommand::NextReplay));
                    }
                }
            } else if current_bar_type == BarType::Directory
                && (property.is_pressed(input, MusicSelectKey::FolderOpen, true)
                    || input.is_control_key_pressed(ControlKeys::Right)
                    || input.is_control_key_pressed(ControlKeys::Enter))
            {
                ctx.events.push(InputEvent::OpenDirectory);
            }

            // NUM7: rival switch
            if input.is_control_key_pressed(ControlKeys::Num7) {
                ctx.events.push(InputEvent::ExecuteEvent(EventType::Rival));
            }
            // NUM8: show songs on same folder
            if input.is_control_key_pressed(ControlKeys::Num8) {
                ctx.events.push(InputEvent::Execute(
                    MusicSelectCommand::ShowSongsOnSameFolder,
                ));
            }
            // NUM9: open document
            if input.is_control_key_pressed(ControlKeys::Num9) {
                ctx.events
                    .push(InputEvent::ExecuteEvent(EventType::OpenDocument));
            }
            // Close folder
            if property.is_pressed(input, MusicSelectKey::FolderClose, true)
                || input.is_control_key_pressed(ControlKeys::Left)
            {
                input.reset_key_changed_time(1);
                ctx.events.push(InputEvent::BarManagerClose);
            }

            // KeyCommand bindings
            if input.is_activated(KeyCommand::AutoplayFolder)
                && current_bar_type == BarType::Directory
            {
                ctx.events
                    .push(InputEvent::SelectSong(BMSPlayerMode::AUTOPLAY));
            }
            if input.is_activated(KeyCommand::OpenIr) {
                ctx.events.push(InputEvent::ExecuteEvent(EventType::OpenIr));
            }
            if input.is_activated(KeyCommand::AddFavoriteSong) {
                ctx.events
                    .push(InputEvent::ExecuteEvent(EventType::FavoriteSong));
            }
            if input.is_activated(KeyCommand::AddFavoriteChart) {
                ctx.events
                    .push(InputEvent::ExecuteEvent(EventType::FavoriteChart));
            }
        }

        // songbar change timer (always active, outside conditional blocks)
        ctx.songbar_timer_switch = true;

        // Update folder (KeyCommand)
        if input.is_activated(KeyCommand::UpdateFolder) {
            ctx.events
                .push(InputEvent::ExecuteEvent(EventType::UpdateFolder));
        }
        // Open explorer
        if input.is_activated(KeyCommand::OpenExplorer) {
            ctx.events
                .push(InputEvent::ExecuteEvent(EventType::OpenWithExplorer));
        }
        // Copy MD5 hash
        if input.is_activated(KeyCommand::CopySongMd5Hash) {
            ctx.events
                .push(InputEvent::Execute(MusicSelectCommand::CopyMd5Hash));
        }
        // Copy SHA256 hash
        if input.is_activated(KeyCommand::CopySongSha256Hash) {
            ctx.events
                .push(InputEvent::Execute(MusicSelectCommand::CopySha256Hash));
        }
        // Copy highlighted menu text
        if input.is_activated(KeyCommand::CopyHighlightedMenuText) {
            ctx.events.push(InputEvent::Execute(
                MusicSelectCommand::CopyHighlightedMenuText,
            ));
        }

        // ESCAPE: close folder or exit
        if input.is_control_key_pressed(ControlKeys::Escape) {
            if ctx.is_top_level {
                ctx.events.push(InputEvent::Exit);
            } else {
                ctx.events.push(InputEvent::BarManagerClose);
            }
        }
    }
}

/// Simplified bar type classification for input dispatch.
/// Avoids holding borrows on MusicSelector during input processing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarType {
    Song,
    Table,
    Hash,
    Function,
    Selectable,
    Directory,
    Other,
    None,
}

impl BarType {
    pub fn classify(bar: Option<&Bar>) -> Self {
        match bar {
            None => BarType::None,
            Some(bar) => {
                if bar.as_function_bar().is_some() {
                    BarType::Function
                } else if bar.as_song_bar().is_some() {
                    BarType::Song
                } else if bar.as_table_bar().is_some() {
                    BarType::Table
                } else if bar.as_hash_bar().is_some() {
                    BarType::Hash
                } else if bar.as_selectable_bar().is_some() {
                    BarType::Selectable
                } else if bar.is_directory_bar() {
                    BarType::Directory
                } else {
                    BarType::Other
                }
            }
        }
    }
}

/// Input events produced by MusicSelectInputProcessor.
/// These are collected during input processing and dispatched by MusicSelector
/// after the borrow on musicinput is released.
#[derive(Clone, Debug)]
pub enum InputEvent {
    Execute(MusicSelectCommand),
    ExecuteEvent(EventType),
    ExecuteEventArg(EventType, i32),
    ExecuteEventArgs(EventType, i32, i32),
    PlaySound(SoundType),
    StopSound(SoundType),
    SelectSong(BMSPlayerMode),
    BarManagerClose,
    OpenDirectory,
    Exit,
    ChangeState(MainStateType),
    SearchRequested,
}

/// Context passed to MusicSelectInputProcessor.input().
/// Contains the needed state extracted from MusicSelector before calling input().
pub struct InputContext<'a> {
    pub input: &'a mut BMSPlayerInputProcessor,
    pub config: &'a mut PlayerConfig,
    pub selected_bar_type: BarType,
    pub selected_replay: i32,
    pub is_top_level: bool,

    // Output fields — set by input processing
    pub events: Vec<InputEvent>,
    pub panel_state: Option<i32>,
    pub bar_renderer_reset_input: bool,
    pub bar_renderer_do_input: bool,
    pub songbar_timer_switch: bool,
}

impl<'a> InputContext<'a> {
    pub fn new(
        input: &'a mut BMSPlayerInputProcessor,
        config: &'a mut PlayerConfig,
        selected_bar_type: BarType,
        selected_replay: i32,
        is_top_level: bool,
    ) -> Self {
        Self {
            input,
            config,
            selected_bar_type,
            selected_replay,
            is_top_level,
            events: Vec::new(),
            panel_state: None,
            bar_renderer_reset_input: false,
            bar_renderer_do_input: false,
            songbar_timer_switch: false,
        }
    }
}

fn now_millis() -> i64 {
    static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let epoch = EPOCH.get_or_init(std::time::Instant::now);
    epoch.elapsed().as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_type_classify_none() {
        assert_eq!(BarType::classify(None), BarType::None);
    }

    #[test]
    fn test_bar_type_classify_song() {
        let song_data = SongData::default();
        let bar = Bar::Song(Box::new(crate::select::bar::song_bar::SongBar::new(
            song_data,
        )));
        assert_eq!(BarType::classify(Some(&bar)), BarType::Song);
    }

    #[test]
    fn test_bar_type_classify_directory() {
        let bar = Bar::Folder(Box::new(crate::select::bar::folder_bar::FolderBar::new(
            None,
            "crc".to_string(),
        )));
        assert_eq!(BarType::classify(Some(&bar)), BarType::Directory);
    }

    #[test]
    fn test_bar_type_classify_function() {
        let bar = Bar::Function(Box::new(
            crate::select::bar::function_bar::FunctionBar::new("test".to_string(), 0),
        ));
        assert_eq!(BarType::classify(Some(&bar)), BarType::Function);
    }

    #[test]
    fn test_bar_type_classify_grade() {
        use rubato_types::course_data::CourseData;
        let bar = Bar::Grade(Box::new(crate::select::bar::grade_bar::GradeBar::new(
            CourseData::default(),
        )));
        // GradeBar has selectable data, so it's Selectable (not a pure song/table/hash/function)
        assert_eq!(BarType::classify(Some(&bar)), BarType::Selectable);
    }

    #[test]
    fn test_input_event_variants() {
        // Verify event construction
        let _e1 = InputEvent::Execute(MusicSelectCommand::ResetReplay);
        let _e2 = InputEvent::ExecuteEvent(EventType::Mode);
        let _e3 = InputEvent::ExecuteEventArg(EventType::Target, 1);
        let _e4 = InputEvent::ExecuteEventArgs(EventType::Duration1p, 1, 10);
        let _e5 = InputEvent::PlaySound(SoundType::FolderOpen);
        let _e6 = InputEvent::SelectSong(BMSPlayerMode::PLAY);
        let _e7 = InputEvent::BarManagerClose;
        let _e8 = InputEvent::OpenDirectory;
        let _e9 = InputEvent::Exit;
    }

    #[test]
    fn test_new_defaults() {
        let proc = MusicSelectInputProcessor::new(300, 50, 10);
        assert_eq!(proc.duration, 0);
        assert_eq!(proc.angle, 0);
        assert_eq!(proc.durationlow, 300);
        assert_eq!(proc.durationhigh, 50);
        assert_eq!(proc.analog_scroll_buffer, 0);
        assert_eq!(proc.analog_ticks_per_scroll, 10);
        assert!(!proc.is_option_key_pressed);
        assert!(!proc.is_option_key_released);
        assert_eq!(proc.time_change_duration, 0);
        assert_eq!(proc.count_change_duration, 0);
    }
}
