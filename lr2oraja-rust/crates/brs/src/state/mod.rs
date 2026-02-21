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

use std::sync::Arc;

use crate::app_state::AppStateType;
use crate::database_manager::DatabaseManager;
use crate::game_state::SharedGameState;
use crate::input_mapper::InputState;
use crate::player_resource::PlayerResource;
use crate::preview_music::PreviewMusicProcessor;
use crate::skin_manager::{SkinManager, SkinTiming};
use crate::system_sound::SystemSoundManager;
use crate::timer_manager::TimerManager;
use bms_config::{Config, PlayerConfig};
use bms_input::keyboard::KeyboardBackend;

/// Download source types (enum dispatch — DownloadSource trait is not object-safe).
pub enum DownloadSourceKind {
    Konmai(bms_download::source::konmai::KonmaiDownloadSource),
    Wriggle(bms_download::source::wriggle::WriggleDownloadSource),
}

impl DownloadSourceKind {
    pub async fn get_download_url(&self, hash: &str) -> anyhow::Result<String> {
        use bms_download::source::DownloadSource;
        match self {
            Self::Konmai(s) => s.get_download_url(hash).await,
            Self::Wriggle(s) => s.get_download_url(hash).await,
        }
    }
}

/// Download configuration and processor handle for background song downloads.
pub struct DownloadHandle {
    pub processor: Arc<bms_download::processor::HttpDownloadProcessor>,
    pub source: DownloadSourceKind,
    pub ipfs_gateway: String,
    pub enable_http: bool,
    pub enable_ipfs: bool,
}

/// Context passed to state handlers on each callback.
pub struct StateContext<'a> {
    pub timer: &'a mut TimerManager,
    pub resource: &'a mut PlayerResource,
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
    pub skin_manager: Option<&'a mut SkinManager>,
    /// System sound playback manager (None in tests or when audio not available).
    pub sound_manager: Option<&'a mut SystemSoundManager>,
    /// Characters typed this frame (from Bevy KeyboardInput events).
    pub received_chars: &'a [char],
    /// Bevy image assets for BGA loading (None in tests or when not available).
    pub bevy_images: Option<&'a mut bevy::prelude::Assets<bevy::prelude::Image>>,
    /// Shared game state for skin property synchronization (None in tests).
    pub shared_state: Option<&'a mut SharedGameState>,
    /// Preview music processor for select screen (None in tests or non-select states).
    pub preview_music: Option<&'a mut PreviewMusicProcessor>,
    /// Download handle for background song downloads (None in tests or when disabled).
    pub download_handle: Option<&'a Arc<DownloadHandle>>,
}

/// Default skin timing values when no skin is loaded (fallback for tests).
const DEFAULT_INPUT_DELAY_MS: i64 = 500;
const DEFAULT_SCENE_DURATION_MS: i64 = 3000;
const DEFAULT_FADEOUT_DURATION_MS: i64 = 500;

impl StateContext<'_> {
    /// Get skin timing values from the loaded skin, or defaults if no skin is loaded.
    pub fn skin_timing(&self) -> SkinTiming {
        self.skin_manager
            .as_ref()
            .map(|mgr| {
                let t = mgr.skin_timing;
                SkinTiming {
                    input_ms: if t.input_ms > 0 {
                        t.input_ms
                    } else {
                        DEFAULT_INPUT_DELAY_MS
                    },
                    scene_ms: if t.scene_ms > 0 {
                        t.scene_ms
                    } else {
                        DEFAULT_SCENE_DURATION_MS
                    },
                    fadeout_ms: if t.fadeout_ms > 0 {
                        t.fadeout_ms
                    } else {
                        DEFAULT_FADEOUT_DURATION_MS
                    },
                }
            })
            .unwrap_or(SkinTiming {
                input_ms: DEFAULT_INPUT_DELAY_MS,
                scene_ms: DEFAULT_SCENE_DURATION_MS,
                fadeout_ms: DEFAULT_FADEOUT_DURATION_MS,
            })
    }
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
