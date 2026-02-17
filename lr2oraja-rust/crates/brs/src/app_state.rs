// AppStateType and StateRegistry — state machine for game states.
//
// Manages state transitions, calling shutdown/create/prepare on handlers.

use std::collections::HashMap;

use bevy::prelude::*;
use tracing::info;

use crate::database_manager::DatabaseManager;
use crate::game_state::SharedGameState;
use crate::input_mapper::InputState;
use crate::player_resource::PlayerResource;
use crate::preview_music::PreviewMusicProcessor;
use crate::skin_manager::SkinManager;
use crate::state::{GameStateHandler, StateContext};
use crate::system_sound::SystemSoundManager;
use crate::timer_manager::TimerManager;
use bms_config::{Config, PlayerConfig};

/// Identifies which game state is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppStateType {
    MusicSelect,
    Decide,
    Play,
    Result,
    CourseResult,
    KeyConfig,
    SkinConfig,
}

impl std::fmt::Display for AppStateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MusicSelect => write!(f, "MusicSelect"),
            Self::Decide => write!(f, "Decide"),
            Self::Play => write!(f, "Play"),
            Self::Result => write!(f, "Result"),
            Self::CourseResult => write!(f, "CourseResult"),
            Self::KeyConfig => write!(f, "KeyConfig"),
            Self::SkinConfig => write!(f, "SkinConfig"),
        }
    }
}

/// Parameters passed to `StateRegistry::tick` each frame.
pub struct TickParams<'a> {
    pub timer: &'a mut TimerManager,
    pub resource: &'a mut PlayerResource,
    pub config: &'a Config,
    pub player_config: &'a mut PlayerConfig,
    pub keyboard_backend: Option<&'a dyn bms_input::keyboard::KeyboardBackend>,
    pub database: Option<&'a DatabaseManager>,
    pub input_state: Option<&'a InputState>,
    pub skin_manager: Option<&'a mut SkinManager>,
    pub sound_manager: Option<&'a mut SystemSoundManager>,
    pub received_chars: &'a [char],
    /// Bevy image assets for BGA loading (None in tests or when not available).
    pub bevy_images: Option<&'a mut Assets<Image>>,
    /// Shared game state for skin property synchronization (None in tests).
    pub shared_state: Option<&'a mut SharedGameState>,
    /// Preview music processor for select screen (None in tests).
    pub preview_music: Option<&'a mut PreviewMusicProcessor>,
}

/// Registry of all state handlers with transition logic.
pub struct StateRegistry {
    current: AppStateType,
    handlers: HashMap<AppStateType, Box<dyn GameStateHandler>>,
    initialized: bool,
}

impl StateRegistry {
    /// Creates a new registry with the given initial state.
    /// Handlers must be registered via `register` before `tick`.
    pub fn new(initial: AppStateType) -> Self {
        Self {
            current: initial,
            handlers: HashMap::new(),
            initialized: false,
        }
    }

    /// Registers a handler for a state type.
    pub fn register(&mut self, state_type: AppStateType, handler: Box<dyn GameStateHandler>) {
        self.handlers.insert(state_type, handler);
    }

    /// Returns the current active state type.
    #[allow(dead_code)] // Used in tests
    pub fn current(&self) -> AppStateType {
        self.current
    }

    /// Runs one frame: initializes if needed, processes render + input,
    /// then handles any pending transition.
    pub fn tick(&mut self, params: &mut TickParams) {
        let mut transition: Option<AppStateType> = None;

        // First-time initialization
        if !self.initialized {
            self.initialized = true;
            info!(state = %self.current, "Initializing state");
            params.timer.reset();
            if let Some(handler) = self.handlers.get_mut(&self.current) {
                let mut ctx = StateContext {
                    timer: params.timer,
                    resource: params.resource,
                    config: params.config,
                    player_config: params.player_config,
                    transition: &mut transition,
                    keyboard_backend: params.keyboard_backend,
                    database: params.database,
                    input_state: params.input_state,
                    skin_manager: params.skin_manager.as_deref_mut(),
                    sound_manager: params.sound_manager.as_deref_mut(),
                    received_chars: params.received_chars,
                    bevy_images: params.bevy_images.as_deref_mut(),
                    shared_state: params.shared_state.as_deref_mut(),
                    preview_music: params.preview_music.as_deref_mut(),
                };
                handler.create(&mut ctx);
                handler.prepare(&mut ctx);
            }
        }

        // Run current state's render and input
        if let Some(handler) = self.handlers.get_mut(&self.current) {
            let mut ctx = StateContext {
                timer: params.timer,
                resource: params.resource,
                config: params.config,
                player_config: params.player_config,
                transition: &mut transition,
                keyboard_backend: params.keyboard_backend,
                database: params.database,
                input_state: params.input_state,
                skin_manager: params.skin_manager.as_deref_mut(),
                sound_manager: params.sound_manager.as_deref_mut(),
                received_chars: params.received_chars,
                bevy_images: params.bevy_images.as_deref_mut(),
                shared_state: params.shared_state.as_deref_mut(),
                preview_music: params.preview_music.as_deref_mut(),
            };
            handler.render(&mut ctx);
            handler.input(&mut ctx);
        }

        // Handle pending transition
        if let Some(next) = transition {
            self.change_state(next, params);
        }
    }

    /// Performs a state transition: shutdown current -> reset timer -> create+prepare next.
    fn change_state(&mut self, next: AppStateType, params: &mut TickParams) {
        info!(from = %self.current, to = %next, "State transition");

        let mut dummy_transition: Option<AppStateType> = None;

        // Shutdown current state
        if let Some(handler) = self.handlers.get_mut(&self.current) {
            let mut ctx = StateContext {
                timer: params.timer,
                resource: params.resource,
                config: params.config,
                player_config: params.player_config,
                transition: &mut dummy_transition,
                keyboard_backend: params.keyboard_backend,
                database: params.database,
                input_state: params.input_state,
                skin_manager: params.skin_manager.as_deref_mut(),
                sound_manager: params.sound_manager.as_deref_mut(),
                received_chars: params.received_chars,
                bevy_images: params.bevy_images.as_deref_mut(),
                shared_state: params.shared_state.as_deref_mut(),
                preview_music: params.preview_music.as_deref_mut(),
            };
            handler.shutdown(&mut ctx);
        }

        // Reset timer for new state
        params.timer.reset();

        self.current = next;

        // Create and prepare new state
        if let Some(handler) = self.handlers.get_mut(&self.current) {
            let mut ctx = StateContext {
                timer: params.timer,
                resource: params.resource,
                config: params.config,
                player_config: params.player_config,
                transition: &mut dummy_transition,
                keyboard_backend: params.keyboard_backend,
                database: params.database,
                input_state: params.input_state,
                skin_manager: params.skin_manager.as_deref_mut(),
                sound_manager: params.sound_manager.as_deref_mut(),
                received_chars: params.received_chars,
                bevy_images: params.bevy_images.as_deref_mut(),
                shared_state: params.shared_state.as_deref_mut(),
                preview_music: params.preview_music.as_deref_mut(),
            };
            handler.create(&mut ctx);
            handler.prepare(&mut ctx);
        }

        // If the new state's create requested another transition, handle it
        // (e.g., MusicSelect immediately transitions to Decide)
        if let Some(chained) = dummy_transition {
            self.change_state(chained, params);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::GameStateHandler;
    use std::sync::Arc;

    use parking_lot::Mutex;

    /// Test helper that records lifecycle calls.
    #[derive(Default)]
    struct RecordingHandler {
        log: Arc<Mutex<Vec<String>>>,
        transition_on_create: Option<AppStateType>,
    }

    impl GameStateHandler for RecordingHandler {
        fn create(&mut self, ctx: &mut StateContext) {
            self.log.lock().push("create".to_string());
            if let Some(next) = self.transition_on_create {
                *ctx.transition = Some(next);
            }
        }
        fn prepare(&mut self, _ctx: &mut StateContext) {
            self.log.lock().push("prepare".to_string());
        }
        fn render(&mut self, _ctx: &mut StateContext) {
            self.log.lock().push("render".to_string());
        }
        fn input(&mut self, _ctx: &mut StateContext) {
            self.log.lock().push("input".to_string());
        }
        fn shutdown(&mut self, _ctx: &mut StateContext) {
            self.log.lock().push("shutdown".to_string());
        }
    }

    fn make_params<'a>(
        timer: &'a mut TimerManager,
        resource: &'a mut PlayerResource,
        config: &'a Config,
        player_config: &'a mut PlayerConfig,
    ) -> TickParams<'a> {
        TickParams {
            timer,
            resource,
            config,
            player_config,
            keyboard_backend: None,
            database: None,
            input_state: None,
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        }
    }

    #[test]
    fn initial_state_is_set() {
        let reg = StateRegistry::new(AppStateType::MusicSelect);
        assert_eq!(reg.current(), AppStateType::MusicSelect);
    }

    #[test]
    fn tick_calls_create_prepare_render_input() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let handler = RecordingHandler {
            log: log.clone(),
            ..Default::default()
        };

        let mut reg = StateRegistry::new(AppStateType::MusicSelect);
        reg.register(AppStateType::MusicSelect, Box::new(handler));

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut params = make_params(&mut timer, &mut resource, &config, &mut player_config);
        reg.tick(&mut params);

        let calls = log.lock();
        assert_eq!(*calls, vec!["create", "prepare", "render", "input"]);
    }

    #[test]
    fn transition_calls_shutdown_and_create() {
        let select_log = Arc::new(Mutex::new(Vec::new()));
        let decide_log = Arc::new(Mutex::new(Vec::new()));

        let select_handler = RecordingHandler {
            log: select_log.clone(),
            ..Default::default()
        };
        let decide_handler = RecordingHandler {
            log: decide_log.clone(),
            ..Default::default()
        };

        let mut reg = StateRegistry::new(AppStateType::MusicSelect);
        reg.register(AppStateType::MusicSelect, Box::new(select_handler));
        reg.register(AppStateType::Decide, Box::new(decide_handler));

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut params = make_params(&mut timer, &mut resource, &config, &mut player_config);

        // Initialize
        reg.tick(&mut params);

        // Clear logs
        select_log.lock().clear();
        decide_log.lock().clear();

        // Manually trigger transition
        reg.change_state(AppStateType::Decide, &mut params);

        assert!(select_log.lock().contains(&"shutdown".to_string()));
        let decide_calls = decide_log.lock();
        assert!(decide_calls.contains(&"create".to_string()));
        assert!(decide_calls.contains(&"prepare".to_string()));
        assert_eq!(reg.current(), AppStateType::Decide);
    }

    #[test]
    fn timer_resets_on_transition() {
        let mut reg = StateRegistry::new(AppStateType::MusicSelect);

        let handler1: Box<dyn GameStateHandler> = Box::new(RecordingHandler::default());
        let handler2: Box<dyn GameStateHandler> = Box::new(RecordingHandler::default());
        reg.register(AppStateType::MusicSelect, handler1);
        reg.register(AppStateType::Decide, handler2);

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut params = make_params(&mut timer, &mut resource, &config, &mut player_config);

        // Set a timer in MusicSelect
        params.timer.set_now_micro_time(5000);
        params
            .timer
            .set_timer_on(bms_skin::property_id::TIMER_STARTINPUT);
        assert!(
            params
                .timer
                .is_timer_on(bms_skin::property_id::TIMER_STARTINPUT)
        );

        // Transition should reset all timers
        reg.change_state(AppStateType::Decide, &mut params);
        assert!(
            !params
                .timer
                .is_timer_on(bms_skin::property_id::TIMER_STARTINPUT)
        );
    }

    #[test]
    fn chained_transition_in_create() {
        // MusicSelect's create immediately transitions to Decide
        let select_handler = RecordingHandler {
            transition_on_create: Some(AppStateType::Decide),
            ..Default::default()
        };
        let decide_handler = RecordingHandler::default();

        let mut reg = StateRegistry::new(AppStateType::MusicSelect);
        reg.register(AppStateType::MusicSelect, Box::new(select_handler));
        reg.register(AppStateType::Decide, Box::new(decide_handler));

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut params = make_params(&mut timer, &mut resource, &config, &mut player_config);

        // First tick should initialize MusicSelect, which chains to Decide
        reg.tick(&mut params);
        assert_eq!(reg.current(), AppStateType::Decide);
    }
}
