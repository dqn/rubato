// Shared test helpers for music_result and course_result test modules.
// Contains mock types that are identical between both result screen test suites.

use crate::core::main_state::SkinDrawable;
use crate::core::sprite_batch_helper::SpriteBatch;
use std::time::{SystemTime, UNIX_EPOCH};

/// Mock skin that fires execute_event with a configurable event_id on mouse press.
pub struct ExecuteEventSkin {
    pub event_id: i32,
}

impl SkinDrawable for ExecuteEventSkin {
    fn draw_all_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_skin::skin_render_context::SkinRenderContext,
    ) {
    }

    fn update_custom_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_skin::skin_render_context::SkinRenderContext,
    ) {
    }

    fn mouse_pressed_at(
        &mut self,
        ctx: &mut dyn rubato_skin::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
        ctx.execute_event(self.event_id, 0, 0);
    }

    fn mouse_dragged_at(
        &mut self,
        _ctx: &mut dyn rubato_skin::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
    }

    fn prepare_skin(&mut self, _state_type: Option<rubato_skin::main_state_type::MainStateType>) {}

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

/// Mock skin that mutates player_config on mouse press (increments random).
pub struct PlayerConfigMutatingSkin;

impl SkinDrawable for PlayerConfigMutatingSkin {
    fn draw_all_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_skin::skin_render_context::SkinRenderContext,
    ) {
    }

    fn update_custom_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_skin::skin_render_context::SkinRenderContext,
    ) {
    }

    fn mouse_pressed_at(
        &mut self,
        ctx: &mut dyn rubato_skin::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
        if let Some(config) = ctx.player_config_mut() {
            config.play_settings.random = (config.play_settings.random + 1) % 10;
        }
    }

    fn mouse_dragged_at(
        &mut self,
        _ctx: &mut dyn rubato_skin::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
    }

    fn prepare_skin(&mut self, _state_type: Option<rubato_skin::main_state_type::MainStateType>) {}

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

/// Create a test Config with a unique temp directory for player data.
pub fn make_test_config(label: &str) -> rubato_skin::config::Config {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let mut config = rubato_skin::config::Config::default();
    let player_dir = std::env::temp_dir().join(format!("rubato-{label}-{unique}"));
    config.paths.playerpath = player_dir.to_string_lossy().into_owned();
    config.playername = Some(format!("test-{label}"));
    config
}
