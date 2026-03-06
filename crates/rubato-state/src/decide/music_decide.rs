// Translated from MusicDecide.java
// Music decide screen state.

use rubato_core::main_state::{MainState, MainStateData, MainStateType};
use rubato_core::system_sound_manager::SoundType;
use rubato_core::timer_manager::TimerManager;
use rubato_skin::skin_property::{TIMER_FADEOUT, TIMER_STARTINPUT};
use rubato_skin::skin_type::SkinType;

use super::stubs::{ControlKeys, MainControllerRef, NullPlayerResource, PlayerResourceAccess};

/// Render context adapter for decide screen skin rendering.
/// Provides config access through SkinRenderContext.
struct DecideRenderContext<'a> {
    timer: &'a mut TimerManager,
    resource: &'a dyn PlayerResourceAccess,
    main: &'a MainControllerRef,
}

impl rubato_types::timer_access::TimerAccess for DecideRenderContext<'_> {
    fn get_now_time(&self) -> i64 {
        self.timer.get_now_time()
    }
    fn get_now_micro_time(&self) -> i64 {
        self.timer.get_now_micro_time()
    }
    fn get_micro_timer(&self, timer_id: i32) -> i64 {
        self.timer.get_micro_timer(timer_id)
    }
    fn get_timer(&self, timer_id: i32) -> i64 {
        self.timer.get_timer(timer_id)
    }
    fn get_now_time_for(&self, timer_id: i32) -> i64 {
        self.timer.get_now_time_for_id(timer_id)
    }
    fn is_timer_on(&self, timer_id: i32) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for DecideRenderContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Decide)
    }

    fn get_player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(self.main.get_player_config())
    }

    fn get_config_ref(&self) -> Option<&rubato_types::config::Config> {
        Some(self.main.get_config())
    }

    fn set_timer_micro(&mut self, timer_id: i32, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn string_value(&self, id: i32) -> String {
        match id {
            10 => self
                .resource
                .get_songdata()
                .map_or_else(String::new, |s| s.title.clone()),
            11 => self
                .resource
                .get_songdata()
                .map_or_else(String::new, |s| s.subtitle.clone()),
            14 => self
                .resource
                .get_songdata()
                .map_or_else(String::new, |s| s.artist.clone()),
            _ => String::new(),
        }
    }

    fn integer_value(&self, id: i32) -> i32 {
        use rubato_types::timer_access::TimerAccess;
        match id {
            // Song BPM from songdata
            90 => self.resource.get_songdata().map_or(0, |s| s.maxbpm),
            91 => self.resource.get_songdata().map_or(0, |s| s.minbpm),
            // Total notes
            350 => self.resource.get_songdata().map_or(0, |s| s.notes),
            // Song duration
            312 => self.resource.get_songdata().map_or(0, |s| s.length),
            // Playtime
            17 => (self.timer.get_now_time() / 3_600_000) as i32,
            18 => ((self.timer.get_now_time() % 3_600_000) / 60_000) as i32,
            19 => ((self.timer.get_now_time() % 60_000) / 1_000) as i32,
            _ => 0,
        }
    }
}

struct DecideMouseContext<'a> {
    timer: &'a mut TimerManager,
    main: &'a mut MainControllerRef,
}

impl rubato_types::timer_access::TimerAccess for DecideMouseContext<'_> {
    fn get_now_time(&self) -> i64 {
        self.timer.get_now_time()
    }

    fn get_now_micro_time(&self) -> i64 {
        self.timer.get_now_micro_time()
    }

    fn get_micro_timer(&self, timer_id: i32) -> i64 {
        self.timer.get_micro_timer(timer_id)
    }

    fn get_timer(&self, timer_id: i32) -> i64 {
        self.timer.get_timer(timer_id)
    }

    fn get_now_time_for(&self, timer_id: i32) -> i64 {
        self.timer.get_now_time_for_id(timer_id)
    }

    fn is_timer_on(&self, timer_id: i32) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for DecideMouseContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Decide)
    }

    fn change_state(&mut self, state: rubato_types::main_state_type::MainStateType) {
        self.main.change_state(state);
    }

    fn set_timer_micro(&mut self, timer_id: i32, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }
}

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
        }
    }
}

impl MainState for MusicDecide {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::Decide)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.data
    }

    fn create(&mut self) {
        self.cancel = false;

        // loadSkin(SkinType.DECIDE)
        self.load_skin(SkinType::Decide.id());

        // resource.setOrgGaugeOption(resource.getPlayerConfig().getGauge())
        let gauge = self.resource.get_player_config().gauge;
        self.resource.set_org_gauge_option(gauge);
    }

    fn prepare(&mut self) {
        // super.prepare() - default empty in MainState
        // play(DECIDE)
        self.main.play_sound(&SoundType::Decide, false);
    }

    fn render_skin(&mut self, sprite: &mut rubato_render::sprite_batch::SpriteBatch) {
        let mut skin = match self.data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.data.timer);

        {
            let mut ctx = DecideRenderContext {
                timer: &mut timer,
                resource: &*self.resource,
                main: &self.main,
            };
            skin.update_custom_objects_timed(&mut ctx);
            skin.swap_sprite_batch(sprite);
            skin.draw_all_objects_timed(&mut ctx);
            skin.swap_sprite_batch(sprite);
        }

        self.data.timer = timer;
        self.data.skin = Some(skin);
    }

    fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.data.timer);

        {
            let mut ctx = DecideMouseContext {
                timer: &mut timer,
                main: &mut self.main,
            };
            skin.mouse_pressed_at(&mut ctx, button, x, y);
        }

        self.data.timer = timer;
        self.data.skin = Some(skin);
    }

    fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        let mut skin = match self.data.skin.take() {
            Some(s) => s,
            None => return,
        };
        let mut timer = std::mem::take(&mut self.data.timer);

        {
            let mut ctx = DecideMouseContext {
                timer: &mut timer,
                main: &mut self.main,
            };
            skin.mouse_dragged_at(&mut ctx, button, x, y);
        }

        self.data.timer = timer;
        self.data.skin = Some(skin);
    }

    fn render(&mut self) {
        let nowtime = self.data.timer.get_now_time();
        // Skin timing values; fall back to 0 when no skin is loaded so the
        // decide screen still transitions to Play instead of stalling forever.
        let input_time = self.data.skin.as_ref().map_or(0, |s| s.get_input() as i64);
        let fadeout_time = self
            .data
            .skin
            .as_ref()
            .map_or(0, |s| s.get_fadeout() as i64);
        let scene_time = self.data.skin.as_ref().map_or(0, |s| s.get_scene() as i64);

        if nowtime > input_time {
            self.data.timer.switch_timer(TIMER_STARTINPUT, true);
        }
        if self.data.timer.is_timer_on(TIMER_FADEOUT) {
            if self.data.timer.get_now_time_for_id(TIMER_FADEOUT) > fadeout_time {
                self.main.change_state(if self.cancel {
                    MainStateType::MusicSelect
                } else {
                    MainStateType::Play
                });
            }
        } else if nowtime > scene_time {
            self.data.timer.set_timer_on(TIMER_FADEOUT);
        }
    }

    fn input(&mut self) {
        if !self.data.timer.is_timer_on(TIMER_FADEOUT)
            && self.data.timer.is_timer_on(TIMER_STARTINPUT)
        {
            // Collect input state first, then release &mut borrow on self.main
            // before calling get_audio_processor_mut (avoids overlapping &mut borrows).
            let (decide, cancel) = {
                let input = self.main.get_input_processor();
                let decide = input.get_key_state(0)
                    || input.get_key_state(2)
                    || input.get_key_state(4)
                    || input.get_key_state(6)
                    || input.is_control_key_pressed(ControlKeys::Enter);
                let cancel = input.is_control_key_pressed(ControlKeys::Escape)
                    || (input.start_pressed() && input.is_select_pressed());
                (decide, cancel)
            };
            if decide {
                self.data.timer.set_timer_on(TIMER_FADEOUT);
            }
            if cancel {
                self.cancel = true;
                if let Some(audio) = self.main.get_audio_processor_mut() {
                    audio.set_global_pitch(1f32);
                }
                self.data.timer.set_timer_on(TIMER_FADEOUT);
            }
        }
    }

    fn sync_input_from(
        &mut self,
        input: &rubato_input::bms_player_input_processor::BMSPlayerInputProcessor,
    ) {
        self.main.sync_input_from(input);
    }

    fn sync_input_back_to(
        &mut self,
        input: &mut rubato_input::bms_player_input_processor::BMSPlayerInputProcessor,
    ) {
        self.main.sync_input_back_to(input);
    }

    fn load_skin(&mut self, skin_type: i32) {
        self.data.skin = rubato_skin::skin_loader::load_skin_from_config(
            self.main.get_config(),
            self.main.get_player_config(),
            skin_type,
        )
        .map(|skin| Box::new(skin) as Box<dyn rubato_core::main_state::SkinDrawable>);
    }

    fn dispose(&mut self) {
        // super.dispose()
        self.data.skin = None;
        self.data.stage = None;
    }

    fn take_player_resource_box(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        let null: Box<dyn PlayerResourceAccess> = Box::new(NullPlayerResource::new());
        let old = std::mem::replace(&mut self.resource, null);
        Some(old.into_any_send())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decide::stubs::{NullMainController, NullPlayerResource};
    use rubato_core::main_state::SkinDrawable;
    use rubato_core::sprite_batch_helper::SpriteBatch;
    use rubato_types::main_controller_access::MainControllerAccess;
    use rubato_types::timer_access::TimerAccess;
    use std::sync::{Arc, Mutex};

    /// Mock SkinDrawable for testing render logic with configurable timing values.
    struct MockSkin {
        input: i32,
        scene: i32,
        fadeout: i32,
    }

    impl MockSkin {
        fn new() -> Self {
            Self {
                input: 0,
                scene: 0,
                fadeout: 0,
            }
        }

        fn with_values(input: i32, scene: i32, fadeout: i32) -> Self {
            Self {
                input,
                scene,
                fadeout,
            }
        }
    }

    impl SkinDrawable for MockSkin {
        fn draw_all_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }
        fn update_custom_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }
        fn mouse_pressed_at(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }
        fn mouse_dragged_at(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }
        fn prepare_skin(&mut self) {}
        fn dispose_skin(&mut self) {}
        fn get_fadeout(&self) -> i32 {
            self.fadeout
        }
        fn get_input(&self) -> i32 {
            self.input
        }
        fn get_scene(&self) -> i32 {
            self.scene
        }
        fn get_width(&self) -> f32 {
            0.0
        }
        fn get_height(&self) -> f32 {
            0.0
        }
        fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
    }

    struct ChangeStateSkin {
        state: MainStateType,
    }

    impl SkinDrawable for ChangeStateSkin {
        fn draw_all_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }

        fn update_custom_objects_timed(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        ) {
        }

        fn mouse_pressed_at(
            &mut self,
            ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
            ctx.change_state(self.state);
        }

        fn mouse_dragged_at(
            &mut self,
            _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }

        fn prepare_skin(&mut self) {}

        fn dispose_skin(&mut self) {}

        fn get_fadeout(&self) -> i32 {
            0
        }

        fn get_input(&self) -> i32 {
            0
        }

        fn get_scene(&self) -> i32 {
            0
        }

        fn get_width(&self) -> f32 {
            0.0
        }

        fn get_height(&self) -> f32 {
            0.0
        }

        fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
    }

    struct RecordingMainController {
        changed_states: Arc<Mutex<Vec<MainStateType>>>,
        config: rubato_types::config::Config,
        player_config: rubato_types::player_config::PlayerConfig,
    }

    impl RecordingMainController {
        fn new(changed_states: Arc<Mutex<Vec<MainStateType>>>) -> Self {
            Self {
                changed_states,
                config: rubato_types::config::Config::default(),
                player_config: rubato_types::player_config::PlayerConfig::default(),
            }
        }
    }

    impl MainControllerAccess for RecordingMainController {
        fn get_config(&self) -> &rubato_types::config::Config {
            &self.config
        }

        fn get_player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }

        fn change_state(&mut self, state: MainStateType) {
            self.changed_states.lock().unwrap().push(state);
        }

        fn save_config(&self) {}

        fn exit(&self) {}

        fn save_last_recording(&self, _reason: &str) {}

        fn update_song(&mut self, _path: Option<&str>) {}

        fn get_player_resource(
            &self,
        ) -> Option<&dyn rubato_types::player_resource_access::PlayerResourceAccess> {
            None
        }

        fn get_player_resource_mut(
            &mut self,
        ) -> Option<&mut dyn rubato_types::player_resource_access::PlayerResourceAccess> {
            None
        }
    }

    fn make_decide() -> MusicDecide {
        MusicDecide::new(
            MainControllerRef::new(Box::new(NullMainController)),
            Box::new(NullPlayerResource::new()),
            TimerManager::new(),
        )
    }

    #[test]
    fn test_state_type() {
        let decide = make_decide();
        assert_eq!(decide.state_type(), Some(MainStateType::Decide));
    }

    #[test]
    fn test_create_resets_cancel() {
        let mut decide = make_decide();
        decide.cancel = true;
        decide.create();
        assert!(!decide.cancel);
    }

    #[test]
    fn test_create_calls_load_skin_with_decide_type() {
        let mut decide = make_decide();
        decide.create();
        assert_eq!(SkinType::Decide.id(), 6);
        assert!(
            decide.data.skin.is_some(),
            "decide create() should load the configured decide skin"
        );
    }

    #[test]
    fn test_create_sets_org_gauge_option() {
        let mut decide = make_decide();
        decide.create();
        // NullPlayerResource returns default gauge (0), verify no panic
    }

    #[test]
    fn test_prepare_plays_decide_sound() {
        let mut decide = make_decide();
        // Should not panic — stub logs warning
        decide.prepare();
    }

    #[test]
    fn test_render_no_skin_no_panic() {
        let mut decide = make_decide();
        // data.skin is None — render should not panic
        decide.render();
    }

    #[test]
    fn test_render_with_skin_nowtime_zero_no_startinput() {
        let mut decide = make_decide();
        decide.data.skin = Some(Box::new(MockSkin::new()));
        // nowmicrotime=0 from fresh TimerManager, get_now_time()=0
        // skin.get_input()=0, condition is nowtime > input i.e. 0 > 0 = false
        decide.render();
        assert!(!decide.data.timer.is_timer_on(TIMER_STARTINPUT));
    }

    #[test]
    fn test_render_with_skin_sets_startinput_when_past_input_time() {
        let mut decide = make_decide();
        // input=-1 so that nowtime(0) > input(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(-1, i32::MAX, 0)));
        decide.render();
        assert!(decide.data.timer.is_timer_on(TIMER_STARTINPUT));
    }

    #[test]
    fn test_render_scene_timeout_triggers_fadeout() {
        let mut decide = make_decide();
        // scene=-1 so that nowtime(0) > scene(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(0, -1, 0)));
        decide.render();
        assert!(decide.data.timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn test_render_fadeout_with_cancel_transitions_to_select() {
        let mut decide = make_decide();
        // fadeout=-1 so that get_now_time_for_id(TIMER_FADEOUT)(=0) > fadeout(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(0, i32::MAX, -1)));
        decide.cancel = true;
        decide.data.timer.set_timer_on(TIMER_FADEOUT);
        decide.render();
        // change_state(MusicSelect) is a stub that logs — verify no panic
    }

    #[test]
    fn test_render_fadeout_without_cancel_transitions_to_play() {
        let mut decide = make_decide();
        // fadeout=-1 so that get_now_time_for_id(TIMER_FADEOUT)(=0) > fadeout(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(0, i32::MAX, -1)));
        decide.cancel = false;
        decide.data.timer.set_timer_on(TIMER_FADEOUT);
        decide.render();
        // change_state(Play) is a stub that logs — verify no panic
    }

    #[test]
    fn test_input_no_timer_no_action() {
        let mut decide = make_decide();
        // Neither TIMER_FADEOUT nor TIMER_STARTINPUT is on — input does nothing
        decide.input();
        assert!(!decide.cancel);
    }

    #[test]
    fn test_input_during_fadeout_no_action() {
        let mut decide = make_decide();
        decide.data.timer.set_timer_on(TIMER_FADEOUT);
        decide.data.timer.set_timer_on(TIMER_STARTINPUT);
        // TIMER_FADEOUT is on — input is blocked
        decide.input();
    }

    #[test]
    fn test_input_startinput_only_no_keys() {
        let mut decide = make_decide();
        decide.data.timer.set_timer_on(TIMER_STARTINPUT);
        // TIMER_STARTINPUT on, TIMER_FADEOUT off — input block entered
        // But no keys pressed (stub returns false for all), so nothing happens
        decide.input();
        assert!(!decide.cancel);
        assert!(!decide.data.timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn test_handle_skin_mouse_pressed_uses_decide_context() {
        let changed_states = Arc::new(Mutex::new(Vec::new()));
        let mut decide = MusicDecide::new(
            MainControllerRef::new(Box::new(RecordingMainController::new(Arc::clone(
                &changed_states,
            )))),
            Box::new(NullPlayerResource::new()),
            TimerManager::new(),
        );
        decide.data.skin = Some(Box::new(ChangeStateSkin {
            state: MainStateType::MusicSelect,
        }));

        <MusicDecide as MainState>::handle_skin_mouse_pressed(&mut decide, 0, 10, 10);

        assert_eq!(
            *changed_states.lock().unwrap(),
            vec![MainStateType::MusicSelect]
        );
    }

    #[test]
    fn test_dispose_clears_skin_and_stage() {
        let mut decide = make_decide();
        decide.dispose();
        assert!(decide.data.skin.is_none());
        assert!(decide.data.stage.is_none());
    }

    #[test]
    fn test_main_state_data_accessors() {
        let mut decide = make_decide();
        let _ = decide.main_state_data();
        let _ = decide.main_state_data_mut();
    }
}
