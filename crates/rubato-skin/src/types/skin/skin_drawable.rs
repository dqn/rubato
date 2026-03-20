/// Adapter that provides timer data to skin objects via the MainState trait.
/// Used by SkinDrawable to bridge beatoraja-core's TimerManager to beatoraja-skin's internal interface.
///
/// Holds a reference to the real `TimerAccess` (typically a `TimerManager`) so that
/// per-timer-id queries return actual values instead of always 0.
struct TimerOnlyMainState<'a> {
    timer: Option<&'a dyn rubato_types::timer_access::TimerAccess>,
    ctx: Option<&'a mut dyn rubato_types::skin_render_context::SkinRenderContext>,
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
            state_type,
            image_registry,
        }
    }
}

impl rubato_types::timer_access::TimerAccess for TimerOnlyMainState<'_> {
    fn now_time(&self) -> i64 {
        if let Some(ctx) = self.ctx.as_deref() {
            ctx.now_time()
        } else {
            self.timer
                .expect("timer-only adapter must carry a timer")
                .now_time()
        }
    }
    fn now_micro_time(&self) -> i64 {
        if let Some(ctx) = self.ctx.as_deref() {
            ctx.now_micro_time()
        } else {
            self.timer
                .expect("timer-only adapter must carry a timer")
                .now_micro_time()
        }
    }
    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        if let Some(ctx) = self.ctx.as_deref() {
            ctx.micro_timer(timer_id)
        } else {
            self.timer
                .expect("timer-only adapter must carry a timer")
                .micro_timer(timer_id)
        }
    }
    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        if let Some(ctx) = self.ctx.as_deref() {
            ctx.timer(timer_id)
        } else {
            self.timer
                .expect("timer-only adapter must carry a timer")
                .timer(timer_id)
        }
    }
    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        if let Some(ctx) = self.ctx.as_deref() {
            ctx.now_time_for(timer_id)
        } else {
            self.timer
                .expect("timer-only adapter must carry a timer")
                .now_time_for(timer_id)
        }
    }
    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        if let Some(ctx) = self.ctx.as_deref() {
            ctx.is_timer_on(timer_id)
        } else {
            self.timer
                .expect("timer-only adapter must carry a timer")
                .is_timer_on(timer_id)
        }
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for TimerOnlyMainState<'_> {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        self.state_type
    }

    fn is_music_selector(&self) -> bool {
        self.ctx
            .as_deref()
            .is_some_and(rubato_types::skin_render_context::SkinRenderContext::is_music_selector)
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

    fn gauge_value(&self) -> f32 {
        self.ctx.as_deref().map_or(0.0, |c| c.gauge_value())
    }

    fn gauge_type(&self) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.gauge_type())
    }

    fn is_mode_changed(&self) -> bool {
        self.ctx
            .as_deref()
            .is_some_and(|c| c.is_mode_changed())
    }

    fn gauge_element_borders(&self) -> Vec<(f32, f32)> {
        self.ctx
            .as_deref()
            .map_or_else(Vec::new, |c| c.gauge_element_borders())
    }

    fn now_judge(&self, player: i32) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.now_judge(player))
    }

    fn now_combo(&self, player: i32) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.now_combo(player))
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        self.ctx
            .as_deref()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::player_config_ref)
    }

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        self.ctx
            .as_deref()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::config_ref)
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        self.ctx
            .as_deref_mut()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::player_config_mut)
    }

    fn config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
        self.ctx
            .as_deref_mut()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::config_mut)
    }

    fn selected_play_config_mut(
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

    fn get_offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.ctx
            .as_deref()
            .and_then(|c| c.get_offset_value(id))
    }
}

impl crate::reexports::MainState for TimerOnlyMainState<'_> {
    fn skin_image(&self, id: i32) -> Option<crate::render_reexports::TextureRegion> {
        self.image_registry.get(&id).cloned()
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

    fn skin_offsets(&self) -> std::collections::HashMap<i32, rubato_types::skin_offset::SkinOffset> {
        self.offset
            .iter()
            .map(|(&id, cfg)| {
                (
                    id,
                    rubato_types::skin_offset::SkinOffset {
                        x: cfg.x,
                        y: cfg.y,
                        w: cfg.w,
                        h: cfg.h,
                        r: cfg.r,
                        a: cfg.a,
                    },
                )
            })
            .collect()
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

    fn execute_custom_event(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        id: i32,
        arg1: i32,
        arg2: i32,
    ) {
        let registry = std::mem::take(&mut self.image_registry);
        let mut adapter = TimerOnlyMainState::from_render_context_with_images(ctx, &registry);
        Skin::execute_custom_event(self, &mut adapter, id, arg1, arg2);
        self.image_registry = registry;
    }

    fn offset_entries(&self) -> Vec<(i32, rubato_types::skin_offset::SkinOffset)> {
        self.offset
            .iter()
            .map(|(&id, cfg)| {
                (
                    id,
                    rubato_types::skin_offset::SkinOffset {
                        x: cfg.x,
                        y: cfg.y,
                        w: cfg.w,
                        h: cfg.h,
                        r: cfg.r,
                        a: cfg.a,
                    },
                )
            })
            .collect()
    }

    fn compute_note_draw_commands(
        &mut self,
        lane_renderer: &mut dyn std::any::Any,
        ctx: Box<dyn std::any::Any>,
    ) {
        let Some(lr) = lane_renderer.downcast_mut::<rubato_play::lane_renderer::LaneRenderer>()
        else {
            log::warn!("compute_note_draw_commands: LaneRenderer downcast failed");
            return;
        };
        let Ok(ctx) = ctx.downcast::<rubato_play::lane_renderer::DrawLaneContext>() else {
            log::warn!("compute_note_draw_commands: DrawLaneContext downcast failed");
            return;
        };
        for obj in &mut self.objects {
            if let SkinObject::Note(note) = obj {
                let lanes = note.inner.lanes();
                let result = lr.draw_lane(&ctx, lanes, &[]);
                note.draw_commands = result.commands;
                return;
            }
        }
        log::warn!(
            "compute_note_draw_commands: no SkinObject::Note found in {} objects",
            self.objects.len()
        );
    }
}

