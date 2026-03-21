use std::collections::HashMap;
use std::path::Path;

use crate::json::json_skin_loader::{
    ResolvedImageEntry, SourceData, SourceDataType, get_path_with_filemap,
};
use crate::json::json_skin_object_loader::{source_image, utilities::resolve_wildcard_path};
use crate::objects::skin_image::SkinImage;
use crate::reexports::TextureRegion;
use crate::types::skin::SkinObject;

/// Loads a texture from the source map, resolving the source ID path.
/// Applies filemap substitution to the resolved path, matching the behavior of
/// `utilities::texture()` in `json_skin_object_loader`.
pub(super) fn get_texture_for_src(
    src_id: Option<&str>,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    _usecim: bool,
    filemap: &HashMap<String, String>,
) -> Option<crate::reexports::Texture> {
    let src_id = src_id?;

    // Check if already loaded
    if let Some(data) = source_map.get(src_id) {
        if data.loaded {
            return match &data.data {
                Some(SourceDataType::Texture(tex)) => Some(tex.clone()),
                _ => None,
            };
        }
    } else {
        return None;
    }

    // Load the texture
    let data_path = source_map.get(src_id)?.path.clone();
    let parent = skin_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    // Java parity: Java's new File(parent, child) ignores parent if child is absolute.
    let image_path = if std::path::Path::new(&data_path).is_absolute() {
        data_path.clone()
    } else {
        format!("{}/{}", parent, data_path)
    };
    // Apply filemap substitution to the full path, matching utilities::texture() behavior.
    let image_file = get_path_with_filemap(&image_path, filemap);

    let resolved_path = if std::path::Path::new(&image_file).exists() {
        Some(image_file)
    } else if image_file.contains('*') {
        resolve_wildcard_path(&image_file)
    } else {
        None
    };
    let result =
        resolved_path.map(|path| SourceDataType::Texture(crate::reexports::Texture::new(&path)));

    let tex_result = match &result {
        Some(SourceDataType::Texture(tex)) => Some(tex.clone()),
        _ => None,
    };

    // Cache the result
    if let Some(data) = source_map.get_mut(src_id) {
        data.data = result;
        data.loaded = true;
    }

    tex_result
}

/// Resolve an ImageSet into a multi-source SkinImage with actual textures.
/// Each entry in the set is looked up and its texture resolved from source_map.
pub(super) fn resolve_image_set(
    entries: &[ResolvedImageEntry],
    ref_id: i32,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    filemap: &HashMap<String, String>,
) -> Option<SkinObject> {
    if entries.is_empty() {
        return None;
    }
    let images: Vec<Vec<TextureRegion>> = entries
        .iter()
        .filter_map(|entry| {
            let tex =
                get_texture_for_src(entry.src.as_deref(), source_map, skin_path, usecim, filemap)?;
            Some(source_image(
                &tex, entry.x, entry.y, entry.w, entry.h, entry.divx, entry.divy,
            ))
        })
        .collect();
    if images.is_empty() {
        return None;
    }
    Some(SkinObject::Image(SkinImage::new_with_int_timer_ref_id(
        images, 0, 0, ref_id,
    )))
}
