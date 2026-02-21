// Mechanical translation of JsonPlaySkinObjectLoader.java
// Play skin object loader (handles note, judge, hidden cover, BGA, etc.)

use std::path::Path;

use crate::json::json_skin;
use crate::json::json_skin_loader::{JSONSkinLoader, SkinData, SkinObjectData};
use crate::json::json_skin_object_loader::{self, JsonSkinObjectLoader};

/// Corresponds to JsonPlaySkinObjectLoader extends JsonSkinObjectLoader<PlaySkin>
pub struct JsonPlaySkinObjectLoader;

impl JsonSkinObjectLoader for JsonPlaySkinObjectLoader {
    fn get_skin(&self, _header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData {
        // PlaySkin creation - stubbed pending Phase 6+ rendering
        SkinData::new()
    }

    fn load_skin_object(
        &self,
        loader: &mut JSONSkinLoader,
        skin: &SkinData,
        sk: &json_skin::Skin,
        dst: &json_skin::Destination,
        p: &Path,
    ) -> Option<SkinObjectData> {
        // Try base loader first
        let obj = json_skin_object_loader::load_base_skin_object(loader, skin, sk, dst, p);
        if obj.is_some() {
            return obj;
        }

        let dst_id = dst.id.as_deref()?;

        // note (playskin only)
        if let Some(ref note) = sk.note
            && dst_id == note.id.as_deref().unwrap_or("")
        {
            let obj = SkinObjectData {
                name: note.id.clone(),
                ..Default::default()
            };
            // SkinNote creation depends on texture loading - stubbed
            // In Java: getNoteTexture, SkinNote, setLaneRegion, etc.
            return Some(obj);
        }

        // hidden cover (playskin only)
        for img in &sk.hidden_cover {
            if dst_id == img.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: img.id.clone(),
                    ..Default::default()
                };
                // SkinHidden creation stubbed
                // In Java: adds OFFSET_LIFT and OFFSET_HIDDEN_COVER to offsets
                return Some(obj);
            }
        }

        // lift cover (playskin only)
        for img in &sk.lift_cover {
            if dst_id == img.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: img.id.clone(),
                    ..Default::default()
                };
                // SkinHidden creation for lift cover stubbed
                // In Java: adds OFFSET_LIFT to offsets
                return Some(obj);
            }
        }

        // bga (playskin only)
        if let Some(ref bga) = sk.bga
            && dst_id == bga.id.as_deref().unwrap_or("")
        {
            let obj = SkinObjectData {
                name: bga.id.clone(),
                ..Default::default()
            };
            // SkinBGA creation stubbed
            return Some(obj);
        }

        // judge (playskin only)
        for judge in &sk.judge {
            if dst_id == judge.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: judge.id.clone(),
                    ..Default::default()
                };
                // SkinJudge creation with images and numbers stubbed
                return Some(obj);
            }
        }

        // POMYU chara
        for chara in &sk.pmchara {
            if dst_id == chara.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: chara.id.clone(),
                    ..Default::default()
                };
                // PomyuCharaLoader stubbed
                return Some(obj);
            }
        }

        None
    }
}
