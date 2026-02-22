// Stubs for types not yet available from other crates or external libraries
// These will be replaced with actual implementations during integration

use beatoraja_core::config::Config;
use beatoraja_core::main_loader::MainLoader as CoreMainLoader;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor;

// === JavaFX → egui stubs ===

/// Stub for egui context (replaces JavaFX Stage/Scene)
#[derive(Clone, Debug, Default)]
pub struct EguiContext;

/// Show a directory chooser dialog using rfd.
pub fn show_directory_chooser(title: &str) -> Option<String> {
    rfd::FileDialog::new()
        .set_title(title)
        .pick_folder()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Show a file chooser dialog using rfd.
pub fn show_file_chooser(title: &str) -> Option<String> {
    rfd::FileDialog::new()
        .set_title(title)
        .pick_file()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Open a URL in the default browser using the open crate.
pub fn open_url_in_browser(url: &str) {
    if let Err(e) = open::that(url) {
        log::error!("Failed to open URL: {}", e);
    }
}

/// Open a folder in the system file manager using the open crate.
pub fn open_folder_in_file_manager(path: &str) {
    if let Err(e) = open::that(path) {
        log::error!("Failed to open folder: {}", e);
    }
}

/// Copy text to system clipboard using arboard crate.
pub fn copy_to_clipboard(text: &str) {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            if let Err(e) = clipboard.set_text(text) {
                log::error!("Failed to copy to clipboard: {}", e);
            }
        }
        Err(e) => {
            log::error!("Failed to access clipboard: {}", e);
        }
    }
}

// === MainLoader stubs ===

#[derive(Clone, Debug, Default)]
pub struct MainLoader;

impl MainLoader {
    /// Create and register a SQLiteSongDatabaseAccessor with the core MainLoader.
    ///
    /// Translated from: MainLoader.getScoreDatabaseAccessor()
    /// Java: if(songdb == null) { songdb = new SQLiteSongDatabaseAccessor(config.getSongpath(), config.getBmsroot()); }
    ///
    /// In Java this is lazily created on first access. In Rust, we eagerly create it
    /// and set it on the core MainLoader's global slot, which then passes it to MainController.
    pub fn init_score_database_accessor(config: &Config) {
        match SQLiteSongDatabaseAccessor::new(config.get_songpath(), config.get_bmsroot()) {
            Ok(accessor) => {
                CoreMainLoader::set_score_database_accessor(Box::new(accessor));
                log::info!(
                    "Song database accessor initialized: {}",
                    config.get_songpath()
                );
            }
            Err(e) => {
                log::error!("Failed to create song database accessor: {}", e);
            }
        }
    }

    pub fn get_version_checker() -> VersionChecker {
        VersionChecker::default()
    }

    pub fn get_available_display_mode() -> Vec<DisplayMode> {
        log::warn!("not yet implemented: MainLoader.get_available_display_mode");
        Vec::new()
    }

    pub fn get_desktop_display_mode() -> DisplayMode {
        log::warn!("not yet implemented: MainLoader.get_desktop_display_mode");
        DisplayMode::default()
    }

    /// Launch the game.
    ///
    /// Translated from: MainLoader.play() (launcher side)
    ///
    /// This creates the song database accessor and delegates to the core MainLoader.play().
    pub fn play(
        path: Option<&str>,
        mode: BMSPlayerMode,
        launcher: bool,
        config: &Config,
        player: &PlayerConfig,
        song_updated: bool,
    ) {
        // Initialize song database accessor before play
        Self::init_score_database_accessor(config);

        // Delegate to core MainLoader
        CoreMainLoader::play(
            path.map(std::path::PathBuf::from),
            Some(mode),
            launcher,
            Some(config.clone()),
            Some(player.clone()),
            song_updated,
        );
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

pub use beatoraja_core::bms_player_mode::BMSPlayerMode;

// === Version (re-exported from beatoraja-core) ===

pub use beatoraja_core::version::Version;

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

// === Twitter stubs — Twitter API not supported in Rust port ===

pub struct TwitterAuth;

impl TwitterAuth {
    pub fn start_auth(
        _consumer_key: &str,
        _consumer_secret: &str,
    ) -> anyhow::Result<(String, String)> {
        anyhow::bail!(
            "Twitter API is not supported in Rust port (twitter4j has no Rust equivalent)"
        )
    }

    pub fn complete_pin_auth(
        _consumer_key: &str,
        _consumer_secret: &str,
        _request_token: &str,
        _request_secret: &str,
        _pin: &str,
    ) -> anyhow::Result<(String, String)> {
        anyhow::bail!(
            "Twitter API is not supported in Rust port (twitter4j has no Rust equivalent)"
        )
    }
}

// === Audio device enumeration via cpal ===

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub name: String,
}

/// Enumerate available audio output devices using cpal.
pub fn get_port_audio_devices() -> anyhow::Result<Vec<DeviceInfo>> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let devices = host
        .output_devices()
        .map_err(|e| anyhow::anyhow!("Failed to enumerate audio devices: {}", e))?;

    let mut result = Vec::new();
    for device in devices {
        let name = device
            .name()
            .unwrap_or_else(|_| "Unknown Device".to_string());
        result.push(DeviceInfo { name });
    }
    Ok(result)
}

// === Monitor enumeration ===

#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub name: String,
    pub virtual_x: i32,
    pub virtual_y: i32,
}

static CACHED_MONITORS: std::sync::Mutex<Vec<MonitorInfo>> = std::sync::Mutex::new(Vec::new());

/// Update cached monitor list from winit's ActiveEventLoop.
/// Call this from the event loop's `resumed()` handler.
pub fn update_monitors_from_winit(event_loop: &winit::event_loop::ActiveEventLoop) {
    let monitors: Vec<MonitorInfo> = event_loop
        .available_monitors()
        .enumerate()
        .map(|(i, handle)| {
            let name = handle
                .name()
                .unwrap_or_else(|| format!("Display {}", i + 1));
            let pos = handle.position();
            MonitorInfo {
                name,
                virtual_x: pos.x,
                virtual_y: pos.y,
            }
        })
        .collect();
    *CACHED_MONITORS.lock().unwrap() = monitors;
}

/// Enumerate available monitors.
/// Uses CoreGraphics FFI on macOS; other platforms use winit-cached monitor list.
pub fn get_monitors() -> Vec<MonitorInfo> {
    #[cfg(target_os = "macos")]
    {
        get_monitors_macos()
    }

    #[cfg(not(target_os = "macos"))]
    {
        let cached = CACHED_MONITORS.lock().unwrap();
        if cached.is_empty() {
            log::warn!(
                "Monitor list not yet populated — call update_monitors_from_winit() from the event loop first"
            );
        }
        cached.clone()
    }
}

#[cfg(target_os = "macos")]
fn get_monitors_macos() -> Vec<MonitorInfo> {
    // CoreGraphics FFI for display enumeration
    type CGDirectDisplayID = u32;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }

    unsafe extern "C" {
        fn CGGetActiveDisplayList(
            max_displays: u32,
            active_displays: *mut CGDirectDisplayID,
            display_count: *mut u32,
        ) -> i32;
        fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;
        fn CGDisplayIsMain(display: CGDirectDisplayID) -> u8;
    }

    let mut display_count: u32 = 0;
    // First call to get count
    let err = unsafe { CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut display_count) };
    if err != 0 || display_count == 0 {
        log::error!("Failed to enumerate displays (error: {})", err);
        return Vec::new();
    }

    let mut displays = vec![0u32; display_count as usize];
    let err =
        unsafe { CGGetActiveDisplayList(display_count, displays.as_mut_ptr(), &mut display_count) };
    if err != 0 {
        log::error!("Failed to get display list (error: {})", err);
        return Vec::new();
    }

    displays
        .iter()
        .enumerate()
        .map(|(i, &display_id)| {
            let bounds = unsafe { CGDisplayBounds(display_id) };
            let is_main = unsafe { CGDisplayIsMain(display_id) } != 0;
            let name = if is_main {
                format!("Display {} (Main)", i + 1)
            } else {
                format!("Display {}", i + 1)
            };
            MonitorInfo {
                name,
                virtual_x: bounds.origin.x as i32,
                virtual_y: bounds.origin.y as i32,
            }
        })
        .collect()
}
