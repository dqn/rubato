// Platform integration helpers (rfd dialogs, arboard clipboard, open crate, cpal audio, monitors).
//

/// Marker type (egui has no Stage/Scene equivalent)
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

// === Audio device enumeration via cpal ===

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub name: String,
}

/// Enumerate available audio output devices using cpal.
pub fn port_audio_devices() -> anyhow::Result<Vec<DeviceInfo>> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let devices = host
        .output_devices()
        .map_err(|e| anyhow::anyhow!("Failed to enumerate audio devices: {}", e))?;

    let mut result = Vec::new();
    for device in devices {
        let name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "Unknown Device".to_string());
        result.push(DeviceInfo { name });
    }
    Ok(result)
}

// === Monitor enumeration ===

/// Monitor information populated from winit's MonitorHandle.
///
/// Translated from: com.badlogic.gdx.Graphics.Monitor
/// Java fields: name, virtualX, virtualY
///
/// Extended with `width` and `height` for the monitor's native resolution,
/// which Java obtains via `Lwjgl3ApplicationConfiguration.getDisplayMode(monitor)`.
#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub name: String,
    pub virtual_x: i32,
    pub virtual_y: i32,
    /// Native width of the monitor (largest available video mode width).
    pub width: u32,
    /// Native height of the monitor (largest available video mode height).
    pub height: u32,
}

/// Full video mode information from winit, including refresh rate and bit depth.
///
/// Translated from: com.badlogic.gdx.Graphics.DisplayMode
/// Java fields: width, height, refreshRate, bitsPerPixel
///
/// Used by `MainLoader::get_available_display_mode()` to provide complete
/// display mode data for fullscreen mode selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VideoModeInfo {
    pub width: u32,
    pub height: u32,
    pub refresh_rate_millihertz: u32,
    pub bit_depth: u16,
}

static CACHED_MONITORS: std::sync::Mutex<Vec<MonitorInfo>> = std::sync::Mutex::new(Vec::new());
static CACHED_DISPLAY_MODES: std::sync::Mutex<Vec<(u32, u32)>> = std::sync::Mutex::new(Vec::new());
static CACHED_FULL_DISPLAY_MODES: std::sync::Mutex<Vec<VideoModeInfo>> =
    std::sync::Mutex::new(Vec::new());
static CACHED_DESKTOP_MODE: std::sync::Mutex<(u32, u32)> = std::sync::Mutex::new((0, 0));

fn lock_or_recover<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Update cached monitor list and display modes from winit's ActiveEventLoop.
/// Call this from the event loop's `resumed()` handler.
pub fn update_monitors_from_winit(event_loop: &winit::event_loop::ActiveEventLoop) {
    let mut display_modes: Vec<(u32, u32)> = Vec::new();
    let mut full_display_modes: Vec<VideoModeInfo> = Vec::new();
    let mut desktop_mode: (u32, u32) = (0, 0);

    let monitors: Vec<MonitorInfo> = event_loop
        .available_monitors()
        .enumerate()
        .map(|(i, handle)| {
            let name = handle
                .name()
                .unwrap_or_else(|| format!("Display {}", i + 1));
            let pos = handle.position();

            // Collect video modes from the primary monitor (index 0)
            // Java: Lwjgl3ApplicationConfiguration.getDisplayModes() returns primary monitor modes
            if i == 0 {
                for mode in handle.video_modes() {
                    let size = mode.size();
                    let pair = (size.width, size.height);
                    if !display_modes.contains(&pair) {
                        display_modes.push(pair);
                    }

                    // Collect full video mode info (all modes, not just unique resolutions)
                    // Java: Graphics.DisplayMode has refreshRate and bitsPerPixel
                    full_display_modes.push(VideoModeInfo {
                        width: size.width,
                        height: size.height,
                        refresh_rate_millihertz: mode.refresh_rate_millihertz(),
                        bit_depth: mode.bit_depth(),
                    });
                }
                // Desktop mode = largest resolution available on primary monitor
                // Java: Lwjgl3ApplicationConfiguration.getDisplayMode() returns current desktop mode
                if let Some(mode) = handle
                    .video_modes()
                    .max_by_key(|m| (m.size().width as u64) * (m.size().height as u64))
                {
                    let s = mode.size();
                    desktop_mode = (s.width, s.height);
                }
            }

            // Native resolution = largest video mode on this monitor
            // Java: Lwjgl3ApplicationConfiguration.getDisplayMode(monitor)
            let (native_w, native_h) = handle
                .video_modes()
                .max_by_key(|m| (m.size().width as u64) * (m.size().height as u64))
                .map(|m| (m.size().width, m.size().height))
                .unwrap_or((0, 0));

            MonitorInfo {
                name,
                virtual_x: pos.x,
                virtual_y: pos.y,
                width: native_w,
                height: native_h,
            }
        })
        .collect();

    display_modes.sort();
    // Sort full modes by resolution, then refresh rate, then bit depth
    full_display_modes.sort_by_key(|m| (m.width, m.height, m.refresh_rate_millihertz, m.bit_depth));

    *lock_or_recover(&CACHED_MONITORS) = monitors;
    *lock_or_recover(&CACHED_DISPLAY_MODES) = display_modes;
    *lock_or_recover(&CACHED_FULL_DISPLAY_MODES) = full_display_modes;
    *lock_or_recover(&CACHED_DESKTOP_MODE) = desktop_mode;
}

/// Enumerate available monitors.
/// Uses CoreGraphics FFI on macOS; other platforms use winit-cached monitor list.
pub fn monitors() -> Vec<MonitorInfo> {
    #[cfg(target_os = "macos")]
    {
        get_monitors_macos()
    }

    #[cfg(not(target_os = "macos"))]
    {
        let cached = lock_or_recover(&CACHED_MONITORS);
        if cached.is_empty() {
            log::warn!(
                "Monitor list not yet populated — call update_monitors_from_winit() from the event loop first"
            );
        }
        cached.clone()
    }
}

/// Get cached display modes (unique width/height pairs from primary monitor).
/// Returns empty if cache not yet populated (call update_monitors_from_winit first).
pub fn cached_display_modes() -> Vec<(u32, u32)> {
    lock_or_recover(&CACHED_DISPLAY_MODES).clone()
}

/// Get cached full display modes (all video modes from primary monitor with refresh rate and bit depth).
///
/// Translated from: Lwjgl3ApplicationConfiguration.getDisplayModes()
/// Java returns: Graphics.DisplayMode[] with width, height, refreshRate, bitsPerPixel
///
/// Returns empty if cache not yet populated (call update_monitors_from_winit first).
pub fn cached_full_display_modes() -> Vec<VideoModeInfo> {
    lock_or_recover(&CACHED_FULL_DISPLAY_MODES).clone()
}

/// Get cached desktop display mode (primary monitor's native resolution).
/// Returns (0, 0) if cache not yet populated.
pub fn cached_desktop_display_mode() -> (u32, u32) {
    *lock_or_recover(&CACHED_DESKTOP_MODE)
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
                width: bounds.size.width as u32,
                height: bounds.size.height as u32,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_info_fields() {
        let info = MonitorInfo {
            name: "Test Monitor".to_string(),
            virtual_x: 1920,
            virtual_y: 0,
            width: 2560,
            height: 1440,
        };
        assert_eq!(info.name, "Test Monitor");
        assert_eq!(info.virtual_x, 1920);
        assert_eq!(info.virtual_y, 0);
        assert_eq!(info.width, 2560);
        assert_eq!(info.height, 1440);
    }

    #[test]
    fn monitor_info_clone() {
        let info = MonitorInfo {
            name: "Primary".to_string(),
            virtual_x: 0,
            virtual_y: 0,
            width: 1920,
            height: 1080,
        };
        let cloned = info.clone();
        assert_eq!(info.name, cloned.name);
        assert_eq!(info.virtual_x, cloned.virtual_x);
        assert_eq!(info.virtual_y, cloned.virtual_y);
        assert_eq!(info.width, cloned.width);
        assert_eq!(info.height, cloned.height);
    }

    #[test]
    fn video_mode_info_equality() {
        let mode1 = VideoModeInfo {
            width: 1920,
            height: 1080,
            refresh_rate_millihertz: 60000,
            bit_depth: 32,
        };
        let mode2 = mode1;
        assert_eq!(mode1, mode2);

        let mode3 = VideoModeInfo {
            refresh_rate_millihertz: 144000,
            ..mode1
        };
        assert_ne!(mode1, mode3);
    }

    #[test]
    fn video_mode_info_sorting() {
        let mut modes = [
            VideoModeInfo {
                width: 2560,
                height: 1440,
                refresh_rate_millihertz: 60000,
                bit_depth: 32,
            },
            VideoModeInfo {
                width: 1920,
                height: 1080,
                refresh_rate_millihertz: 144000,
                bit_depth: 32,
            },
            VideoModeInfo {
                width: 1920,
                height: 1080,
                refresh_rate_millihertz: 60000,
                bit_depth: 32,
            },
        ];
        modes.sort_by_key(|m| (m.width, m.height, m.refresh_rate_millihertz, m.bit_depth));

        assert_eq!(modes[0].width, 1920);
        assert_eq!(modes[0].refresh_rate_millihertz, 60000);
        assert_eq!(modes[1].width, 1920);
        assert_eq!(modes[1].refresh_rate_millihertz, 144000);
        assert_eq!(modes[2].width, 2560);
    }

    #[test]
    fn cached_display_modes_default_empty() {
        // Before any winit initialization, caches are empty/zero
        // (Note: in a test environment, other tests may populate these, so we just
        // verify the getter doesn't panic)
        let _ = cached_display_modes();
        let _ = cached_full_display_modes();
        let _ = cached_desktop_display_mode();
    }

    #[test]
    fn cached_display_modes_recover_after_poison() {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = CACHED_DISPLAY_MODES.lock().expect("mutex poisoned");
            panic!("poison display modes");
        }));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = CACHED_FULL_DISPLAY_MODES.lock().expect("mutex poisoned");
            panic!("poison full display modes");
        }));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = CACHED_DESKTOP_MODE.lock().expect("mutex poisoned");
            panic!("poison desktop mode");
        }));

        assert!(cached_display_modes().is_empty());
        assert!(cached_full_display_modes().is_empty());
        assert_eq!(cached_desktop_display_mode(), (0, 0));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn get_monitors_macos_does_not_panic() {
        // In headless CI environments, CGGetActiveDisplayList may return 0 displays.
        // This test just verifies the function doesn't panic.
        let monitors = get_monitors_macos();
        // If displays are available, verify basic invariants
        if !monitors.is_empty() {
            // Primary monitor should be at position (0, 0)
            assert!(
                monitors
                    .iter()
                    .any(|m| m.virtual_x == 0 && m.virtual_y == 0)
            );
            // All monitors should have non-zero dimensions
            for m in &monitors {
                assert!(m.width > 0, "Monitor {} has zero width", m.name);
                assert!(m.height > 0, "Monitor {} has zero height", m.name);
            }
        }
    }

    #[test]
    fn monitor_info_format_for_config() {
        // Java stores monitor selection as "name [virtualX, virtualY]"
        // This tests the format used to match config.monitorName
        let info = MonitorInfo {
            name: "DELL U2720Q".to_string(),
            virtual_x: 0,
            virtual_y: 0,
            width: 3840,
            height: 2160,
        };
        let formatted = format!("{} [{}, {}]", info.name, info.virtual_x, info.virtual_y);
        assert_eq!(formatted, "DELL U2720Q [0, 0]");
    }

    #[test]
    fn monitor_info_format_with_negative_offset() {
        // Multi-monitor setups can have negative virtual positions
        let info = MonitorInfo {
            name: "Display 2".to_string(),
            virtual_x: -1920,
            virtual_y: 0,
            width: 1920,
            height: 1080,
        };
        let formatted = format!("{} [{}, {}]", info.name, info.virtual_x, info.virtual_y);
        assert_eq!(formatted, "Display 2 [-1920, 0]");
    }
}
