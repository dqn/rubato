// Platform integration helpers (rfd dialogs, arboard clipboard, open crate, cpal audio, monitors).
// Moved from stubs.rs in Phase 25a.

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
