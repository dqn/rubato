use crate::screen_shot_exporter::{self, ScreenShotExporter};
use crate::stubs::{
    BufferUtils, GdxGraphics, ImGuiNotify, IntegerPropertyFactory, MainState, NUMBER_PLAYLEVEL,
    Pixmap, PixmapIO, PlayerConfig, STRING_FULLTITLE, STRING_TABLE_LEVEL, STRING_TABLE_NAME,
    ScreenType, StatusUpdate, StringPropertyFactory, TwitterConfigurationBuilder, TwitterFactory,
};

/// ScreenShotTwitterExporter - posts screenshots to Twitter.
/// Translated from Java: ScreenShotTwitterExporter implements ScreenShotExporter
pub struct ScreenShotTwitterExporter {
    consumer_key: String,
    consumer_secret: String,
    access_token: String,
    access_token_secret: String,
}

impl ScreenShotTwitterExporter {
    pub fn new(player: &PlayerConfig) -> Self {
        Self {
            consumer_key: player
                .twitter_consumer_key()
                .unwrap_or_default()
                .to_string(),
            consumer_secret: player
                .twitter_consumer_secret()
                .unwrap_or_default()
                .to_string(),
            access_token: player
                .twitter_access_token()
                .unwrap_or_default()
                .to_string(),
            access_token_secret: player
                .twitter_access_token_secret()
                .unwrap_or_default()
                .to_string(),
        }
    }
}

impl ScreenShotExporter for ScreenShotTwitterExporter {
    fn send(&self, current_state: &MainState, pixels: &[u8]) -> bool {
        let mut builder = String::new();

        let screen_type = get_screen_type(current_state);

        // MusicSelector and MusicDecide: no-op (Java stub)

        if screen_type == ScreenType::BMSPlayer {
            let tablename =
                StringPropertyFactory::string_property(STRING_TABLE_NAME).get(current_state);
            let tablelevel =
                StringPropertyFactory::string_property(STRING_TABLE_LEVEL).get(current_state);

            if !tablename.is_empty() {
                builder += &tablelevel;
            } else {
                builder += &format!(
                    "LEVEL{}",
                    IntegerPropertyFactory::integer_property(NUMBER_PLAYLEVEL).get(current_state)
                );
            }
            let fulltitle =
                StringPropertyFactory::string_property(STRING_FULLTITLE).get(current_state);
            if !fulltitle.is_empty() {
                builder += &format!(" {}", fulltitle);
            }
        } else if screen_type == ScreenType::MusicResult || screen_type == ScreenType::CourseResult
        {
            if screen_type == ScreenType::MusicResult {
                let tablename =
                    StringPropertyFactory::string_property(STRING_TABLE_NAME).get(current_state);
                let tablelevel =
                    StringPropertyFactory::string_property(STRING_TABLE_LEVEL).get(current_state);
                if !tablename.is_empty() {
                    builder += &tablelevel;
                } else {
                    builder += &format!(
                        "LEVEL{}",
                        IntegerPropertyFactory::integer_property(NUMBER_PLAYLEVEL)
                            .get(current_state)
                    );
                }
            }
            let fulltitle =
                StringPropertyFactory::string_property(STRING_FULLTITLE).get(current_state);
            if !fulltitle.is_empty() {
                builder += &format!(" {}", fulltitle);
            }
            builder += " ";
            builder += &screen_shot_exporter::clear_type_name(current_state);
            builder += " ";
            builder += &screen_shot_exporter::rank_type_name(current_state);
        } else if screen_type == ScreenType::KeyConfiguration {
            // empty
        }

        let mut text = builder;
        text = text
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

        let cb = TwitterConfigurationBuilder::new()
            .set_o_auth_consumer_key(&self.consumer_key)
            .set_o_auth_consumer_secret(&self.consumer_secret)
            .set_o_auth_access_token(&self.access_token)
            .set_o_auth_access_token_secret(&self.access_token_secret);
        let twitter_factory = TwitterFactory::new(cb.build());
        let twitter = twitter_factory.instance();

        let width = GdxGraphics::back_buffer_width();
        let height = GdxGraphics::back_buffer_height();
        let mut pixmap = Pixmap::new(width, height);
        let result: Result<bool, Box<dyn std::error::Error>> = (|| {
            // create png byte stream
            let pixel_buf = pixmap.pixels();
            BufferUtils::copy(pixels, 0, pixel_buf, pixels.len());
            let image_bytes = PixmapIO::encode_png_bytes(&pixmap);

            // Upload Media and Post
            let mediastatus = twitter.upload_media("from beatoraja", &image_bytes)?;
            log::info!("Twitter Media Upload:{}", mediastatus);
            let mut update = StatusUpdate::new(text.clone());
            update.media_ids = vec![mediastatus.media_id];
            let status = twitter.update_status(&update)?;
            log::info!("Twitter Post:{}", status);
            pixmap.dispose();
            ImGuiNotify::info_with_dismiss(&format!("Twitter Upload: {}", text), 2000);
            Ok(true)
        })();

        match result {
            Ok(r) => r,
            Err(e) => {
                log::error!("Twitter upload error: {}", e);
                pixmap.dispose();
                false
            }
        }
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
    use crate::stubs::NullMainController;

    fn make_state(screen_type: ScreenType) -> MainState {
        MainState {
            main: NullMainController,
            resource: Default::default(),
            screen_type,
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
            get_screen_type(&make_state(ScreenType::Other)),
            ScreenType::Other
        );
    }
}
