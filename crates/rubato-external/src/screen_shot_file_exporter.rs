use crate::screen_shot_exporter::{self, ScreenShotExporter};
use crate::webhook_handler::WebhookHandler;
use crate::{
    BufferUtils, ClipboardHelper, GdxGraphics, ImGuiNotify, IntegerPropertyFactory, MainState,
    NUMBER_PLAYLEVEL, Pixmap, PixmapIO, STRING_FULLTITLE, STRING_TABLE_LEVEL, ScreenType,
    StringPropertyFactory,
};

/// ScreenShotFileExporter - saves screenshots to file and optionally copies to clipboard / sends webhook.
/// Translated from Java: ScreenShotFileExporter implements ScreenShotExporter
pub struct ScreenShotFileExporter {
    /// JoinHandle for the most recent webhook send background thread.
    webhook_thread: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

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
            webhook_thread: std::sync::Mutex::new(None),
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

        let handle = std::thread::spawn(move || {
            for webhook_url in &webhook_urls {
                handler.send_webhook_with_image(&payload, &path, webhook_url);
            }
        });

        // Store the handle; join any previous thread that is still running.
        if let Ok(mut guard) = self.webhook_thread.lock() {
            if let Some(prev) = guard.take()
                && prev.is_finished()
                && let Err(e) = prev.join()
            {
                log::warn!("Previous webhook thread panicked: {:?}", e);
            }
            // If previous thread is still running, we let it detach
            // (it will finish on its own). Storing the new handle ensures
            // at least the latest thread is joined on drop.
            *guard = Some(handle);
        }
    }
}

impl Drop for ScreenShotFileExporter {
    fn drop(&mut self) {
        // Webhook sends are fire-and-forget. Let any in-flight thread
        // finish on its own instead of busy-waiting up to 10s, which
        // would freeze the render thread during shutdown.
        // If the thread finished, join to collect panics; otherwise detach (drop the JoinHandle).
        if let Ok(mut guard) = self.webhook_thread.lock()
            && let Some(handle) = guard.take()
            && handle.is_finished()
            && let Err(e) = handle.join()
        {
            log::warn!("Webhook send thread panicked: {:?}", e);
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
}
