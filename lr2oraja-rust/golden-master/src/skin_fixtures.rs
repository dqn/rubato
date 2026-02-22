// Skin snapshot infrastructure for golden-master testing.
//
// Converts a Skin into a lightweight summary (SkinSnapshot) for
// structural comparison without requiring Serialize on all skin types.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use beatoraja_skin::skin::{Skin, SkinObject};

// ---------------------------------------------------------------------------
// Snapshot types
// ---------------------------------------------------------------------------

/// Lightweight snapshot of a Skin for structural comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkinSnapshot {
    // header
    pub name: String,
    pub resolution_w: f32,
    pub resolution_h: f32,
    pub option_count: usize,
    pub file_count: usize,
    pub offset_count: usize,
    // skin
    pub width: f32,
    pub height: f32,
    pub scale_x: f64,
    pub scale_y: f64,
    pub input: i32,
    pub scene: i32,
    pub fadeout: i32,
    pub object_count: usize,
    /// Object counts by type name (e.g. "Image" -> 147).
    pub objects_by_type: BTreeMap<String, usize>,
    pub custom_event_count: usize,
    pub custom_timer_count: usize,
    /// Per-object summaries.
    pub objects: Vec<ObjectSnapshot>,
}

/// Summary of a single SkinObject.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectSnapshot {
    pub kind: String,
    pub destination_count: usize,
    pub blend: i32,
    pub first_dst: Option<DstSnapshot>,
}

/// Summary of a Destination keyframe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DstSnapshot {
    pub time: i64,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub a: f32,
    pub angle: i32,
}

// ---------------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------------

fn object_snapshot(obj: &SkinObject) -> ObjectSnapshot {
    let data = obj.data();
    let first_dst = data.dst.first().map(|d| DstSnapshot {
        time: d.time,
        x: d.region.x,
        y: d.region.y,
        w: d.region.width,
        h: d.region.height,
        a: d.color.a,
        angle: d.angle,
    });
    ObjectSnapshot {
        kind: obj.get_type_name().to_string(),
        destination_count: data.dst.len(),
        blend: data.get_blend(),
        first_dst,
    }
}

/// Converts a Skin into a SkinSnapshot for comparison.
pub fn snapshot_from_skin(skin: &Skin) -> SkinSnapshot {
    let objects = skin.get_objects();
    let mut objects_by_type = BTreeMap::new();
    for obj in objects {
        *objects_by_type
            .entry(obj.get_type_name().to_string())
            .or_insert(0) += 1;
    }

    let object_snapshots: Vec<ObjectSnapshot> = objects.iter().map(object_snapshot).collect();

    SkinSnapshot {
        name: skin.header.get_name().unwrap_or_default().to_string(),
        resolution_w: skin.get_width(),
        resolution_h: skin.get_height(),
        option_count: skin.header.get_custom_options().len(),
        file_count: skin.header.get_custom_files().len(),
        offset_count: skin.header.get_custom_offsets().len(),
        width: skin.get_width(),
        height: skin.get_height(),
        scale_x: skin.get_scale_x(),
        scale_y: skin.get_scale_y(),
        input: skin.get_input(),
        scene: skin.get_scene(),
        fadeout: skin.get_fadeout(),
        object_count: objects.len(),
        objects_by_type,
        custom_event_count: skin.get_custom_events_count(),
        custom_timer_count: skin.get_custom_timers_count(),
        objects: object_snapshots,
    }
}

// ---------------------------------------------------------------------------
// Snapshot comparison helpers
// ---------------------------------------------------------------------------

/// Loads a snapshot fixture from a JSON file.
pub fn load_snapshot(path: &Path) -> anyhow::Result<SkinSnapshot> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Saves a snapshot fixture to a JSON file.
pub fn save_snapshot(snapshot: &SkinSnapshot, path: &Path) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(snapshot)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Returns true if the UPDATE_SNAPSHOTS env var is set.
pub fn should_update_snapshots() -> bool {
    std::env::var("UPDATE_SNAPSHOTS").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_skin::skin_header::SkinHeader;
    use beatoraja_skin::skin_image::SkinImage;

    #[test]
    fn test_empty_skin_snapshot() {
        let skin = Skin::new(SkinHeader::default());
        let snap = snapshot_from_skin(&skin);
        assert_eq!(snap.object_count, 0);
        assert!(snap.objects_by_type.is_empty());
        assert!(snap.objects.is_empty());
    }

    #[test]
    fn test_snapshot_with_image() {
        let mut skin = Skin::new(SkinHeader::default());
        skin.add(SkinObject::Image(SkinImage::new_with_image_id(0)));
        skin.add(SkinObject::Image(SkinImage::new_with_image_id(1)));

        let snap = snapshot_from_skin(&skin);
        assert_eq!(snap.object_count, 2);
        assert_eq!(snap.objects_by_type.get("Image"), Some(&2));
        assert_eq!(snap.objects.len(), 2);
        assert_eq!(snap.objects[0].kind, "Image");
        assert_eq!(snap.objects[1].kind, "Image");
    }

    #[test]
    fn test_snapshot_serde_round_trip() {
        let skin = Skin::new(SkinHeader::default());
        let snap = snapshot_from_skin(&skin);
        let json = serde_json::to_string(&snap).unwrap();
        let back: SkinSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap, back);
    }
}
