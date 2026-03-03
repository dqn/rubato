// Mechanical translation of JsonCourseResultSkinObjectLoader.java

use crate::json::json_skin_loader::SkinData;
use crate::json::json_skin_object_loader::JsonSkinObjectLoader;

/// Corresponds to JsonCourseResultSkinObjectLoader extends JsonSkinObjectLoader<CourseResultSkin>
pub struct JsonCourseResultSkinObjectLoader;

impl JsonSkinObjectLoader for JsonCourseResultSkinObjectLoader {
    fn get_skin(&self, header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData {
        // Corresponds to Java: new CourseResultSkin(header)
        SkinData::from_header(header, crate::skin_type::SkinType::CourseResult)
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
    fn test_get_skin_returns_course_result_type() {
        let loader = JsonCourseResultSkinObjectLoader;
        let header = SkinHeaderData {
            skin_type: SkinType::CourseResult.id(),
            name: "Test Course Result Skin".to_string(),
            ..Default::default()
        };
        let skin = loader.get_skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::CourseResult));
        assert!(skin.header.is_some());
        assert_eq!(skin.header.unwrap().name, "Test Course Result Skin");
    }

    #[test]
    fn test_get_skin_default_fields_are_zero() {
        let loader = JsonCourseResultSkinObjectLoader;
        let header = SkinHeaderData::default();
        let skin = loader.get_skin(&header);
        assert_eq!(skin.fadeout, 0);
        assert!(skin.objects.is_empty());
    }
}
