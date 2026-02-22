// Mechanical translation of JsonPlaySkinObjectLoader.java
// Play skin object loader (handles note, judge, hidden cover, BGA, etc.)

use std::path::Path;

use crate::json::json_skin;
use crate::json::json_skin_loader::{JSONSkinLoader, SkinData, SkinObjectData, SkinObjectType};
use crate::json::json_skin_object_loader::{self, JsonSkinObjectLoader};

/// Corresponds to JsonPlaySkinObjectLoader extends JsonSkinObjectLoader<PlaySkin>
pub struct JsonPlaySkinObjectLoader;

impl JsonSkinObjectLoader for JsonPlaySkinObjectLoader {
    fn get_skin(&self, _header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData {
        // PlaySkin creation - stubbed pending rendering pipeline
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
                object_type: SkinObjectType::Note,
                ..Default::default()
            };
            return Some(obj);
        }

        // hidden cover (playskin only)
        for img in &sk.hidden_cover {
            if dst_id == img.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: img.id.clone(),
                    object_type: SkinObjectType::HiddenCover {
                        src: img.src.clone(),
                        x: img.x,
                        y: img.y,
                        w: img.w,
                        h: img.h,
                        divx: img.divx,
                        divy: img.divy,
                        timer: img.timer,
                        cycle: img.cycle,
                        disapear_line: img.disapear_line,
                        is_disapear_line_link_lift: img.is_disapear_line_link_lift,
                    },
                    ..Default::default()
                };
                return Some(obj);
            }
        }

        // lift cover (playskin only)
        for img in &sk.lift_cover {
            if dst_id == img.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: img.id.clone(),
                    object_type: SkinObjectType::LiftCover {
                        src: img.src.clone(),
                        x: img.x,
                        y: img.y,
                        w: img.w,
                        h: img.h,
                        divx: img.divx,
                        divy: img.divy,
                        timer: img.timer,
                        cycle: img.cycle,
                        disapear_line: img.disapear_line,
                        is_disapear_line_link_lift: img.is_disapear_line_link_lift,
                    },
                    ..Default::default()
                };
                return Some(obj);
            }
        }

        // bga (playskin only)
        if let Some(ref bga) = sk.bga
            && dst_id == bga.id.as_deref().unwrap_or("")
        {
            let obj = SkinObjectData {
                name: bga.id.clone(),
                object_type: SkinObjectType::Bga {
                    bga_expand: loader.bga_expand,
                },
                ..Default::default()
            };
            return Some(obj);
        }

        // judge (playskin only)
        for judge in &sk.judge {
            if dst_id == judge.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: judge.id.clone(),
                    object_type: SkinObjectType::Judge {
                        index: judge.index,
                        shift: judge.shift,
                    },
                    ..Default::default()
                };
                return Some(obj);
            }
        }

        // POMYU chara
        for chara in &sk.pmchara {
            if dst_id == chara.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: chara.id.clone(),
                    object_type: SkinObjectType::PmChara {
                        src: chara.src.clone(),
                        color: chara.color,
                        chara_type: chara.chara_type,
                        side: chara.side,
                    },
                    ..Default::default()
                };
                return Some(obj);
            }
        }

        None
    }
}
