use crate::bga::bg_image_processor::BGImageProcessor;

/// Movie file extensions supported for BGA
pub static MOV_EXTENSION: &[&str] = &[
    "mp4", "wmv", "m4v", "webm", "mpg", "mpeg", "m1v", "m2v", "avi",
];

/// BGA resource manager and renderer
pub struct BGAProcessor {
    progress: f32,
    /// Currently playing BGA id
    playingbgaid: i32,
    /// Currently playing layer id
    playinglayerid: i32,
    /// Miss layer display start time
    misslayertime: i64,
    get_misslayer_duration: i64,
    time: i64,
    cache: Option<BGImageProcessor>,
    pos: usize,
    rbga: bool,
    rlayer: bool,
}

impl Default for BGAProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl BGAProcessor {
    pub fn new() -> Self {
        BGAProcessor {
            progress: 0.0,
            playingbgaid: -1,
            playinglayerid: -1,
            misslayertime: 0,
            get_misslayer_duration: 0,
            time: 0,
            cache: Some(BGImageProcessor::new(256, 1)),
            pos: 0,
            rbga: false,
            rlayer: false,
        }
    }

    pub fn set_model(&mut self, _model_path: Option<&str>) {
        self.progress = 0.0;
        if let Some(ref mut cache) = self.cache {
            cache.clear();
        }
        self.reset_currently_playing_bga();

        // TODO: Phase 7+ dependency - requires BMSModel, MovieProcessor, file I/O
        // In Java, this loads all BGA resources (images and movies) from the BMS directory

        self.progress = 1.0;
    }

    pub fn abort(&mut self) {
        self.progress = 1.0;
    }

    pub fn dispose_old(&mut self) {
        if let Some(ref mut cache) = self.cache {
            cache.dispose_old();
        }
    }

    pub fn prepare(&mut self, _player: &dyn std::any::Any) {
        self.pos = 0;
        if let Some(ref mut cache) = self.cache {
            cache.prepare(&[]);
        }
        self.reset_currently_playing_bga();
        self.time = 0;
    }

    fn reset_currently_playing_bga(&mut self) {
        self.playingbgaid = -1;
        self.playinglayerid = -1;
        self.misslayertime = 0;
    }

    pub fn prepare_bga(&mut self, time: i64) {
        if time < 0 {
            self.time = -1;
            return;
        }
        // TODO: Phase 7+ dependency - requires TimeLine array for BGA event processing
        // In Java, this iterates through timelines to update playingbgaid/playinglayerid/misslayer
        self.time = time;
    }

    pub fn draw_bga(&self) {
        // TODO: Phase 7+ dependency - requires SkinBGA, SkinObjectRenderer, Texture rendering
        // In Java, this draws the current BGA frame, layer, or miss layer
    }

    pub fn set_misslayer_tme(&mut self, time: i64) {
        self.misslayertime = time;
        // TODO: Phase 7+ dependency - getMisslayerDuration from PlayerConfig
        self.get_misslayer_duration = 500;
    }

    pub fn stop(&mut self) {
        // TODO: Phase 7+ dependency - stop all MovieProcessors
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut cache) = self.cache {
            cache.dispose();
        }
    }

    pub fn get_progress(&self) -> f32 {
        self.progress
    }

    /// Get BGA texture data for the given id at the specified time.
    /// Corresponds to Java getBGAData(long time, int id, boolean cont).
    fn get_bga_data(&self, _time: i64, id: i32, _cont: bool) -> Option<()> {
        if self.progress != 1.0 || id == -1 {
            return None;
        }
        // TODO: Phase 7+ dependency - requires MovieProcessor array and BGImageProcessor cache
        // In Java:
        // - If movies[id] != null: play if !cont, return getFrame(time)
        // - Otherwise: return cache.getTexture(id)
        None
    }

    /// Draw BGA with fixed aspect ratio.
    /// Corresponds to Java drawBGAFixRatio(SkinBGA dst, SkinObjectRenderer sprite, Rectangle r, Texture bga).
    fn draw_bga_fix_ratio(&self) {
        // TODO: Phase 7+ dependency - requires SkinBGA, SkinObjectRenderer, Texture, Rectangle
        // In Java, this calculates the aspect-ratio-preserving draw rectangle and renders
        // the BGA texture with proper stretch mode handling.
    }
}
