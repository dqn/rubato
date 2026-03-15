use std::path::Path;

use crate::json::json_skin;
use crate::json::json_skin_loader::{
    JSONSkinLoader, SkinNumberOffset, SkinObjectData, SkinObjectType, SourceDataType,
    get_path_with_filemap,
};
use crate::reexports::*;

/// Convert JSON offset entries to SkinNumberOffset vec.
pub(super) fn map_number_offsets(
    offsets: Option<&Vec<json_skin::Value>>,
) -> Option<Vec<SkinNumberOffset>> {
    offsets.map(|ofs| {
        ofs.iter()
            .map(|o| SkinNumberOffset {
                x: o.x,
                y: o.y,
                w: o.w,
                h: o.h,
            })
            .collect()
    })
}

/// Get texture from source id and path.
/// Corresponds to Java JsonSkinObjectLoader.getTexture(String srcid, Path p)
pub fn texture(loader: &mut JSONSkinLoader, srcid: Option<&str>, p: &Path) -> Option<Texture> {
    let srcid = srcid?;
    let data = loader.source_map.get(srcid)?;
    if data.loaded {
        return match &data.data {
            Some(SourceDataType::Texture(t)) => Some(t.clone()),
            _ => None,
        };
    }
    let data_path = data.path.clone();
    let parent = p
        .parent()
        .map(|pp| pp.to_string_lossy().to_string())
        .unwrap_or_default();
    let image_path = format!("{}/{}", parent, data_path);
    let image_file = get_path_with_filemap(&image_path, &loader.filemap);

    let mut result: Option<Texture> = None;
    if std::path::Path::new(&image_file).exists() {
        let tex = Texture::new(&image_file);
        result = Some(tex.clone());
        if let Some(data) = loader.source_map.get_mut(srcid) {
            data.data = Some(SourceDataType::Texture(tex));
            data.loaded = true;
        }
    } else if let Some(data) = loader.source_map.get_mut(srcid) {
        data.loaded = true;
    }
    result
}

/// Get note textures from image ids.
/// Corresponds to Java JsonSkinObjectLoader.getNoteTexture(String[] images, Path p)
pub fn note_texture(
    loader: &mut JSONSkinLoader,
    images: &[String],
    p: &Path,
) -> Vec<Option<Vec<TextureRegion>>> {
    let sk = match &loader.sk {
        Some(sk) => sk.clone(),
        None => return vec![None; images.len()],
    };
    let mut note_images: Vec<Option<Vec<TextureRegion>>> = Vec::with_capacity(images.len());
    for image_id in images {
        let mut found = false;
        for img in &sk.image {
            if img.id.as_deref() == Some(image_id.as_str()) {
                let tex = texture(loader, img.src.as_deref(), p);
                if let Some(tex) = tex {
                    let regions =
                        source_image(&tex, img.x, img.y, img.w, img.h, img.divx, img.divy);
                    note_images.push(Some(regions));
                } else {
                    note_images.push(None);
                }
                found = true;
                break;
            }
        }
        if !found {
            note_images.push(None);
        }
    }
    note_images
}

/// Create a SkinText from JSON text definition.
/// Corresponds to Java JsonSkinObjectLoader.createText(JsonSkin.Text, Path)
///
/// Resolves the font ID to a font file path from the skin's font list,
/// then returns a `SkinObjectData` with `SkinObjectType::Text`. The actual
/// font loading (BitmapFont for .fnt, FreeTypeFontGenerator for .ttf/.otf)
/// is handled downstream by the `object_converter` when building the
/// concrete `SkinTextBitmap` or `SkinTextFont`.
pub fn create_text(
    loader: &mut JSONSkinLoader,
    text: &json_skin::Text,
    skin_path: &Path,
) -> Option<SkinObjectData> {
    let sk = loader.sk.as_ref()?;
    for font in &sk.font {
        if font.id.as_deref() == text.font.as_deref() {
            let font_path_str = font.path.as_deref().unwrap_or("");
            // Resolve the font path relative to the skin file's directory.
            // The resolved path is stored in the Text variant so that the
            // object_converter can load the correct font file later.
            let resolved_font_path = skin_path
                .parent()
                .map(|pp| pp.join(font_path_str).to_string_lossy().to_string())
                .unwrap_or_else(|| font_path_str.to_string());

            return Some(SkinObjectData {
                name: text.id.clone(),
                object_type: SkinObjectType::Text {
                    font: Some(resolved_font_path),
                    size: text.size,
                    align: text.align,
                    ref_id: text.ref_id,
                    value: text.value,
                    constant_text: text.constant_text.clone(),
                    wrapping: text.wrapping,
                    overflow: text.overflow,
                    outline_color: text.outline_color.clone(),
                    outline_width: text.outline_width,
                    shadow_color: text.shadow_color.clone(),
                    shadow_offset_x: text.shadow_offset_x,
                    shadow_offset_y: text.shadow_offset_y,
                    shadow_smoothness: text.shadow_smoothness,
                },
                ..Default::default()
            });
        }
    }
    log::warn!(
        "create_text: font ID {:?} not found in skin font list",
        text.font
    );
    None
}

/// Get the file path for a source id.
/// Corresponds to Java JsonSkinObjectLoader.getSrcIdPath(String srcid, Path p)
pub fn src_id_path(loader: &JSONSkinLoader, srcid: Option<&str>, p: &Path) -> Option<String> {
    let srcid = srcid?;
    let data = loader.source_map.get(srcid)?;
    let parent = p
        .parent()
        .map(|pp| pp.to_string_lossy().to_string())
        .unwrap_or_default();
    let path = format!("{}/{}", parent, data.path);
    Some(get_path_with_filemap(&path, &loader.filemap))
}

/// Helper: get source image regions from texture
pub fn source_image(
    image: &Texture,
    x: i32,
    y: i32,
    mut w: i32,
    mut h: i32,
    mut divx: i32,
    mut divy: i32,
) -> Vec<TextureRegion> {
    if w == -1 {
        w = image.width;
    }
    if h == -1 {
        h = image.height;
    }
    if divx <= 0 {
        divx = 1;
    }
    if divy <= 0 {
        divy = 1;
    }
    let mut images = Vec::with_capacity((divx * divy) as usize);
    for i in 0..divx {
        for j in 0..divy {
            images.push(TextureRegion::from_texture_region(
                image.clone(),
                x + w / divx * i,
                y + h / divy * j,
                w / divx,
                h / divy,
            ));
        }
    }
    // Reorder: Java uses [divx * j + i] indexing
    let mut result = vec![TextureRegion::new(); (divx * divy) as usize];
    for i in 0..divx {
        for j in 0..divy {
            let src_idx = (i * divy + j) as usize;
            let dst_idx = (divx * j + i) as usize;
            if src_idx < images.len() && dst_idx < result.len() {
                result[dst_idx] = images[src_idx].clone();
            }
        }
    }
    result
}

/// Helper: parse hex color string
pub fn parse_hex_color(hex: &str, fallback: Color) -> Color {
    // Simple hex color parser: "RRGGBBAA" or "RRGGBB"
    if hex.len() >= 6 && hex.is_ascii() {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255) as f32 / 255.0;
        let a = if hex.len() >= 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f32 / 255.0
        } else {
            1.0
        };
        Color::new(r, g, b, a)
    } else {
        fallback
    }
}
