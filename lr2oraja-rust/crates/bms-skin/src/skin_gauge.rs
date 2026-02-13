// SkinGauge — groove gauge skin object.
//
// Groove gauge display with 4-part or 6-part animation modes.
// This is constructed by the LR2 CSV loader's SRC_GROOVEGAUGE / DST_GROOVEGAUGE
// commands and by the JSON loader's gauge object.

use crate::image_handle::ImageRegion;
use crate::skin_object::SkinObjectBase;

// ---------------------------------------------------------------------------
// SkinGauge
// ---------------------------------------------------------------------------

/// Gauge part type (front/back, active/inactive colors).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugePartType {
    /// Active gauge (front, red/survival zone).
    FrontRed = 0,
    /// Active gauge (front, green/normal zone).
    FrontGreen = 1,
    /// Inactive gauge (back, red zone).
    BackRed = 2,
    /// Inactive gauge (back, green zone).
    BackGreen = 3,
    /// EX active (front, red, extended).
    ExFrontRed = 4,
    /// EX active (front, green, extended).
    ExFrontGreen = 5,
}

/// A single gauge part with animation frames.
#[derive(Debug, Clone)]
pub struct GaugePart {
    /// Part type identifier.
    pub part_type: GaugePartType,
    /// Animation frames (typically 6 frames for blinking).
    pub images: Vec<ImageRegion>,
    /// Animation timer ID.
    pub timer: Option<i32>,
    /// Animation cycle in milliseconds.
    pub cycle: i32,
}

/// A skin gauge object that displays groove gauge with animated parts.
#[derive(Debug, Clone)]
pub struct SkinGauge {
    /// Base animation/destination properties.
    pub base: SkinObjectBase,
    /// Gauge parts (4 for standard, 6 for EX mode).
    pub parts: Vec<GaugePart>,
    /// Number of gauge nodes (typically 50).
    pub nodes: i32,
    /// Animation type (0=RANDOM, 1=INCREASE, 2=DECREASE, 3=FLICKERING).
    pub animation_type: i32,
    /// Animation range (default: 3).
    pub animation_range: i32,
    /// Animation interval in milliseconds (default: 33).
    pub duration: i32,
    /// Result screen gauge fill start time in milliseconds.
    pub starttime: i32,
    /// Result screen gauge fill end time in milliseconds.
    pub endtime: i32,
}

impl Default for SkinGauge {
    fn default() -> Self {
        Self {
            base: SkinObjectBase::default(),
            parts: Vec::new(),
            nodes: 50,
            animation_type: 0,
            animation_range: 3,
            duration: 33,
            starttime: 0,
            endtime: 500,
        }
    }
}

impl SkinGauge {
    pub fn new(nodes: i32) -> Self {
        Self {
            nodes,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let gauge = SkinGauge::default();
        assert_eq!(gauge.nodes, 50);
        assert!(gauge.parts.is_empty());
        assert_eq!(gauge.animation_type, 0);
        assert_eq!(gauge.animation_range, 3);
        assert_eq!(gauge.duration, 33);
        assert_eq!(gauge.starttime, 0);
        assert_eq!(gauge.endtime, 500);
    }

    #[test]
    fn test_new() {
        let gauge = SkinGauge::new(100);
        assert_eq!(gauge.nodes, 100);
    }
}
