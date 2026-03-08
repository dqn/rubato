use rubato_types::distribution_data::DistributionData;

use super::bar::directory_bar::DirectoryBarData;
use super::bar::function_bar::FunctionBar;
use super::bar::song_bar::SongBar;
use super::stubs::*;

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
        let _count = if graph_type == 0 { 11 } else { 28 };
        Self {
            graph_type,
            current_image: default_images,
            current_bar: None,
            draw: false,
            region: SkinRegion::default(),
        }
    }

    pub fn new_with_images(graph_type: i32, images: Vec<TextureRegion>) -> Self {
        Self {
            graph_type,
            current_image: images.into_iter().map(Some).collect(),
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
            (0..LAMP.len() as i32)
                .map(|i| Some(TextureRegion::from_texture_region(tex.clone(), i, 0, 1, 1)))
                .collect()
        } else {
            // Rank: 28 colors
            let mut pixmap = Pixmap::new(28, 1, PixmapFormat::RGBA8888);
            for (i, hex) in RANK.iter().enumerate() {
                let c = Color::value_of(hex);
                let rgba = Color::rgba8888(c.r, c.g, c.b, c.a);
                pixmap.draw_pixel(i as i32, 0, rgba);
            }
            let tex = Texture::from_pixmap(&pixmap);
            (0..RANK.len() as i32)
                .map(|i| Some(TextureRegion::from_texture_region(tex.clone(), i, 0, 1, 1)))
                .collect()
        }
    }

    pub fn prepare(&mut self, _time: i64, state: &dyn MainState) {
        // Java: casts state to MusicSelector, gets selected bar, checks folderlamp config.
        self.current_bar = state.get_distribution_data();

        let is_folderlamp = state
            .get_config_ref()
            .is_none_or(|config| config.select.folderlamp);
        if !is_folderlamp {
            self.draw = false;
            return;
        }

        self.draw = true;
    }

    pub fn draw_default(&self, sprite: &mut SkinObjectRenderer) {
        if let Some(ref data) = self.current_bar {
            self.draw_distribution(sprite, &data.lamps, &data.ranks, 0.0, 0.0);
        }
    }

    pub fn draw_directory(
        &self,
        sprite: &mut SkinObjectRenderer,
        current: &DirectoryBarData,
        offset_x: f32,
        offset_y: f32,
    ) {
        self.draw_distribution(sprite, &current.lamps, &current.ranks, offset_x, offset_y);
    }

    pub fn draw_function_bar(
        &self,
        sprite: &mut SkinObjectRenderer,
        current: &FunctionBar,
        offset_x: f32,
        offset_y: f32,
    ) {
        let lamps = current.lamps();
        let empty_ranks = [0i32; 28];
        self.draw_distribution(sprite, lamps, &empty_ranks, offset_x, offset_y);
    }

    pub fn draw_song_bar_download(
        &self,
        sprite: &mut SkinObjectRenderer,
        _current: &SongBar,
        task: &DownloadTask,
        offset_x: f32,
        offset_y: f32,
    ) {
        let percent: f32 = match task.download_task_status() {
            DownloadTaskStatus::Prepare => 0.0,
            DownloadTaskStatus::Downloading => {
                task.download_size as f32 / task.content_length as f32
            }
            DownloadTaskStatus::Downloaded => 1.0,
            DownloadTaskStatus::Extracted => 1.0,
            DownloadTaskStatus::Error => 1.0,
            DownloadTaskStatus::Cancel => 1.0,
        };

        // Draw background bar (full width)
        if let Some(bg) = self.current_image.first().and_then(|i| i.as_ref()) {
            sprite.draw(
                bg,
                self.region.x + offset_x,
                self.region.y + offset_y,
                self.region.width,
                self.region.height,
            );
        }
        // Draw foreground bar (proportional to progress)
        if let Some(fg) = self.current_image.last().and_then(|i| i.as_ref()) {
            sprite.draw(
                fg,
                self.region.x + offset_x,
                self.region.y + offset_y,
                self.region.width * percent,
                self.region.height,
            );
        }
    }

    /// Shared draw logic for distribution bars (lamps or ranks)
    fn draw_distribution(
        &self,
        sprite: &mut SkinObjectRenderer,
        lamps: &[i32],
        ranks: &[i32],
        offset_x: f32,
        offset_y: f32,
    ) {
        let (data, image_count) = if self.graph_type == 0 {
            (lamps, 11usize)
        } else {
            (ranks, 28usize)
        };

        let count: i32 = data.iter().take(image_count).sum();
        if count == 0 {
            return;
        }

        let mut x = 0i32;
        for i in (0..image_count).rev() {
            if i < data.len() && data[i] > 0 {
                if let Some(image) = self.current_image.get(i).and_then(|i| i.as_ref()) {
                    sprite.draw(
                        image,
                        self.region.x + x as f32 * self.region.width / count as f32 + offset_x,
                        self.region.y + offset_y,
                        data[i] as f32 * self.region.width / count as f32,
                        self.region.height,
                    );
                }
                x += data[i];
            }
        }
    }

    pub fn dispose(&self) {
        // In Java: disposes all lamp images
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_skin::stubs::{SkinOffset, TextureRegion};
    use rubato_song::md_processor::download_task::{DownloadTask, DownloadTaskStatus};

    #[test]
    fn test_download_percent_prepare() {
        let task = DownloadTask::new(1, "http://example.com".into(), "test".into(), "abc".into());
        // Prepare status → 0% progress
        assert_eq!(task.download_task_status(), DownloadTaskStatus::Prepare);
        let percent = compute_download_percent(&task);
        assert_eq!(percent, 0.0);
    }

    #[test]
    fn test_download_percent_downloading() {
        let mut task =
            DownloadTask::new(2, "http://example.com".into(), "test".into(), "def".into());
        task.set_download_task_status(DownloadTaskStatus::Downloading);
        task.download_size = 50;
        task.content_length = 100;
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
        let config = rubato_types::config::Config {
            select: rubato_types::config::SelectConfig {
                folderlamp: false,
                ..Default::default()
            },
            ..Default::default()
        };
        state.config = Some(config);
        graph.prepare(0, &state);
        assert!(!graph.draw);
    }

    #[test]
    fn test_prepare_folderlamp_enabled() {
        let mut graph = SkinDistributionGraph::new(1);
        let mut state = MockMainState::default();
        let config = rubato_types::config::Config {
            select: rubato_types::config::SelectConfig {
                folderlamp: true,
                ..Default::default()
            },
            ..Default::default()
        };
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
        config: Option<rubato_types::config::Config>,
    }

    impl MainState for MockMainState {
        fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
            static NULL: rubato_types::timer_access::NullTimer =
                rubato_types::timer_access::NullTimer;
            &NULL
        }
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
        }
        fn get_main(&self) -> &rubato_skin::stubs::MainController {
            static MAIN: std::sync::OnceLock<rubato_skin::stubs::MainController> =
                std::sync::OnceLock::new();
            MAIN.get_or_init(|| rubato_skin::stubs::MainController { debug: false })
        }
        fn get_image(&self, _id: i32) -> Option<TextureRegion> {
            None
        }
        fn get_resource(&self) -> &rubato_skin::stubs::PlayerResource {
            static RES: std::sync::OnceLock<rubato_skin::stubs::PlayerResource> =
                std::sync::OnceLock::new();
            RES.get_or_init(|| rubato_skin::stubs::PlayerResource)
        }
        fn get_distribution_data(&self) -> Option<DistributionData> {
            self.distribution.clone()
        }
        fn get_config_ref(&self) -> Option<&rubato_types::config::Config> {
            self.config.as_ref()
        }
    }

    /// Helper that mirrors the logic in draw_song_bar_download
    fn compute_download_percent(task: &DownloadTask) -> f32 {
        match task.download_task_status() {
            DownloadTaskStatus::Prepare => 0.0,
            DownloadTaskStatus::Downloading => {
                task.download_size as f32 / task.content_length as f32
            }
            DownloadTaskStatus::Downloaded => 1.0,
            DownloadTaskStatus::Extracted => 1.0,
            DownloadTaskStatus::Error => 1.0,
            DownloadTaskStatus::Cancel => 1.0,
        }
    }
}
