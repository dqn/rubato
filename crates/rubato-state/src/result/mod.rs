// Stubs for external dependencies
pub mod stubs;

// IR types (moved from stubs.rs)
pub mod ir_initializer;
pub mod ir_resend;
pub mod ir_send_status;
pub mod ir_status;

// Result screen modules
pub mod abstract_result;
pub mod course_result;
pub mod music_result;
pub(crate) mod result_common;
pub mod result_key_property;
pub mod result_skin_data;
pub(crate) mod shared_render_context;
pub mod skin_gauge_graph_object;
#[cfg(test)]
pub(crate) mod test_helpers;

// Backward-compatible re-exports: both old skin modules now alias ResultSkinData.
pub mod music_result_skin {
    pub type MusicResultSkin = super::result_skin_data::ResultSkinData;
}
pub mod course_result_skin {
    pub type CourseResultSkin = super::result_skin_data::ResultSkinData;
}

/// Generate the `MainState` trait implementation for a result screen type.
///
/// Both `MusicResult` and `CourseResult` share identical delegation patterns for
/// `main_state_data`, `create`, `prepare`, `render`, `input`, `sync_input_*`,
/// `load_skin`, `render_skin`, `handle_skin_mouse_*`, and `take_player_resource_box`.
///
/// Parameters:
/// - `$result_type`: the struct implementing MainState (e.g. `MusicResult`)
/// - `$state_variant`: the MainStateType variant (e.g. `Result` or `CourseResult`)
/// - `$render_ctx`: the render context struct (e.g. `ResultRenderContext`)
/// - `$mouse_ctx`: the mouse context struct (e.g. `ResultMouseContext`)
macro_rules! impl_result_main_state {
    (
        $result_type:ty,
        $state_variant:ident,
        $render_ctx:ident,
        $mouse_ctx:ident
    ) => {
        fn state_type(&self) -> Option<rubato_core::main_state::MainStateType> {
            Some(rubato_core::main_state::MainStateType::$state_variant)
        }

        fn main_state_data(&self) -> &rubato_core::main_state::MainStateData {
            &self.main_data
        }

        fn main_state_data_mut(&mut self) -> &mut rubato_core::main_state::MainStateData {
            &mut self.main_data
        }

        fn create(&mut self) {
            self.do_create();
        }

        fn prepare(&mut self) {
            self.do_prepare();
        }

        fn render(&mut self) {
            self.do_render();
        }

        fn render_skin(&mut self, sprite: &mut rubato_render::sprite_batch::SpriteBatch) {
            let mut skin = match self.main_data.skin.take() {
                Some(s) => s,
                None => return,
            };
            let mut timer = std::mem::take(&mut self.main_data.timer);

            {
                let mut ctx = $render_ctx {
                    timer: &mut timer,
                    data: &self.data,
                    resource: &self.resource,
                    main: &self.main,
                };
                skin.update_custom_objects_timed(&mut ctx);
                skin.swap_sprite_batch(sprite);
                skin.draw_all_objects_timed(&mut ctx);
                skin.swap_sprite_batch(sprite);
            }

            self.main_data.timer = timer;
            self.main_data.skin = Some(skin);
        }

        fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
            let mut skin = match self.main_data.skin.take() {
                Some(s) => s,
                None => return,
            };
            let mut timer = std::mem::take(&mut self.main_data.timer);

            {
                let mut ctx = $mouse_ctx {
                    timer: &mut timer,
                    result: self,
                };
                skin.mouse_pressed_at(&mut ctx, button, x, y);
            }

            self.main_data.timer = timer;
            self.main_data.skin = Some(skin);
        }

        fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
            let mut skin = match self.main_data.skin.take() {
                Some(s) => s,
                None => return,
            };
            let mut timer = std::mem::take(&mut self.main_data.timer);

            {
                let mut ctx = $mouse_ctx {
                    timer: &mut timer,
                    result: self,
                };
                skin.mouse_dragged_at(&mut ctx, button, x, y);
            }

            self.main_data.timer = timer;
            self.main_data.skin = Some(skin);
        }

        fn input(&mut self) {
            self.do_input();
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
            if let Some(skin) = rubato_skin::skin_loader::load_skin_from_config(
                self.main.config(),
                self.resource.player_config(),
                skin_type,
            ) {
                self.skin =
                    Some(crate::result::result_skin_data::ResultSkinData::from_loaded_skin(&skin));
                self.main_data.skin = Some(Box::new(skin));
            } else {
                self.skin = None;
                self.main_data.skin = None;
            }
        }

        fn take_player_resource_box(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
            self.resource.take_inner().map(|b| b.into_any_send())
        }
    };
}

pub(crate) use impl_result_main_state;
