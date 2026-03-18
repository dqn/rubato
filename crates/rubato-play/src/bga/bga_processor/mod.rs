mod render;

#[cfg(test)]
mod tests;

use bms_model::bms_model::BMSModel;
use bms_model::layer::{EventType, Layer};
use rubato_render::color::Rectangle;
use rubato_render::texture::{Texture, TextureRegion};

use crate::bga::bg_image_processor::BGImageProcessor;
use crate::bga::movie_processor::MovieProcessor;

/// Movie file extensions supported for BGA
pub static MOV_EXTENSION: &[&str] = &[
    "mp4", "wmv", "m4v", "webm", "mpg", "mpeg", "m1v", "m2v", "avi",
];

/// Renderer type hint for BGA drawing.
/// Corresponds to SkinObjectRenderer.TYPE_* constants in Java.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BgaRenderType {
    /// Linear filtering (for static images and miss layer)
    Linear,
    /// FFmpeg shader (for movie frames)
    Ffmpeg,
    /// Layer blending (for static image layers)
    Layer,
}

/// Trait for BGA sprite rendering, abstracting SkinObjectRenderer.
/// Implemented by the skin rendering system to draw BGA textures.
pub trait BgaRenderer {
    /// Set the drawing color.
    fn set_color_rgba(&mut self, r: f32, g: f32, b: f32, a: f32);
    /// Set the blend mode.
    fn set_blend(&mut self, blend: i32);
    /// Set the renderer type (shader selection).
    fn set_type(&mut self, render_type: BgaRenderType);
    /// Draw a texture region at the given position and size.
    fn draw(&mut self, image: &TextureRegion, x: f32, y: f32, w: f32, h: f32);
}

/// Lightweight BGA timeline entry extracted from BMSModel timelines.
/// Stores only the BGA-relevant fields (time, bga id, layer id, event layers).
struct BgaTimeline {
    /// Timeline time in milliseconds (matching Java TimeLine.getTime())
    time_ms: i64,
    /// BGA id (-1 = no change, -2 = stop)
    bga: i32,
    /// Layer id (-1 = no change, -2 = stop)
    layer: i32,
    /// Event layers (POOR layer etc.)
    eventlayer: Vec<Layer>,
}

/// BGA resource manager and renderer.
/// Translated from: BGAProcessor.java
pub struct BGAProcessor {
    progress: f32,
    /// Currently playing BGA id
    playingbgaid: i32,
    /// Currently playing layer id
    playinglayerid: i32,
    /// Miss layer display start time
    misslayertime: i64,
    pub get_misslayer_duration: i64,
    /// Current miss layer sequence
    misslayer: Option<Layer>,
    /// Current time in milliseconds (matching Java BGAProcessor.time)
    time: i64,
    cache: Option<BGImageProcessor>,
    /// Movie processors indexed by BGA id (None = not a movie)
    movies: Vec<Option<Box<dyn MovieProcessor>>>,
    /// 1x1 black texture for empty BGA display
    blanktex: Texture,
    /// Scratch TextureRegion for drawing
    image: TextureRegion,
    /// Scratch Rectangle for drawing
    tmp_rect: Rectangle,
    /// Filtered timelines containing BGA/layer/eventlayer data
    timelines: Vec<BgaTimeline>,
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
    pub fn new_with_resource_gen(song_resource_gen: i32) -> Self {
        let maxgen = song_resource_gen.max(1);
        BGAProcessor {
            progress: 0.0,
            playingbgaid: -1,
            playinglayerid: -1,
            misslayertime: 0,
            get_misslayer_duration: 500,
            misslayer: None,
            time: 0,
            cache: Some(BGImageProcessor::new(256, maxgen)),
            movies: Vec::new(),
            blanktex: Texture {
                width: 1,
                height: 1,
                disposed: false,
                ..Default::default()
            },
            image: TextureRegion::new(),
            tmp_rect: Rectangle::default(),
            timelines: Vec::new(),
            pos: 0,
            rbga: false,
            rlayer: false,
        }
    }

    pub fn new() -> Self {
        Self::new_with_resource_gen(1)
    }

    /// Create a BGAProcessor and load timeline data from the given model.
    /// Convenience constructor for testing.
    pub fn from_model(model: &BMSModel) -> Self {
        let mut proc = Self::new();
        proc.set_model_timelines(model);
        proc
    }

    /// Extract and store BGA-relevant timelines from the model.
    /// Corresponds to the timeline-filtering part of Java BGAProcessor.setModel().
    pub fn set_model_timelines(&mut self, model: &BMSModel) {
        self.progress = 0.0;
        if let Some(ref mut cache) = self.cache {
            cache.clear();
        }
        self.reset_currently_playing_bga();

        let mut tls = Vec::new();
        for tl in &model.timelines {
            if tl.bga != -1 || tl.layer != -1 || !tl.eventlayer.is_empty() {
                tls.push(BgaTimeline {
                    // Java TimeLine.getTime() returns (int)(time / 1000) i.e. milliseconds
                    time_ms: tl.time(),
                    bga: tl.bga,
                    layer: tl.layer,
                    eventlayer: tl.eventlayer.to_vec(),
                });
            }
        }
        self.timelines = tls;

        self.progress = 1.0;
    }

    pub fn set_model(&mut self, _model_path: Option<&str>) {
        self.progress = 0.0;
        if let Some(ref mut cache) = self.cache {
            cache.clear();
        }
        self.reset_currently_playing_bga();

        // Resource loading (images/movies) is dispatched by the caller (PlayerResource).
        // Timeline loading is done separately via set_model_timelines().
        // Static images: caller calls cache.put(id, path) via put_image().
        // Movies: caller calls set_movie_count() + set_movie().

        self.progress = 1.0;
    }

    /// Load a static BGA image into the cache at the given id.
    /// Called by PlayerResource for each image BGA definition.
    pub fn put_image(&mut self, id: usize, path: &std::path::Path) {
        if let Some(ref mut cache) = self.cache {
            cache.put(id, path);
        }
    }

    /// Set the number of BGA slots (movie + image).
    /// Must be called before set_movie().
    pub fn set_movie_count(&mut self, count: usize) {
        self.movies = Vec::with_capacity(count);
        self.movies.resize_with(count, || None);
    }

    /// Set a movie processor for the given BGA id.
    pub fn set_movie(&mut self, id: usize, movie: Box<dyn MovieProcessor>) {
        if id >= self.movies.len() {
            self.movies.resize_with(id + 1, || None);
        }
        self.movies[id] = Some(movie);
    }

    /// Check if the given BGA id is a movie (has a MovieProcessor).
    pub fn is_movie(&self, id: i32) -> bool {
        if id < 0 {
            return false;
        }
        let idx = id as usize;
        idx < self.movies.len() && self.movies[idx].is_some()
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
        // Java: cache.prepare(timelines) — skipped for now (resource pre-caching)
        for mp in self.movies.iter_mut().flatten() {
            mp.stop();
        }
        self.reset_currently_playing_bga();
        self.time = 0;
    }

    fn reset_currently_playing_bga(&mut self) {
        self.playingbgaid = -1;
        self.playinglayerid = -1;
        self.misslayertime = 0;
        self.misslayer = None;
    }

    /// Update BGA state to the given time (microseconds).
    /// Public API using project-standard microsecond timing.
    pub fn update(&mut self, time_us: i64) {
        // Convert to milliseconds for internal comparison (matching Java)
        self.prepare_bga(time_us / 1000);
    }

    /// Scan timelines and update playingbgaid/playinglayerid/misslayer.
    /// Corresponds to Java BGAProcessor.prepareBGA(long time) where time is in ms.
    pub fn prepare_bga(&mut self, time: i64) {
        if time < 0 {
            self.time = -1;
            return;
        }
        // Reset scan position when seeking backward (e.g. practice mode scrubbing)
        // so that timelines before the old position are not skipped.
        if time < self.time {
            self.pos = 0;
            self.time = -1;
            self.playingbgaid = -1;
            self.playinglayerid = -1;
            self.rbga = false;
            self.rlayer = false;
        }
        for i in self.pos..self.timelines.len() {
            let tl = &self.timelines[i];
            if tl.time_ms > time {
                break;
            }

            if tl.time_ms > self.time {
                let bga = tl.bga;
                if bga == -2 {
                    self.playingbgaid = -1;
                    self.rbga = false;
                } else if bga >= 0 {
                    self.playingbgaid = bga;
                    self.rbga = false;
                }

                let layer = tl.layer;
                if layer == -2 {
                    self.playinglayerid = -1;
                    self.rlayer = false;
                } else if layer >= 0 {
                    self.playinglayerid = layer;
                    self.rlayer = false;
                }

                let eventlayer = &tl.eventlayer;
                for poor in eventlayer {
                    if poor.event.event_type == EventType::Miss {
                        self.misslayer = Some(poor.clone());
                    }
                }
            } else {
                self.pos += 1;
            }
        }

        self.time = time;
    }

    pub fn set_misslayer_tme(&mut self, time: i64) {
        self.misslayertime = time;
        // Duration is set via set_misslayer_duration() during init from PlayerConfig.
    }

    /// Stop all BGA playback.
    pub fn stop(&mut self) {
        for mp in self.movies.iter_mut().flatten() {
            mp.stop();
        }
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut cache) = self.cache {
            cache.dispose();
        }
        for mp in self.movies.iter_mut().flatten() {
            mp.dispose();
        }
        self.movies.clear();
    }

    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// Get currently playing BGA id.
    pub fn current_bga_id(&self) -> i32 {
        self.playingbgaid
    }

    /// Get currently playing layer id.
    pub fn current_layer_id(&self) -> i32 {
        self.playinglayerid
    }

    /// Get BGA texture data for the given id at the specified time.
    /// Translated from: Java BGAProcessor.getBGAData(long time, int id, boolean cont)
    ///
    /// `cont` = true means playback is continuing (don't restart), false = new BGA.
    fn bga_data(&mut self, time: i64, id: i32, cont: bool) -> Option<Texture> {
        if self.progress != 1.0 || id < 0 {
            return None;
        }
        let idx = id as usize;
        if idx < self.movies.len()
            && let Some(ref mut mp) = self.movies[idx]
        {
            if !cont {
                mp.play(time, false);
            }
            return mp.frame(time);
        }
        // Fall back to static image cache
        if let Some(ref mut cache) = self.cache
            && let Some(tex) = cache.texture(idx)
        {
            return Some(tex.clone());
        }
        None
    }
}
