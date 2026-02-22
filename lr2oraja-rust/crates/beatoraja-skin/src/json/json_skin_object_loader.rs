// Mechanical translation of JsonSkinObjectLoader.java
// Object loader base class (abstract in Java, trait in Rust)

use std::path::Path;

use crate::json::json_skin;
use crate::json::json_skin_loader::{JSONSkinLoader, SkinData, SkinObjectData, SourceDataType};
use crate::stubs::*;

/// Corresponds to JsonSkinObjectLoader<S extends Skin>
/// In Java this is an abstract class parameterized by skin type.
/// In Rust we use a trait.
pub trait JsonSkinObjectLoader {
    fn get_skin(&self, header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData;

    fn load_skin_object(
        &self,
        loader: &mut JSONSkinLoader,
        skin: &SkinData,
        sk: &json_skin::Skin,
        dst: &json_skin::Destination,
        p: &Path,
    ) -> Option<SkinObjectData> {
        load_base_skin_object(loader, skin, sk, dst, p)
    }
}

/// Base skin object loading logic (translated from JsonSkinObjectLoader.loadSkinObject)
pub fn load_base_skin_object(
    loader: &mut JSONSkinLoader,
    _skin: &SkinData,
    sk: &json_skin::Skin,
    dst: &json_skin::Destination,
    p: &Path,
) -> Option<SkinObjectData> {
    let dst_id = dst.id.as_deref()?;

    // image
    for img in &sk.image {
        if dst_id == img.id.as_deref().unwrap_or("") {
            let src = img.src.as_deref();
            if let Some(srcid) = src {
                let _data = loader.get_source(srcid, p);
                // SkinImage creation depends on Texture/Movie - stubbed
                let obj = SkinObjectData {
                    name: img.id.clone(),
                    ..Default::default()
                };
                if let Some(act) = img.act {
                    // obj.set_click_event / click_event_type stubbed
                    let _ = act;
                }
                return Some(obj);
            }
            return None;
        }
    }

    // imageset
    for imgs in &sk.imageset {
        if dst_id == imgs.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: imgs.id.clone(),
                ..Default::default()
            };
            if let Some(act) = imgs.act {
                // click event stubbed
                let _ = act;
            }
            return Some(obj);
        }
    }

    // value (SkinNumber)
    for value in &sk.value {
        if dst_id == value.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: value.id.clone(),
                ..Default::default()
            };
            // SkinNumber creation depends on TextureRegion - stubbed
            return Some(obj);
        }
    }

    // floatvalue (SkinFloat)
    for fv in &sk.floatvalue {
        if dst_id == fv.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: fv.id.clone(),
                ..Default::default()
            };
            // SkinFloat creation depends on TextureRegion - stubbed
            return Some(obj);
        }
    }

    // text
    for text in &sk.text {
        if dst_id == text.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: text.id.clone(),
                ..Default::default()
            };
            // SkinText creation stubbed
            return Some(obj);
        }
    }

    // slider
    for slider in &sk.slider {
        if dst_id == slider.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: slider.id.clone(),
                ..Default::default()
            };
            // SkinSlider creation stubbed
            return Some(obj);
        }
    }

    // graph
    for graph in &sk.graph {
        if dst_id == graph.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: graph.id.clone(),
                ..Default::default()
            };
            // SkinGraph / SkinDistributionGraph creation stubbed
            return Some(obj);
        }
    }

    // gaugegraph
    for ggraph in &sk.gaugegraph {
        if dst_id == ggraph.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: ggraph.id.clone(),
                ..Default::default()
            };
            // SkinGaugeGraphObject creation stubbed
            return Some(obj);
        }
    }

    // judgegraph
    for ggraph in &sk.judgegraph {
        if dst_id == ggraph.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData::default();
            // SkinNoteDistributionGraph creation stubbed
            return Some(obj);
        }
    }

    // bpmgraph
    for ggraph in &sk.bpmgraph {
        if dst_id == ggraph.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData::default();
            // SkinBPMGraph creation stubbed
            return Some(obj);
        }
    }

    // hiterrorvisualizer
    for hev in &sk.hiterrorvisualizer {
        if dst_id == hev.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData::default();
            return Some(obj);
        }
    }

    // timingvisualizer
    for tv in &sk.timingvisualizer {
        if dst_id == tv.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData::default();
            return Some(obj);
        }
    }

    // timingdistributiongraph
    for td in &sk.timingdistributiongraph {
        if dst_id == td.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData::default();
            return Some(obj);
        }
    }

    // gauge
    if let Some(ref gauge) = sk.gauge
        && dst_id == gauge.id.as_deref().unwrap_or("")
    {
        let obj = SkinObjectData {
            name: gauge.id.clone(),
            ..Default::default()
        };
        // SkinGauge creation stubbed
        return Some(obj);
    }

    None
}

/// Get texture from source id and path.
/// Corresponds to Java JsonSkinObjectLoader.getTexture(String srcid, Path p)
pub fn get_texture(loader: &mut JSONSkinLoader, srcid: Option<&str>, p: &Path) -> Option<Texture> {
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

use crate::json::json_skin_loader::get_path_with_filemap;

/// Get note textures from image ids.
/// Corresponds to Java JsonSkinObjectLoader.getNoteTexture(String[] images, Path p)
pub fn get_note_texture(
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
                let tex = get_texture(loader, img.src.as_deref(), p);
                if let Some(tex) = tex {
                    let regions =
                        get_source_image(&tex, img.x, img.y, img.w, img.h, img.divx, img.divy);
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
pub fn create_text(
    loader: &mut JSONSkinLoader,
    text: &json_skin::Text,
    skin_path: &Path,
) -> Option<SkinObjectData> {
    let sk = loader.sk.as_ref()?;
    for font in &sk.font {
        if font.id.as_deref() == text.font.as_deref() {
            let font_path_str = font.path.as_deref().unwrap_or("");
            let _path = skin_path.parent().map(|pp| pp.join(font_path_str));
            // In Java: creates SkinTextBitmap or SkinTextFont based on file extension.
            // Stubbed: requires font loading infrastructure.
            let obj = SkinObjectData {
                name: text.id.clone(),
                ..Default::default()
            };
            return Some(obj);
        }
    }
    None
}

/// Get the file path for a source id.
/// Corresponds to Java JsonSkinObjectLoader.getSrcIdPath(String srcid, Path p)
pub fn get_src_id_path(loader: &JSONSkinLoader, srcid: Option<&str>, p: &Path) -> Option<String> {
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
pub fn get_source_image(
    image: &Texture,
    x: i32,
    y: i32,
    mut w: i32,
    mut h: i32,
    mut divx: i32,
    mut divy: i32,
) -> Vec<TextureRegion> {
    if w == -1 {
        w = image.get_width();
    }
    if h == -1 {
        h = image.get_height();
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
    if hex.len() >= 6 {
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

/// Helper: set destination on a skin object (used by subclass loaders)
pub fn set_destination_on_object(obj: &mut SkinObjectData, dst: &json_skin::Destination) {
    let mut prev: Option<json_skin::Animation> = None;
    for a_orig in &dst.dst {
        let mut a = a_orig.clone();
        if let Some(ref p) = prev {
            a.time = if a.time == i32::MIN { p.time } else { a.time };
            a.x = if a.x == i32::MIN { p.x } else { a.x };
            a.y = if a.y == i32::MIN { p.y } else { a.y };
            a.w = if a.w == i32::MIN { p.w } else { a.w };
            a.h = if a.h == i32::MIN { p.h } else { a.h };
            a.acc = if a.acc == i32::MIN { p.acc } else { a.acc };
            a.angle = if a.angle == i32::MIN {
                p.angle
            } else {
                a.angle
            };
            a.a = if a.a == i32::MIN { p.a } else { a.a };
            a.r = if a.r == i32::MIN { p.r } else { a.r };
            a.g = if a.g == i32::MIN { p.g } else { a.g };
            a.b = if a.b == i32::MIN { p.b } else { a.b };
        } else {
            a.time = if a.time == i32::MIN { 0 } else { a.time };
            a.x = if a.x == i32::MIN { 0 } else { a.x };
            a.y = if a.y == i32::MIN { 0 } else { a.y };
            a.w = if a.w == i32::MIN { 0 } else { a.w };
            a.h = if a.h == i32::MIN { 0 } else { a.h };
            a.acc = if a.acc == i32::MIN { 0 } else { a.acc };
            a.angle = if a.angle == i32::MIN { 0 } else { a.angle };
            a.a = if a.a == i32::MIN { 255 } else { a.a };
            a.r = if a.r == i32::MIN { 255 } else { a.r };
            a.g = if a.g == i32::MIN { 255 } else { a.g };
            a.b = if a.b == i32::MIN { 255 } else { a.b };
        }
        prev = Some(a);
    }

    let mut offsets: Vec<i32> = Vec::with_capacity(dst.offsets.len() + 1);
    for o in &dst.offsets {
        offsets.push(*o);
    }
    offsets.push(dst.offset);
    obj.offset_ids = offsets;

    if dst.stretch >= 0 {
        obj.stretch = dst.stretch;
    }
}
