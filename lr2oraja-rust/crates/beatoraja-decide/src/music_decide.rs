// Translated from MusicDecide.java
// Music decide screen state.

use beatoraja_core::main_state::{MainStateData, MainStateType};
use beatoraja_core::system_sound_manager::SoundType;
use beatoraja_core::timer_manager::TimerManager;
use beatoraja_skin::skin_property::{TIMER_FADEOUT, TIMER_STARTINPUT};
use beatoraja_skin::skin_type::SkinType;

use crate::stubs::{ControlKeysStub, MainControllerRef, PlayerResourceAccess, SkinStub};

/// MusicDecide - music decide screen state
///
/// Translated from MusicDecide.java
/// In Java, MusicDecide extends MainState. In Rust, we use composition
/// with MainStateData and hold references to MainController and PlayerResource.
pub struct MusicDecide {
    pub data: MainStateData,
    pub main: MainControllerRef,
    pub resource: Box<dyn PlayerResourceAccess>,
    cancel: bool,
    skin: Option<SkinStub>,
}

impl MusicDecide {
    pub fn new(
        main: MainControllerRef,
        resource: Box<dyn PlayerResourceAccess>,
        timer: TimerManager,
    ) -> Self {
        Self {
            data: MainStateData::new(timer),
            main,
            resource,
            cancel: false,
            skin: None,
        }
    }

    pub fn create(&mut self) {
        self.cancel = false;

        // loadSkin(SkinType.DECIDE)
        self.skin = crate::stubs::load_skin(SkinType::Decide);

        // resource.setOrgGaugeOption(resource.getPlayerConfig().getGauge())
        let gauge = self.resource.get_player_config().gauge;
        self.resource.set_org_gauge_option(gauge);
    }

    pub fn prepare(&mut self) {
        // super.prepare() - default empty in MainState
        // play(DECIDE)
        crate::stubs::play_sound(SoundType::Decide);
    }

    pub fn render(&mut self) {
        let nowtime = self.data.timer.get_now_time();
        if let Some(ref skin) = self.skin
            && nowtime > skin.get_input() as i64
        {
            self.data.timer.switch_timer(TIMER_STARTINPUT, true);
        }
        if self.data.timer.is_timer_on(TIMER_FADEOUT) {
            if let Some(ref skin) = self.skin
                && self.data.timer.get_now_time_for_id(TIMER_FADEOUT) > skin.get_fadeout() as i64
            {
                self.main.change_state(if self.cancel {
                    MainStateType::MusicSelect
                } else {
                    MainStateType::Play
                });
            }
        } else if let Some(ref skin) = self.skin
            && nowtime > skin.get_scene() as i64
        {
            self.data.timer.set_timer_on(TIMER_FADEOUT);
        }
    }

    pub fn input(&mut self) {
        if !self.data.timer.is_timer_on(TIMER_FADEOUT)
            && self.data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            let input = self.main.get_input_processor();
            if input.get_key_state(0)
                || input.get_key_state(2)
                || input.get_key_state(4)
                || input.get_key_state(6)
                || input.is_control_key_pressed(ControlKeysStub::Enter)
            {
                self.data.timer.set_timer_on(TIMER_FADEOUT);
            }
            if input.is_control_key_pressed(ControlKeysStub::Escape)
                || (input.start_pressed() && input.is_select_pressed())
            {
                self.cancel = true;
                self.main.get_audio_processor().set_global_pitch(1f32);
                self.data.timer.set_timer_on(TIMER_FADEOUT);
            }
        }
    }

    pub fn dispose(&mut self) {
        // super.dispose()
        self.data.skin = None;
        self.data.stage = None;
    }
}
