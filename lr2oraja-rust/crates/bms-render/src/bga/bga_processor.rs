// BGA processor: timeline management and image lookup.
//
// Ported from Java BGAProcessor.java (~401 lines).
// Manages BGA/layer/poor layer state, timeline advancement, and provides
// current images for rendering.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use bms_model::{BgaEvent, BgaLayer, BmsModel};

use super::bg_image_processor::{BgImageProcessor, PIC_EXTENSIONS};
use super::movie_processor::MovieProcessor;

/// Supported movie file extensions.
pub const MOV_EXTENSIONS: &[&str] = &[
    "mp4", "wmv", "m4v", "webm", "mpg", "mpeg", "m1v", "m2v", "avi",
];

/// Pre-sorted timeline entry for efficient BGA lookup during playback.
#[derive(Debug, Clone)]
struct TimelineEntry {
    time_us: i64,
    /// BGA base layer ID change (-1 = no change, -2 = clear)
    bga_id: i32,
    /// Overlay layer ID change (-1 = no change, -2 = clear)
    layer_id: i32,
    /// Poor/miss layer ID change (-1 = no change, -2 = clear)
    poor_id: i32,
}

/// BGA processor manages BGA resources and playback state.
///
/// During playback, call `update()` each frame with the current time to
/// advance the timeline, then call `get_bga_image()` / `get_layer_image()` /
/// `get_poor_image()` to retrieve the current images for rendering.
pub struct BgaProcessor {
    /// Static image cache
    bg_image_processor: BgImageProcessor,
    /// Movie processors indexed by BMP ID
    movie_processors: HashMap<i32, Box<dyn MovieProcessor>>,
    /// Cached image handles for movie frames (updated by `update_movie_frames`)
    movie_frame_handles: HashMap<i32, Handle<Image>>,
    /// Pre-sorted timeline entries
    timeline: Vec<TimelineEntry>,
    /// Current position in timeline
    pos: usize,
    /// Last update time
    last_time_us: i64,

    /// Currently playing BGA base layer BMP ID (-1 = none)
    current_bga: i32,
    /// Currently playing overlay layer BMP ID (-1 = none)
    current_layer: i32,
    /// Currently playing poor layer BMP ID (-1 = none)
    current_poor: i32,

    /// Whether poor layer is being shown
    poor_active: bool,
    /// Time when miss was triggered (microseconds)
    poor_start_us: i64,
    /// Duration to show poor layer (microseconds)
    poor_duration_us: i64,

    /// Frameskip setting (1/n frame display rate)
    frameskip: i32,
}

impl BgaProcessor {
    /// Build a BGA processor from a BMS model.
    ///
    /// Extracts BGA events from the model and builds a pre-sorted timeline
    /// for efficient playback.
    pub fn new(model: &BmsModel) -> Self {
        let timeline = Self::build_timeline(&model.bga_events);

        Self {
            bg_image_processor: BgImageProcessor::new(),
            movie_processors: HashMap::new(),
            movie_frame_handles: HashMap::new(),
            timeline,
            pos: 0,
            last_time_us: -1,
            current_bga: -1,
            current_layer: -1,
            current_poor: -1,
            poor_active: false,
            poor_start_us: 0,
            poor_duration_us: 500_000, // default 500ms
            frameskip: 1,
        }
    }

    /// Build pre-sorted timeline from BGA events.
    ///
    /// Groups events by time and collapses into single entries per timestamp,
    /// storing changes for each layer. -1 means "no change" at that time.
    fn build_timeline(events: &[BgaEvent]) -> Vec<TimelineEntry> {
        if events.is_empty() {
            return Vec::new();
        }

        // Group events by time
        let mut by_time: HashMap<i64, (i32, i32, i32)> = HashMap::new();
        for event in events {
            let entry = by_time.entry(event.time_us).or_insert((-1, -1, -1));
            match event.layer {
                BgaLayer::Bga => entry.0 = event.id,
                BgaLayer::Layer => entry.1 = event.id,
                BgaLayer::Poor => entry.2 = event.id,
            }
        }

        let mut timeline: Vec<TimelineEntry> = by_time
            .into_iter()
            .map(|(time_us, (bga, layer, poor))| TimelineEntry {
                time_us,
                bga_id: bga,
                layer_id: layer,
                poor_id: poor,
            })
            .collect();
        timeline.sort_by_key(|e| e.time_us);
        timeline
    }

    /// Preload BGA images from the model's BMP definitions.
    ///
    /// Resolves image file paths relative to `base_path` (the BMS file's
    /// parent directory), trying common image extensions if the exact file
    /// is not found.
    pub fn prepare(&mut self, model: &BmsModel, base_path: &Path, images: &mut Assets<Image>) {
        for (&bmp_id, bmp_path) in &model.bmp_defs {
            let id = bmp_id as i32;

            // Try the path as-is first
            if bmp_path.exists() {
                let ext = bmp_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                if MOV_EXTENSIONS.contains(&ext.as_str()) {
                    self.create_movie_processor(id, bmp_path);
                    continue;
                }

                if PIC_EXTENSIONS.contains(&ext.as_str()) {
                    self.bg_image_processor.load(id, bmp_path, images);
                    continue;
                }
            }

            // Try resolving with common image extensions
            let stem = bmp_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.is_empty() {
                continue;
            }

            let dir = bmp_path.parent().unwrap_or(base_path);
            let mut found = false;
            for ext in PIC_EXTENSIONS {
                let candidate = dir.join(format!("{stem}.{ext}"));
                if candidate.exists() {
                    self.bg_image_processor.load(id, &candidate, images);
                    found = true;
                    break;
                }
            }

            if !found {
                // Try movie extensions
                for ext in MOV_EXTENSIONS {
                    let candidate = dir.join(format!("{stem}.{ext}"));
                    if candidate.exists() {
                        self.create_movie_processor(id, &candidate);
                        break;
                    }
                }
            }
        }
    }

    /// Create a movie processor for the given BMP ID and file path.
    fn create_movie_processor(&mut self, id: i32, path: &Path) {
        #[cfg(feature = "movie")]
        {
            use super::ffmpeg_movie_processor::FfmpegMovieProcessor;
            match FfmpegMovieProcessor::new(path, self.frameskip) {
                Ok(mp) => {
                    self.movie_processors.insert(id, Box::new(mp));
                }
                Err(e) => {
                    tracing::warn!("Failed to create movie processor for {:?}: {e}", path);
                }
            }
        }
        #[cfg(not(feature = "movie"))]
        {
            let _ = (id, path);
        }
    }

    /// Upload decoded movie frames to Bevy image assets.
    ///
    /// Call this once per frame from the main thread to pull latest
    /// decoded frames from movie decoder threads into Bevy images.
    pub fn update_movie_frames(&mut self, images: &mut Assets<Image>) {
        for (&id, mp) in &mut self.movie_processors {
            if let Some(handle) = mp.update_frame(images) {
                self.movie_frame_handles.insert(id, handle);
            }
        }
    }

    /// Set the frameskip value for movie decoding.
    pub fn set_frameskip(&mut self, frameskip: i32) {
        self.frameskip = frameskip;
    }

    /// Reset playback state to the beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
        self.last_time_us = -1;
        self.current_bga = -1;
        self.current_layer = -1;
        self.current_poor = -1;
        self.poor_active = false;
        self.poor_start_us = 0;

        // Stop all movie processors
        for mp in self.movie_processors.values_mut() {
            mp.stop();
        }
    }

    /// Advance the BGA timeline to the given time.
    ///
    /// Updates current BGA/layer/poor IDs based on timeline events.
    pub fn update(&mut self, time_us: i64) {
        if time_us < 0 {
            self.last_time_us = -1;
            return;
        }

        for i in self.pos..self.timeline.len() {
            let entry = &self.timeline[i];
            if entry.time_us > time_us {
                break;
            }

            if entry.time_us > self.last_time_us {
                // Apply BGA base change
                if entry.bga_id == -2 {
                    self.current_bga = -1;
                } else if entry.bga_id >= 0 {
                    self.current_bga = entry.bga_id;
                }

                // Apply layer change
                if entry.layer_id == -2 {
                    self.current_layer = -1;
                } else if entry.layer_id >= 0 {
                    self.current_layer = entry.layer_id;
                }

                // Apply poor/miss layer change
                if entry.poor_id == -2 {
                    self.current_poor = -1;
                } else if entry.poor_id >= 0 {
                    self.current_poor = entry.poor_id;
                }
            } else {
                self.pos = i + 1;
            }
        }

        // Update poor layer visibility
        if self.poor_active && time_us >= self.poor_start_us + self.poor_duration_us {
            self.poor_active = false;
        }

        self.last_time_us = time_us;
    }

    /// Get the current BGA base layer image handle.
    ///
    /// Movie frames take priority over static images.
    pub fn get_bga_image(&self) -> Option<&Handle<Image>> {
        if self.current_bga < 0 {
            return None;
        }
        self.get_image_for_id(self.current_bga)
    }

    /// Get the current overlay layer image handle.
    ///
    /// Movie frames take priority over static images.
    pub fn get_layer_image(&self) -> Option<&Handle<Image>> {
        if self.current_layer < 0 {
            return None;
        }
        self.get_image_for_id(self.current_layer)
    }

    /// Get the current poor/miss layer image handle, if active.
    ///
    /// Movie frames take priority over static images.
    pub fn get_poor_image(&self) -> Option<&Handle<Image>> {
        if !self.poor_active || self.current_poor < 0 {
            return None;
        }
        self.get_image_for_id(self.current_poor)
    }

    /// Look up the image handle for a BMP ID, preferring movie frame over static image.
    fn get_image_for_id(&self, id: i32) -> Option<&Handle<Image>> {
        self.movie_frame_handles
            .get(&id)
            .or_else(|| self.bg_image_processor.get(id))
    }

    /// Trigger the poor/miss layer display.
    ///
    /// - `time_us`: time when the miss occurred (microseconds)
    pub fn set_miss_triggered(&mut self, time_us: i64) {
        self.poor_start_us = time_us;
        self.poor_active = true;
    }

    /// Set the duration for poor layer display.
    ///
    /// - `duration_us`: duration in microseconds
    pub fn set_poor_duration(&mut self, duration_us: i64) {
        self.poor_duration_us = duration_us;
    }

    /// Get current BGA base ID.
    pub fn current_bga_id(&self) -> i32 {
        self.current_bga
    }

    /// Get current layer ID.
    pub fn current_layer_id(&self) -> i32 {
        self.current_layer
    }

    /// Get current poor layer ID.
    pub fn current_poor_id(&self) -> i32 {
        self.current_poor
    }

    /// Whether the poor layer is currently active.
    pub fn is_poor_active(&self) -> bool {
        self.poor_active
    }

    /// Release all resources.
    pub fn dispose(&mut self) {
        self.bg_image_processor.dispose();
        for mp in self.movie_processors.values_mut() {
            mp.dispose();
        }
        self.movie_processors.clear();
        self.movie_frame_handles.clear();
        self.timeline.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_model_with_bga_events(events: Vec<BgaEvent>) -> BmsModel {
        BmsModel {
            bga_events: events,
            ..Default::default()
        }
    }

    #[test]
    fn empty_model_produces_empty_timeline() {
        let model = BmsModel::default();
        let proc = BgaProcessor::new(&model);
        assert!(proc.timeline.is_empty());
        assert_eq!(proc.current_bga_id(), -1);
        assert_eq!(proc.current_layer_id(), -1);
        assert_eq!(proc.current_poor_id(), -1);
    }

    #[test]
    fn single_bga_event_updates_current_id() {
        let model = make_model_with_bga_events(vec![BgaEvent {
            time_us: 1_000_000,
            layer: BgaLayer::Bga,
            id: 5,
        }]);
        let mut proc = BgaProcessor::new(&model);

        // Before the event time
        proc.update(500_000);
        assert_eq!(proc.current_bga_id(), -1);

        // At/after the event time
        proc.update(1_000_000);
        assert_eq!(proc.current_bga_id(), 5);
    }

    #[test]
    fn layer_event_updates_layer_id() {
        let model = make_model_with_bga_events(vec![BgaEvent {
            time_us: 2_000_000,
            layer: BgaLayer::Layer,
            id: 10,
        }]);
        let mut proc = BgaProcessor::new(&model);

        proc.update(2_000_000);
        assert_eq!(proc.current_layer_id(), 10);
        assert_eq!(proc.current_bga_id(), -1); // unchanged
    }

    #[test]
    fn poor_event_updates_poor_id() {
        let model = make_model_with_bga_events(vec![BgaEvent {
            time_us: 3_000_000,
            layer: BgaLayer::Poor,
            id: 20,
        }]);
        let mut proc = BgaProcessor::new(&model);

        proc.update(3_000_000);
        assert_eq!(proc.current_poor_id(), 20);
    }

    #[test]
    fn multiple_events_at_same_time() {
        let model = make_model_with_bga_events(vec![
            BgaEvent {
                time_us: 1_000_000,
                layer: BgaLayer::Bga,
                id: 1,
            },
            BgaEvent {
                time_us: 1_000_000,
                layer: BgaLayer::Layer,
                id: 2,
            },
            BgaEvent {
                time_us: 1_000_000,
                layer: BgaLayer::Poor,
                id: 3,
            },
        ]);
        let mut proc = BgaProcessor::new(&model);

        proc.update(1_000_000);
        assert_eq!(proc.current_bga_id(), 1);
        assert_eq!(proc.current_layer_id(), 2);
        assert_eq!(proc.current_poor_id(), 3);
    }

    #[test]
    fn bga_change_across_time() {
        let model = make_model_with_bga_events(vec![
            BgaEvent {
                time_us: 1_000_000,
                layer: BgaLayer::Bga,
                id: 1,
            },
            BgaEvent {
                time_us: 2_000_000,
                layer: BgaLayer::Bga,
                id: 2,
            },
            BgaEvent {
                time_us: 3_000_000,
                layer: BgaLayer::Bga,
                id: 3,
            },
        ]);
        let mut proc = BgaProcessor::new(&model);

        proc.update(1_500_000);
        assert_eq!(proc.current_bga_id(), 1);

        proc.update(2_500_000);
        assert_eq!(proc.current_bga_id(), 2);

        proc.update(3_500_000);
        assert_eq!(proc.current_bga_id(), 3);
    }

    #[test]
    fn miss_triggered_activates_poor() {
        let model = make_model_with_bga_events(vec![BgaEvent {
            time_us: 1_000_000,
            layer: BgaLayer::Poor,
            id: 10,
        }]);
        let mut proc = BgaProcessor::new(&model);
        proc.update(1_000_000);

        assert!(!proc.is_poor_active());

        proc.set_miss_triggered(1_500_000);
        assert!(proc.is_poor_active());

        // Still active within duration
        proc.update(1_800_000);
        assert!(proc.is_poor_active());

        // Past duration (default 500ms = 500000us)
        proc.update(2_100_000);
        assert!(!proc.is_poor_active());
    }

    #[test]
    fn reset_clears_state() {
        let model = make_model_with_bga_events(vec![BgaEvent {
            time_us: 1_000_000,
            layer: BgaLayer::Bga,
            id: 5,
        }]);
        let mut proc = BgaProcessor::new(&model);
        proc.update(2_000_000);
        assert_eq!(proc.current_bga_id(), 5);

        proc.reset();
        assert_eq!(proc.current_bga_id(), -1);
        assert_eq!(proc.current_layer_id(), -1);
        assert_eq!(proc.current_poor_id(), -1);
        assert!(!proc.is_poor_active());
    }

    #[test]
    fn negative_time_resets_last_time() {
        let model = make_model_with_bga_events(vec![BgaEvent {
            time_us: 1_000_000,
            layer: BgaLayer::Bga,
            id: 5,
        }]);
        let mut proc = BgaProcessor::new(&model);
        proc.update(2_000_000);

        proc.update(-1);
        // IDs are not cleared by negative time, but last_time resets
        assert_eq!(proc.current_bga_id(), 5);
    }

    #[test]
    fn set_poor_duration() {
        let model = BmsModel::default();
        let mut proc = BgaProcessor::new(&model);
        proc.set_poor_duration(1_000_000);
        proc.set_miss_triggered(0);

        proc.update(500_000);
        assert!(proc.is_poor_active());

        proc.update(1_500_000);
        assert!(!proc.is_poor_active());
    }

    #[test]
    fn get_images_returns_none_when_no_images_loaded() {
        let model = make_model_with_bga_events(vec![BgaEvent {
            time_us: 0,
            layer: BgaLayer::Bga,
            id: 1,
        }]);
        let mut proc = BgaProcessor::new(&model);
        proc.update(1_000_000);

        // ID is set but no image loaded
        assert_eq!(proc.current_bga_id(), 1);
        assert!(proc.get_bga_image().is_none());
        assert!(proc.get_layer_image().is_none());
        assert!(proc.get_poor_image().is_none());
    }

    #[test]
    fn dispose_clears_everything() {
        let model = make_model_with_bga_events(vec![BgaEvent {
            time_us: 0,
            layer: BgaLayer::Bga,
            id: 1,
        }]);
        let mut proc = BgaProcessor::new(&model);
        proc.dispose();
        assert!(proc.timeline.is_empty());
    }
}
