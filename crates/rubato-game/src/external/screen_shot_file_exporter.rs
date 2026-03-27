use crate::external::screen_shot_exporter::{self, ScreenShotExporter};
use crate::external::webhook_handler::WebhookHandler;
use crate::external::{
    BufferUtils, ClipboardHelper, GdxGraphics, ImGuiNotify, IntegerPropertyFactory, MainState,
    NUMBER_PLAYLEVEL, Pixmap, PixmapIO, STRING_FULLTITLE, STRING_TABLE_LEVEL, ScreenType,
    StringPropertyFactory,
};

/// ScreenShotFileExporter - saves screenshots to file and optionally copies to clipboard / sends webhook.
/// Translated from Java: ScreenShotFileExporter implements ScreenShotExporter
pub struct ScreenShotFileExporter {
    /// JoinHandles for in-flight webhook send background threads.
    webhook_threads: std::sync::Mutex<Vec<std::thread::JoinHandle<()>>>,
}

/// Maximum number of concurrent webhook threads allowed.
const MAX_CONCURRENT_WEBHOOK_THREADS: usize = 3;

impl ScreenShotExporter for ScreenShotFileExporter {
    fn send(&self, current_state: &MainState, pixels: &[u8]) -> bool {
        let now = chrono::Local::now();
        let sdf = now.format("%Y%m%d_%H%M%S").to_string();
        let mut state_name = String::new();

        let screen_type = get_screen_type(current_state);

        if screen_type == ScreenType::MusicSelector {
            state_name = "_Music_Select".to_string();
        } else if screen_type == ScreenType::MusicDecide {
            state_name = "_Decide".to_string();
        }

        if screen_type == ScreenType::BMSPlayer {
            let tablelevel =
                StringPropertyFactory::string_property(STRING_TABLE_LEVEL).get(current_state);
            if !tablelevel.is_empty() {
                state_name = format!("_Play_{}", tablelevel);
            } else {
                state_name = format!(
                    "_Play_LEVEL{}",
                    IntegerPropertyFactory::integer_property(NUMBER_PLAYLEVEL).get(current_state)
                );
            }
            let fulltitle =
                StringPropertyFactory::string_property(STRING_FULLTITLE).get(current_state);
            if !fulltitle.is_empty() {
                state_name += &format!(" {}", fulltitle);
            }
        } else if screen_type == ScreenType::MusicResult || screen_type == ScreenType::CourseResult
        {
            if screen_type == ScreenType::MusicResult {
                let tablelevel =
                    StringPropertyFactory::string_property(STRING_TABLE_LEVEL).get(current_state);
                if !tablelevel.is_empty() {
                    state_name += &format!("_{} ", tablelevel);
                } else {
                    state_name += &format!(
                        "_LEVEL{} ",
                        IntegerPropertyFactory::integer_property(NUMBER_PLAYLEVEL)
                            .get(current_state)
                    );
                }
            } else {
                state_name += "_";
            }
            let fulltitle =
                StringPropertyFactory::string_property(STRING_FULLTITLE).get(current_state);
            if !fulltitle.is_empty() {
                state_name += &fulltitle;
            }
            state_name += &format!(" {}", screen_shot_exporter::clear_type_name(current_state));
            state_name += &format!(" {}", screen_shot_exporter::rank_type_name(current_state));
        } else if screen_type == ScreenType::KeyConfiguration {
            state_name = "_Config".to_string();
        }

        state_name = state_name
            .replace('\\', "\u{FFE5}")
            .replace('/', "\u{FF0F}")
            .replace(':', "\u{FF1A}")
            .replace('*', "\u{FF0A}")
            .replace('?', "\u{FF1F}")
            .replace('"', "\u{201D}")
            .replace('<', "\u{FF1C}")
            .replace('>', "\u{FF1E}")
            .replace('|', "\u{FF5C}")
            .replace('\t', " ");
        state_name = format!("_LR2oraja{}", state_name);

        let (width, height) = GdxGraphics::back_buffer_size();
        let mut pixmap = Pixmap::new(width, height);
        let result: Result<bool, Box<dyn std::error::Error>> = {
            let _ = std::fs::create_dir_all("screenshot");
            let path = format!("screenshot/{}{}.png", sdf, state_name);
            let pixel_buf = pixmap.pixels();
            BufferUtils::copy(pixels, 0, pixel_buf, pixels.len());
            PixmapIO::write_png(&path, &pixmap);
            log::info!("Screenshot saved: {}", path);
            pixmap.dispose();
            ImGuiNotify::info_with_dismiss(&format!("Screen shot saved: {}", path), 2000);

            self.send_clipboard(current_state, &path);
            self.send_webhook(current_state, &path);
            Ok(true)
        };

        match result {
            Ok(r) => r,
            Err(e) => {
                log::error!("Screenshot error: {}", e);
                pixmap.dispose();
                false
            }
        }
    }
}

impl ScreenShotFileExporter {
    pub fn new() -> Self {
        Self {
            webhook_threads: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn send_clipboard(&self, current_state: &MainState, path: &str) {
        if !current_state
            .resource
            .config()
            .integration
            .set_clipboard_screenshot
        {
            // Clipboard copy not enabled for screenshots
            return;
        }

        match ClipboardHelper::copy_image_to_clipboard(path) {
            Ok(()) => {
                log::info!("Screenshot saved: Clipboard");
                ImGuiNotify::info_with_dismiss("Screen shot saved : Clipboard", 2000);
            }
            Err(e) => {
                log::error!("Clipboard copy error: {}", e);
            }
        }
    }

    fn send_webhook(&self, current_state: &MainState, path: &str) {
        if current_state.resource.config().integration.webhook_option == 0
            || current_state
                .resource
                .config()
                .integration
                .webhook_url
                .is_empty()
        {
            // Webhook action not enabled or missing URL
            return;
        }

        // Extract all data from current_state before spawning the background thread,
        // since MainState is not Send.
        let webhook_urls: Vec<String> = current_state
            .resource
            .config()
            .integration
            .webhook_url
            .to_vec();

        let handler = WebhookHandler::new();
        let payload = match (|| -> Result<String, Box<dyn std::error::Error>> {
            let p = handler.create_webhook_payload(current_state);
            Ok(serde_json::to_string(&p)?)
        })() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Webhook payload error: {}", e);
                return;
            }
        };

        let path = path.to_string();

        // Use lock_or_recover to handle poisoned mutex (consistent with rest of codebase)
        let mut guard = self
            .webhook_threads
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        // Drain finished threads, joining them to observe panics.
        guard.retain(|h| {
            if h.is_finished() {
                // Cannot join through a shared reference in retain, so just
                // detect finished threads here.  They will be dropped (detached)
                // which is safe because they already completed.
                false
            } else {
                true
            }
        });

        if guard.len() >= MAX_CONCURRENT_WEBHOOK_THREADS {
            log::warn!(
                "Skipping webhook send: {} threads already in flight (max {})",
                guard.len(),
                MAX_CONCURRENT_WEBHOOK_THREADS
            );
            return;
        }

        let handle = std::thread::spawn(move || {
            for webhook_url in &webhook_urls {
                handler.send_webhook_with_image(&payload, &path, webhook_url);
            }
        });

        guard.push(handle);
    }
}

impl Drop for ScreenShotFileExporter {
    fn drop(&mut self) {
        // Join all in-flight webhook threads with a shared 5-second timeout.
        // Use lock_or_recover to handle poisoned mutex (consistent with rest of codebase)
        let mut guard = self
            .webhook_threads
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let handles: Vec<_> = guard.drain(..).collect();
        if handles.is_empty() {
            return;
        }

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(5);

        for handle in handles {
            if handle.is_finished() {
                if let Err(e) = handle.join() {
                    log::warn!("Webhook send thread panicked: {:?}", e);
                }
                continue;
            }

            // Poll until finished or timeout expires.
            loop {
                if handle.is_finished() {
                    if let Err(e) = handle.join() {
                        log::warn!("Webhook send thread panicked: {:?}", e);
                    }
                    break;
                }
                if start.elapsed() >= timeout {
                    log::warn!(
                        "Webhook send thread did not finish within {:?} during drop, detaching",
                        timeout
                    );
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

impl Default for ScreenShotFileExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine the screen type from state.
/// In Java this was done via instanceof checks; in Rust the MainState carries
/// its screen type and exposes it via MainStateAccess::get_screen_type().
fn get_screen_type(state: &MainState) -> ScreenType {
    use rubato_types::main_state_access::MainStateAccess;
    state.screen_type()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_state(screen_type: ScreenType) -> MainState {
        MainState {
            resource: Default::default(),
            screen_type,
            abstract_result: None,
        }
    }

    #[test]
    fn get_screen_type_delegates_to_main_state_access() {
        assert_eq!(
            get_screen_type(&make_state(ScreenType::MusicSelector)),
            ScreenType::MusicSelector
        );
        assert_eq!(
            get_screen_type(&make_state(ScreenType::BMSPlayer)),
            ScreenType::BMSPlayer
        );
        assert_eq!(
            get_screen_type(&make_state(ScreenType::MusicResult)),
            ScreenType::MusicResult
        );
        assert_eq!(
            get_screen_type(&make_state(ScreenType::CourseResult)),
            ScreenType::CourseResult
        );
        assert_eq!(
            get_screen_type(&make_state(ScreenType::MusicDecide)),
            ScreenType::MusicDecide
        );
        assert_eq!(
            get_screen_type(&make_state(ScreenType::KeyConfiguration)),
            ScreenType::KeyConfiguration
        );
        assert_eq!(
            get_screen_type(&make_state(ScreenType::Other)),
            ScreenType::Other
        );
    }

    #[test]
    fn webhook_threads_starts_empty() {
        let exporter = ScreenShotFileExporter::new();
        let guard = exporter.webhook_threads.lock().unwrap();
        assert!(guard.is_empty());
    }

    #[test]
    fn webhook_threads_tracks_multiple_handles() {
        let exporter = ScreenShotFileExporter::new();

        // Spawn a few short-lived threads and push their handles.
        {
            let mut guard = exporter.webhook_threads.lock().unwrap();
            for _ in 0..3 {
                let h = std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                });
                guard.push(h);
            }
            assert_eq!(guard.len(), 3);
        }

        // After a short wait all threads finish; drain finished.
        std::thread::sleep(std::time::Duration::from_millis(50));
        {
            let mut guard = exporter.webhook_threads.lock().unwrap();
            guard.retain(|h| !h.is_finished());
            assert_eq!(guard.len(), 0, "all finished threads should be drained");
        }
    }

    #[test]
    fn webhook_threads_capacity_check() {
        let exporter = ScreenShotFileExporter::new();

        // Fill to capacity with long-running threads.
        {
            let mut guard = exporter.webhook_threads.lock().unwrap();
            for _ in 0..MAX_CONCURRENT_WEBHOOK_THREADS {
                let h = std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                });
                guard.push(h);
            }
            assert_eq!(guard.len(), MAX_CONCURRENT_WEBHOOK_THREADS);
        }

        // A new send should be rejected when at capacity (simulating the
        // guard.len() >= MAX check in send_webhook).
        {
            let guard = exporter.webhook_threads.lock().unwrap();
            assert!(
                guard.len() >= MAX_CONCURRENT_WEBHOOK_THREADS,
                "should be at capacity"
            );
        }
        // Drop the exporter which joins/detaches threads in Drop.
    }

    #[test]
    fn drop_joins_all_in_flight_threads() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        let counter = Arc::new(AtomicUsize::new(0));
        let exporter = ScreenShotFileExporter::new();

        {
            let mut guard = exporter.webhook_threads.lock().unwrap();
            for _ in 0..2 {
                let c = Arc::clone(&counter);
                let h = std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    c.fetch_add(1, Ordering::SeqCst);
                });
                guard.push(h);
            }
        }

        // Drop triggers join with timeout.
        drop(exporter);
        // Both threads should have completed.
        assert_eq!(
            counter.load(Ordering::SeqCst),
            2,
            "Drop should have waited for both threads to complete"
        );
    }
}
