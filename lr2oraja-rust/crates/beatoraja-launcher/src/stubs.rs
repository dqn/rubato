// Stubs for types not yet available from other crates or external libraries
// These will be replaced with actual implementations during integration

use beatoraja_core::config::Config;
use beatoraja_core::main_controller::SongDatabaseAccessor;
use beatoraja_core::player_config::PlayerConfig;

// === JavaFX → egui stubs ===

/// Stub for egui context (replaces JavaFX Stage/Scene)
#[derive(Clone, Debug, Default)]
pub struct EguiContext;

/// Stub for file chooser dialog
pub fn show_directory_chooser(_title: &str) -> Option<String> {
    todo!("egui file dialog integration")
}

/// Stub for file chooser dialog
pub fn show_file_chooser(_title: &str) -> Option<String> {
    todo!("egui file dialog integration")
}

/// Stub for opening URL in browser
pub fn open_url_in_browser(_url: &str) {
    todo!("open URL in browser")
}

/// Stub for opening folder in file manager
pub fn open_folder_in_file_manager(_path: &str) {
    todo!("open folder in file manager")
}

/// Stub for clipboard operations
pub fn copy_to_clipboard(_text: &str) {
    todo!("clipboard integration")
}

// === MainLoader stubs ===

#[derive(Clone, Debug, Default)]
pub struct MainLoader;

impl MainLoader {
    pub fn get_score_database_accessor() -> SongDatabaseAccessor {
        todo!("MainLoader dependency")
    }

    pub fn get_version_checker() -> VersionChecker {
        VersionChecker::default()
    }

    pub fn get_available_display_mode() -> Vec<DisplayMode> {
        todo!("LibGDX display mode")
    }

    pub fn get_desktop_display_mode() -> DisplayMode {
        todo!("LibGDX display mode")
    }

    pub fn play(
        _path: Option<&str>,
        _mode: BMSPlayerMode,
        _launcher: bool,
        _config: &Config,
        _player: &PlayerConfig,
        _song_updated: bool,
    ) {
        todo!("MainLoader play")
    }
}

#[derive(Clone, Debug, Default)]
pub struct VersionChecker {
    pub message: String,
    pub download_url: Option<String>,
}

impl VersionChecker {
    pub fn get_message(&self) -> &str {
        &self.message
    }

    pub fn get_download_url(&self) -> Option<&str> {
        self.download_url.as_deref()
    }
}

#[derive(Clone, Debug, Default)]
pub struct DisplayMode {
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, Default)]
pub enum BMSPlayerMode {
    #[default]
    Play,
    Autoplay,
    Replay,
    Practice,
}

// === Version stub ===

pub struct Version;

impl Version {
    pub fn get_version() -> String {
        "0.1.0".to_string()
    }

    pub fn compare_to_string(_other: &str) -> i32 {
        0
    }
}

// === SongDatabaseUpdateListener stub ===

#[derive(Clone, Debug, Default)]
pub struct SongDatabaseUpdateListener {
    bms_files_count: i32,
    processed_bms_files_count: i32,
    new_bms_files_count: i32,
}

impl SongDatabaseUpdateListener {
    pub fn get_bms_files_count(&self) -> i32 {
        self.bms_files_count
    }

    pub fn get_processed_bms_files_count(&self) -> i32 {
        self.processed_bms_files_count
    }

    pub fn get_new_bms_files_count(&self) -> i32 {
        self.new_bms_files_count
    }
}

// === Twitter stubs (fully stubbed — no Rust equivalent) ===

pub struct TwitterAuth;

impl TwitterAuth {
    pub fn start_auth(
        _consumer_key: &str,
        _consumer_secret: &str,
    ) -> anyhow::Result<(String, String)> {
        todo!("Twitter4j fully stubbed — no direct Rust equivalent")
    }

    pub fn complete_pin_auth(
        _consumer_key: &str,
        _consumer_secret: &str,
        _request_token: &str,
        _request_secret: &str,
        _pin: &str,
    ) -> anyhow::Result<(String, String)> {
        todo!("Twitter4j fully stubbed — no direct Rust equivalent")
    }
}

// === PortAudio device stubs ===

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub name: String,
}

pub fn get_port_audio_devices() -> anyhow::Result<Vec<DeviceInfo>> {
    todo!("PortAudio device enumeration")
}

// === Monitor stubs ===

#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub name: String,
    pub virtual_x: i32,
    pub virtual_y: i32,
}

pub fn get_monitors() -> Vec<MonitorInfo> {
    todo!("monitor enumeration")
}
