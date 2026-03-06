/// Macro to define a simple JSON skin object loader that delegates to the base trait.
///
/// All five "simple" loaders (Result, Decide, CourseResult, KeyConfiguration,
/// SkinConfiguration) follow the exact same pattern: a unit struct that
/// implements `JsonSkinObjectLoader` by returning `SkinData::from_header`
/// with a specific `SkinType` variant. This macro eliminates that boilerplate.
///
/// # Arguments
/// * `$name` - The struct name (e.g. `JsonResultSkinObjectLoader`)
/// * `$java_class` - The original Java class name for the doc comment
/// * `$skin_type` - The `SkinType` variant (e.g. `Result`)
/// * `$test_skin_name` - A human-readable skin name used in tests
macro_rules! define_json_skin_loader {
    ($name:ident, $java_class:expr, $skin_type:ident, $test_skin_name:expr) => {
        use crate::json::json_skin_loader::SkinData;
        use crate::json::json_skin_object_loader::JsonSkinObjectLoader;

        #[doc = concat!("Corresponds to ", $java_class, " extends JsonSkinObjectLoader")]
        pub struct $name;

        impl JsonSkinObjectLoader for $name {
            fn skin(&self, header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData {
                SkinData::from_header(header, crate::skin_type::SkinType::$skin_type)
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
            fn test_get_skin_returns_correct_type() {
                let loader = $name;
                let header = SkinHeaderData {
                    skin_type: SkinType::$skin_type.id(),
                    name: $test_skin_name.to_string(),
                    ..Default::default()
                };
                let skin = loader.skin(&header);
                assert_eq!(skin.skin_type, Some(SkinType::$skin_type));
                assert!(skin.header.is_some());
                assert_eq!(skin.header.unwrap().name, $test_skin_name);
            }

            #[test]
            fn test_get_skin_default_fields_are_zero() {
                let loader = $name;
                let header = SkinHeaderData::default();
                let skin = loader.skin(&header);
                assert_eq!(skin.fadeout, 0);
                assert_eq!(skin.input, 0);
                assert!(skin.objects.is_empty());
            }
        }
    };
}

pub(crate) use define_json_skin_loader;
