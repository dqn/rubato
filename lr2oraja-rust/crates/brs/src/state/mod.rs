// Game state handler trait and state modules.
//
// Corresponds to Java MainState abstract class.

pub mod course_result;
mod course_result_skin_state;
pub mod decide;
mod decide_skin_state;
mod ir_submission;
pub mod key_config;
pub mod play;
pub mod result;
mod result_skin_state;
pub mod select;
pub mod skin_config;

use crate::app_state::AppStateType;
use crate::database_manager::DatabaseManager;
use crate::game_state::SharedGameState;
use crate::input_mapper::InputState;
use crate::player_resource::PlayerResource;
use crate::preview_music::PreviewMusicProcessor;
use crate::skin_manager::SkinManager;
use crate::system_sound::SystemSoundManager;
use crate::timer_manager::TimerManager;
use bms_config::{Config, PlayerConfig};
use bms_input::keyboard::KeyboardBackend;

/// Context passed to state handlers on each callback.
pub struct StateContext<'a> {
    pub timer: &'a mut TimerManager,
    pub resource: &'a mut PlayerResource,
    #[allow(dead_code)] // TODO: integrate with state handlers needing config
    pub config: &'a Config,
    pub player_config: &'a mut PlayerConfig,
    /// Set this to request a state transition at the end of the frame.
    pub transition: &'a mut Option<AppStateType>,
    /// Keyboard backend for input polling (None in tests or non-Bevy contexts).
    pub keyboard_backend: Option<&'a dyn KeyboardBackend>,
    /// Database connections (None when DB is not available).
    pub database: Option<&'a DatabaseManager>,
    /// Input state for the current frame (control keys + commands).
    pub input_state: Option<&'a InputState>,
    /// Skin loading manager (None in tests or when skin system not available).
    #[allow(dead_code)] // TODO: integrate with skin load system
    pub skin_manager: Option<&'a mut SkinManager>,
    /// System sound playback manager (None in tests or when audio not available).
    #[allow(dead_code)] // TODO: integrate with system sound playback
    pub sound_manager: Option<&'a mut SystemSoundManager>,
    /// Characters typed this frame (from Bevy KeyboardInput events).
    pub received_chars: &'a [char],
    /// Bevy image assets for BGA loading (None in tests or when not available).
    pub bevy_images: Option<&'a mut bevy::prelude::Assets<bevy::prelude::Image>>,
    /// Shared game state for skin property synchronization (None in tests).
    #[allow(dead_code)] // TODO: integrate with remaining state handlers
    pub shared_state: Option<&'a mut SharedGameState>,
    /// Preview music processor for select screen (None in tests or non-select states).
    pub preview_music: Option<&'a mut PreviewMusicProcessor>,
}

/// Trait for game state handlers. Each variant of `AppStateType` has
/// a corresponding implementation.
///
/// Lifecycle: `create` -> `prepare` -> (`render` + `input`)* -> `shutdown` -> `dispose`
pub trait GameStateHandler: Send + Sync {
    /// Called when entering this state (after previous state's shutdown).
    fn create(&mut self, ctx: &mut StateContext);

    /// Called once after `create`, before the first frame.
    fn prepare(&mut self, _ctx: &mut StateContext) {}

    /// Called every frame. Update timers, check transitions.
    fn render(&mut self, ctx: &mut StateContext);

    /// Called every frame for input processing.
    fn input(&mut self, _ctx: &mut StateContext) {}

    /// Called when leaving this state (before next state's create).
    fn shutdown(&mut self, _ctx: &mut StateContext) {}

    /// Called for final cleanup (resource deallocation).
    #[allow(dead_code)] // Parsed for completeness (Java MainState lifecycle)
    fn dispose(&mut self) {}
}
