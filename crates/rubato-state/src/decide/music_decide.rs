// Translated from MusicDecide.java
// Music decide screen state.

use rubato_core::main_state::{MainState, MainStateData, MainStateType};
use rubato_core::system_sound_manager::SoundType;
use rubato_core::timer_manager::TimerManager;
use rubato_skin::skin_property::{TIMER_FADEOUT, TIMER_STARTINPUT};
use rubato_skin::skin_type::SkinType;

use super::main_controller_ref::MainControllerRef;
use super::{ControlKeys, NullPlayerResource, PlayerResourceAccess};

/// Render context adapter for decide screen skin rendering.
/// Provides config access through SkinRenderContext.
struct DecideRenderContext<'a> {
    timer: &'a mut TimerManager,
    resource: &'a dyn PlayerResourceAccess,
    main: &'a MainControllerRef,
}

impl rubato_types::timer_access::TimerAccess for DecideRenderContext<'_> {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }
    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }
    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(timer_id)
    }
    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.timer(timer_id)
    }
    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for_id(timer_id)
    }
    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for DecideRenderContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Decide)
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(self.main.player_config())
    }

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        Some(self.main.config())
    }

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        self.resource.songdata()
    }

    fn score_data_ref(&self) -> Option<&rubato_core::score_data::ScoreData> {
        self.resource.score_data()
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        let mode = self
            .resource
            .songdata()
            .and_then(|song| match song.chart.mode {
                5 => Some(bms_model::mode::Mode::BEAT_5K),
                7 => Some(bms_model::mode::Mode::BEAT_7K),
                9 => Some(bms_model::mode::Mode::POPN_9K),
                10 => Some(bms_model::mode::Mode::BEAT_10K),
                14 => Some(bms_model::mode::Mode::BEAT_14K),
                25 => Some(bms_model::mode::Mode::KEYBOARD_24K),
                50 => Some(bms_model::mode::Mode::KEYBOARD_24K_DOUBLE),
                _ => None,
            })?;
        Some(
            &self
                .resource
                .player_config()
                .play_config_ref(mode)
                .playconfig,
        )
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn string_value(&self, id: i32) -> String {
        match id {
            10 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.title.clone()),
            11 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.subtitle.clone()),
            14 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.artist.clone()),
            _ => String::new(),
        }
    }

    fn image_index_value(&self, id: i32) -> i32 {
        match id {
            // Java IntegerPropertyFactory ID 308 (lnmode): on Decide screen, override
            // from chart data when the chart explicitly defines LN types.
            308 => {
                if let Some(song) = self.resource.songdata()
                    && let Some(override_val) =
                        rubato_types::skin_render_context::compute_lnmode_from_chart(&song.chart)
                {
                    return override_val;
                }
                self.default_image_index_value(id)
            }
            _ => self.default_image_index_value(id),
        }
    }

    fn integer_value(&self, id: i32) -> i32 {
        match id {
            // Song BPM from songdata
            90 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| s.chart.maxbpm),
            91 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| s.chart.minbpm),
            // mainbpm: prefer SongInformation.mainbpm when available.
            // Java returns Integer.MIN_VALUE when SongInformation is absent,
            // signaling "no data" so skin renderers hide the value.
            92 => self.resource.songdata().map_or(i32::MIN, |s| {
                s.info
                    .as_ref()
                    .map(|i| i.mainbpm as i32)
                    .unwrap_or(i32::MIN)
            }),
            // Total notes
            350 => self.resource.songdata().map_or(0, |s| s.chart.notes),
            // Song duration
            312 => self.resource.songdata().map_or(0, |s| s.chart.length),
            1163 => self
                .resource
                .songdata()
                .map_or(0, |s| s.chart.length.max(0) / 60000),
            1164 => self
                .resource
                .songdata()
                .map_or(0, |s| (s.chart.length.max(0) % 60000) / 1000),
            // Playtime
            17 => (self.timer.now_time() / 3_600_000) as i32,
            18 => ((self.timer.now_time() % 3_600_000) / 60_000) as i32,
            19 => ((self.timer.now_time() % 60_000) / 1_000) as i32,
            _ => 0,
        }
    }
}

impl rubato_skin::reexports::MainState for DecideRenderContext<'_> {}

struct DecideMouseContext<'a> {
    timer: &'a mut TimerManager,
    main: &'a mut MainControllerRef,
    resource: &'a mut dyn PlayerResourceAccess,
}

impl rubato_types::timer_access::TimerAccess for DecideMouseContext<'_> {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }

    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }

    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(timer_id)
    }

    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.timer(timer_id)
    }

    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for_id(timer_id)
    }

    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for DecideMouseContext<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        Some(rubato_types::main_state_type::MainStateType::Decide)
    }

    fn execute_event(&mut self, _id: i32, _arg1: i32, _arg2: i32) {
        // Decide screen has no state-specific event handling.
        // Custom events (1000-1999) require skin access which the mouse context
        // cannot provide (borrow conflict with skin.mouse_pressed_at).
    }

    fn change_state(&mut self, state: rubato_types::main_state_type::MainStateType) {
        self.main.change_state(state);
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        self.timer.set_micro_timer(timer_id, micro_time);
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(self.resource.player_config())
    }

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        Some(self.main.config())
    }

    fn score_data_ref(&self) -> Option<&rubato_core::score_data::ScoreData> {
        self.resource.score_data()
    }

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        self.resource.songdata()
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        let mode = self
            .resource
            .songdata()
            .and_then(|song| match song.chart.mode {
                5 => Some(bms_model::mode::Mode::BEAT_5K),
                7 => Some(bms_model::mode::Mode::BEAT_7K),
                9 => Some(bms_model::mode::Mode::POPN_9K),
                10 => Some(bms_model::mode::Mode::BEAT_10K),
                14 => Some(bms_model::mode::Mode::BEAT_14K),
                25 => Some(bms_model::mode::Mode::KEYBOARD_24K),
                50 => Some(bms_model::mode::Mode::KEYBOARD_24K_DOUBLE),
                _ => None,
            })?;
        Some(
            &self
                .resource
                .player_config()
                .play_config_ref(mode)
                .playconfig,
        )
    }

    fn integer_value(&self, id: i32) -> i32 {
        match id {
            90 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| s.chart.maxbpm),
            91 => self
                .resource
                .songdata()
                .map_or(i32::MIN, |s| s.chart.minbpm),
            92 => self.resource.songdata().map_or(i32::MIN, |s| {
                s.info
                    .as_ref()
                    .map(|i| i.mainbpm as i32)
                    .unwrap_or(i32::MIN)
            }),
            350 => self.resource.songdata().map_or(0, |s| s.chart.notes),
            312 => self.resource.songdata().map_or(0, |s| s.chart.length),
            1163 => self
                .resource
                .songdata()
                .map_or(0, |s| s.chart.length.max(0) / 60000),
            1164 => self
                .resource
                .songdata()
                .map_or(0, |s| (s.chart.length.max(0) % 60000) / 1000),
            17 => (self.timer.now_time() / 3_600_000) as i32,
            18 => ((self.timer.now_time() % 3_600_000) / 60_000) as i32,
            19 => ((self.timer.now_time() % 60_000) / 1_000) as i32,
            _ => 0,
        }
    }

    fn image_index_value(&self, id: i32) -> i32 {
        match id {
            308 => {
                if let Some(song) = self.resource.songdata()
                    && let Some(override_val) =
                        rubato_types::skin_render_context::compute_lnmode_from_chart(&song.chart)
                {
                    return override_val;
                }
                self.default_image_index_value(id)
            }
            _ => self.default_image_index_value(id),
        }
    }

    fn string_value(&self, id: i32) -> String {
        match id {
            10 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.title.clone()),
            11 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.subtitle.clone()),
            14 => self
                .resource
                .songdata()
                .map_or_else(String::new, |s| s.metadata.artist.clone()),
            _ => String::new(),
        }
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        self.resource.player_config_mut()
    }

    fn play_option_change_sound(&mut self) {
        self.main.play_sound(&SoundType::OptionChange, false);
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
        let gauge = self.resource.player_config().play_settings.gauge;
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
                resource: &mut *self.resource,
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
                resource: &mut *self.resource,
            };
            skin.mouse_dragged_at(&mut ctx, button, x, y);
        }

        self.data.timer = timer;
        self.data.skin = Some(skin);
    }

    fn render(&mut self) {
        let nowtime = self.data.timer.now_time();
        // Skin timing values; fall back to 0 when no skin is loaded so the
        // decide screen still transitions to Play instead of stalling forever.
        let input_time = self.data.skin.as_ref().map_or(0, |s| s.input() as i64);
        let fadeout_time = self.data.skin.as_ref().map_or(0, |s| s.fadeout() as i64);
        let scene_time = self.data.skin.as_ref().map_or(0, |s| s.scene() as i64);

        if nowtime > input_time {
            self.data.timer.switch_timer(TIMER_STARTINPUT, true);
        }
        if self.data.timer.is_timer_on(TIMER_FADEOUT) {
            if self.data.timer.now_time_for_id(TIMER_FADEOUT) > fadeout_time {
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
            // before calling audio_processor_mut (avoids overlapping &mut borrows).
            let (decide, cancel) = {
                let input = self.main.input_processor();
                let decide = input.key_state(0)
                    || input.key_state(2)
                    || input.key_state(4)
                    || input.key_state(6)
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
                if let Some(audio) = self.main.audio_processor_mut() {
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
        let skin_path = rubato_skin::skin_loader::skin_path_from_player_config(
            self.main.player_config(),
            skin_type,
        );
        let skin = {
            let mut ctx = DecideRenderContext {
                timer: &mut self.data.timer,
                resource: &*self.resource,
                main: &self.main,
            };
            skin_path.as_deref().and_then(|path| {
                rubato_skin::skin_loader::load_skin_from_path_with_state(&mut ctx, skin_type, path)
            })
        };
        self.data.skin =
            skin.map(|skin| Box::new(skin) as Box<dyn rubato_core::main_state::SkinDrawable>);
    }

    fn dispose(&mut self) {
        // super.dispose()
        if let Some(ref mut skin) = self.data.skin {
            skin.dispose_skin();
        }
        self.data.skin = None;
    }

    fn take_player_resource_box(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        let null: Box<dyn PlayerResourceAccess> = Box::new(NullPlayerResource::new());
        let old = std::mem::replace(&mut self.resource, null);
        Some(old.into_any_send())
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::decide::{NullMainController, NullPlayerResource};
    use rubato_core::main_state::SkinDrawable;
    use rubato_core::sprite_batch_helper::SpriteBatch;
    use rubato_types::main_controller_access::MainControllerAccess;
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
        fn fadeout(&self) -> i32 {
            self.fadeout
        }
        fn input(&self) -> i32 {
            self.input
        }
        fn scene(&self) -> i32 {
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

        fn fadeout(&self) -> i32 {
            0
        }

        fn input(&self) -> i32 {
            0
        }

        fn scene(&self) -> i32 {
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
        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }

        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }

        fn change_state(&mut self, state: MainStateType) {
            self.changed_states
                .lock()
                .expect("mutex poisoned")
                .push(state);
        }

        fn save_config(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn exit(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn save_last_recording(&self, _reason: &str) {}

        fn update_song(&mut self, _path: Option<&str>) {}

        fn player_resource(
            &self,
        ) -> Option<&dyn rubato_types::player_resource_access::PlayerResourceAccess> {
            None
        }

        fn player_resource_mut(
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
        // nowmicrotime=0 from fresh TimerManager, now_time()=0
        // skin.input()=0, condition is nowtime > input i.e. 0 > 0 = false
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
        // fadeout=-1 so that now_time_for_id(TIMER_FADEOUT)(=0) > fadeout(-1) is true
        decide.data.skin = Some(Box::new(MockSkin::with_values(0, i32::MAX, -1)));
        decide.cancel = true;
        decide.data.timer.set_timer_on(TIMER_FADEOUT);
        decide.render();
        // change_state(MusicSelect) is a stub that logs — verify no panic
    }

    #[test]
    fn test_render_fadeout_without_cancel_transitions_to_play() {
        let mut decide = make_decide();
        // fadeout=-1 so that now_time_for_id(TIMER_FADEOUT)(=0) > fadeout(-1) is true
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
            *changed_states.lock().expect("mutex poisoned"),
            vec![MainStateType::MusicSelect]
        );
    }

    #[test]
    fn test_dispose_clears_skin() {
        let mut decide = make_decide();
        decide.dispose();
        assert!(decide.data.skin.is_none());
    }

    #[test]
    fn test_main_state_data_accessors() {
        let mut decide = make_decide();
        let _ = decide.main_state_data();
        let _ = decide.main_state_data_mut();
    }

    /// Mock PlayerResourceAccess that returns a SongData with a given chart.length.
    struct SongLengthResource {
        song: rubato_types::song_data::SongData,
        config: rubato_types::config::Config,
        player_config: rubato_types::player_config::PlayerConfig,
        score: Option<rubato_core::score_data::ScoreData>,
    }

    impl SongLengthResource {
        fn with_length_ms(length: i32) -> Self {
            let mut song = rubato_types::song_data::SongData::default();
            song.chart.length = length;
            Self {
                song,
                config: rubato_types::config::Config::default(),
                player_config: rubato_types::player_config::PlayerConfig::default(),
                score: None,
            }
        }
    }

    impl rubato_types::player_resource_access::ConfigAccess for SongLengthResource {
        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }
        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }
    }

    impl rubato_types::player_resource_access::ScoreAccess for SongLengthResource {
        fn score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            self.score.as_ref()
        }
        fn rival_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            None
        }
        fn target_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            None
        }
        fn course_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
            None
        }
        fn set_course_score_data(&mut self, _score: rubato_core::score_data::ScoreData) {}
        fn score_data_mut(&mut self) -> Option<&mut rubato_core::score_data::ScoreData> {
            None
        }
    }

    impl rubato_types::player_resource_access::SongAccess for SongLengthResource {
        fn songdata(&self) -> Option<&rubato_types::song_data::SongData> {
            Some(&self.song)
        }
        fn songdata_mut(&mut self) -> Option<&mut rubato_types::song_data::SongData> {
            Some(&mut self.song)
        }
        fn set_songdata(&mut self, _data: Option<rubato_types::song_data::SongData>) {}
        fn course_song_data(&self) -> Vec<rubato_types::song_data::SongData> {
            vec![]
        }
    }

    impl rubato_types::player_resource_access::ReplayAccess for SongLengthResource {
        fn replay_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
            None
        }
        fn replay_data_mut(&mut self) -> Option<&mut rubato_types::replay_data::ReplayData> {
            None
        }
        fn course_replay(&self) -> &[rubato_types::replay_data::ReplayData] {
            &[]
        }
        fn add_course_replay(&mut self, _rd: rubato_types::replay_data::ReplayData) {}
        fn course_replay_mut(&mut self) -> &mut Vec<rubato_types::replay_data::ReplayData> {
            static mut EMPTY: Vec<rubato_types::replay_data::ReplayData> = Vec::new();
            // SAFETY: only used in tests, never concurrently
            unsafe { &mut *std::ptr::addr_of_mut!(EMPTY) }
        }
    }

    impl rubato_types::player_resource_access::CourseAccess for SongLengthResource {
        fn course_data(&self) -> Option<&rubato_types::course_data::CourseData> {
            None
        }
        fn course_index(&self) -> usize {
            0
        }
        fn next_course(&mut self) -> bool {
            false
        }
        fn constraint(&self) -> Vec<rubato_types::course_data::CourseDataConstraint> {
            vec![]
        }
        fn set_course_data(&mut self, _data: rubato_types::course_data::CourseData) {}
        fn clear_course_data(&mut self) {}
    }

    impl rubato_types::player_resource_access::GaugeAccess for SongLengthResource {
        fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
            None
        }
        fn groove_gauge(&self) -> Option<&rubato_types::groove_gauge::GrooveGauge> {
            None
        }
        fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
            static EMPTY: Vec<Vec<Vec<f32>>> = Vec::new();
            &EMPTY
        }
        fn add_course_gauge(&mut self, _gauge: Vec<Vec<f32>>) {}
        fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
            static mut EMPTY: Vec<Vec<Vec<f32>>> = Vec::new();
            // SAFETY: only used in tests, never concurrently
            unsafe { &mut *std::ptr::addr_of_mut!(EMPTY) }
        }
    }

    impl rubato_types::player_resource_access::PlayerStateAccess for SongLengthResource {
        fn maxcombo(&self) -> i32 {
            0
        }
        fn org_gauge_option(&self) -> i32 {
            0
        }
        fn set_org_gauge_option(&mut self, _val: i32) {}
        fn assist(&self) -> i32 {
            0
        }
        fn is_update_score(&self) -> bool {
            false
        }
        fn is_update_course_score(&self) -> bool {
            false
        }
        fn is_force_no_ir_send(&self) -> bool {
            false
        }
        fn is_freq_on(&self) -> bool {
            false
        }
    }

    impl rubato_types::player_resource_access::SessionMutation for SongLengthResource {
        fn clear(&mut self) {}
        fn set_bms_file(
            &mut self,
            _path: &std::path::Path,
            _mode_type: i32,
            _mode_id: i32,
        ) -> bool {
            false
        }
        fn set_course_bms_files(&mut self, _files: &[std::path::PathBuf]) -> bool {
            false
        }
        fn set_tablename(&mut self, _name: &str) {}
        fn set_tablelevel(&mut self, _level: &str) {}
        fn set_rival_score_data_option(
            &mut self,
            _score: Option<rubato_core::score_data::ScoreData>,
        ) {
        }
        fn set_chart_option_data(
            &mut self,
            _option: Option<rubato_types::replay_data::ReplayData>,
        ) {
        }
    }

    impl rubato_types::player_resource_access::MediaAccess for SongLengthResource {
        fn reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }
        fn reverse_lookup_levels(&self) -> Vec<String> {
            vec![]
        }
    }

    impl PlayerResourceAccess for SongLengthResource {
        fn into_any_send(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }
    }

    #[test]
    fn decide_render_context_song_duration_minutes_seconds() {
        // 150_000 ms = 2 minutes 30 seconds
        let resource = SongLengthResource::with_length_ms(150_000);
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(312), 150_000, "ID 312: raw ms");
        assert_eq!(ctx.integer_value(1163), 2, "ID 1163: minutes");
        assert_eq!(ctx.integer_value(1164), 30, "ID 1164: seconds");
    }

    #[test]
    fn decide_render_context_song_duration_no_songdata() {
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let resource = NullPlayerResource::new();
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(1163), 0);
        assert_eq!(ctx.integer_value(1164), 0);
    }

    #[test]
    fn decide_render_context_song_data_ref_returns_songdata() {
        let resource = SongLengthResource::with_length_ms(100_000);
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.song_data_ref().is_some());
        assert_eq!(ctx.song_data_ref().unwrap().chart.length, 100_000);
    }

    #[test]
    fn decide_render_context_song_data_ref_none_when_no_song() {
        let resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.song_data_ref().is_none());
    }

    #[test]
    fn decide_render_context_current_play_config_ref_for_7k() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.mode = 7;
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.current_play_config_ref().is_some());
    }

    #[test]
    fn decide_render_context_current_play_config_ref_none_for_unknown_mode() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.mode = 999;
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.current_play_config_ref().is_none());
    }

    #[test]
    fn decide_render_context_current_play_config_ref_none_when_no_songdata() {
        let resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(ctx.current_play_config_ref().is_none());
    }

    #[test]
    fn decide_render_context_favorite_image_index_uses_song_data_ref() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.favorite = rubato_types::song_data::FAVORITE_SONG;
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // ID 89 (favorite_song) should now return 1 instead of -1
        assert_eq!(ctx.image_index_value(89), 1);
    }

    #[test]
    fn decide_render_context_mainbpm_from_song_information() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.maxbpm = 200;
        resource.song.chart.minbpm = 100;
        // Set SongInformation with mainbpm = 160
        let mut info = rubato_types::song_information::SongInformation::default();
        info.mainbpm = 160.0;
        resource.song.info = Some(info);

        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // ID 92 should return mainbpm from SongInformation
        assert_eq!(ctx.integer_value(92), 160);
    }

    #[test]
    fn decide_render_context_mainbpm_no_info_returns_min_value() {
        // When SongInformation is absent, Java returns Integer.MIN_VALUE
        // so skin renderers hide the value.
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.maxbpm = 180;
        // No SongInformation set -> should return i32::MIN, not maxbpm

        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(92), i32::MIN);
    }

    #[test]
    fn decide_render_context_mainbpm_no_songdata_returns_min_value() {
        // When songdata is absent, Java returns Integer.MIN_VALUE.
        let resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(92), i32::MIN);
    }

    #[test]
    fn decide_render_context_maxbpm_no_songdata_returns_min_value() {
        // When songdata is absent, ID 90 (maxbpm) should return i32::MIN
        // so skin renderers hide the value, matching select screen behavior.
        let resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(90), i32::MIN);
    }

    #[test]
    fn decide_render_context_minbpm_no_songdata_returns_min_value() {
        // When songdata is absent, ID 91 (minbpm) should return i32::MIN
        // so skin renderers hide the value, matching select screen behavior.
        let resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(91), i32::MIN);
    }

    #[test]
    fn decide_render_context_maxbpm_with_songdata_returns_value() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.maxbpm = 200;
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(90), 200);
    }

    #[test]
    fn decide_render_context_minbpm_with_songdata_returns_value() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.minbpm = 120;
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(ctx.integer_value(91), 120);
    }

    #[test]
    fn decide_render_context_negative_length_clamped_to_zero() {
        // Negative chart.length should be clamped to 0, not produce
        // negative minutes/seconds.
        let resource = SongLengthResource::with_length_ms(-120_000);
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.integer_value(1163),
            0,
            "negative length minutes should be 0"
        );
        assert_eq!(
            ctx.integer_value(1164),
            0,
            "negative length seconds should be 0"
        );
    }

    // ============================================================
    // DecideRenderContext image_index_value ID 308 (lnmode) tests
    // ============================================================

    #[test]
    fn decide_render_context_lnmode_308_override_longnote() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.feature = rubato_types::song_data::FEATURE_LONGNOTE;
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(308),
            0,
            "ID 308 should return 0 (LN) when chart has FEATURE_LONGNOTE"
        );
    }

    #[test]
    fn decide_render_context_lnmode_308_override_chargenote() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.feature = rubato_types::song_data::FEATURE_CHARGENOTE;
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(308),
            1,
            "ID 308 should return 1 (CN) when chart has FEATURE_CHARGENOTE"
        );
    }

    #[test]
    fn decide_render_context_lnmode_308_override_hellchargenote() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.feature = rubato_types::song_data::FEATURE_HELLCHARGENOTE;
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(308),
            2,
            "ID 308 should return 2 (HCN) when chart has FEATURE_HELLCHARGENOTE"
        );
    }

    #[test]
    fn decide_render_context_lnmode_308_no_override_falls_through() {
        // No LN features -> falls through to config-based default
        let resource = SongLengthResource::with_length_ms(0);
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // default_image_index_value uses player_config.play_settings.lnmode (default 0)
        let default_lnmode = ctx.player_config_ref().unwrap().play_settings.lnmode;
        assert_eq!(
            ctx.image_index_value(308),
            default_lnmode,
            "ID 308 should fall through to config lnmode when chart has no LN features"
        );
    }

    #[test]
    fn decide_render_context_lnmode_308_undefined_ln_falls_through() {
        // UNDEFINEDLN set -> no override (has_undefined_long_note is true)
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.feature = rubato_types::song_data::FEATURE_UNDEFINEDLN;
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let default_lnmode = ctx.player_config_ref().unwrap().play_settings.lnmode;
        assert_eq!(
            ctx.image_index_value(308),
            default_lnmode,
            "ID 308 should fall through when chart has FEATURE_UNDEFINEDLN"
        );
    }

    #[test]
    fn decide_render_context_lnmode_308_no_songdata_falls_through() {
        let resource = NullPlayerResource::new();
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        // No songdata -> falls through to config-based default
        let default_lnmode = ctx
            .player_config_ref()
            .map(|pc| pc.play_settings.lnmode)
            .unwrap_or(0);
        assert_eq!(
            ctx.image_index_value(308),
            default_lnmode,
            "ID 308 should fall through when no songdata available"
        );
    }

    // ============================================================
    // DecideRenderContext score_data_ref / image_index 370/371 tests
    // ============================================================

    #[test]
    fn decide_render_context_image_index_370_returns_clear_type() {
        // Regression: image_index_value(370) must return the clear type from
        // score_data_ref, not -1. Without score_data_ref delegation, the
        // default trait method returns None and 370 maps to -1.
        let mut resource = SongLengthResource::with_length_ms(0);
        let mut score = rubato_core::score_data::ScoreData::default();
        score.clear = 5; // e.g. ClearType::FullCombo
        resource.score = Some(score);

        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(370),
            5,
            "ID 370 (cleartype) should return score_data.clear, not -1"
        );
    }

    #[test]
    fn decide_render_context_image_index_370_no_score_returns_minus_one() {
        // When no score data is available, 370 should still return -1.
        let resource = SongLengthResource::with_length_ms(0);
        let mut timer = TimerManager::new();
        let main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideRenderContext {
            timer: &mut timer,
            resource: &resource,
            main: &main,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(370),
            -1,
            "ID 370 should return -1 when no score data is available"
        );
    }

    // ============================================================
    // DecideMouseContext missing delegation tests (Finding 2)
    // ============================================================

    #[test]
    fn decide_mouse_context_score_data_ref_delegates_to_resource() {
        let mut resource = SongLengthResource::with_length_ms(0);
        let mut score = rubato_core::score_data::ScoreData::default();
        score.clear = 4;
        resource.score = Some(score);

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let sd = ctx.score_data_ref();
        assert!(
            sd.is_some(),
            "DecideMouseContext::score_data_ref() must delegate, not return None"
        );
        assert_eq!(sd.unwrap().clear, 4);
    }

    #[test]
    fn decide_mouse_context_song_data_ref_delegates_to_resource() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.metadata.title = "DecideTest".to_string();

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        let song = ctx.song_data_ref();
        assert!(
            song.is_some(),
            "DecideMouseContext::song_data_ref() must delegate, not return None"
        );
        assert_eq!(song.unwrap().metadata.title, "DecideTest");
    }

    #[test]
    fn decide_mouse_context_current_play_config_ref_delegates_for_7k() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.mode = 7;

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert!(
            ctx.current_play_config_ref().is_some(),
            "DecideMouseContext::current_play_config_ref() must delegate, not return None"
        );
    }

    #[test]
    fn decide_mouse_context_integer_value_delegates_bpm_ids() {
        let mut resource = SongLengthResource::with_length_ms(150_000);
        resource.song.chart.maxbpm = 200;
        resource.song.chart.minbpm = 100;

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.integer_value(90),
            200,
            "DecideMouseContext::integer_value(90) must delegate maxbpm, not return 0"
        );
        assert_eq!(
            ctx.integer_value(91),
            100,
            "DecideMouseContext::integer_value(91) must delegate minbpm, not return 0"
        );
    }

    #[test]
    fn decide_mouse_context_image_index_value_delegates_lnmode() {
        // Set lnmode config to a non-zero sentinel so we can distinguish
        // the chart-based override (CHARGENOTE -> 1) from the config fallback.
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.chart.feature = rubato_types::song_data::FEATURE_CHARGENOTE;
        resource.player_config.play_settings.lnmode = 99;

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.image_index_value(308),
            1,
            "DecideMouseContext::image_index_value(308) must return 1 (CN) from chart override, not config lnmode (99)"
        );
    }

    #[test]
    fn decide_mouse_context_string_value_delegates_title() {
        let mut resource = SongLengthResource::with_length_ms(0);
        resource.song.metadata.title = "DecideTitle".to_string();

        let mut timer = TimerManager::new();
        let mut main = MainControllerRef::new(Box::new(NullMainController));
        let ctx = DecideMouseContext {
            timer: &mut timer,
            main: &mut main,
            resource: &mut resource,
        };
        use rubato_types::skin_render_context::SkinRenderContext;
        assert_eq!(
            ctx.string_value(10),
            "DecideTitle",
            "DecideMouseContext::string_value(10) must delegate title, not return empty"
        );
    }
}
