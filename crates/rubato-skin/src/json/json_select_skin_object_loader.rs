// Mechanical translation of JsonSelectSkinObjectLoader.java
// Select skin object loader (handles song list bar)

use std::path::Path;

use crate::json::json_skin;
use crate::json::json_skin_loader::{JSONSkinLoader, SkinData, SkinObjectData, SkinObjectType};
use crate::json::json_skin_object_loader::{self, JsonSkinObjectLoader};

/// Corresponds to JsonSelectSkinObjectLoader extends JsonSkinObjectLoader<MusicSelectSkin>
pub struct JsonSelectSkinObjectLoader;

impl JsonSkinObjectLoader for JsonSelectSkinObjectLoader {
    fn get_skin(&self, header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData {
        // Corresponds to Java: new MusicSelectSkin(header)
        SkinData::from_header(header, crate::skin_type::SkinType::MusicSelect)
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

        // songlist
        if let Some(ref songlist) = sk.songlist
            && dst_id == songlist.id.as_deref().unwrap_or("")
        {
            let obj = SkinObjectData {
                name: songlist.id.clone(),
                object_type: SkinObjectType::SongList {
                    center: songlist.center,
                    clickable: songlist.clickable.clone(),
                },
                ..Default::default()
            };
            return Some(obj);
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

    #[test]
    fn test_get_skin_returns_music_select_type() {
        let loader = JsonSelectSkinObjectLoader;
        let header = SkinHeaderData {
            skin_type: SkinType::MusicSelect.id(),
            name: "Test Select Skin".to_string(),
            ..Default::default()
        };
        let skin = loader.get_skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::MusicSelect));
        assert!(skin.header.is_some());
        assert_eq!(skin.header.unwrap().name, "Test Select Skin");
    }

    #[test]
    fn test_get_skin_default_fields_are_zero() {
        let loader = JsonSelectSkinObjectLoader;
        let header = SkinHeaderData::default();
        let skin = loader.get_skin(&header);
        assert_eq!(skin.fadeout, 0);
        assert!(skin.objects.is_empty());
    }
}
