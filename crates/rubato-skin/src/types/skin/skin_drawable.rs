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

    fn boot_time_millis(&self) -> i64 {
        self.ctx.as_deref().map_or(0, |c| c.boot_time_millis())
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

    fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
        static DEFAULT: std::sync::OnceLock<rubato_types::score_data_property::ScoreDataProperty> =
            std::sync::OnceLock::new();
        self.ctx
            .as_deref()
            .map(|c| c.score_data_property())
            .unwrap_or_else(|| {
                DEFAULT.get_or_init(rubato_types::score_data_property::ScoreDataProperty::default)
            })
    }

    fn get_offset_value(
        &self,
        id: i32,
    ) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.ctx.as_deref().and_then(|c| c.get_offset_value(id))
    }

    fn get_distribution_data(
        &self,
    ) -> Option<rubato_types::distribution_data::DistributionData> {
        self.ctx.as_deref().and_then(|c| c.get_distribution_data())
    }

    fn gauge_history(&self) -> Option<&Vec<Vec<f32>>> {
        self.ctx.as_deref().and_then(|c| c.gauge_history())
    }

    fn course_gauge_history(&self) -> &[Vec<Vec<f32>>] {
        self.ctx
            .as_deref()
            .map_or(&[] as &[Vec<Vec<f32>>], |c| c.course_gauge_history())
    }

    fn gauge_border_max(&self) -> Option<(f32, f32)> {
        self.ctx.as_deref().and_then(|c| c.gauge_border_max())
    }

    fn gauge_min(&self) -> f32 {
        self.ctx.as_deref().map_or(0.0, |c| c.gauge_min())
    }

    fn is_gauge_max(&self) -> bool {
        self.ctx.as_deref().is_some_and(|c| c.is_gauge_max())
    }

    fn result_gauge_type(&self) -> i32 {
        self.ctx
            .as_deref()
            .map_or_else(|| self.gauge_type(), |c| c.result_gauge_type())
    }

    fn is_media_load_finished(&self) -> bool {
        self.ctx
            .as_deref()
            .is_some_and(|c| c.is_media_load_finished())
    }

    fn is_practice_mode(&self) -> bool {
        self.ctx.as_deref().is_some_and(|c| c.is_practice_mode())
    }

    fn get_timing_distribution(
        &self,
    ) -> Option<&rubato_types::timing_distribution::TimingDistribution> {
        self.ctx
            .as_deref()
            .and_then(|c| c.get_timing_distribution())
    }

    fn judge_area(&self) -> Option<Vec<Vec<i32>>> {
        self.ctx.as_deref().and_then(|c| c.judge_area())
    }

    fn prepare_fps(&self) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.prepare_fps())
    }

    fn is_debug(&self) -> bool {
        self.ctx.as_deref().is_some_and(|c| c.is_debug())
    }

    fn mouse_x(&self) -> f32 {
        self.ctx.as_deref().map_or(0.0, |c| c.mouse_x())
    }

    fn mouse_y(&self) -> f32 {
        self.ctx.as_deref().map_or(0.0, |c| c.mouse_y())
    }

    fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        self.ctx.as_deref().and_then(|c| c.replay_option_data())
    }

    fn target_score_data(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.ctx.as_deref().and_then(|c| c.target_score_data())
    }

    fn score_data_ref(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.ctx.as_deref().and_then(|c| c.score_data_ref())
    }

    fn rival_score_data_ref(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.ctx.as_deref().and_then(|c| c.rival_score_data_ref())
    }

    fn ranking_score_clear_type(&self, slot: i32) -> i32 {
        self.ctx
            .as_deref()
            .map_or(-1, |c| c.ranking_score_clear_type(slot))
    }

    fn ranking_offset(&self) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.ranking_offset())
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        self.ctx
            .as_deref()
            .and_then(|c| c.current_play_config_ref())
    }

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        self.ctx.as_deref().and_then(|c| c.song_data_ref())
    }

    fn lane_shuffle_pattern_value(&self, player: usize, lane: usize) -> i32 {
        self.ctx
            .as_deref()
            .map_or(-1, |c| c.lane_shuffle_pattern_value(player, lane))
    }

    fn mode_image_index(&self) -> Option<i32> {
        self.ctx.as_deref().and_then(|c| c.mode_image_index())
    }

    fn sort_image_index(&self) -> Option<i32> {
        self.ctx.as_deref().and_then(|c| c.sort_image_index())
    }

    fn notify_audio_config_changed(&mut self) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.notify_audio_config_changed();
        }
    }

    fn select_song_mode(&mut self, event_id: i32) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.select_song_mode(event_id);
        }
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

#[cfg(test)]
mod skin_drawable_delegation_tests {
    use super::*;
    use rubato_types::main_state_type::MainStateType;
    use rubato_types::skin_render_context::SkinRenderContext;
    use rubato_types::timer_access::TimerAccess;
    use rubato_types::timer_id::TimerId;

    /// A mock SkinRenderContext that returns distinctive non-default values
    /// for every method. Used to verify TimerOnlyMainState delegates each
    /// method to the wrapped context instead of silently returning trait defaults.
    struct FullMockContext {
        score: rubato_types::score_data::ScoreData,
        replay: rubato_types::replay_data::ReplayData,
        song: rubato_types::song_data::SongData,
        play_config: rubato_types::play_config::PlayConfig,
        timing_dist: rubato_types::timing_distribution::TimingDistribution,
        score_prop: rubato_types::score_data_property::ScoreDataProperty,
        gauge_hist: Vec<Vec<f32>>,
        course_gauge_hist: Vec<Vec<Vec<f32>>>,
    }

    impl FullMockContext {
        fn new() -> Self {
            let mut score = rubato_types::score_data::ScoreData::default();
            score.clear = 7;

            Self {
                score,
                replay: rubato_types::replay_data::ReplayData::default(),
                song: rubato_types::song_data::SongData::default(),
                play_config: rubato_types::play_config::PlayConfig::default(),
                timing_dist: rubato_types::timing_distribution::TimingDistribution::default(),
                score_prop: rubato_types::score_data_property::ScoreDataProperty::default(),
                gauge_hist: vec![vec![0.5, 0.6]],
                course_gauge_hist: vec![vec![vec![0.1, 0.2]]],
            }
        }
    }

    impl TimerAccess for FullMockContext {
        fn now_time(&self) -> i64 {
            42
        }
        fn now_micro_time(&self) -> i64 {
            42_000
        }
        fn micro_timer(&self, _: TimerId) -> i64 {
            i64::MIN
        }
        fn timer(&self, _: TimerId) -> i64 {
            i64::MIN
        }
        fn now_time_for(&self, _: TimerId) -> i64 {
            i64::MIN
        }
        fn is_timer_on(&self, _: TimerId) -> bool {
            false
        }
    }

    impl SkinRenderContext for FullMockContext {
        fn current_state_type(&self) -> Option<MainStateType> {
            Some(MainStateType::Result)
        }

        fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
            Some(&self.replay)
        }

        fn target_score_data(&self) -> Option<&rubato_types::score_data::ScoreData> {
            Some(&self.score)
        }

        fn score_data_ref(&self) -> Option<&rubato_types::score_data::ScoreData> {
            Some(&self.score)
        }

        fn rival_score_data_ref(&self) -> Option<&rubato_types::score_data::ScoreData> {
            Some(&self.score)
        }

        fn ranking_score_clear_type(&self, slot: i32) -> i32 {
            slot + 100
        }

        fn ranking_offset(&self) -> i32 {
            5
        }

        fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
            Some(&self.play_config)
        }

        fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
            Some(&self.song)
        }

        fn lane_shuffle_pattern_value(&self, player: usize, lane: usize) -> i32 {
            (player * 10 + lane) as i32
        }

        fn mode_image_index(&self) -> Option<i32> {
            Some(3)
        }

        fn sort_image_index(&self) -> Option<i32> {
            Some(7)
        }

        fn mouse_x(&self) -> f32 {
            123.0
        }

        fn mouse_y(&self) -> f32 {
            456.0
        }

        fn prepare_fps(&self) -> i32 {
            30
        }

        fn is_debug(&self) -> bool {
            true
        }

        fn get_timing_distribution(
            &self,
        ) -> Option<&rubato_types::timing_distribution::TimingDistribution> {
            Some(&self.timing_dist)
        }

        fn judge_area(&self) -> Option<Vec<Vec<i32>>> {
            Some(vec![vec![10, 20, 30]])
        }

        fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
            &self.score_prop
        }

        fn gauge_history(&self) -> Option<&Vec<Vec<f32>>> {
            Some(&self.gauge_hist)
        }

        fn course_gauge_history(&self) -> &[Vec<Vec<f32>>] {
            &self.course_gauge_hist
        }

        fn gauge_border_max(&self) -> Option<(f32, f32)> {
            Some((0.8, 1.0))
        }

        fn gauge_min(&self) -> f32 {
            0.02
        }

        fn result_gauge_type(&self) -> i32 {
            99
        }

        fn is_gauge_max(&self) -> bool {
            true
        }

        fn is_media_load_finished(&self) -> bool {
            true
        }

        fn is_practice_mode(&self) -> bool {
            true
        }

        fn get_distribution_data(
            &self,
        ) -> Option<rubato_types::distribution_data::DistributionData> {
            Some(rubato_types::distribution_data::DistributionData::default())
        }
    }

    /// Regression test: verify that TimerOnlyMainState delegates ALL SkinRenderContext
    /// methods to the wrapped context. Before this fix, many result-screen and data
    /// accessor methods were missing, causing skin objects like SkinGaugeGraphObject
    /// to silently receive default/empty values.
    #[test]
    fn test_timer_only_main_state_delegates_all_skin_render_context_methods() {
        let registry = HashMap::new();
        let mut ctx = FullMockContext::new();
        let adapter = TimerOnlyMainState::from_render_context_with_images(&mut ctx, &registry);

        // -- Data accessors (Option<&T>) --
        assert!(
            adapter.replay_option_data().is_some(),
            "replay_option_data must delegate"
        );
        assert!(
            adapter.target_score_data().is_some(),
            "target_score_data must delegate"
        );
        assert_eq!(
            adapter.score_data_ref().unwrap().clear,
            7,
            "score_data_ref must delegate"
        );
        assert!(
            adapter.rival_score_data_ref().is_some(),
            "rival_score_data_ref must delegate"
        );
        assert!(
            adapter.current_play_config_ref().is_some(),
            "current_play_config_ref must delegate"
        );
        assert!(
            adapter.song_data_ref().is_some(),
            "song_data_ref must delegate"
        );
        assert!(
            adapter.get_timing_distribution().is_some(),
            "get_timing_distribution must delegate"
        );
        assert!(
            adapter.get_distribution_data().is_some(),
            "get_distribution_data must delegate"
        );

        // -- Scalar values --
        assert_eq!(
            adapter.ranking_score_clear_type(2),
            102,
            "ranking_score_clear_type must delegate"
        );
        assert_eq!(
            adapter.ranking_offset(),
            5,
            "ranking_offset must delegate"
        );
        assert_eq!(
            adapter.lane_shuffle_pattern_value(1, 3),
            13,
            "lane_shuffle_pattern_value must delegate"
        );
        assert_eq!(
            adapter.mode_image_index(),
            Some(3),
            "mode_image_index must delegate"
        );
        assert_eq!(
            adapter.sort_image_index(),
            Some(7),
            "sort_image_index must delegate"
        );
        assert_eq!(adapter.mouse_x(), 123.0, "mouse_x must delegate");
        assert_eq!(adapter.mouse_y(), 456.0, "mouse_y must delegate");
        assert_eq!(adapter.prepare_fps(), 30, "prepare_fps must delegate");
        assert_eq!(adapter.gauge_min(), 0.02, "gauge_min must delegate");
        assert_eq!(
            adapter.result_gauge_type(),
            99,
            "result_gauge_type must delegate"
        );

        // -- Boolean values --
        assert!(adapter.is_debug(), "is_debug must delegate");
        assert!(adapter.is_gauge_max(), "is_gauge_max must delegate");
        assert!(
            adapter.is_media_load_finished(),
            "is_media_load_finished must delegate"
        );
        assert!(
            adapter.is_practice_mode(),
            "is_practice_mode must delegate"
        );

        // -- Complex return types --
        assert_eq!(
            adapter.judge_area(),
            Some(vec![vec![10, 20, 30]]),
            "judge_area must delegate"
        );
        assert!(
            adapter.gauge_history().is_some(),
            "gauge_history must delegate"
        );
        assert_eq!(
            adapter.gauge_history().unwrap().len(),
            1,
            "gauge_history must delegate with correct data"
        );
        assert_eq!(
            adapter.course_gauge_history().len(),
            1,
            "course_gauge_history must delegate"
        );
        assert_eq!(
            adapter.gauge_border_max(),
            Some((0.8, 1.0)),
            "gauge_border_max must delegate"
        );
        // score_data_property returns a reference; just verify it doesn't panic
        let _ = adapter.score_data_property();
    }

    /// Verify that when ctx is None (timer-only mode), all methods return
    /// their trait defaults and do not panic.
    #[test]
    fn test_timer_only_mode_returns_defaults_for_all_methods() {
        let timer = rubato_types::timer_access::NullTimer;
        let adapter = TimerOnlyMainState::from_timer(&timer);

        assert!(adapter.replay_option_data().is_none());
        assert!(adapter.target_score_data().is_none());
        assert!(adapter.score_data_ref().is_none());
        assert!(adapter.rival_score_data_ref().is_none());
        assert!(adapter.current_play_config_ref().is_none());
        assert!(adapter.song_data_ref().is_none());
        assert!(adapter.get_timing_distribution().is_none());
        assert!(adapter.get_distribution_data().is_none());

        assert_eq!(adapter.ranking_score_clear_type(0), -1);
        assert_eq!(adapter.ranking_offset(), 0);
        assert_eq!(adapter.lane_shuffle_pattern_value(0, 0), -1);
        assert_eq!(adapter.mode_image_index(), None);
        assert_eq!(adapter.sort_image_index(), None);
        assert_eq!(adapter.mouse_x(), 0.0);
        assert_eq!(adapter.mouse_y(), 0.0);
        assert_eq!(adapter.prepare_fps(), 0);
        assert_eq!(adapter.gauge_min(), 0.0);
        // result_gauge_type falls back to gauge_type() which returns 0
        assert_eq!(adapter.result_gauge_type(), 0);

        assert!(!adapter.is_debug());
        assert!(!adapter.is_gauge_max());
        assert!(!adapter.is_media_load_finished());
        assert!(!adapter.is_practice_mode());

        assert!(adapter.judge_area().is_none());
        assert!(adapter.gauge_history().is_none());
        assert!(adapter.course_gauge_history().is_empty());
        assert!(adapter.gauge_border_max().is_none());
    }

    /// Verify mutable context methods (notify_audio_config_changed, select_song_mode)
    /// delegate through without panicking.
    #[test]
    fn test_mutable_methods_delegate_without_panic() {
        let registry = HashMap::new();
        let mut ctx = FullMockContext::new();
        let mut adapter =
            TimerOnlyMainState::from_render_context_with_images(&mut ctx, &registry);

        // These should delegate to no-op defaults on FullMockContext without panicking
        adapter.notify_audio_config_changed();
        adapter.select_song_mode(15);
    }
}

