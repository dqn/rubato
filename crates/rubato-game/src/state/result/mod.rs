// MainController and PlayerResource wrappers
pub mod main_controller_wrapper;
pub mod player_resource_wrapper;

// Re-exports
pub use crate::core::bms_player_mode::BMSPlayerMode;
pub use crate::core::bms_player_mode::Mode as BMSPlayerModeType;
pub use crate::core::play_data_accessor::PlayDataAccessor;
pub use crate::core::timer_manager::TimerManager;
pub use crate::ir::ir_connection::IRConnection;
pub use crate::ir::ir_course_data::IRCourseData;
pub use crate::ir::ir_score_data::IRScoreData;
pub use crate::ir::ranking_data::RankingData;
pub use rubato_input::key_command::KeyCommand;
pub use rubato_input::keyboard_input_processor::ControlKeys;
pub use rubato_skin::reexports::Color;
pub use rubato_skin::reexports::Pixmap;
pub use rubato_skin::reexports::PixmapFormat;
pub use rubato_skin::reexports::Rectangle;
pub use rubato_skin::reexports::Texture;
pub use rubato_skin::reexports::TextureRegion;
pub use rubato_skin::skin::Skin;
pub use rubato_skin::skin_header::SkinHeader;
pub use rubato_skin::skin_object::SkinObjectData;
pub use rubato_skin::skin_object::SkinObjectRenderer;
pub use rubato_types::groove_gauge::GrooveGauge;
pub use rubato_types::main_controller_access::{MainControllerAccess, NullMainController};
pub use rubato_types::player_resource_access::{NullPlayerResource, PlayerResourceAccess};

// Convenience re-exports for the wrapper types
pub use main_controller_wrapper::MainController;
pub use player_resource_wrapper::PlayerResource;

// FreqTrainerMenu re-export
pub use crate::state::modmenu::freq_trainer_menu::FreqTrainerMenu;

// IR types
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
#[allow(unused_macros)]
macro_rules! impl_result_main_state {
    (
        $result_type:ty,
        $state_variant:ident
    ) => {
        fn state_type(&self) -> Option<crate::core::main_state::MainStateType> {
            Some(crate::core::main_state::MainStateType::$state_variant)
        }

        fn main_state_data(&self) -> &crate::core::main_state::MainStateData {
            &self.main_data
        }

        fn main_state_data_mut(&mut self) -> &mut crate::core::main_state::MainStateData {
            &mut self.main_data
        }

        fn groove_gauge_value(&self) -> Option<f32> {
            self.resource.groove_gauge().map(|g| g.value())
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

            let mut snapshot = self.build_snapshot(&timer);
            skin.update_custom_objects_timed(&mut snapshot);
            skin.swap_sprite_batch(sprite);
            skin.draw_all_objects_timed(&mut snapshot);
            skin.swap_sprite_batch(sprite);

            // Drain non-event actions (timers, audio, state changes)
            self.drain_actions(&mut snapshot.actions, &mut timer);

            // Replay queued custom events now that the skin is available again.
            let mut pending_events = std::mem::take(&mut snapshot.actions.custom_events);
            let mut depth = 0;
            while !pending_events.is_empty() && depth < 8 {
                let mut replay_snapshot = self.build_snapshot(&timer);
                for (id, arg1, arg2) in pending_events {
                    skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
                }
                self.drain_actions(&mut replay_snapshot.actions, &mut timer);
                pending_events = replay_snapshot.actions.custom_events;
                depth += 1;
            }
            if depth >= 8 {
                log::warn!(
                    "{} render_skin event replay exceeded depth limit",
                    stringify!($state_variant)
                );
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
                let mut snapshot = self.build_snapshot(&timer);
                skin.mouse_pressed_at(&mut snapshot, button, x, y);
                self.drain_actions(&mut snapshot.actions, &mut timer);

                // Replay queued custom events with depth limit
                let mut pending_events = std::mem::take(&mut snapshot.actions.custom_events);
                let mut depth = 0;
                while !pending_events.is_empty() && depth < 8 {
                    let mut replay_snapshot = self.build_snapshot(&timer);
                    for (id, arg1, arg2) in pending_events {
                        skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
                    }
                    self.drain_actions(&mut replay_snapshot.actions, &mut timer);
                    pending_events = replay_snapshot.actions.custom_events;
                    depth += 1;
                }
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
                let mut snapshot = self.build_snapshot(&timer);
                skin.mouse_dragged_at(&mut snapshot, button, x, y);
                self.drain_actions(&mut snapshot.actions, &mut timer);

                // Replay queued custom events with depth limit
                let mut pending_events = std::mem::take(&mut snapshot.actions.custom_events);
                let mut depth = 0;
                while !pending_events.is_empty() && depth < 8 {
                    let mut replay_snapshot = self.build_snapshot(&timer);
                    for (id, arg1, arg2) in pending_events {
                        skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
                    }
                    self.drain_actions(&mut replay_snapshot.actions, &mut timer);
                    pending_events = replay_snapshot.actions.custom_events;
                    depth += 1;
                }
            }

            self.main_data.timer = timer;
            self.main_data.skin = Some(skin);
        }

        fn input(&mut self) {
            self.do_input();
        }

        fn sync_input_snapshot(&mut self, snapshot: &rubato_input::input_snapshot::InputSnapshot) {
            self.input_snapshot = Some(snapshot.clone());
        }

        fn load_skin(&mut self, skin_type: i32) {
            let skin_path = self
                .resource
                .player_config()
                .skin
                .get(skin_type as usize)
                .and_then(|skin| skin.as_ref())
                .and_then(|skin| skin.path.clone())
                .or_else(|| rubato_types::skin_config::SkinConfig::default_for_id(skin_type).path);
            let mut timer = std::mem::take(&mut self.main_data.timer);
            let loaded = {
                let mut snapshot = self.build_snapshot(&timer);
                let registry = std::collections::HashMap::new();
                let mut state = rubato_skin::snapshot_main_state::SnapshotMainState::new(
                    &mut snapshot,
                    &registry,
                );
                skin_path.as_deref().and_then(|path| {
                    rubato_skin::skin_loader::load_skin_from_path_with_state(
                        &mut state, skin_type, path,
                    )
                })
            };
            self.main_data.timer = timer;
            if let Some(skin) = loaded {
                self.skin = Some(
                    crate::state::result::result_skin_data::ResultSkinData::from_loaded_skin(&skin),
                );
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

#[allow(unused_imports)]
pub(crate) use impl_result_main_state;
