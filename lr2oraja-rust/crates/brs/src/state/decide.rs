// MusicDecide state — ported from Java MusicDecide.java.
//
// Simplest full state: loads skin, runs timer-based sequence, transitions.

use tracing::info;

use bms_input::control_keys::ControlKeys;
use bms_skin::property_id::{TIMER_FADEOUT, TIMER_STARTINPUT};

use crate::app_state::AppStateType;
use crate::skin_manager::SkinType;
use crate::state::{GameStateHandler, StateContext};
use crate::system_sound::SystemSound;

/// Music decide state — brief interstitial between song selection and play.
pub struct MusicDecideState {
    cancel: bool,
}

impl MusicDecideState {
    pub fn new() -> Self {
        Self { cancel: false }
    }
}

impl Default for MusicDecideState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateHandler for MusicDecideState {
    fn create(&mut self, ctx: &mut StateContext) {
        self.cancel = false;
        info!("MusicDecide: create");
        if let Some(skin_mgr) = ctx.skin_manager.as_deref_mut() {
            skin_mgr.request_load(SkinType::Decide);
        }
        ctx.resource.org_gauge_option = ctx.player_config.gauge;
    }

    fn prepare(&mut self, ctx: &mut StateContext) {
        info!("MusicDecide: prepare");
        if let Some(sound_mgr) = ctx.sound_manager.as_deref_mut() {
            sound_mgr.play(SystemSound::Decide);
        }
    }

    fn render(&mut self, ctx: &mut StateContext) {
        let now = ctx.timer.now_time();
        let timing = ctx.skin_timing();

        // Enable input after initial delay (Java: getSkin().getInput())
        if now > timing.input_ms {
            ctx.timer.switch_timer(TIMER_STARTINPUT, true);
        }

        // Check fadeout -> transition (Java: getSkin().getFadeout())
        if ctx.timer.is_timer_on(TIMER_FADEOUT) {
            if ctx.timer.now_time_of(TIMER_FADEOUT) > timing.fadeout_ms {
                let next = if self.cancel {
                    AppStateType::MusicSelect
                } else {
                    AppStateType::Play
                };
                info!(next = %next, cancel = self.cancel, "MusicDecide: transition");
                *ctx.transition = Some(next);
            }
        } else if now > timing.scene_ms {
            info!("MusicDecide: scene timer expired, starting fadeout");
            ctx.timer.set_timer_on(TIMER_FADEOUT);
        }

        // Sync decide state to shared game state for skin rendering
        if let Some(shared) = &mut ctx.shared_state
            && let Some(model) = &ctx.resource.bms_model
        {
            super::decide_skin_state::sync_decide_state(shared, model);
        }
    }

    fn input(&mut self, ctx: &mut StateContext) {
        if ctx.timer.is_timer_on(TIMER_FADEOUT) || !ctx.timer.is_timer_on(TIMER_STARTINPUT) {
            return;
        }

        // Check real input from InputMapper
        if let Some(input_state) = ctx.input_state {
            for key in &input_state.pressed_keys {
                match key {
                    ControlKeys::Enter => {
                        self.do_confirm(ctx);
                        return;
                    }
                    ControlKeys::Escape => {
                        self.do_cancel(ctx);
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    fn shutdown(&mut self, _ctx: &mut StateContext) {
        info!("MusicDecide: shutdown");
    }
}

impl MusicDecideState {
    fn do_confirm(&mut self, ctx: &mut StateContext) {
        if !ctx.timer.is_timer_on(TIMER_FADEOUT) && ctx.timer.is_timer_on(TIMER_STARTINPUT) {
            ctx.timer.set_timer_on(TIMER_FADEOUT);
        }
    }

    fn do_cancel(&mut self, ctx: &mut StateContext) {
        if !ctx.timer.is_timer_on(TIMER_FADEOUT) && ctx.timer.is_timer_on(TIMER_STARTINPUT) {
            self.cancel = true;
            ctx.timer.set_timer_on(TIMER_FADEOUT);
        }
    }
}

/// Test helper: simulates confirm input (key press to proceed to Play).
#[cfg(test)]
impl MusicDecideState {
    pub(crate) fn confirm(&mut self, ctx: &mut StateContext) {
        self.do_confirm(ctx);
    }

    pub(crate) fn cancel(&mut self, ctx: &mut StateContext) {
        self.do_cancel(ctx);
    }

    pub(crate) fn is_cancel(&self) -> bool {
        self.cancel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_resource::PlayerResource;
    use crate::timer_manager::TimerManager;
    use bms_config::{Config, PlayerConfig};
    use bms_input::control_keys::ControlKeys;
    use bms_input::keyboard::VirtualKeyboardBackend;

    fn make_ctx<'a>(
        timer: &'a mut TimerManager,
        resource: &'a mut PlayerResource,
        config: &'a Config,
        player_config: &'a mut PlayerConfig,
        transition: &'a mut Option<AppStateType>,
    ) -> StateContext<'a> {
        StateContext {
            timer,
            resource,
            config,
            player_config,
            transition,
            keyboard_backend: None,
            database: None,
            input_state: None,
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
            download_handle: None,
        }
    }

    #[test]
    fn create_resets_cancel() {
        let mut state = MusicDecideState::new();
        state.cancel = true;

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.create(&mut ctx);
        assert!(!state.is_cancel());
    }

    #[test]
    fn render_enables_input_after_delay() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Before delay
        timer.set_now_micro_time(400_000); // 400ms
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert!(!timer.is_timer_on(TIMER_STARTINPUT));

        // After delay
        timer.set_now_micro_time(501_000); // 501ms
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert!(timer.is_timer_on(TIMER_STARTINPUT));
    }

    #[test]
    fn render_starts_fadeout_after_scene_duration() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(3_001_000); // 3001ms
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert!(timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn render_transitions_to_play_after_fadeout() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Set up: FADEOUT timer on at time 1000
        timer.set_now_micro_time(1_000_000);
        timer.set_timer_on(TIMER_FADEOUT);

        // Advance past fadeout duration
        timer.set_now_micro_time(1_501_000); // 501ms after FADEOUT
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(transition, Some(AppStateType::Play));
    }

    #[test]
    fn cancel_transitions_to_select() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Enable input and trigger cancel
        timer.set_now_micro_time(600_000);
        timer.switch_timer(TIMER_STARTINPUT, true);

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.cancel(&mut ctx);
        assert!(state.is_cancel());
        assert!(timer.is_timer_on(TIMER_FADEOUT));

        // Advance past fadeout
        timer.set_now_micro_time(1_200_000);
        transition = None;
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(transition, Some(AppStateType::MusicSelect));
    }

    #[test]
    fn confirm_starts_fadeout() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Enable input
        timer.set_now_micro_time(600_000);
        timer.switch_timer(TIMER_STARTINPUT, true);

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.confirm(&mut ctx);
        assert!(!state.is_cancel());
        assert!(timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn confirm_ignored_before_input_enabled() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Input not yet enabled
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.confirm(&mut ctx);
        assert!(!timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn confirm_ignored_during_fadeout() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Enable input and start fadeout
        timer.set_now_micro_time(600_000);
        timer.switch_timer(TIMER_STARTINPUT, true);
        timer.set_timer_on(TIMER_FADEOUT);
        let fadeout_time = timer.micro_timer(TIMER_FADEOUT);

        // Trying to confirm should not change anything
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.confirm(&mut ctx);
        assert_eq!(timer.micro_timer(TIMER_FADEOUT), fadeout_time);
    }

    #[test]
    fn input_enter_triggers_confirm() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Enable input
        timer.set_now_micro_time(600_000);
        timer.switch_timer(TIMER_STARTINPUT, true);

        let _backend = VirtualKeyboardBackend::new();
        let input_state = crate::input_mapper::InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Enter],
        };

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: Some(&input_state),
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
            download_handle: None,
        };
        state.input(&mut ctx);
        assert!(timer.is_timer_on(TIMER_FADEOUT));
        assert!(!state.is_cancel());
    }

    #[test]
    fn input_escape_triggers_cancel() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Enable input
        timer.set_now_micro_time(600_000);
        timer.switch_timer(TIMER_STARTINPUT, true);

        let input_state = crate::input_mapper::InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Escape],
        };

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: Some(&input_state),
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
            download_handle: None,
        };
        state.input(&mut ctx);
        assert!(timer.is_timer_on(TIMER_FADEOUT));
        assert!(state.is_cancel());
    }

    #[test]
    fn create_requests_skin_load() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;
        let mut skin_mgr = crate::skin_manager::SkinManager::new();

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: None,
            skin_manager: Some(&mut skin_mgr),
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
            download_handle: None,
        };
        state.create(&mut ctx);
        assert_eq!(skin_mgr.take_request(), Some(SkinType::Decide));
    }

    #[test]
    fn prepare_queues_decide_sound() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;
        let mut sound_mgr = crate::system_sound::SystemSoundManager::new();

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: None,
            skin_manager: None,
            sound_manager: Some(&mut sound_mgr),
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
            download_handle: None,
        };
        state.prepare(&mut ctx);
        let drained = sound_mgr.drain();
        assert!(drained.contains(&SystemSound::Decide));
    }

    #[test]
    fn create_without_skin_manager_does_not_panic() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        // skin_manager is None by default in make_ctx
        state.create(&mut ctx);
    }

    #[test]
    fn prepare_without_sound_manager_does_not_panic() {
        let mut state = MusicDecideState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        // sound_manager is None by default in make_ctx
        state.prepare(&mut ctx);
    }
}
