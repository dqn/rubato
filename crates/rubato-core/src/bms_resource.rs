use std::collections::VecDeque;
use std::thread;

use bms_model::bms_model::BMSModel;
use rubato_render::pixmap::Pixmap;
use rubato_render::texture::Texture;

use crate::bms_player_mode::{BMSPlayerMode, Mode};
use crate::config::{BgaMode, Config};
use crate::main_loader::MainLoader;
use crate::pixmap_resource_pool::PixmapResourcePool;
use crate::player_config::PlayerConfig;
use std::path::Path;

/// TextureRegion re-exported from beatoraja-render (LibGDX equivalent)
pub use rubato_render::texture::TextureRegion;

/// BMSResource manages BMS stagefile, backbmp, and banner image resources,
/// and tracks background loader threads for audio and BGA.
///
/// Architecture note: In Java, BMSResource also owns the AudioDriver and
/// BGAProcessor instances. In Rust, those are managed separately:
/// - AudioDriver is owned by MainController and injected via set_audio_driver().
/// - BGAProcessor is owned by BMSPlayer (rubato-play crate) and shared via
///   Arc<Mutex<BGAProcessor>> through PlayerResource for cache reuse.
///
/// BMSResource retains the loader thread tracking and image resource management
/// (stagefile, backbmp, banner) from the Java original.
pub struct BMSResource {
    /// Whether BGA is enabled for the current song
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
    pub fn new(_config: &Config, _player: &PlayerConfig) -> Self {
        Self {
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

    /// Load stagefile, backbmp images from the BMS model and clean up finished
    /// loader threads.
    ///
    /// Java: BMSResource.setBMSFile(BMSModel, Path, Config, BMSPlayerMode)
    pub fn set_bms_file(
        &mut self,
        model: &BMSModel,
        f: &Path,
        config: &Config,
        mode: &BMSPlayerMode,
    ) -> bool {
        // Dispose old stagefile and try to load the new one
        self.stagefile = None;
        if let Some(parent) = f.parent() {
            if !model.stagefile.is_empty()
                && rubato_audio::audio_driver::is_bms_resource_path_safe(&model.stagefile)
            {
                let stagefile_path = parent.join(&model.stagefile);
                if let Some(pix) =
                    PixmapResourcePool::load_picture(&stagefile_path.to_string_lossy())
                {
                    let tex = Texture::from_pixmap(&pix);
                    self.stagefile = Some(TextureRegion::from_texture(tex));
                }
            }

            // Dispose old backbmp and try to load the new one
            self.backbmp = None;
            if !model.backbmp.is_empty()
                && rubato_audio::audio_driver::is_bms_resource_path_safe(&model.backbmp)
            {
                let backbmp_path = parent.join(&model.backbmp);
                if let Some(pix) = PixmapResourcePool::load_picture(&backbmp_path.to_string_lossy())
                {
                    let tex = Texture::from_pixmap(&pix);
                    self.backbmp = Some(TextureRegion::from_texture(tex));
                }
            }
        }

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
            // Determine whether BGA should be enabled for this song.
            // Java: config.getBga() == Config.BGA_ON || (config.getBga() == Config.BGA_AUTO
            //        && (mode == AUTOPLAY || mode == REPLAY))
            let bga_mode = config.render.bga;
            let _bga_enabled = bga_mode == BgaMode::On
                || (bga_mode == BgaMode::Auto
                    && (mode.mode == Mode::Autoplay || mode.mode == Mode::Replay));
            self.bgaon = _bga_enabled;

            // Audio loading and BGA resource loading are handled externally:
            // - Audio: MainController owns the AudioDriver; loading is triggered
            //   through the audio system after set_bms_file returns.
            // - BGA: BMSPlayer (rubato-play) owns the BGAProcessor; resource loading
            //   (images/movies) is dispatched by PlayerResource during create().
        }

        true
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

    pub fn backbmp(&self) -> Option<&TextureRegion> {
        self.backbmp.as_ref()
    }

    pub fn stagefile(&self) -> Option<&TextureRegion> {
        self.stagefile.as_ref()
    }

    pub fn banner(&self) -> Option<&TextureRegion> {
        self.banner.as_ref()
    }

    /// Set the stagefile pixmap.
    /// Java: BMSResource.setStagefile(Pixmap pixmap)
    pub fn set_stagefile(&mut self, pixmap: Option<Pixmap>) {
        if let Some(p) = pixmap {
            let tex = Texture::from_pixmap(&p);
            self.stagefile = Some(TextureRegion::from_texture(tex));
            self.stagefile_pix = Some(p);
        } else {
            self.stagefile = None;
            self.stagefile_pix = None;
        }
    }

    /// Set the banner pixmap.
    /// Java: BMSResource.setBanner(Pixmap pixmap)
    pub fn set_banner(&mut self, pixmap: Option<Pixmap>) {
        if let Some(p) = pixmap {
            let tex = Texture::from_pixmap(&p);
            self.banner = Some(TextureRegion::from_texture(tex));
            self.banner_pix = Some(p);
        } else {
            self.banner = None;
            self.banner_pix = None;
        }
    }

    /// Get the stagefile pixmap reference.
    pub fn stagefile_pix(&self) -> Option<&Pixmap> {
        self.stagefile_pix.as_ref()
    }

    /// Get the banner pixmap reference.
    pub fn banner_pix(&self) -> Option<&Pixmap> {
        self.banner_pix.as_ref()
    }

    pub fn dispose(&mut self) {
        // Audio and BGA disposal is handled by their respective owners:
        // - AudioDriver: disposed by MainController
        // - BGAProcessor: disposed by BMSPlayer (rubato-play)
        self.stagefile = None;
        self.stagefile_pix = None;
        self.backbmp = None;
        self.banner = None;
        self.banner_pix = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_render::pixmap::PixmapFormat;

    fn make_test_pixmap(w: i32, h: i32) -> Pixmap {
        Pixmap::new(w, h, PixmapFormat::RGBA8888)
    }

    fn make_bms_resource() -> BMSResource {
        let config = Config::default();
        let player = PlayerConfig::default();
        BMSResource::new(&config, &player)
    }

    #[test]
    fn test_set_banner_some_stores_pixmap_and_texture() {
        let mut res = make_bms_resource();
        let pixmap = make_test_pixmap(128, 40);
        res.set_banner(Some(pixmap));
        let pix = res.banner_pix().unwrap();
        assert_eq!(pix.width, 128);
        assert_eq!(pix.height, 40);
        // TextureRegion should also be created
        let tex_region = res.banner().unwrap();
        assert_eq!(tex_region.region_width, 128);
        assert_eq!(tex_region.region_height, 40);
    }

    #[test]
    fn test_set_banner_none_clears_pixmap() {
        let mut res = make_bms_resource();
        let pixmap = make_test_pixmap(128, 40);
        res.set_banner(Some(pixmap));
        assert!(res.banner_pix().is_some());

        res.set_banner(None);
        assert!(res.banner_pix().is_none());
        assert!(res.banner().is_none());
    }

    #[test]
    fn test_set_stagefile_some_stores_pixmap_and_texture() {
        let mut res = make_bms_resource();
        let pixmap = make_test_pixmap(640, 480);
        res.set_stagefile(Some(pixmap));
        let pix = res.stagefile_pix().unwrap();
        assert_eq!(pix.width, 640);
        assert_eq!(pix.height, 480);
        // TextureRegion should also be created
        let tex_region = res.stagefile().unwrap();
        assert_eq!(tex_region.region_width, 640);
        assert_eq!(tex_region.region_height, 480);
    }

    #[test]
    fn test_set_stagefile_none_clears_pixmap() {
        let mut res = make_bms_resource();
        let pixmap = make_test_pixmap(640, 480);
        res.set_stagefile(Some(pixmap));
        assert!(res.stagefile_pix().is_some());

        res.set_stagefile(None);
        assert!(res.stagefile_pix().is_none());
        assert!(res.stagefile().is_none());
    }

    #[test]
    fn test_initial_state_no_pixmaps() {
        let res = make_bms_resource();
        assert!(res.banner_pix().is_none());
        assert!(res.stagefile_pix().is_none());
        assert!(res.banner().is_none());
        assert!(res.stagefile().is_none());
    }

    #[test]
    fn test_dispose_clears_all_image_resources() {
        let mut res = make_bms_resource();
        res.set_stagefile(Some(make_test_pixmap(640, 480)));
        res.set_banner(Some(make_test_pixmap(128, 40)));
        assert!(res.stagefile().is_some());
        assert!(res.banner().is_some());

        res.dispose();
        assert!(res.stagefile().is_none());
        assert!(res.stagefile_pix().is_none());
        assert!(res.backbmp().is_none());
        assert!(res.banner().is_none());
        assert!(res.banner_pix().is_none());
    }

    #[test]
    fn test_set_bms_file_loads_stagefile_from_real_image() {
        let dir = tempfile::tempdir().unwrap();
        let img = image::RgbaImage::from_pixel(320, 240, image::Rgba([0, 0, 255, 255]));
        img.save(dir.path().join("stage.png")).unwrap();

        let bms_path = dir.path().join("test.bms");
        std::fs::write(&bms_path, "").unwrap();

        let mut model = BMSModel::default();
        model.stagefile = "stage.png".to_string();

        let config = Config::default();
        let mode = BMSPlayerMode::PLAY;
        let mut res = make_bms_resource();

        res.set_bms_file(&model, &bms_path, &config, &mode);
        let sf = res.stagefile().expect("stagefile should be loaded");
        assert_eq!(sf.region_width, 320);
        assert_eq!(sf.region_height, 240);
    }

    #[test]
    fn test_set_bms_file_loads_backbmp_from_real_image() {
        let dir = tempfile::tempdir().unwrap();
        let img = image::RgbaImage::from_pixel(256, 256, image::Rgba([255, 0, 0, 255]));
        img.save(dir.path().join("back.png")).unwrap();

        let bms_path = dir.path().join("test.bms");
        std::fs::write(&bms_path, "").unwrap();

        let mut model = BMSModel::default();
        model.backbmp = "back.png".to_string();

        let config = Config::default();
        let mode = BMSPlayerMode::PLAY;
        let mut res = make_bms_resource();

        res.set_bms_file(&model, &bms_path, &config, &mode);
        let bb = res.backbmp().expect("backbmp should be loaded");
        assert_eq!(bb.region_width, 256);
        assert_eq!(bb.region_height, 256);
    }

    #[test]
    fn test_set_bms_file_missing_image_leaves_none() {
        let dir = tempfile::tempdir().unwrap();
        let bms_path = dir.path().join("test.bms");
        std::fs::write(&bms_path, "").unwrap();

        let mut model = BMSModel::default();
        model.stagefile = "nonexistent.png".to_string();
        model.backbmp = "also_missing.png".to_string();

        let config = Config::default();
        let mode = BMSPlayerMode::PLAY;
        let mut res = make_bms_resource();

        res.set_bms_file(&model, &bms_path, &config, &mode);
        assert!(res.stagefile().is_none());
        assert!(res.backbmp().is_none());
    }

    #[test]
    fn test_set_bms_file_empty_stagefile_string_skips_loading() {
        let dir = tempfile::tempdir().unwrap();
        let bms_path = dir.path().join("test.bms");
        std::fs::write(&bms_path, "").unwrap();

        let model = BMSModel::default(); // stagefile and backbmp are empty strings

        let config = Config::default();
        let mode = BMSPlayerMode::PLAY;
        let mut res = make_bms_resource();

        res.set_bms_file(&model, &bms_path, &config, &mode);
        assert!(res.stagefile().is_none());
        assert!(res.backbmp().is_none());
    }

    #[test]
    fn test_set_bms_file_bgaon_follows_config() {
        let dir = tempfile::tempdir().unwrap();
        let bms_path = dir.path().join("test.bms");
        std::fs::write(&bms_path, "").unwrap();
        let model = BMSModel::default();

        // BGA_ON mode => bgaon = true regardless of play mode
        let mut config = Config::default();
        config.render.bga = BgaMode::On;
        let mut res = make_bms_resource();
        res.set_bms_file(&model, &bms_path, &config, &BMSPlayerMode::PLAY);
        assert!(res.is_bga_on());

        // BGA_OFF mode => bgaon = false
        config.render.bga = BgaMode::Off;
        let mut res = make_bms_resource();
        res.set_bms_file(&model, &bms_path, &config, &BMSPlayerMode::PLAY);
        assert!(!res.is_bga_on());

        // BGA_AUTO + AUTOPLAY => bgaon = true
        config.render.bga = BgaMode::Auto;
        let mut res = make_bms_resource();
        res.set_bms_file(&model, &bms_path, &config, &BMSPlayerMode::AUTOPLAY);
        assert!(res.is_bga_on());

        // BGA_AUTO + PLAY => bgaon = false
        let mut res = make_bms_resource();
        res.set_bms_file(&model, &bms_path, &config, &BMSPlayerMode::PLAY);
        assert!(!res.is_bga_on());
    }
}
