use bms_model::bms_model::BMSModel;
use bms_model::layer::{EventType, Layer, Sequence};
use rubato_render::color::Rectangle;
use rubato_render::texture::{Texture, TextureRegion};

use crate::bga::bg_image_processor::BGImageProcessor;
use crate::bga::movie_processor::MovieProcessor;
use crate::skin::bga::StretchType;

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
            if tl.bga() != -1 || tl.layer != -1 || !tl.eventlayer.is_empty() {
                tls.push(BgaTimeline {
                    // Java TimeLine.getTime() returns (int)(time / 1000) i.e. milliseconds
                    time_ms: tl.time() as i64,
                    bga: tl.bga(),
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

    /// Draw BGA content to the given renderer.
    /// Translated from: Java BGAProcessor.drawBGA(SkinBGA dst, SkinObjectRenderer sprite, Rectangle r)
    ///
    /// The `stretch` parameter comes from SkinBGA.stretch_type().
    /// The `color` and `blend` are from the SkinObject destination state.
    pub fn draw_bga(
        &mut self,
        renderer: &mut dyn BgaRenderer,
        r: &Rectangle,
        stretch: StretchType,
        color: (f32, f32, f32, f32),
        blend: i32,
    ) {
        renderer.set_color_rgba(color.0, color.1, color.2, color.3);
        renderer.set_blend(blend);

        if self.time < 0 {
            // Blank screen before playback starts
            let blank_region = TextureRegion::from_texture(self.blanktex.clone());
            renderer.draw(&blank_region, r.x, r.y, r.width, r.height);
            return;
        }

        if self.misslayer.is_some()
            && self.misslayertime != 0
            && self.time >= self.misslayertime
            && self.time < self.misslayertime + self.get_misslayer_duration
        {
            // Draw miss layer
            let miss_index = self.miss_layer_index();
            if miss_index != Sequence::END {
                let miss = self.bga_data(self.time, miss_index, true);
                if let Some(tex) = miss {
                    renderer.set_type(BgaRenderType::Linear);
                    self.draw_bga_fix_ratio(renderer, r, &tex, stretch);
                }
            }
        } else {
            // Draw BGA
            let bga_id = self.playingbgaid;
            let rbga = self.rbga;
            let bga_tex = self.bga_data(self.time, bga_id, rbga);
            self.rbga = true;
            if let Some(tex) = bga_tex {
                let is_movie = self.is_movie(bga_id);
                if is_movie {
                    renderer.set_type(BgaRenderType::Ffmpeg);
                } else {
                    renderer.set_type(BgaRenderType::Linear);
                }
                self.draw_bga_fix_ratio(renderer, r, &tex, stretch);
            } else {
                let blank_region = TextureRegion::from_texture(self.blanktex.clone());
                renderer.draw(&blank_region, r.x, r.y, r.width, r.height);
            }

            // Draw layer
            let layer_id = self.playinglayerid;
            let rlayer = self.rlayer;
            let layer_tex = self.bga_data(self.time, layer_id, rlayer);
            self.rlayer = true;
            if let Some(tex) = layer_tex {
                let is_movie = self.is_movie(layer_id);
                if is_movie {
                    renderer.set_type(BgaRenderType::Ffmpeg);
                } else {
                    renderer.set_type(BgaRenderType::Layer);
                }
                self.draw_bga_fix_ratio(renderer, r, &tex, stretch);
            }
        }
    }

    /// Get the BGA id from the miss layer sequence for the current time.
    /// Returns Sequence::END if no valid index.
    fn miss_layer_index(&self) -> i32 {
        if let Some(ref misslayer) = self.misslayer
            && !misslayer.sequence.is_empty()
            && !misslayer.sequence[0].is_empty()
        {
            let seq = &misslayer.sequence[0];
            let elapsed = self.time - self.misslayertime;
            let duration = self.get_misslayer_duration;
            if duration > 0 {
                let idx = ((seq.len() as i64 - 1) * elapsed / duration).min(seq.len() as i64 - 1);
                return seq[idx as usize].id;
            }
        }
        Sequence::END
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

    /// Draw BGA with aspect-ratio correction.
    /// Translated from: Java BGAProcessor.drawBGAFixRatio(SkinBGA dst, SkinObjectRenderer sprite, Rectangle r, Texture bga)
    fn draw_bga_fix_ratio(
        &mut self,
        renderer: &mut dyn BgaRenderer,
        r: &Rectangle,
        bga: &Texture,
        stretch: StretchType,
    ) {
        self.tmp_rect.set(r);
        self.image.set_texture(bga.clone());
        self.image.set_region_from(0, 0, bga.width, bga.height);

        // Apply stretch type to modify rectangle and image region
        stretch.stretch_rect(&mut self.tmp_rect, &mut self.image);

        renderer.draw(
            &self.image,
            self.tmp_rect.x,
            self.tmp_rect.y,
            self.tmp_rect.width,
            self.tmp_rect.height,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::layer::{Event, Layer, Sequence};
    use bms_model::time_line::TimeLine;

    /// Helper: create a BMSModel with BGA timelines from (time_us, bga, layer) tuples.
    /// bga/layer values are set directly (including -2 for stop).
    fn model_with_bga_timelines(entries: &[(i64, i32, i32)]) -> BMSModel {
        let mut model = BMSModel::new();
        let mut timelines = Vec::new();
        for &(time_us, bga, layer) in entries {
            let mut tl = TimeLine::new(0.0, time_us, 18);
            tl.bga = bga;
            tl.layer = layer;
            timelines.push(tl);
        }
        model.timelines = timelines;
        model
    }

    // =========================================================================
    // Mock MovieProcessor for testing
    // =========================================================================

    struct MockMovieProcessor {
        frame_tex: Texture,
        playing: bool,
        play_calls: Vec<(i64, bool)>,
        stop_count: usize,
        dispose_count: usize,
    }

    impl MockMovieProcessor {
        fn new(width: i32, height: i32) -> Self {
            MockMovieProcessor {
                frame_tex: Texture {
                    width,
                    height,
                    disposed: false,
                    ..Default::default()
                },
                playing: false,
                play_calls: Vec::new(),
                stop_count: 0,
                dispose_count: 0,
            }
        }
    }

    impl MovieProcessor for MockMovieProcessor {
        fn frame(&mut self, _time: i64) -> Option<Texture> {
            if self.playing {
                Some(self.frame_tex.clone())
            } else {
                None
            }
        }

        fn play(&mut self, time: i64, loop_play: bool) {
            self.playing = true;
            self.play_calls.push((time, loop_play));
        }

        fn stop(&mut self) {
            self.playing = false;
            self.stop_count += 1;
        }

        fn dispose(&mut self) {
            self.dispose_count += 1;
        }
    }

    // =========================================================================
    // Mock BgaRenderer for testing draw_bga()
    // =========================================================================

    #[derive(Default)]
    struct MockBgaRenderer {
        draw_calls: Vec<(f32, f32, f32, f32)>,
        render_types: Vec<BgaRenderType>,
        blend_values: Vec<i32>,
        color_values: Vec<(f32, f32, f32, f32)>,
    }

    impl BgaRenderer for MockBgaRenderer {
        fn set_color_rgba(&mut self, r: f32, g: f32, b: f32, a: f32) {
            self.color_values.push((r, g, b, a));
        }
        fn set_blend(&mut self, blend: i32) {
            self.blend_values.push(blend);
        }
        fn set_type(&mut self, render_type: BgaRenderType) {
            self.render_types.push(render_type);
        }
        fn draw(&mut self, _image: &TextureRegion, x: f32, y: f32, w: f32, h: f32) {
            self.draw_calls.push((x, y, w, h));
        }
    }

    // =========================================================================
    // Existing tests (timeline management)
    // =========================================================================

    #[test]
    fn test_empty_model() {
        let model = BMSModel::new();
        let mut proc = BGAProcessor::from_model(&model);
        assert_eq!(proc.current_bga_id(), -1);
        assert_eq!(proc.current_layer_id(), -1);

        proc.update(1_000_000); // 1 second
        assert_eq!(proc.current_bga_id(), -1);
        assert_eq!(proc.current_layer_id(), -1);
    }

    #[test]
    fn test_single_bga_event() {
        // BGA id 5 at time 1000ms (1_000_000 us)
        let model = model_with_bga_timelines(&[(1_000_000, 5, -1)]);
        let mut proc = BGAProcessor::from_model(&model);

        // Before the event
        proc.update(500_000); // 500ms
        assert_eq!(proc.current_bga_id(), -1);

        // At the event time
        proc.update(1_000_000); // 1000ms
        assert_eq!(proc.current_bga_id(), 5);

        // After the event
        proc.update(2_000_000); // 2000ms
        assert_eq!(proc.current_bga_id(), 5);
    }

    #[test]
    fn test_bga_stop_event() {
        // BGA id 3 at 1s, BGA stop (-2) at 2s
        let model = model_with_bga_timelines(&[(1_000_000, 3, -1), (2_000_000, -2, -1)]);

        let mut proc = BGAProcessor::from_model(&model);

        proc.update(1_500_000); // 1500ms — should see BGA 3
        assert_eq!(proc.current_bga_id(), 3);

        proc.update(2_500_000); // 2500ms — BGA stopped
        assert_eq!(proc.current_bga_id(), -1);
    }

    #[test]
    fn test_layer_events() {
        let model = model_with_bga_timelines(&[(500_000, -1, 10), (1_500_000, -1, 20)]);

        let mut proc = BGAProcessor::from_model(&model);

        proc.update(0);
        assert_eq!(proc.current_layer_id(), -1);

        proc.update(500_000);
        assert_eq!(proc.current_layer_id(), 10);

        proc.update(2_000_000);
        assert_eq!(proc.current_layer_id(), 20);
    }

    #[test]
    fn test_bga_and_layer_combined() {
        let model = model_with_bga_timelines(&[(1_000_000, 5, 10)]);

        let mut proc = BGAProcessor::from_model(&model);
        proc.update(1_000_000);
        assert_eq!(proc.current_bga_id(), 5);
        assert_eq!(proc.current_layer_id(), 10);
    }

    #[test]
    fn test_multiple_bga_changes() {
        let entries: Vec<(i64, i32, i32)> = (0..5)
            .map(|i| ((i + 1) * 1_000_000, i as i32, -1))
            .collect();
        let model = model_with_bga_timelines(&entries);

        let mut proc = BGAProcessor::from_model(&model);

        // Step through each second
        for i in 0..5 {
            proc.update((i + 1) * 1_000_000);
            assert_eq!(proc.current_bga_id(), i as i32);
        }
    }

    #[test]
    fn test_negative_time() {
        let model = model_with_bga_timelines(&[(1_000_000, 5, -1)]);
        let mut proc = BGAProcessor::from_model(&model);

        proc.update(-1_000_000); // negative time
        assert_eq!(proc.current_bga_id(), -1);

        // After negative time, positive time should still work
        proc.update(1_000_000);
        assert_eq!(proc.current_bga_id(), 5);
    }

    #[test]
    fn test_prepare_resets_state() {
        let model = model_with_bga_timelines(&[(1_000_000, 5, -1)]);

        let mut proc = BGAProcessor::from_model(&model);
        proc.update(1_000_000);
        assert_eq!(proc.current_bga_id(), 5);

        // Reset via prepare()
        proc.prepare(&());
        assert_eq!(proc.current_bga_id(), -1);

        // Should be able to replay
        proc.update(1_000_000);
        assert_eq!(proc.current_bga_id(), 5);
    }

    // =========================================================================
    // Phase 40e: Movie processor integration tests
    // =========================================================================

    #[test]
    fn test_set_movie_and_is_movie() {
        let mut proc = BGAProcessor::new();
        proc.set_movie_count(5);
        assert!(!proc.is_movie(0));
        assert!(!proc.is_movie(3));

        proc.set_movie(3, Box::new(MockMovieProcessor::new(320, 240)));
        assert!(!proc.is_movie(0));
        assert!(proc.is_movie(3));
        assert!(!proc.is_movie(-1));
    }

    #[test]
    fn test_bga_data_returns_none_when_not_ready() {
        let mut proc = BGAProcessor::new();
        proc.progress = 0.5; // not finished loading
        assert!(proc.bga_data(0, 0, false).is_none());
    }

    #[test]
    fn test_bga_data_returns_none_for_invalid_id() {
        let mut proc = BGAProcessor::new();
        assert!(proc.bga_data(0, -1, false).is_none());
        assert!(proc.bga_data(0, -2, false).is_none());
    }

    #[test]
    fn test_bga_data_movie_returns_frame() {
        let mut proc = BGAProcessor::new();
        proc.progress = 1.0; // mark as loaded
        proc.set_movie_count(2);
        proc.set_movie(1, Box::new(MockMovieProcessor::new(640, 480)));

        // First call (cont=false) should trigger play + frame
        let tex = proc.bga_data(1000, 1, false);
        assert!(tex.is_some());
        let tex = tex.unwrap();
        assert_eq!(tex.width, 640);
        assert_eq!(tex.height, 480);

        // Subsequent call (cont=true) should just frame without play
        let tex2 = proc.bga_data(2000, 1, true);
        assert!(tex2.is_some());
    }

    #[test]
    fn test_bga_data_movie_not_cont_triggers_play() {
        let mut proc = BGAProcessor::new();
        proc.progress = 1.0;
        proc.set_movie_count(1);
        proc.set_movie(0, Box::new(MockMovieProcessor::new(320, 240)));

        // cont=false: play() should be called, returns Some
        let result1 = proc.bga_data(5000, 0, false);
        assert!(result1.is_some());

        // cont=true: play() should NOT be called again, still returns Some (already playing)
        let result2 = proc.bga_data(6000, 0, true);
        assert!(result2.is_some());
    }

    #[test]
    fn test_stop_stops_all_movies() {
        let mut proc = BGAProcessor::new();
        proc.progress = 1.0;
        proc.set_movie_count(3);
        proc.set_movie(0, Box::new(MockMovieProcessor::new(100, 100)));
        proc.set_movie(2, Box::new(MockMovieProcessor::new(200, 200)));

        // Start playing
        proc.bga_data(0, 0, false);
        proc.bga_data(0, 2, false);

        proc.stop();
        // After stop, frame should return None (movies stopped)
        let t0 = proc.bga_data(100, 0, true);
        let t2 = proc.bga_data(100, 2, true);
        assert!(t0.is_none());
        assert!(t2.is_none());
    }

    #[test]
    fn test_prepare_stops_movies_and_resets() {
        let model = model_with_bga_timelines(&[(1_000_000, 0, -1)]);
        let mut proc = BGAProcessor::from_model(&model);
        proc.set_movie_count(1);
        proc.set_movie(0, Box::new(MockMovieProcessor::new(100, 100)));

        // Play and advance
        proc.update(1_000_000);
        assert_eq!(proc.current_bga_id(), 0);

        // Prepare should reset and stop movies
        proc.prepare(&());
        assert_eq!(proc.current_bga_id(), -1);
        assert_eq!(proc.time, 0);
    }

    // =========================================================================
    // Phase 40e: draw_bga() tests
    // =========================================================================

    #[test]
    fn test_draw_bga_negative_time_draws_blank() {
        let mut proc = BGAProcessor::new();
        proc.time = -1;

        let mut renderer = MockBgaRenderer::default();
        let rect = Rectangle::new(0.0, 0.0, 256.0, 256.0);
        proc.draw_bga(
            &mut renderer,
            &rect,
            StretchType::Stretch,
            (1.0, 1.0, 1.0, 1.0),
            0,
        );

        // Should draw blank texture
        assert_eq!(renderer.draw_calls.len(), 1);
        assert_eq!(renderer.draw_calls[0], (0.0, 0.0, 256.0, 256.0));
    }

    #[test]
    fn test_draw_bga_no_playing_draws_blank() {
        let mut proc = BGAProcessor::new();
        proc.time = 1000;
        proc.playingbgaid = -1;

        let mut renderer = MockBgaRenderer::default();
        let rect = Rectangle::new(0.0, 0.0, 256.0, 256.0);
        proc.draw_bga(
            &mut renderer,
            &rect,
            StretchType::Stretch,
            (1.0, 1.0, 1.0, 1.0),
            0,
        );

        // Should draw blank (no BGA data for id -1)
        assert_eq!(renderer.draw_calls.len(), 1);
    }

    #[test]
    fn test_draw_bga_with_movie_uses_ffmpeg_type() {
        let mut proc = BGAProcessor::new();
        proc.progress = 1.0;
        proc.set_movie_count(1);
        proc.set_movie(0, Box::new(MockMovieProcessor::new(320, 240)));
        proc.time = 1000;
        proc.playingbgaid = 0;
        proc.rbga = false;

        let mut renderer = MockBgaRenderer::default();
        let rect = Rectangle::new(0.0, 0.0, 320.0, 240.0);
        proc.draw_bga(
            &mut renderer,
            &rect,
            StretchType::Stretch,
            (1.0, 1.0, 1.0, 1.0),
            0,
        );

        // Should have set FFMPEG render type
        assert!(renderer.render_types.contains(&BgaRenderType::Ffmpeg));
        // Should have drawn the frame
        assert!(!renderer.draw_calls.is_empty());
    }

    #[test]
    fn test_draw_bga_sets_color_and_blend() {
        let mut proc = BGAProcessor::new();
        proc.time = 1000;
        proc.playingbgaid = -1;

        let mut renderer = MockBgaRenderer::default();
        let rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        proc.draw_bga(
            &mut renderer,
            &rect,
            StretchType::Stretch,
            (0.5, 0.6, 0.7, 0.8),
            2,
        );

        assert_eq!(renderer.color_values[0], (0.5, 0.6, 0.7, 0.8));
        assert_eq!(renderer.blend_values[0], 2);
    }

    #[test]
    fn test_draw_bga_with_layer_uses_layer_type() {
        let mut proc = BGAProcessor::new();
        proc.progress = 1.0;
        proc.set_movie_count(2);
        // BGA 0 = no movie (static image in cache)
        // Layer 1 = no movie (static image in cache)
        proc.time = 1000;
        proc.playingbgaid = -1; // no main BGA
        proc.playinglayerid = 0;
        proc.rlayer = false;

        // Put a test texture in cache
        if let Some(ref mut cache) = proc.cache {
            cache.put_texture(
                0,
                Texture {
                    width: 256,
                    height: 256,
                    disposed: false,
                    ..Default::default()
                },
            );
        }

        let mut renderer = MockBgaRenderer::default();
        let rect = Rectangle::new(0.0, 0.0, 256.0, 256.0);
        proc.draw_bga(
            &mut renderer,
            &rect,
            StretchType::Stretch,
            (1.0, 1.0, 1.0, 1.0),
            0,
        );

        // Should use Layer type for the layer draw
        assert!(renderer.render_types.contains(&BgaRenderType::Layer));
    }

    #[test]
    fn test_draw_bga_rbga_flag_set_after_draw() {
        let mut proc = BGAProcessor::new();
        proc.time = 1000;
        proc.playingbgaid = 0;
        proc.rbga = false;

        let mut renderer = MockBgaRenderer::default();
        let rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        proc.draw_bga(
            &mut renderer,
            &rect,
            StretchType::Stretch,
            (1.0, 1.0, 1.0, 1.0),
            0,
        );

        // rbga should be set to true after draw
        assert!(proc.rbga);
    }

    // =========================================================================
    // Phase 40e: Miss layer index calculation tests
    // =========================================================================

    #[test]
    fn test_miss_layer_index_no_misslayer() {
        let proc = BGAProcessor::new();
        assert_eq!(proc.miss_layer_index(), Sequence::END);
    }

    #[test]
    fn test_miss_layer_index_single_sequence() {
        let mut proc = BGAProcessor::new();
        proc.misslayer = Some(Layer::new(
            Event::new(EventType::Miss, 0),
            vec![vec![Sequence::new(0, 42)]],
        ));
        proc.misslayertime = 100;
        proc.get_misslayer_duration = 500;
        proc.time = 200; // elapsed = 100

        assert_eq!(proc.miss_layer_index(), 42);
    }

    #[test]
    fn test_miss_layer_index_multi_sequence() {
        let mut proc = BGAProcessor::new();
        proc.misslayer = Some(Layer::new(
            Event::new(EventType::Miss, 0),
            vec![vec![
                Sequence::new(0, 10),
                Sequence::new(0, 20),
                Sequence::new(0, 30),
            ]],
        ));
        proc.misslayertime = 0;
        proc.get_misslayer_duration = 300;

        // At time 0: index = (3-1)*0/300 = 0 -> id 10
        proc.time = 0;
        assert_eq!(proc.miss_layer_index(), 10);

        // At time 150: index = (3-1)*150/300 = 1 -> id 20
        proc.time = 150;
        assert_eq!(proc.miss_layer_index(), 20);

        // At time 299: index = (3-1)*299/300 = 1 -> id 20 (integer math)
        proc.time = 299;
        assert_eq!(proc.miss_layer_index(), 20);
    }

    #[test]
    fn test_draw_bga_miss_layer_active() {
        let mut proc = BGAProcessor::new();
        proc.progress = 1.0;
        proc.set_movie_count(1);

        // Put a test texture in cache for the miss layer BGA id (42)
        if let Some(ref mut cache) = proc.cache {
            cache.put_texture(
                42,
                Texture {
                    width: 256,
                    height: 256,
                    disposed: false,
                    ..Default::default()
                },
            );
        }

        proc.misslayer = Some(Layer::new(
            Event::new(EventType::Miss, 0),
            vec![vec![Sequence::new(0, 42)]],
        ));
        proc.misslayertime = 100;
        proc.get_misslayer_duration = 500;
        proc.time = 200; // within miss layer window

        let mut renderer = MockBgaRenderer::default();
        let rect = Rectangle::new(0.0, 0.0, 256.0, 256.0);
        proc.draw_bga(
            &mut renderer,
            &rect,
            StretchType::Stretch,
            (1.0, 1.0, 1.0, 1.0),
            0,
        );

        // Should draw using Linear type for miss layer
        assert!(renderer.render_types.contains(&BgaRenderType::Linear));
        assert!(!renderer.draw_calls.is_empty());
    }

    #[test]
    fn test_dispose_cleans_up_movies() {
        let mut proc = BGAProcessor::new();
        proc.set_movie_count(2);
        proc.set_movie(0, Box::new(MockMovieProcessor::new(100, 100)));
        proc.set_movie(1, Box::new(MockMovieProcessor::new(200, 200)));

        proc.dispose();
        assert!(proc.movies.is_empty());
    }
}
