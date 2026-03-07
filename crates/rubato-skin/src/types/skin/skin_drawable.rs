/// Adapter that provides timer data to skin objects via the stubs::MainState interface.
/// Used by SkinDrawable to bridge beatoraja-core's TimerManager to beatoraja-skin's internal interface.
///
/// Holds a reference to the real `TimerAccess` (typically a `TimerManager`) so that
/// per-timer-id queries return actual values instead of always 0.
struct TimerOnlyMainState<'a> {
    timer: Option<&'a dyn rubato_types::timer_access::TimerAccess>,
    ctx: Option<&'a mut dyn rubato_types::skin_render_context::SkinRenderContext>,
    main_controller: crate::stubs::MainController,
    resource: crate::stubs::PlayerResource,
    state_type: Option<rubato_types::main_state_type::MainStateType>,
    image_registry: &'a HashMap<i32, TextureRegion>,
}

impl<'a> TimerOnlyMainState<'a> {
    fn from_timer(timer: &'a dyn rubato_types::timer_access::TimerAccess) -> Self {
        static EMPTY: std::sync::LazyLock<HashMap<i32, TextureRegion>> =
            std::sync::LazyLock::new(HashMap::new);
        Self {
            timer: Some(timer),
            ctx: None,
            main_controller: crate::stubs::MainController { debug: false },
            resource: crate::stubs::PlayerResource,
            state_type: None,
            image_registry: &EMPTY,
        }
    }

    fn from_render_context_with_images(
        ctx: &'a mut dyn rubato_types::skin_render_context::SkinRenderContext,
        image_registry: &'a HashMap<i32, TextureRegion>,
    ) -> Self {
        let state_type = ctx.current_state_type();
        Self {
            timer: None,
            ctx: Some(ctx),
            main_controller: crate::stubs::MainController { debug: false },
            resource: crate::stubs::PlayerResource,
            state_type,
            image_registry,
        }
    }
}

impl crate::stubs::MainState for TimerOnlyMainState<'_> {
    fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
        if let Some(ctx) = self.ctx.as_deref() {
            ctx
        } else {
            self.timer.expect("timer-only adapter must carry a timer")
        }
    }

    fn get_offset_value(&self, _id: i32) -> Option<&crate::stubs::SkinOffset> {
        None
    }

    fn get_main(&self) -> &crate::stubs::MainController {
        &self.main_controller
    }

    fn get_image(&self, id: i32) -> Option<crate::rendering_stubs::TextureRegion> {
        self.image_registry.get(&id).cloned()
    }

    fn get_resource(&self) -> &crate::stubs::PlayerResource {
        &self.resource
    }

    fn is_music_selector(&self) -> bool {
        self.ctx
            .as_deref()
            .is_some_and(rubato_types::skin_render_context::SkinStateQuery::is_music_selector)
            || self.state_type == Some(rubato_types::main_state_type::MainStateType::MusicSelect)
    }

    fn is_result_state(&self) -> bool {
        self.ctx
            .as_deref()
            .is_some_and(rubato_types::skin_render_context::SkinRenderContext::is_result_state)
            || matches!(
                self.state_type,
                Some(
                    rubato_types::main_state_type::MainStateType::Result
                        | rubato_types::main_state_type::MainStateType::CourseResult
                )
            )
    }

    fn is_bms_player(&self) -> bool {
        self.state_type == Some(rubato_types::main_state_type::MainStateType::Play)
    }

    fn recent_judges(&self) -> &[i64] {
        self.ctx
            .as_deref()
            .map_or(&[] as &[i64], |c| c.recent_judges())
    }

    fn recent_judges_index(&self) -> usize {
        self.ctx.as_deref().map_or(0, |c| c.recent_judges_index())
    }

    fn integer_value(&self, id: i32) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.integer_value(id))
    }

    fn image_index_value(&self, id: i32) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.image_index_value(id))
    }

    fn boolean_value(&self, id: i32) -> bool {
        self.ctx.as_deref().is_some_and(|c| c.boolean_value(id))
    }

    fn float_value(&self, id: i32) -> f32 {
        self.ctx.as_deref().map_or(0.0, |c| c.float_value(id))
    }

    fn string_value(&self, id: i32) -> String {
        self.ctx
            .as_deref()
            .map_or_else(String::new, |c| c.string_value(id))
    }

    fn set_float_value(&mut self, id: i32, value: f32) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.set_float_value(id, value);
        }
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        self.ctx
            .as_deref()
            .map_or(0, |c| c.judge_count(judge, fast))
    }

    fn get_gauge_value(&self) -> f32 {
        self.ctx.as_deref().map_or(0.0, |c| c.gauge_value())
    }

    fn gauge_type(&self) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.gauge_type())
    }

    fn get_now_judge(&self, player: i32) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.now_judge(player))
    }

    fn get_now_combo(&self, player: i32) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.now_combo(player))
    }

    fn get_player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        self.ctx
            .as_deref()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::player_config_ref)
    }

    fn get_config_ref(&self) -> Option<&rubato_types::config::Config> {
        self.ctx
            .as_deref()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::config_ref)
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        self.ctx
            .as_deref_mut()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::player_config_mut)
    }

    fn get_config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
        self.ctx
            .as_deref_mut()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::config_mut)
    }

    fn get_selected_play_config_mut(
        &mut self,
    ) -> Option<&mut rubato_types::play_config::PlayConfig> {
        self.ctx.as_deref_mut().and_then(
            rubato_types::skin_render_context::SkinRenderContext::selected_play_config_mut,
        )
    }

    fn play_option_change_sound(&mut self) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.play_option_change_sound();
        }
    }

    fn update_bar_after_change(&mut self) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.update_bar_after_change();
        }
    }

    fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.execute_event(id, arg1, arg2);
        }
    }

    fn change_state(&mut self, state_type: rubato_types::main_state_type::MainStateType) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.change_state(state_type);
        }
    }

    fn select_song(&mut self, mode: rubato_core::bms_player_mode::BMSPlayerMode) {
        let Some(ctx) = self.ctx.as_deref_mut() else {
            return;
        };
        let event_id = match mode.mode {
            rubato_core::bms_player_mode::Mode::Play => 15,
            rubato_core::bms_player_mode::Mode::Autoplay => 16,
            rubato_core::bms_player_mode::Mode::Practice => 315,
            rubato_core::bms_player_mode::Mode::Replay => return,
        };
        ctx.select_song_mode(event_id);
    }

    fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.set_timer_micro(timer_id, micro_time);
        }
    }

    fn audio_play(&mut self, path: &str, volume: f32, is_loop: bool) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.audio_play(path, volume, is_loop);
        }
    }

    fn audio_stop(&mut self, path: &str) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.audio_stop(path);
        }
    }
}

impl rubato_core::main_state::SkinDrawable for Skin {
    fn prepare_skin(&mut self) {
        let null_timer = rubato_types::timer_access::NullTimer;
        let adapter = TimerOnlyMainState::from_timer(&null_timer);
        self.prepare(&adapter);
    }

    fn draw_all_objects_timed(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
        // Take image registry out to avoid borrow conflict (&mut self vs &self.image_registry)
        let registry = std::mem::take(&mut self.image_registry);
        let adapter = TimerOnlyMainState::from_render_context_with_images(ctx, &registry);
        self.draw_all_objects(&adapter);
        self.image_registry = registry;
    }

    fn update_custom_objects_timed(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
        let registry = std::mem::take(&mut self.image_registry);
        let mut adapter = TimerOnlyMainState::from_render_context_with_images(ctx, &registry);
        self.update_custom_objects(&mut adapter);
        self.image_registry = registry;
    }

    fn mouse_pressed_at(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        button: i32,
        x: i32,
        y: i32,
    ) {
        let registry = std::mem::take(&mut self.image_registry);
        let mut adapter = TimerOnlyMainState::from_render_context_with_images(ctx, &registry);
        self.mouse_pressed(&mut adapter, button, x, y);
        self.image_registry = registry;
    }

    fn mouse_dragged_at(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        button: i32,
        x: i32,
        y: i32,
    ) {
        let registry = std::mem::take(&mut self.image_registry);
        let mut adapter = TimerOnlyMainState::from_render_context_with_images(ctx, &registry);
        self.mouse_dragged(&mut adapter, button, x, y);
        self.image_registry = registry;
    }

    fn dispose_skin(&mut self) {
        self.dispose();
    }

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
        self.width
    }

    fn get_height(&self) -> f32 {
        self.height
    }

    fn swap_sprite_batch(&mut self, batch: &mut rubato_render::sprite_batch::SpriteBatch) {
        if self.renderer.is_none() {
            self.renderer = Some(SkinObjectRenderer::new());
        }
        std::mem::swap(
            &mut self.renderer.as_mut().expect("renderer is Some").sprite,
            batch,
        );
    }
}

