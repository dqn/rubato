use std::collections::VecDeque;
use std::thread;

use beatoraja_render::pixmap::Pixmap;
use bms_model::bms_model::BMSModel;

use crate::bms_player_mode::BMSPlayerMode;
use crate::config::Config;
use crate::main_loader::MainLoader;
use crate::player_config::PlayerConfig;
use std::path::Path;

/// TextureRegion stub (LibGDX equivalent)
#[derive(Clone)]
pub struct TextureRegion;

/// BGAProcessor stub (Phase 5+ dependency)
pub struct BGAProcessor;

impl BGAProcessor {
    pub fn new(_config: &Config, _player: &PlayerConfig) -> Self {
        Self
    }

    pub fn abort(&mut self) {
        // Phase 5+ dependency
    }

    pub fn set_model(&mut self, _model: Option<&BMSModel>) {
        // Phase 5+ dependency
    }

    pub fn dispose(&mut self) {
        // Phase 5+ dependency
    }
}

/// AudioDriver stub (Phase 5+ dependency) - re-exported from beatoraja-audio
pub struct AudioDriverStub;

impl AudioDriverStub {
    pub fn abort(&mut self) {}
    pub fn set_model(&mut self, _model: ()) {}
    pub fn dispose(&mut self) {}
}

/// BMSResource - manages BMS audio and BGA resources
pub struct BMSResource {
    /// Audio driver
    audio: Option<AudioDriverStub>,
    /// BGA processor
    bga: Option<BGAProcessor>,
    /// Whether BGA is enabled
    bgaon: bool,
    /// Audio loader threads
    audioloaders: VecDeque<thread::JoinHandle<()>>,
    /// BGA loader threads
    bgaloaders: VecDeque<thread::JoinHandle<()>>,
    /// backbmp texture
    backbmp: Option<TextureRegion>,
    /// stagefile texture
    stagefile: Option<TextureRegion>,
    /// stagefile pixmap reference
    stagefile_pix: Option<Pixmap>,
    /// banner texture
    banner: Option<TextureRegion>,
    /// banner pixmap reference
    banner_pix: Option<Pixmap>,
}

impl BMSResource {
    pub fn new(config: &Config, player: &PlayerConfig) -> Self {
        Self {
            audio: Some(AudioDriverStub),
            bga: Some(BGAProcessor::new(config, player)),
            bgaon: false,
            audioloaders: VecDeque::new(),
            bgaloaders: VecDeque::new(),
            backbmp: None,
            stagefile: None,
            stagefile_pix: None,
            banner: None,
            banner_pix: None,
        }
    }

    pub fn set_bms_file(
        &mut self,
        _model: &BMSModel,
        _f: &Path,
        _config: &Config,
        _mode: &BMSPlayerMode,
    ) -> bool {
        // Dispose old stagefile
        self.stagefile = None;
        // Try to load stagefile
        // Phase 5+ dependency: PixmapResourcePool.loadPicture, Texture creation

        // Dispose old backbmp
        self.backbmp = None;
        // Try to load backbmp
        // Phase 5+ dependency: PixmapResourcePool.loadPicture, Texture creation

        // Clean up finished loader threads
        while let Some(front) = self.audioloaders.front() {
            if front.is_finished() {
                self.audioloaders.pop_front();
            } else {
                break;
            }
        }
        while let Some(front) = self.bgaloaders.front() {
            if front.is_finished() {
                self.bgaloaders.pop_front();
            } else {
                break;
            }
        }

        if MainLoader::get_illegal_song_count() == 0 {
            // Audio and BGA both have caches, so always do a full reload
            // Phase 5+ dependency: spawn BGA and audio loader threads
        }

        true
    }

    pub fn get_audio_driver(&self) -> Option<&AudioDriverStub> {
        self.audio.as_ref()
    }

    pub fn get_bga_processor(&self) -> Option<&BGAProcessor> {
        self.bga.as_ref()
    }

    pub fn is_bga_on(&self) -> bool {
        self.bgaon
    }

    pub fn media_load_finished(&self) -> bool {
        if let Some(last) = self.audioloaders.back()
            && !last.is_finished()
        {
            return false;
        }
        if let Some(last) = self.bgaloaders.back()
            && !last.is_finished()
        {
            return false;
        }
        true
    }

    pub fn get_backbmp(&self) -> Option<&TextureRegion> {
        self.backbmp.as_ref()
    }

    pub fn get_stagefile(&self) -> Option<&TextureRegion> {
        self.stagefile.as_ref()
    }

    pub fn get_banner(&self) -> Option<&TextureRegion> {
        self.banner.as_ref()
    }

    /// Set the stagefile pixmap.
    /// Java: BMSResource.setStagefile(Pixmap pixmap)
    pub fn set_stagefile(&mut self, pixmap: Option<Pixmap>) {
        let _old_stagefile = self.stagefile.clone();
        if let Some(p) = pixmap {
            // Store pixmap for later TextureRegion creation (Phase 29a rendering)
            self.stagefile_pix = Some(p);
            // TextureRegion creation deferred to rendering pipeline
        } else {
            self.stagefile = None;
            self.stagefile_pix = None;
        }
        // Dispose old if changed
    }

    /// Set the banner pixmap.
    /// Java: BMSResource.setBanner(Pixmap pixmap)
    pub fn set_banner(&mut self, pixmap: Option<Pixmap>) {
        let _old_banner = self.banner.clone();
        if let Some(p) = pixmap {
            // Store pixmap for later TextureRegion creation (Phase 29a rendering)
            self.banner_pix = Some(p);
            // TextureRegion creation deferred to rendering pipeline
        } else {
            self.banner = None;
            self.banner_pix = None;
        }
        // Dispose old if changed
    }

    /// Get the stagefile pixmap reference.
    pub fn get_stagefile_pix(&self) -> Option<&Pixmap> {
        self.stagefile_pix.as_ref()
    }

    /// Get the banner pixmap reference.
    pub fn get_banner_pix(&self) -> Option<&Pixmap> {
        self.banner_pix.as_ref()
    }

    pub fn dispose(&mut self) {
        if let Some(mut audio) = self.audio.take() {
            audio.dispose();
        }
        if let Some(mut bga) = self.bga.take() {
            bga.dispose();
        }
        self.stagefile = None;
        self.backbmp = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_render::pixmap::PixmapFormat;

    fn make_test_pixmap(w: i32, h: i32) -> Pixmap {
        Pixmap::new(w, h, PixmapFormat::RGBA8888)
    }

    fn make_bms_resource() -> BMSResource {
        let config = Config::default();
        let player = PlayerConfig::default();
        BMSResource::new(&config, &player)
    }

    #[test]
    fn test_set_banner_some_stores_pixmap() {
        let mut res = make_bms_resource();
        let pixmap = make_test_pixmap(128, 40);
        res.set_banner(Some(pixmap));
        let pix = res.get_banner_pix().unwrap();
        assert_eq!(pix.get_width(), 128);
        assert_eq!(pix.get_height(), 40);
    }

    #[test]
    fn test_set_banner_none_clears_pixmap() {
        let mut res = make_bms_resource();
        let pixmap = make_test_pixmap(128, 40);
        res.set_banner(Some(pixmap));
        assert!(res.get_banner_pix().is_some());

        res.set_banner(None);
        assert!(res.get_banner_pix().is_none());
        assert!(res.get_banner().is_none());
    }

    #[test]
    fn test_set_stagefile_some_stores_pixmap() {
        let mut res = make_bms_resource();
        let pixmap = make_test_pixmap(640, 480);
        res.set_stagefile(Some(pixmap));
        let pix = res.get_stagefile_pix().unwrap();
        assert_eq!(pix.get_width(), 640);
        assert_eq!(pix.get_height(), 480);
    }

    #[test]
    fn test_set_stagefile_none_clears_pixmap() {
        let mut res = make_bms_resource();
        let pixmap = make_test_pixmap(640, 480);
        res.set_stagefile(Some(pixmap));
        assert!(res.get_stagefile_pix().is_some());

        res.set_stagefile(None);
        assert!(res.get_stagefile_pix().is_none());
        assert!(res.get_stagefile().is_none());
    }

    #[test]
    fn test_initial_state_no_pixmaps() {
        let res = make_bms_resource();
        assert!(res.get_banner_pix().is_none());
        assert!(res.get_stagefile_pix().is_none());
        assert!(res.get_banner().is_none());
        assert!(res.get_stagefile().is_none());
    }
}
