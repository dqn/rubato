// Mechanical translation of JsonKeyConfigurationSkinObjectLoader.java

use crate::json::json_skin_loader::SkinData;
use crate::json::json_skin_object_loader::JsonSkinObjectLoader;

/// Corresponds to JsonKeyConfigurationSkinObjectLoader extends JsonSkinObjectLoader<KeyConfigurationSkin>
pub struct JsonKeyConfigurationSkinObjectLoader;

impl JsonSkinObjectLoader for JsonKeyConfigurationSkinObjectLoader {
    fn get_skin(&self, header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData {
        // Corresponds to Java: new KeyConfigurationSkin(header)
        SkinData::from_header(header, crate::skin_type::SkinType::KeyConfig)
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
    fn test_get_skin_returns_key_config_type() {
        let loader = JsonKeyConfigurationSkinObjectLoader;
        let header = SkinHeaderData {
            skin_type: SkinType::KeyConfig.id(),
            name: "Test KeyConfig Skin".to_string(),
            ..Default::default()
        };
        let skin = loader.get_skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::KeyConfig));
        assert!(skin.header.is_some());
        assert_eq!(skin.header.unwrap().name, "Test KeyConfig Skin");
    }

    #[test]
    fn test_get_skin_default_fields_are_zero() {
        let loader = JsonKeyConfigurationSkinObjectLoader;
        let header = SkinHeaderData::default();
        let skin = loader.get_skin(&header);
        assert_eq!(skin.fadeout, 0);
        assert!(skin.objects.is_empty());
    }
}
