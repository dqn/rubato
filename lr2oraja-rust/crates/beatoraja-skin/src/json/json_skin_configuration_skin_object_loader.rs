// Mechanical translation of JsonSkinConfigurationSkinObjectLoader.java

use crate::json::json_skin_loader::SkinData;
use crate::json::json_skin_object_loader::JsonSkinObjectLoader;

/// Corresponds to JsonSkinConfigurationSkinObjectLoader extends JsonSkinObjectLoader<SkinConfigurationSkin>
pub struct JsonSkinConfigurationSkinObjectLoader;

impl JsonSkinObjectLoader for JsonSkinConfigurationSkinObjectLoader {
    fn get_skin(&self, header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData {
        // Corresponds to Java: new SkinConfigurationSkin(header)
        SkinData::from_header(header, crate::skin_type::SkinType::SkinSelect)
    }

    // Uses default load_skin_object from trait (base loader only)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::json_skin_loader::SkinHeaderData;
    use crate::json::json_skin_object_loader::JsonSkinObjectLoader;
    use crate::skin_type::SkinType;

    #[test]
    fn test_get_skin_returns_skin_select_type() {
        let loader = JsonSkinConfigurationSkinObjectLoader;
        let header = SkinHeaderData {
            skin_type: SkinType::SkinSelect.id(),
            name: "Test SkinConfig Skin".to_string(),
            ..Default::default()
        };
        let skin = loader.get_skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::SkinSelect));
        assert!(skin.header.is_some());
        assert_eq!(skin.header.unwrap().name, "Test SkinConfig Skin");
    }

    #[test]
    fn test_get_skin_default_fields_are_zero() {
        let loader = JsonSkinConfigurationSkinObjectLoader;
        let header = SkinHeaderData::default();
        let skin = loader.get_skin(&header);
        assert_eq!(skin.fadeout, 0);
        assert!(skin.objects.is_empty());
    }
}
