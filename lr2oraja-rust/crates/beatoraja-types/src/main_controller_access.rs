use crate::config::Config;
use crate::main_state_type::MainStateType;
use crate::player_config::PlayerConfig;
use crate::player_resource_access::PlayerResourceAccess;
use crate::replay_data::ReplayData;
use crate::sound_type::SoundType;

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
    fn get_config(&self) -> &Config;

    /// Get player config reference
    fn get_player_config(&self) -> &PlayerConfig;

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
    fn get_player_resource(&self) -> Option<&dyn PlayerResourceAccess>;

    /// Get player resource (mutable)
    fn get_player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess>;

    /// Play a system sound effect or BGM.
    fn play_sound(&mut self, _sound: &SoundType, _loop_sound: bool) {
        // default no-op
    }

    /// Stop a system sound effect or BGM.
    fn stop_sound(&mut self, _sound: &SoundType) {
        // default no-op
    }

    /// Check if a sound exists for the given type.
    fn get_sound_path(&self, _sound: &SoundType) -> Option<String> {
        None
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
    fn get_ir_song_url(&self, _song_data: &crate::song_data::SongData) -> Option<String> {
        None
    }

    /// Get IR course page URL for the given course data.
    /// Returns None if no IR connection is available.
    fn get_ir_course_url(&self, _course_data: &crate::course_data::CourseData) -> Option<String> {
        None
    }

    /// Update difficulty table data in background.
    fn update_table(&mut self, _source: Box<dyn crate::table_update_source::TableUpdateSource>) {
        // default no-op
    }

    /// Get HTTP download submitter for submitting chart download tasks.
    fn get_http_downloader(
        &self,
    ) -> Option<&dyn crate::http_download_submitter::HttpDownloadSubmitter> {
        None
    }

    /// Get rival player count.
    fn get_rival_count(&self) -> usize {
        0
    }

    /// Get rival player information by index.
    fn get_rival_information(
        &self,
        _index: usize,
    ) -> Option<crate::player_information::PlayerInformation> {
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
    fn get_config(&self) -> &Config {
        log::warn!("NullMainController::get_config called — returning default");
        Self::null_config()
    }
    fn get_player_config(&self) -> &PlayerConfig {
        log::warn!("NullMainController::get_player_config called — returning default");
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
    fn get_player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        None
    }
    fn get_player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
        None
    }
}
