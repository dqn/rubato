// SkinSource hierarchy ported from Java:
// - SkinSource (abstract) → SkinSourceImage, SkinSourceReference, SkinSourceMovie
// - SkinSourceSet (abstract) → SkinSourceImageSet
//
// Provides image animation frame selection based on timer and cycle.
// Actual texture data is represented as ImageHandle (Phase 10 provides GPU textures).

use crate::image_handle::{ImageHandle, ImageRegion};

// ---------------------------------------------------------------------------
// Image index calculation
// ---------------------------------------------------------------------------

/// Calculates the animation frame index for the given time.
///
/// This matches the Java `getImageIndex()` formula exactly:
/// `(time * length / cycle) % length`
///
/// - `length`: number of animation frames
/// - `time`: elapsed time in milliseconds (already adjusted for timer offset)
/// - `cycle`: animation cycle duration in milliseconds (0 = no animation)
///
/// Returns 0 if cycle is 0, time is negative, or length is 0.
pub fn image_index(length: usize, time: i64, cycle: i32) -> usize {
    if cycle == 0 || length == 0 || time < 0 {
        return 0;
    }
    let len = length as i64;
    let cyc = cycle as i64;
    ((time * len / cyc) % len) as usize
}

// ---------------------------------------------------------------------------
// SkinSource — single image sources
// ---------------------------------------------------------------------------

/// A source that produces a single image at a given time.
#[derive(Debug, Clone)]
pub enum SkinSource {
    /// Reference to a globally loaded image by ID.
    /// The image is resolved at runtime from the skin's image table.
    Reference {
        /// Global image ID.
        id: i32,
    },
    /// Inline animation frames with timer and cycle.
    Image {
        /// Animation frame handles.
        images: Vec<ImageHandle>,
        /// Timer ID that drives the animation. None = use raw time.
        timer: Option<i32>,
        /// Animation cycle in milliseconds. 0 = static (first frame).
        cycle: i32,
    },
    /// Movie (FFmpeg) source.
    Movie {
        /// Path to the movie file.
        path: String,
        /// Timer ID for playback synchronization.
        timer: i32,
    },
}

impl SkinSource {
    /// Returns the image handle for the given time.
    ///
    /// For Reference sources, returns None (must be resolved at runtime).
    /// For Image sources, computes the animation frame index.
    /// For Movie sources, returns None (handled by video pipeline).
    ///
    /// `timer_time`: the elapsed time adjusted for the source's timer offset.
    pub fn get_image(&self, timer_time: i64) -> Option<ImageHandle> {
        match self {
            Self::Reference { .. } | Self::Movie { .. } => None,
            Self::Image { images, cycle, .. } => {
                if images.is_empty() {
                    return None;
                }
                let idx = image_index(images.len(), timer_time, *cycle);
                Some(images[idx])
            }
        }
    }

    /// Returns the number of animation frames, or 0 for non-image sources.
    pub fn frame_count(&self) -> usize {
        match self {
            Self::Image { images, .. } => images.len(),
            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// SkinSourceSet — image set sources (2D: state × frame)
// ---------------------------------------------------------------------------

/// A source that produces a set of images (e.g., digit glyphs) at a given time.
///
/// This corresponds to Java's SkinSourceImageSet which holds a 2D array
/// `TextureRegion[state][frame]` and selects the current row by animation index.
#[derive(Debug, Clone)]
pub struct SkinSourceSet {
    /// 2D image array: `images[state_index][frame_index]`.
    /// The outer dimension is the animation state (selected by timer/cycle).
    /// The inner dimension holds the frame set (e.g., digits 0-9 + period + minus).
    pub images: Vec<Vec<ImageRegion>>,
    /// Timer ID that drives the state animation. None = use raw time.
    pub timer: Option<i32>,
    /// Animation cycle in milliseconds. 0 = static (first state).
    pub cycle: i32,
}

impl SkinSourceSet {
    pub fn new(images: Vec<Vec<ImageRegion>>, timer: Option<i32>, cycle: i32) -> Self {
        Self {
            images,
            timer,
            cycle,
        }
    }

    /// Returns the current image set (row) for the given time.
    ///
    /// `timer_time`: elapsed time adjusted for the source's timer offset.
    pub fn get_images(&self, timer_time: i64) -> Option<&[ImageRegion]> {
        if self.images.is_empty() {
            return None;
        }
        let idx = image_index(self.images.len(), timer_time, self.cycle);
        Some(&self.images[idx])
    }

    /// Returns the number of animation states (outer dimension).
    pub fn state_count(&self) -> usize {
        self.images.len()
    }
}

// ---------------------------------------------------------------------------
// Grid splitting utility
// ---------------------------------------------------------------------------

/// Splits a source image into a grid of sub-regions.
///
/// Produces `divx * divy` regions in Java's `getSourceImage()` order:
/// `regions[divx * j + i]` where i is column and j is row.
///
/// Returns an empty Vec if divx or divy is <= 0, or if w/h are <= 0.
pub fn split_grid(
    handle: ImageHandle,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    divx: i32,
    divy: i32,
) -> Vec<ImageRegion> {
    if divx <= 0 || divy <= 0 || w <= 0 || h <= 0 {
        return Vec::new();
    }
    let cell_w = w as f32 / divx as f32;
    let cell_h = h as f32 / divy as f32;
    let mut regions = Vec::with_capacity((divx * divy) as usize);
    for j in 0..divy {
        for i in 0..divx {
            regions.push(ImageRegion {
                handle,
                x: x as f32 + i as f32 * cell_w,
                y: y as f32 + j as f32 * cell_h,
                w: cell_w,
                h: cell_h,
            });
        }
    }
    regions
}

// ---------------------------------------------------------------------------
// Number source set building
// ---------------------------------------------------------------------------

/// Builds a `SkinSourceSet` for a SkinNumber from grid images.
///
/// Matches Java `JsonSkinObjectLoader.java:100-170`:
/// - 24-frame divisible: 12 positive + 12 negative per state → has_minus = true
/// - Otherwise: d = (len % 10 == 0) ? 10 : 11, states × d frames
///
/// Returns `(source_set, has_minus, zeropadding_override)`.
pub fn build_number_source_set(
    images: &[ImageRegion],
    timer: Option<i32>,
    cycle: i32,
) -> (SkinSourceSet, bool, Option<i32>) {
    let len = images.len();
    if len == 0 {
        return (SkinSourceSet::new(vec![], timer, cycle), false, None);
    }

    if len.is_multiple_of(24) {
        // 24-frame: 12 positive + 12 negative per state
        let states = len / 24;
        let mut positive = Vec::with_capacity(states);
        for j in 0..states {
            let row: Vec<ImageRegion> = (0..12).map(|i| images[j * 24 + i]).collect();
            positive.push(row);
        }
        // negative images stored but only positive set is populated into SkinSourceSet
        // (matching Java: pn/mn are separate arrays, SkinNumber(pn, mn, ...) stores both)
        // For now we populate positive only; negative is deferred.
        (SkinSourceSet::new(positive, timer, cycle), true, None)
    } else {
        let d = if len.is_multiple_of(10) { 10 } else { 11 };
        let states = len / d;
        let mut rows = Vec::with_capacity(states);
        for j in 0..states {
            let row: Vec<ImageRegion> = (0..d).map(|i| images[j * d + i]).collect();
            rows.push(row);
        }
        let zeropadding_override = if d > 10 { Some(2) } else { None };
        (
            SkinSourceSet::new(rows, timer, cycle),
            false,
            zeropadding_override,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_index_static() {
        assert_eq!(image_index(3, 0, 0), 0);
        assert_eq!(image_index(3, 1000, 0), 0);
    }

    #[test]
    fn test_image_index_animation() {
        // 3 frames, cycle=300ms
        // At t=0: 0*3/300 % 3 = 0
        assert_eq!(image_index(3, 0, 300), 0);
        // At t=100: 100*3/300 % 3 = 1
        assert_eq!(image_index(3, 100, 300), 1);
        // At t=200: 200*3/300 % 3 = 2
        assert_eq!(image_index(3, 200, 300), 2);
        // At t=300: 300*3/300 % 3 = 0 (wrap)
        assert_eq!(image_index(3, 300, 300), 0);
        // At t=400: 400*3/300 % 3 = 1
        assert_eq!(image_index(3, 400, 300), 1);
    }

    #[test]
    fn test_image_index_negative_time() {
        assert_eq!(image_index(3, -50, 300), 0);
    }

    #[test]
    fn test_image_index_zero_length() {
        assert_eq!(image_index(0, 100, 300), 0);
    }

    #[test]
    fn test_skin_source_image() {
        let src = SkinSource::Image {
            images: vec![ImageHandle(10), ImageHandle(20), ImageHandle(30)],
            timer: None,
            cycle: 300,
        };
        assert_eq!(src.get_image(0), Some(ImageHandle(10)));
        assert_eq!(src.get_image(100), Some(ImageHandle(20)));
        assert_eq!(src.get_image(200), Some(ImageHandle(30)));
        assert_eq!(src.frame_count(), 3);
    }

    #[test]
    fn test_skin_source_reference() {
        let src = SkinSource::Reference { id: 42 };
        assert_eq!(src.get_image(0), None);
        assert_eq!(src.frame_count(), 0);
    }

    #[test]
    fn test_skin_source_movie() {
        let src = SkinSource::Movie {
            path: "bg.mp4".to_string(),
            timer: 0,
        };
        assert_eq!(src.get_image(0), None);
        assert_eq!(src.frame_count(), 0);
    }

    /// Helper: create an ImageRegion with full image (x=0, y=0, w=1, h=1).
    fn r(handle_id: u32) -> ImageRegion {
        ImageRegion {
            handle: ImageHandle(handle_id),
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        }
    }

    #[test]
    fn test_skin_source_set() {
        let set = SkinSourceSet::new(
            vec![vec![r(1), r(2)], vec![r(3), r(4)], vec![r(5), r(6)]],
            None,
            300,
        );
        assert_eq!(set.state_count(), 3);

        // t=0 -> state 0
        let imgs = set.get_images(0).unwrap();
        assert_eq!(imgs[0].handle, ImageHandle(1));
        assert_eq!(imgs[1].handle, ImageHandle(2));

        // t=100 -> state 1
        let imgs = set.get_images(100).unwrap();
        assert_eq!(imgs[0].handle, ImageHandle(3));
        assert_eq!(imgs[1].handle, ImageHandle(4));

        // t=200 -> state 2
        let imgs = set.get_images(200).unwrap();
        assert_eq!(imgs[0].handle, ImageHandle(5));
        assert_eq!(imgs[1].handle, ImageHandle(6));
    }

    #[test]
    fn test_skin_source_set_static() {
        let set = SkinSourceSet::new(vec![vec![r(1), r(2)]], Some(41), 0);
        // cycle=0 always returns first state
        let imgs = set.get_images(9999).unwrap();
        assert_eq!(imgs[0].handle, ImageHandle(1));
        assert_eq!(imgs[1].handle, ImageHandle(2));
    }

    #[test]
    fn test_skin_source_set_empty() {
        let set = SkinSourceSet::new(vec![], None, 100);
        assert!(set.get_images(0).is_none());
    }

    #[test]
    fn test_split_grid_2x3() {
        let h = ImageHandle(42);
        let regions = split_grid(h, 0, 0, 200, 300, 2, 3);
        assert_eq!(regions.len(), 6);
        // Row 0: (0,0), (100,0)
        assert_eq!(regions[0].x, 0.0);
        assert_eq!(regions[0].w, 100.0);
        assert_eq!(regions[0].h, 100.0);
        assert_eq!(regions[1].x, 100.0);
        assert_eq!(regions[1].y, 0.0);
        // Row 1: (0,100), (100,100)
        assert_eq!(regions[2].x, 0.0);
        assert_eq!(regions[2].y, 100.0);
        assert_eq!(regions[3].x, 100.0);
        assert_eq!(regions[3].y, 100.0);
        // Row 2: (0,200), (100,200)
        assert_eq!(regions[4].y, 200.0);
        assert_eq!(regions[5].y, 200.0);
        // All share the same handle
        for r in &regions {
            assert_eq!(r.handle, h);
        }
    }

    #[test]
    fn test_split_grid_1x10() {
        let h = ImageHandle(1);
        let regions = split_grid(h, 10, 20, 100, 50, 10, 1);
        assert_eq!(regions.len(), 10);
        for (i, r) in regions.iter().enumerate() {
            assert_eq!(r.x, 10.0 + i as f32 * 10.0);
            assert_eq!(r.y, 20.0);
            assert_eq!(r.w, 10.0);
            assert_eq!(r.h, 50.0);
        }
    }

    #[test]
    fn test_split_grid_invalid() {
        assert!(split_grid(ImageHandle(0), 0, 0, 100, 100, 0, 1).is_empty());
        assert!(split_grid(ImageHandle(0), 0, 0, 0, 100, 2, 2).is_empty());
    }

    // -- build_number_source_set tests --

    fn make_regions(count: usize) -> Vec<ImageRegion> {
        (0..count)
            .map(|i| ImageRegion {
                handle: ImageHandle(i as u32),
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            })
            .collect()
    }

    #[test]
    fn test_build_number_source_set_empty() {
        let (set, has_minus, zp) = build_number_source_set(&[], None, 0);
        assert_eq!(set.state_count(), 0);
        assert!(!has_minus);
        assert!(zp.is_none());
    }

    #[test]
    fn test_build_number_source_set_10_frames() {
        let images = make_regions(10);
        let (set, has_minus, zp) = build_number_source_set(&images, Some(1), 100);
        assert_eq!(set.state_count(), 1);
        assert_eq!(set.images[0].len(), 10);
        assert!(!has_minus);
        assert!(zp.is_none());
        assert_eq!(set.timer, Some(1));
        assert_eq!(set.cycle, 100);
    }

    #[test]
    fn test_build_number_source_set_11_frames() {
        let images = make_regions(11);
        let (set, has_minus, zp) = build_number_source_set(&images, None, 0);
        assert_eq!(set.state_count(), 1);
        assert_eq!(set.images[0].len(), 11);
        assert!(!has_minus);
        assert_eq!(zp, Some(2)); // ZeroPadding::Space
    }

    #[test]
    fn test_build_number_source_set_24_frames() {
        let images = make_regions(24);
        let (set, has_minus, zp) = build_number_source_set(&images, Some(5), 200);
        assert_eq!(set.state_count(), 1);
        assert_eq!(set.images[0].len(), 12);
        assert!(has_minus);
        assert!(zp.is_none());
        assert_eq!(set.timer, Some(5));
        assert_eq!(set.cycle, 200);
    }

    #[test]
    fn test_build_number_source_set_48_frames() {
        let images = make_regions(48);
        let (set, has_minus, _) = build_number_source_set(&images, None, 0);
        assert_eq!(set.state_count(), 2);
        assert_eq!(set.images[0].len(), 12);
        assert_eq!(set.images[1].len(), 12);
        assert!(has_minus);
        // Verify correct image mapping: state 0 = images[0..12], state 1 = images[24..36]
        assert_eq!(set.images[0][0].handle, ImageHandle(0));
        assert_eq!(set.images[0][11].handle, ImageHandle(11));
        assert_eq!(set.images[1][0].handle, ImageHandle(24));
        assert_eq!(set.images[1][11].handle, ImageHandle(35));
    }
}
