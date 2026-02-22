use crate::bar::bar::Bar;
use crate::bar::directory_bar::DirectoryBarData;
use crate::bar::function_bar::FunctionBar;
use crate::bar::song_bar::SongBar;
use crate::stubs::*;

/// Distribution graph for folder lamp/rank display
/// Translates: bms.player.beatoraja.select.SkinDistributionGraph
pub struct SkinDistributionGraph {
    /// Graph type: 0 = clear lamp, 1 = score rank
    pub graph_type: i32,
    /// Current graph images
    pub current_image: Vec<Option<TextureRegion>>,
    /// Current directory bar
    pub current_bar: Option<usize>,
    /// Draw flag
    pub draw: bool,
    /// Region for drawing
    pub region: SkinRegion,
}

static LAMP: [&str; 11] = [
    "ff404040", "ff000080", "ff800080", "ffff00ff", "ff40ff40", "ff00c0f0", "ffffffff", "ff88ffff",
    "ffffff88", "ff8888ff", "ff0000ff",
];

static RANK: [&str; 28] = [
    "ff404040", "ff400040", "ff400040", "ff400040", "ff400040", "ff400040", "ff000040", "ff000040",
    "ff000040", "ff004040", "ff004040", "ff004040", "ff00c000", "ff00c000", "ff00c000", "ff80c000",
    "ff80c000", "ff80c000", "ff0080f0", "ff0080f0", "ff0080f0", "ffe0e0e0", "ffe0e0e0", "ffe0e0e0",
    "ff44ffff", "ff44ffff", "ff44ffff", "ffccffff",
];

impl SkinDistributionGraph {
    pub fn new(graph_type: i32) -> Self {
        let default_images = Self::create_default_images(graph_type);
        let count = if graph_type == 0 { 11 } else { 28 };
        Self {
            graph_type,
            current_image: default_images,
            current_bar: None,
            draw: false,
            region: SkinRegion::default(),
        }
    }

    /// Create default 1-pixel colored images for lamp/rank display.
    /// Corresponds to Java SkinDistributionGraph.createDefaultImages(int type)
    fn create_default_images(graph_type: i32) -> Vec<Option<TextureRegion>> {
        if graph_type == 0 {
            // Lamp: 11 colors
            let mut pixmap = Pixmap::new(11, 1, PixmapFormat::RGBA8888);
            for (i, hex) in LAMP.iter().enumerate() {
                let c = Color::value_of(hex);
                let rgba = Color::rgba8888(c.r, c.g, c.b, c.a);
                pixmap.draw_pixel(i as i32, 0, rgba);
            }
            let tex = Texture::from_pixmap(&pixmap);
            let mut result = Vec::with_capacity(11);
            for i in 0..LAMP.len() as i32 {
                result.push(Some(TextureRegion::from_texture_region(
                    tex.clone(),
                    i,
                    0,
                    1,
                    1,
                )));
            }
            result
        } else {
            // Rank: 28 colors
            let mut pixmap = Pixmap::new(28, 1, PixmapFormat::RGBA8888);
            for (i, hex) in RANK.iter().enumerate() {
                let c = Color::value_of(hex);
                let rgba = Color::rgba8888(c.r, c.g, c.b, c.a);
                pixmap.draw_pixel(i as i32, 0, rgba);
            }
            let tex = Texture::from_pixmap(&pixmap);
            let mut result = Vec::with_capacity(28);
            for i in 0..RANK.len() as i32 {
                result.push(Some(TextureRegion::from_texture_region(
                    tex.clone(),
                    i,
                    0,
                    1,
                    1,
                )));
            }
            result
        }
    }

    pub fn prepare(&mut self, _time: i64, _state: &dyn MainState) {
        // In Java: gets the current Bar from MusicSelector, checks folderlamp config,
        // and prepares image sources. Stubbed since it needs rendering integration.
        log::warn!(
            "not yet implemented: SkinDistributionGraph.prepare - requires rendering integration"
        );
    }

    pub fn draw_default(&self, _sprite: &SkinObjectRenderer) {
        // In Java: draws using currentBar. Calls draw(sprite, currentBar, 0, 0)
        log::warn!(
            "not yet implemented: SkinDistributionGraph.draw - requires rendering integration"
        );
    }

    pub fn draw_directory(
        &self,
        _sprite: &SkinObjectRenderer,
        current: &DirectoryBarData,
        _offset_x: f32,
        _offset_y: f32,
    ) {
        let lamps = &current.lamps;
        let ranks = &current.ranks;
        let mut count = 0;
        for &lamp in lamps.iter() {
            count += lamp;
        }

        if count != 0 {
            if self.graph_type == 0 {
                let mut _x = 0;
                for i in (0..=10).rev() {
                    // sprite.draw(currentImage[i], region.x + x * region.width / count + offsetx, ...)
                    _x += lamps[i];
                }
            } else {
                let mut _x = 0;
                for i in (0..=27).rev() {
                    _x += ranks[i];
                }
            }
        }
    }

    pub fn draw_function_bar(
        &self,
        _sprite: &SkinObjectRenderer,
        current: &FunctionBar,
        _offset_x: f32,
        _offset_y: f32,
    ) {
        let lamps = current.get_lamps();
        let mut count = 0;
        for &lamp in lamps.iter() {
            count += lamp;
        }
        if count == 0 {
            return;
        }

        let mut _x = 0;
        for i in (0..=10).rev() {
            // sprite.draw(currentImage[i], ...)
            _x += lamps[i];
        }
    }

    pub fn draw_song_bar_download(
        &self,
        _sprite: &SkinObjectRenderer,
        _current: &SongBar,
        task: &DownloadTask,
        _offset_x: f32,
        _offset_y: f32,
    ) {
        let _percent: f32 = match task.get_download_task_status() {
            DownloadTaskStatus::Prepare => 0.0,
            DownloadTaskStatus::Downloading => {
                task.get_download_size() as f32 / task.get_content_length() as f32
            }
            DownloadTaskStatus::Downloaded => 1.0,
            DownloadTaskStatus::Extracted => 1.0,
            DownloadTaskStatus::Error => 1.0,
            DownloadTaskStatus::Cancel => 1.0,
        };

        // In Java: draws background and foreground bars
        // sprite.draw(bg, ...); sprite.draw(fg, ...);
    }

    pub fn dispose(&self) {
        // In Java: disposes all lamp images
    }
}
