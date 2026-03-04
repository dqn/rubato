// Mechanical translation of JsonPlaySkinObjectLoader.java
// Play skin object loader (handles note, judge, hidden cover, BGA, etc.)

use std::path::Path;

use crate::json::json_skin;
use crate::json::json_skin_loader::{JSONSkinLoader, SkinData, SkinObjectData, SkinObjectType};
use crate::json::json_skin_object_loader::{self, JsonSkinObjectLoader};

/// Corresponds to JsonPlaySkinObjectLoader extends JsonSkinObjectLoader<PlaySkin>
pub struct JsonPlaySkinObjectLoader;

impl JsonSkinObjectLoader for JsonPlaySkinObjectLoader {
    fn get_skin(&self, header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData {
        // Corresponds to Java: new PlaySkin(header)
        let skin_type = crate::skin_type::SkinType::get_skin_type_by_id(header.skin_type)
            .unwrap_or(crate::skin_type::SkinType::Play7Keys);
        SkinData::from_header(header, skin_type)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::json_skin_loader::SkinHeaderData;
    use crate::json::json_skin_object_loader::JsonSkinObjectLoader;
    use crate::skin_type::SkinType;

    fn make_header(skin_type_id: i32) -> SkinHeaderData {
        SkinHeaderData {
            skin_type: skin_type_id,
            name: "Test Play Skin".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_get_skin_returns_play7keys_for_7key_header() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(SkinType::Play7Keys.id());
        let skin = loader.get_skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::Play7Keys));
        assert!(skin.header.is_some());
        assert_eq!(skin.header.unwrap().name, "Test Play Skin");
    }

    #[test]
    fn test_get_skin_returns_play5keys_for_5key_header() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(SkinType::Play5Keys.id());
        let skin = loader.get_skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::Play5Keys));
    }

    #[test]
    fn test_get_skin_returns_play14keys_for_14key_header() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(SkinType::Play14Keys.id());
        let skin = loader.get_skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::Play14Keys));
    }

    #[test]
    fn test_get_skin_returns_play24keys_for_24key_header() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(SkinType::Play24Keys.id());
        let skin = loader.get_skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::Play24Keys));
    }

    #[test]
    fn test_get_skin_fallback_to_play7keys_for_unknown_id() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(-999);
        let skin = loader.get_skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::Play7Keys));
    }

    #[test]
    fn test_get_skin_default_fields_are_zero() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(SkinType::Play7Keys.id());
        let skin = loader.get_skin(&header);
        assert_eq!(skin.fadeout, 0);
        assert_eq!(skin.input, 0);
        assert_eq!(skin.scene, 0);
        assert!(skin.objects.is_empty());
    }
}
