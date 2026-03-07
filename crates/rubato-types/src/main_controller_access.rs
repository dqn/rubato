use std::any::Any;
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::main_state_type::MainStateType;
use crate::player_config::PlayerConfig;
use crate::player_data::PlayerData;
use crate::player_resource_access::PlayerResourceAccess;
use crate::ranking_data_cache_access::RankingDataCacheAccess;
use crate::replay_data::ReplayData;
use crate::score_data::ScoreData;
use crate::song_information_db::SongInformationDb;
use crate::sound_type::SoundType;

/// Cross-crate side-effect commands issued by state-facing MainController proxies.
///
/// Rust states cannot hold a live `&mut MainController` due to ownership rules, so launcher-side
/// proxies enqueue these commands and MainController drains them at safe points.
pub enum MainControllerCommand {
    ChangeState(MainStateType),
    SaveConfig,
    Exit,
    SaveLastRecording(String),
    UpdateSong(Option<String>),
    PlaySound(SoundType, bool),
    StopSound(SoundType),
    ShuffleSounds,
    UpdateTable(Box<dyn crate::table_update_source::TableUpdateSource>),
    StartIpfsDownload(Box<crate::song_data::SongData>),
    SetGlobalPitch(f32),
    StopAllNotes,
    PlayAudioPath(String, f32, bool),
    SetAudioPathVolume(String, f32),
    StopAudioPath(String),
    DisposeAudioPath(String),
}

/// Shared command queue for state-facing MainController proxies.
#[derive(Clone, Default)]
pub struct MainControllerCommandQueue {
    inner: Arc<Mutex<Vec<MainControllerCommand>>>,
}

fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl MainControllerCommandQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, command: MainControllerCommand) {
        lock_or_recover(&self.inner).push(command);
    }

    pub fn drain(&self) -> Vec<MainControllerCommand> {
        std::mem::take(&mut *lock_or_recover(&self.inner))
    }

    pub fn is_empty(&self) -> bool {
        lock_or_recover(&self.inner).is_empty()
    }
}

/// Trait interface for MainController access.
///
/// Downstream crates use `&dyn MainControllerAccess` instead of concrete MainController stubs.
/// The real implementation in beatoraja-core implements this trait.
///
/// Methods that return types not available in beatoraja-types (e.g., BMSPlayerInputProcessor,
/// SystemSoundManager, IRStatus) are NOT included here. Downstream crates that need those
/// methods should keep local extension stubs until the types are unified.
pub trait MainControllerAccess {
    /// Get config reference
    fn config(&self) -> &Config;
    /// Get player config reference
    fn player_config(&self) -> &PlayerConfig;
    /// Change to a different state
    fn change_state(&mut self, state: MainStateType);

    /// Save config to disk
    fn save_config(&self);

    /// Exit the application
    fn exit(&self);

    /// Save OBS last recording with the given reason tag
    fn save_last_recording(&self, reason: &str);

    /// Update song database for the given path
    fn update_song(&mut self, path: Option<&str>);

    /// Get player resource (immutable)
    fn player_resource(&self) -> Option<&dyn PlayerResourceAccess>;
    /// Get player resource (mutable)
    fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess>;
    /// Play a system sound effect or BGM.
    fn play_sound(&mut self, _sound: &SoundType, _loop_sound: bool) {
        // default no-op
    }

    /// Stop a system sound effect or BGM.
    fn stop_sound(&mut self, _sound: &SoundType) {
        // default no-op
    }

    /// Check if a sound exists for the given type.
    fn sound_path(&self, _sound: &SoundType) -> Option<String> {
        None
    }

    /// Play an arbitrary audio path, typically used for preview music.
    fn play_audio_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {
        // default no-op
    }

    /// Update the volume of an arbitrary audio path.
    fn set_audio_path_volume(&mut self, _path: &str, _volume: f32) {
        // default no-op
    }

    /// Check whether an arbitrary audio path is still considered active.
    fn is_audio_path_playing(&self, _path: &str) -> bool {
        false
    }

    /// Stop an arbitrary audio path.
    fn stop_audio_path(&mut self, _path: &str) {
        // default no-op
    }

    /// Dispose cached state for an arbitrary audio path.
    fn dispose_audio_path(&mut self, _path: &str) {
        // default no-op
    }

    /// Shuffle select-screen sounds (BGM, cursor, decide sounds).
    /// Java: MainController.getSoundManager().shuffle()
    fn shuffle_sounds(&mut self) {
        // default no-op
    }

    /// Read replay data for the given song hash.
    /// Delegates to PlayDataAccessor internally.
    fn read_replay_data(
        &self,
        _sha256: &str,
        _has_ln: bool,
        _lnmode: i32,
        _index: i32,
    ) -> Option<ReplayData> {
        None
    }

    /// Get IR song page URL for the given song data.
    /// Returns None if no IR connection is available.
    fn ir_song_url(&self, _song_data: &crate::song_data::SongData) -> Option<String> {
        None
    }

    /// Get IR course page URL for the given course data.
    /// Returns None if no IR connection is available.
    fn ir_course_url(&self, _course_data: &crate::course_data::CourseData) -> Option<String> {
        None
    }

    /// Update difficulty table data in background.
    fn update_table(&mut self, _source: Box<dyn crate::table_update_source::TableUpdateSource>) {
        // default no-op
    }

    /// Get HTTP download submitter for submitting chart download tasks.
    fn http_downloader(
        &self,
    ) -> Option<&dyn crate::http_download_submitter::HttpDownloadSubmitter> {
        None
    }

    /// Check whether the IPFS download daemon is alive.
    /// Java: main.getMusicDownloadProcessor() != null && main.getMusicDownloadProcessor().isAlive()
    fn is_ipfs_download_alive(&self) -> bool {
        false
    }

    /// Start IPFS download for the given song.
    /// Returns true if the download was started, false otherwise.
    /// The default implementation returns false (no IPFS support).
    fn start_ipfs_download(&mut self, _song: &crate::song_data::SongData) -> bool {
        false
    }

    /// Get ranking data cache (immutable).
    /// Java: MainController.getRankingDataCache()
    fn ranking_data_cache(&self) -> Option<&dyn RankingDataCacheAccess> {
        None
    }

    /// Get ranking data cache (mutable).
    fn ranking_data_cache_mut(&mut self) -> Option<&mut (dyn RankingDataCacheAccess + 'static)> {
        None
    }

    /// Get rival player count.
    fn rival_count(&self) -> usize {
        0
    }

    /// Get rival player information by index.
    fn rival_information(
        &self,
        _index: usize,
    ) -> Option<crate::player_information::PlayerInformation> {
        None
    }

    /// Get IR table URLs for connected IR services.
    /// Returns (ir_name, table_url) pairs.
    /// Java: MainController.getIRStatus() → IRStatus.tables
    fn ir_table_urls(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// Read score data for a given song hash.
    /// Java: PlayDataAccessor.readScoreData(hash, ln, lnmode)
    fn read_score_data_by_hash(&self, _hash: &str, _ln: bool, _lnmode: i32) -> Option<ScoreData> {
        None
    }

    /// Read player data (aggregate play statistics).
    /// Java: PlayDataAccessor.readPlayerData()
    fn read_player_data(&self) -> Option<PlayerData> {
        None
    }

    /// Get song information database reference.
    /// Java: MainController.getInfoDatabase()
    fn info_database(&self) -> Option<&dyn SongInformationDb> {
        None
    }

    /// Get the first IR connection (type-erased).
    ///
    /// Returns a reference to the stored `Arc<dyn IRConnection + Send + Sync>` from beatoraja-ir,
    /// erased as `&dyn Any`. Callers downcast via
    /// `any.downcast_ref::<Arc<dyn IRConnection + Send + Sync>>()` and clone the Arc.
    /// Java: MainController.getIRStatus()[0].connection
    fn ir_connection_any(&self) -> Option<&dyn Any> {
        None
    }
}

/// Null implementation of MainControllerAccess for stub contexts.
/// All methods log a warning and return defaults.
pub struct NullMainController;

impl NullMainController {
    fn null_config() -> &'static Config {
        use std::sync::OnceLock;
        static CONFIG: OnceLock<Config> = OnceLock::new();
        CONFIG.get_or_init(Config::default)
    }

    fn null_player_config() -> &'static PlayerConfig {
        use std::sync::OnceLock;
        static PCONFIG: OnceLock<PlayerConfig> = OnceLock::new();
        PCONFIG.get_or_init(PlayerConfig::default)
    }
}

impl MainControllerAccess for NullMainController {
    fn config(&self) -> &Config {
        log::warn!("NullMainController::config called — returning default");
        Self::null_config()
    }
    fn player_config(&self) -> &PlayerConfig {
        log::warn!("NullMainController::player_config called — returning default");
        Self::null_player_config()
    }
    fn change_state(&mut self, _state: MainStateType) {
        log::warn!("NullMainController::change_state called — no-op");
    }
    fn save_config(&self) {
        log::warn!("NullMainController::save_config called — no-op");
    }
    fn exit(&self) {
        log::warn!("NullMainController::exit called — no-op");
    }
    fn save_last_recording(&self, _reason: &str) {
        log::warn!("NullMainController::save_last_recording called — no-op");
    }
    fn update_song(&mut self, _path: Option<&str>) {
        log::warn!("NullMainController::update_song called — no-op");
    }
    fn player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        None
    }
    fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
        None
    }
}

/// Config-backed implementation of MainControllerAccess.
/// Holds cloned Config/PlayerConfig from the real MainController.
/// State changes and sounds are no-ops (handled via outbox pattern in MainState).
pub struct ConfigMainControllerAccess {
    config: Config,
    player_config: PlayerConfig,
}

impl ConfigMainControllerAccess {
    pub fn new(config: Config, player_config: PlayerConfig) -> Self {
        Self {
            config,
            player_config,
        }
    }
}

impl MainControllerAccess for ConfigMainControllerAccess {
    fn config(&self) -> &Config {
        &self.config
    }
    fn player_config(&self) -> &PlayerConfig {
        &self.player_config
    }
    fn change_state(&mut self, _state: MainStateType) {
        // No-op: states use outbox pattern (pending_state_change)
    }
    fn save_config(&self) {}
    fn exit(&self) {}
    fn save_last_recording(&self, _reason: &str) {}
    fn update_song(&mut self, _path: Option<&str>) {}
    fn player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        None
    }
    fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_queue_recovers_after_poison() {
        let queue = MainControllerCommandQueue::new();
        let poisoned = queue.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = poisoned.inner.lock().expect("mutex poisoned");
            panic!("poison queue");
        }));

        queue.push(MainControllerCommand::Exit);
        assert!(!queue.is_empty());
        assert_eq!(queue.drain().len(), 1);
    }
}
