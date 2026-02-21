use std::collections::VecDeque;
use std::thread;

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

    pub fn set_model(&mut self, _model: Option<()>) {
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
    _stagefile_pix: Option<()>,
    /// banner texture
    banner: Option<TextureRegion>,
    /// banner pixmap reference
    _banner_pix: Option<()>,
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
            _stagefile_pix: None,
            banner: None,
            _banner_pix: None,
        }
    }

    pub fn set_bms_file(
        &mut self,
        _model: &(),
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

    pub fn set_stagefile(&mut self, pixmap: Option<()>) {
        let _old_stagefile = self.stagefile.clone();
        if pixmap.is_some() {
            // Phase 5+: create TextureRegion from pixmap
            // For now, stub
        } else {
            self.stagefile = None;
            self._stagefile_pix = None;
        }
        // Dispose old if changed
    }

    pub fn set_banner(&mut self, pixmap: Option<()>) {
        let _old_banner = self.banner.clone();
        if pixmap.is_some() {
            // Phase 5+: create TextureRegion from pixmap
        } else {
            self.banner = None;
            self._banner_pix = None;
        }
        // Dispose old if changed
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
