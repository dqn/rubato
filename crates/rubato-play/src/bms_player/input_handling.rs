use super::skin_context::PlayMouseContext;
use super::*;

impl BMSPlayer {
    pub(super) fn handle_skin_mouse_pressed_impl(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.main_state_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.main_state_data.timer);

        {
            let mut ctx = PlayMouseContext {
                timer: &mut timer,
                player: self,
            };
            skin.mouse_pressed_at(&mut ctx, button, x, y);
        }

        self.main_state_data.timer = timer;
        self.main_state_data.skin = Some(skin);
    }

    pub(super) fn handle_skin_mouse_dragged_impl(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.main_state_data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.main_state_data.timer);

        {
            let mut ctx = PlayMouseContext {
                timer: &mut timer,
                player: self,
            };
            skin.mouse_dragged_at(&mut ctx, button, x, y);
        }

        self.main_state_data.timer = timer;
        self.main_state_data.skin = Some(skin);
    }

    pub(super) fn input_impl(&mut self) {
        // Compute values before taking mutable borrows
        let is_note_end = self.is_note_end();
        let is_timer_play_on = self.main_state_data.timer.is_timer_on(TIMER_PLAY);
        // Use monotonic game timer instead of wall clock (SystemTime) to avoid
        // mixing clock domains. The control input only needs monotonically
        // increasing milliseconds for debounce/rate-limiting logic.
        let now_millis = self.main_state_data.timer.now_time();

        // Process control input (START+SELECT, lane cover, hispeed, etc.)
        if let (Some(mut control), Some(lanerender)) =
            (self.input.control.take(), self.lanerender.as_mut())
        {
            let pending_analog_resets = &mut self.input.pending_analog_resets;
            let input_analog_recent_ms = &mut self.input.input_analog_recent_ms;
            let input_analog_diff_ticks = &mut self.input.input_analog_diff_ticks;
            let mut analog_diff_and_reset = |key: usize, ms_tolerance: i32| -> i32 {
                if key >= input_analog_recent_ms.len() || key >= input_analog_diff_ticks.len() {
                    return 0;
                }
                let d_ticks = if input_analog_recent_ms[key] <= ms_tolerance as i64 {
                    0.max(input_analog_diff_ticks[key])
                } else {
                    0
                };
                input_analog_recent_ms[key] = i64::MAX;
                input_analog_diff_ticks[key] = 0;
                if !pending_analog_resets.contains(&key) {
                    pending_analog_resets.push(key);
                }
                d_ticks
            };
            let mut ctx = crate::control_input_processor::ControlInputContext {
                lanerender,
                start_pressed: self.input.input_start_pressed,
                select_pressed: self.input.input_select_pressed,
                control_key_up: self.input.control_key_up,
                control_key_down: self.input.control_key_down,
                control_key_escape_pressed: self.input.control_key_escape_pressed,
                control_key_num1: self.input.control_key_num1,
                control_key_num2: self.input.control_key_num2,
                control_key_num3: self.input.control_key_num3,
                control_key_num4: self.input.control_key_num4,
                key_states: &self.input.input_key_states,
                scroll: self.input.input_scroll,
                is_analog: &self.input.input_is_analog,
                analog_diff_and_reset: &mut analog_diff_and_reset,
                is_timer_play_on,
                is_note_end,
                window_hold: self.player_config.select_settings.is_window_hold,
                autoplay_mode: self.play_mode.mode,
                now_millis,
            };

            let result = control.input(&mut ctx);

            // Apply result actions
            if let Some(speed) = result.play_speed {
                self.set_play_speed(speed);
            }
            if result.clear_start {
                self.input.input_start_pressed = false;
            }
            if result.clear_select {
                self.input.input_select_pressed = false;
            }
            if result.reset_scroll {
                self.input.input_scroll = 0;
            }
            if result.stop_play {
                // Restore control before stopping (stop_play may need it)
                self.input.control = Some(control);
                self.stop_play();
            } else {
                self.input.control = Some(control);
            }
        }

        // Build InputContext for key input processing.
        let auto_presstime = self.judge.auto_presstime().to_vec();
        let now = self.main_state_data.timer.now_time();
        let is_autoplay = self.play_mode.mode == rubato_core::bms_player_mode::Mode::Autoplay;
        if let Some(ref mut keyinput) = self.input.keyinput {
            let mut ctx = crate::key_input_processor::InputContext {
                now,
                key_states: &self.input.input_key_states,
                auto_presstime: &auto_presstime,
                is_autoplay,
                timer: &mut self.main_state_data.timer,
            };
            keyinput.input(&mut ctx);
        }
    }

    pub(super) fn sync_input_from_impl(&mut self, input: &BMSPlayerInputProcessor) {
        self.input.input_start_pressed = input.start_pressed();
        self.input.input_select_pressed = input.is_select_pressed();
        self.input.input_key_states.clear();
        self.input
            .input_key_states
            .extend((0..KEYSTATE_SIZE as i32).map(|i| input.key_state(i)));
        self.input.input_key_changed_times.clear();
        self.input
            .input_key_changed_times
            .extend((0..KEYSTATE_SIZE as i32).map(|i| input.key_changed_time(i)));
        self.input.control_key_up = input.control_key_state(ControlKeys::Up);
        self.input.control_key_down = input.control_key_state(ControlKeys::Down);
        self.input.control_key_left = input.control_key_state(ControlKeys::Left);
        self.input.control_key_right = input.control_key_state(ControlKeys::Right);
        self.input.control_key_escape_pressed = input.control_key_state(ControlKeys::Escape);
        self.input.control_key_num1 = input.control_key_state(ControlKeys::Num1);
        self.input.control_key_num2 = input.control_key_state(ControlKeys::Num2);
        self.input.control_key_num3 = input.control_key_state(ControlKeys::Num3);
        self.input.control_key_num4 = input.control_key_state(ControlKeys::Num4);
        self.input.input_scroll = input.scroll();
        self.input.input_is_analog.clear();
        self.input
            .input_is_analog
            .extend((0..KEYSTATE_SIZE).map(|i| input.is_analog_input(i)));
        self.input.input_analog_diff_ticks.clear();
        self.input
            .input_analog_diff_ticks
            .extend((0..KEYSTATE_SIZE).map(|i| input.analog_diff(i)));
        self.input.input_analog_recent_ms.clear();
        self.input
            .input_analog_recent_ms
            .extend((0..KEYSTATE_SIZE).map(|i| input.time_since_last_analog_reset(i)));
        self.input.pending_analog_resets.clear();
        self.device_type = input.device_type();
    }

    pub(super) fn sync_input_back_to_impl(&mut self, input: &mut BMSPlayerInputProcessor) {
        if !self.input.input_start_pressed {
            input.start_changed(false);
        }
        if !self.input.input_select_pressed {
            input.select_pressed = false;
        }
        if self.input.input_scroll == 0 {
            input.reset_scroll();
        }
        for key in self.input.pending_analog_resets.drain(..) {
            input.reset_analog_input(key);
        }
        // Apply pending start time / margin time for key logging.
        // Java: input.setKeyLogMarginTime(resource.getMarginTime()) then
        //       input.setStartTime(micronow + timer.getStartMicroTime() - starttimeoffset * 1000)
        if let Some(margin) = self.input.pending_key_log_margin_time.take() {
            input.set_key_log_margin_time(margin);
        }
        if let Some(start) = self.input.pending_input_start_time.take() {
            input.set_start_time(start);
        }
    }

    pub(super) fn sync_audio_impl(
        &mut self,
        audio: &mut dyn rubato_audio::audio_driver::AudioDriver,
    ) {
        if self.pending.pending_stop_all_notes {
            self.pending.pending_stop_all_notes = false;
            audio.stop_note(None);
        }
        for cmd in self.drain_pending_bg_notes() {
            audio.play_note(&cmd.note, cmd.volume, 0);
        }
        // Gameplay lane keysound playback from JudgeManager events.
        // Corresponds to Java keysound.play(note, keyvolume, 0) calls.
        for (note, volume) in self.pending.pending_keysound_plays.drain(..) {
            audio.play_note(&note, volume, 0);
        }
        // Gameplay lane keysound volume changes from JudgeManager events.
        // Corresponds to Java keysound.setVolume(note, vol) calls.
        for (note, volume) in self.pending.pending_keysound_volume_sets.drain(..) {
            audio.set_volume_note(&note, volume);
        }
    }
}
