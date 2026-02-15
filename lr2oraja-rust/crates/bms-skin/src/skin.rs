// Skin container ported from Skin.java.
//
// Holds all skin objects, resolution scaling factors, option/offset state,
// and custom event/timer definitions.

use std::collections::HashMap;
use std::path::PathBuf;

use bms_config::resolution::Resolution;
use bms_config::skin_config::Offset;

use crate::custom_event::{CustomEventDef, CustomTimerDef};
use crate::image_handle::ImageHandle;
use crate::music_select_skin::MusicSelectSkinConfig;
use crate::play_skin::PlaySkinConfig;
use crate::result_skin::{CourseResultSkinConfig, ResultSkinConfig};
use crate::skin_header::SkinHeader;
use crate::skin_object::Rect;
use crate::skin_object_type::SkinObjectType;

// ---------------------------------------------------------------------------
// Skin
// ---------------------------------------------------------------------------

/// The main skin container.
///
/// Stores all skin objects and manages resolution scaling, option state,
/// and custom event/timer definitions.
#[derive(Debug, Clone)]
pub struct Skin {
    /// Skin header metadata.
    pub header: SkinHeader,
    /// Display width (destination resolution).
    pub width: f32,
    /// Display height (destination resolution).
    pub height: f32,
    /// Scale factor: destination width / source width.
    pub scale_x: f32,
    /// Scale factor: destination height / source height.
    pub scale_y: f32,
    /// All skin objects in draw order.
    pub objects: Vec<SkinObjectType>,
    /// Input start time in milliseconds.
    pub input: i32,
    /// Scene duration in milliseconds.
    pub scene: i32,
    /// Fade-out duration in milliseconds.
    pub fadeout: i32,
    /// Rank display time in milliseconds (Result/CourseResult only).
    pub rank_time: i32,
    /// Active option values: option_id -> value (0 or 1).
    pub options: HashMap<i32, i32>,
    /// Active offset values: offset_id -> Offset.
    pub offsets: HashMap<i32, Offset>,
    /// Custom event definitions.
    pub custom_events: Vec<CustomEventDef>,
    /// Custom timer definitions.
    pub custom_timers: Vec<CustomTimerDef>,
    /// Play state-specific configuration.
    pub play_config: Option<PlaySkinConfig>,
    /// Music select state-specific configuration.
    pub select_config: Option<MusicSelectSkinConfig>,
    /// Result state-specific configuration.
    pub result_config: Option<ResultSkinConfig>,
    /// Course result state-specific configuration.
    pub course_result_config: Option<CourseResultSkinConfig>,
    /// Extra image paths for PomyuChara (handle -> (path, needs_color_key)).
    pub extra_image_paths: HashMap<ImageHandle, (PathBuf, bool)>,
    /// PomyuChara motion cycle times: [1P_NEUTRAL, 1P_FEVER, 1P_GREAT, 1P_GOOD, 1P_BAD, 2P_NEUTRAL, 2P_GREAT, 2P_BAD]
    pub pomyu_chara_times: [i32; 8],
}

impl Skin {
    /// Creates a new Skin from a header, computing scale factors from
    /// source and destination resolutions.
    pub fn new(header: SkinHeader) -> Self {
        let src = header.source_resolution.unwrap_or(header.resolution);
        let dst = header.destination_resolution.unwrap_or(Resolution::Hd);
        let width = dst.width() as f32;
        let height = dst.height() as f32;
        let scale_x = width / src.width() as f32;
        let scale_y = height / src.height() as f32;

        Self {
            header,
            width,
            height,
            scale_x,
            scale_y,
            objects: Vec::new(),
            input: 0,
            scene: 3_600_000 * 24, // Java default: 24 hours
            fadeout: 0,
            rank_time: 0,
            options: HashMap::new(),
            offsets: HashMap::new(),
            custom_events: Vec::new(),
            custom_timers: Vec::new(),
            play_config: None,
            select_config: None,
            result_config: None,
            course_result_config: None,
            extra_image_paths: HashMap::new(),
            pomyu_chara_times: [1; 8],
        }
    }

    /// Adds a skin object to the draw list.
    pub fn add(&mut self, object: SkinObjectType) {
        self.objects.push(object);
    }

    /// Scales a rectangle from skin source coordinates to destination coordinates.
    pub fn scale_rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect {
            x: x * self.scale_x,
            y: y * self.scale_y,
            w: w * self.scale_x,
            h: h * self.scale_y,
        }
    }

    /// Returns the number of objects.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_header(src: Resolution, dst: Resolution) -> SkinHeader {
        SkinHeader {
            source_resolution: Some(src),
            destination_resolution: Some(dst),
            ..Default::default()
        }
    }

    #[test]
    fn test_scale_factors() {
        // SD (640x480) -> HD (1280x720)
        let skin = Skin::new(make_header(Resolution::Sd, Resolution::Hd));
        assert!((skin.scale_x - 2.0).abs() < 0.001);
        assert!((skin.scale_y - 1.5).abs() < 0.001);
        assert_eq!(skin.width, 1280.0);
        assert_eq!(skin.height, 720.0);
    }

    #[test]
    fn test_scale_rect() {
        let skin = Skin::new(make_header(Resolution::Sd, Resolution::Hd));
        let r = skin.scale_rect(100.0, 200.0, 50.0, 30.0);
        assert!((r.x - 200.0).abs() < 0.001);
        assert!((r.y - 300.0).abs() < 0.001);
        assert!((r.w - 100.0).abs() < 0.001);
        assert!((r.h - 45.0).abs() < 0.001);
    }

    #[test]
    fn test_default_scene() {
        let skin = Skin::new(SkinHeader::default());
        assert_eq!(skin.scene, 3_600_000 * 24);
        assert_eq!(skin.fadeout, 0);
        assert_eq!(skin.input, 0);
    }

    #[test]
    fn test_add_objects() {
        use crate::skin_image::SkinImage;
        let mut skin = Skin::new(SkinHeader::default());
        assert_eq!(skin.object_count(), 0);
        skin.add(SkinImage::default().into());
        skin.add(SkinImage::from_reference(1).into());
        assert_eq!(skin.object_count(), 2);
    }

    #[test]
    fn test_same_resolution() {
        let skin = Skin::new(make_header(Resolution::Hd, Resolution::Hd));
        assert!((skin.scale_x - 1.0).abs() < 0.001);
        assert!((skin.scale_y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_default_header_resolution() {
        // No source/destination set: uses SD as source, HD as destination
        let skin = Skin::new(SkinHeader::default());
        assert!((skin.scale_x - 2.0).abs() < 0.001);
    }
}
