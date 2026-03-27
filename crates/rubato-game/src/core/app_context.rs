use std::sync::atomic::{AtomicBool, Ordering};

use rubato_audio::audio_system::AudioSystem;

use crate::core::config::Config;
use crate::core::main_controller::{DatabaseState, IntegrationState, LifecycleState, SkinOffset};
use crate::core::player_config::PlayerConfig;
use crate::core::player_resource::PlayerResource;
use crate::core::system_sound_manager::SystemSoundManager;
use crate::core::timer_manager::TimerManager;
use rubato_input::bms_player_input_processor::BMSPlayerInputProcessor;
use rubato_types::sound_type::SoundType;

/// Backward-compatible type alias for `GameContext`.
///
/// New code should use `GameContext` directly. This alias exists to avoid
/// a flag-day rename across the entire codebase; it will be removed once
/// all call sites have been migrated.
pub type AppContext = GameContext;

/// Shared application context holding config, audio, input, timer, database,
/// display, integration, and lifecycle state. Extracted from `MainController`
/// to separate application-wide concerns from state-machine mechanics.
///
/// Renamed from `AppContext` to `GameContext` as part of the Phase 5 migration
/// to a unified context pattern. The `AppContext` type alias is provided for
/// backward compatibility during the transition.
pub struct GameContext {
    // --- Config ---
    pub config: Config,
    pub player: PlayerConfig,

    // --- Audio ---
    pub audio: Option<AudioSystem>,
    pub sound: Option<SystemSoundManager>,
    pub loudness_analyzer: Option<rubato_audio::bms_loudness_analyzer::BMSLoudnessAnalyzer>,

    // --- Timer ---
    pub timer: TimerManager,

    // --- Input ---
    pub input: Option<BMSPlayerInputProcessor>,
    pub input_poll_quit: std::sync::Arc<AtomicBool>,

    // --- Database ---
    pub db: DatabaseState,

    // --- Display ---
    pub offset: Vec<SkinOffset>,
    pub showfps: bool,
    pub debug: bool,

    // --- Integration ---
    pub integration: IntegrationState,

    // --- Lifecycle ---
    pub lifecycle: LifecycleState,
    pub exit_requested: AtomicBool,

    // --- Player Resource ---
    /// Player resource (gameplay session state).
    /// During active play, this is borrowed by the current state.
    pub resource: Option<PlayerResource>,

    // --- Frame transition ---
    /// Pending state transition from `render_with_game_context`.
    /// Stored here so the outbox drain runs before the transition is applied.
    pub transition: Option<crate::core::main_state::StateTransition>,
}

impl GameContext {
    // --- Audio convenience methods ---

    pub fn play_sound(&mut self, sound: &SoundType, loop_sound: bool) {
        let volume = self.config.audio.as_ref().map_or(1.0, |a| a.systemvolume);
        let path = self.sound.as_ref().and_then(|sm| sm.sound(sound).cloned());
        if let Some(path) = path
            && let Some(ref mut audio) = self.audio
        {
            audio.play_path(&path, volume, loop_sound);
        }
    }

    pub fn stop_sound(&mut self, sound: &SoundType) {
        let path = self.sound.as_ref().and_then(|sm| sm.sound(sound).cloned());
        if let Some(path) = path
            && let Some(ref mut audio) = self.audio
        {
            audio.stop_path(&path);
        }
    }

    pub fn sound_path(&self, sound: &SoundType) -> Option<String> {
        self.sound.as_ref().and_then(|sm| sm.sound(sound).cloned())
    }

    pub fn play_audio_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        if path.is_empty() {
            return;
        }
        if let Some(ref mut audio) = self.audio {
            audio.play_path(path, volume, loop_play);
        }
    }

    pub fn set_audio_path_volume(&mut self, path: &str, volume: f32) {
        if path.is_empty() {
            return;
        }
        if let Some(ref mut audio) = self.audio {
            audio.set_volume_path(path, volume);
        }
    }

    pub fn is_audio_path_playing(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        self.audio
            .as_ref()
            .is_some_and(|audio| audio.is_playing_path(path))
    }

    pub fn stop_audio_path(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        if let Some(ref mut audio) = self.audio {
            audio.stop_path(path);
        }
    }

    pub fn dispose_audio_path(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        if let Some(ref mut audio) = self.audio {
            audio.dispose_path(path);
        }
    }

    pub fn shuffle_sounds(&mut self) {
        if let Some(ref mut sm) = self.sound {
            let old_paths = sm.shuffle();
            if let Some(ref mut audio) = self.audio {
                for path in &old_paths {
                    audio.dispose_path(path);
                }
            }
        }
        if let Some(ref sm) = self.sound {
            let paths: Vec<String> = SoundType::values()
                .iter()
                .filter_map(|st| sm.sound(st).cloned())
                .collect();
            if let Some(ref mut audio) = self.audio {
                for path in &paths {
                    audio.preload_path(path);
                }
            }
        }
    }

    pub fn set_global_pitch(&mut self, pitch: f32) {
        if let Some(ref mut audio) = self.audio {
            audio.set_global_pitch(pitch);
        }
    }

    pub fn stop_all_notes(&mut self) {
        if let Some(ref mut audio) = self.audio {
            audio.stop_note(None);
        }
    }

    // --- Config convenience methods ---

    pub fn update_audio_config(&mut self, audio: rubato_types::audio_config::AudioConfig) {
        self.config.audio = Some(audio);
    }

    pub fn update_skin_config(
        &mut self,
        id: usize,
        skin_config: Option<rubato_types::skin_config::SkinConfig>,
    ) {
        if id < self.player.skin.len() {
            self.player.skin[id] = skin_config;
        }
    }

    pub fn update_skin_history(
        &mut self,
        skin_path: &str,
        skin_config: rubato_types::skin_config::SkinConfig,
    ) {
        if let Some(entry) = self
            .player
            .skin_history
            .iter_mut()
            .find(|h| h.path().is_some_and(|p| p == skin_path))
        {
            *entry = skin_config;
        } else {
            self.player.skin_history.push(skin_config);
        }
    }

    pub fn save_config(&self) {
        if let Err(e) = Config::write(&self.config) {
            log::error!("Failed to write config: {}", e);
        }
        if let Err(e) = PlayerConfig::write(&self.config.paths.playerpath, &self.player) {
            log::error!("Failed to write player config: {}", e);
        }
        log::info!("Config saved");
    }

    pub fn request_exit(&self) {
        self.exit_requested.store(true, Ordering::Release);
        self.save_config();
        log::info!("Exit requested");
    }

    pub fn save_last_recording(&self, reason: &str) {
        if let Some(ref client) = self.integration.obs_client {
            client.save_last_recording(reason);
        }
    }

    pub fn start_ipfs_download(&self, song: &rubato_types::song_data::SongData) -> bool {
        if let Some(ref dl) = self.integration.download {
            dl.start_download(song);
            true
        } else {
            false
        }
    }
}
