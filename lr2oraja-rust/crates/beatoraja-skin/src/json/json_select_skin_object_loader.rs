// Mechanical translation of JsonSelectSkinObjectLoader.java
// Select skin object loader (handles song list bar)

use std::path::Path;

use crate::json::json_skin;
use crate::json::json_skin_loader::{JSONSkinLoader, SkinData, SkinObjectData, SkinObjectType};
use crate::json::json_skin_object_loader::{self, JsonSkinObjectLoader};

/// Corresponds to JsonSelectSkinObjectLoader extends JsonSkinObjectLoader<MusicSelectSkin>
pub struct JsonSelectSkinObjectLoader;

impl JsonSkinObjectLoader for JsonSelectSkinObjectLoader {
    fn get_skin(&self, _header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData {
        // MusicSelectSkin creation - stubbed pending rendering pipeline
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
