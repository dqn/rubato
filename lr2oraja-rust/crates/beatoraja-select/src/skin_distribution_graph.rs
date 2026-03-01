use beatoraja_types::distribution_data::DistributionData;

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
    /// Distribution data from the currently selected directory bar
    pub current_bar: Option<DistributionData>,
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

    pub fn prepare(&mut self, _time: i64, state: &dyn MainState) {
        // Java: casts state to MusicSelector, gets selected bar, checks folderlamp config.
        self.current_bar = state.get_distribution_data();

        let is_folderlamp = state
            .get_config_ref()
            .is_none_or(|config| config.folderlamp);
        if !is_folderlamp {
            self.draw = false;
            return;
        }

        self.draw = true;
    }

    pub fn draw_default(&self, _sprite: &SkinObjectRenderer) {
        // In Java: draws using currentBar. Calls draw(sprite, currentBar, 0, 0)
        // Blocked: requires wgpu rendering pipeline integration.
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

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_skin::stubs::{SkinOffset, TextureRegion};
    use md_processor::download_task::{DownloadTask, DownloadTaskStatus};

    #[test]
    fn test_download_percent_prepare() {
        let task = DownloadTask::new(1, "http://example.com".into(), "test".into(), "abc".into());
        // Prepare status → 0% progress
        assert_eq!(task.get_download_task_status(), DownloadTaskStatus::Prepare);
        let percent = compute_download_percent(&task);
        assert_eq!(percent, 0.0);
    }

    #[test]
    fn test_download_percent_downloading() {
        let mut task =
            DownloadTask::new(2, "http://example.com".into(), "test".into(), "def".into());
        task.set_download_task_status(DownloadTaskStatus::Downloading);
        task.set_download_size(50);
        task.set_content_length(100);
        let percent = compute_download_percent(&task);
        assert!((percent - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_download_percent_completed_statuses() {
        let mut task =
            DownloadTask::new(3, "http://example.com".into(), "test".into(), "ghi".into());
        for status in [
            DownloadTaskStatus::Downloaded,
            DownloadTaskStatus::Extracted,
            DownloadTaskStatus::Error,
            DownloadTaskStatus::Cancel,
        ] {
            task.set_download_task_status(status);
            let percent = compute_download_percent(&task);
            assert_eq!(percent, 1.0, "Expected 1.0 for status {:?}", status);
        }
    }

    #[test]
    fn test_prepare_no_distribution_data() {
        let mut graph = SkinDistributionGraph::new(0);
        let state = MockMainState::default();
        graph.prepare(0, &state);
        assert!(graph.current_bar.is_none());
        assert!(graph.draw); // folderlamp defaults to true when config is None
    }

    #[test]
    fn test_prepare_with_distribution_data() {
        let mut graph = SkinDistributionGraph::new(0);
        let mut state = MockMainState::default();
        let mut dist = DistributionData::default();
        dist.lamps[0] = 5;
        dist.lamps[6] = 3;
        state.distribution = Some(dist.clone());
        graph.prepare(0, &state);
        assert!(graph.current_bar.is_some());
        assert_eq!(graph.current_bar.as_ref().unwrap().lamps[0], 5);
        assert_eq!(graph.current_bar.as_ref().unwrap().lamps[6], 3);
        assert!(graph.draw);
    }

    #[test]
    fn test_prepare_folderlamp_disabled() {
        let mut graph = SkinDistributionGraph::new(0);
        let mut state = MockMainState::default();
        let mut config = beatoraja_types::config::Config::default();
        config.folderlamp = false;
        state.config = Some(config);
        graph.prepare(0, &state);
        assert!(!graph.draw);
    }

    #[test]
    fn test_prepare_folderlamp_enabled() {
        let mut graph = SkinDistributionGraph::new(1);
        let mut state = MockMainState::default();
        let mut config = beatoraja_types::config::Config::default();
        config.folderlamp = true;
        state.config = Some(config);
        let mut dist = DistributionData::default();
        dist.ranks[0] = 2;
        state.distribution = Some(dist);
        graph.prepare(0, &state);
        assert!(graph.draw);
        assert!(graph.current_bar.is_some());
    }

    #[derive(Default)]
    struct MockMainState {
        distribution: Option<DistributionData>,
        config: Option<beatoraja_types::config::Config>,
    }

    impl MainState for MockMainState {
        fn get_timer(&self) -> &dyn beatoraja_types::timer_access::TimerAccess {
            static NULL: beatoraja_types::timer_access::NullTimer =
                beatoraja_types::timer_access::NullTimer;
            &NULL
        }
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
        }
        fn get_main(&self) -> &beatoraja_skin::stubs::MainController {
            static MAIN: std::sync::OnceLock<beatoraja_skin::stubs::MainController> =
                std::sync::OnceLock::new();
            MAIN.get_or_init(|| beatoraja_skin::stubs::MainController { debug: false })
        }
        fn get_image(&self, _id: i32) -> Option<TextureRegion> {
            None
        }
        fn get_resource(&self) -> &beatoraja_skin::stubs::PlayerResource {
            static RES: std::sync::OnceLock<beatoraja_skin::stubs::PlayerResource> =
                std::sync::OnceLock::new();
            RES.get_or_init(|| beatoraja_skin::stubs::PlayerResource)
        }
        fn get_distribution_data(&self) -> Option<DistributionData> {
            self.distribution.clone()
        }
        fn get_config_ref(&self) -> Option<&beatoraja_types::config::Config> {
            self.config.as_ref()
        }
    }

    /// Helper that mirrors the logic in draw_song_bar_download
    fn compute_download_percent(task: &DownloadTask) -> f32 {
        match task.get_download_task_status() {
            DownloadTaskStatus::Prepare => 0.0,
            DownloadTaskStatus::Downloading => {
                task.get_download_size() as f32 / task.get_content_length() as f32
            }
            DownloadTaskStatus::Downloaded => 1.0,
            DownloadTaskStatus::Extracted => 1.0,
            DownloadTaskStatus::Error => 1.0,
            DownloadTaskStatus::Cancel => 1.0,
        }
    }
}
