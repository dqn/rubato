// Mechanical translation of JsonSkinObjectLoader.java
// Object loader base class (abstract in Java, trait in Rust)

use std::path::Path;

use crate::json::json_skin;
use crate::json::json_skin_loader::{
    JSONSkinLoader, SkinData, SkinNumberOffset, SkinObjectData, SkinObjectType, SourceDataType,
};
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
            let data = loader.get_source(img.src.as_deref().unwrap_or(""), p);
            let is_movie = matches!(&data, Some(SourceDataType::Movie(_)));

            if data.is_some() {
                let obj = SkinObjectData {
                    name: img.id.clone(),
                    object_type: SkinObjectType::Image {
                        src: img.src.clone(),
                        x: img.x,
                        y: img.y,
                        w: img.w,
                        h: img.h,
                        divx: img.divx,
                        divy: img.divy,
                        timer: img.timer,
                        cycle: img.cycle,
                        len: img.len,
                        ref_id: img.ref_id,
                        act: img.act,
                        click: img.click,
                        is_movie,
                    },
                    ..Default::default()
                };
                if img.act.is_some() {
                    // Click event info captured in SkinObjectType::Image
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
                object_type: SkinObjectType::ImageSet {
                    images: imgs.images.clone(),
                    ref_id: imgs.ref_id,
                    value: imgs.value,
                    act: imgs.act,
                    click: imgs.click,
                },
                ..Default::default()
            };
            return Some(obj);
        }
    }

    // value (SkinNumber)
    for value in &sk.value {
        if dst_id == value.id.as_deref().unwrap_or("") {
            let offsets = value.offset.as_ref().map(|ofs| {
                ofs.iter()
                    .map(|o| SkinNumberOffset {
                        x: o.x,
                        y: o.y,
                        w: o.w,
                        h: o.h,
                    })
                    .collect()
            });
            let obj = SkinObjectData {
                name: value.id.clone(),
                object_type: SkinObjectType::Number {
                    src: value.src.clone(),
                    x: value.x,
                    y: value.y,
                    w: value.w,
                    h: value.h,
                    divx: value.divx,
                    divy: value.divy,
                    timer: value.timer,
                    cycle: value.cycle,
                    digit: value.digit,
                    padding: value.padding,
                    zeropadding: value.zeropadding,
                    space: value.space,
                    ref_id: value.ref_id,
                    value: value.value,
                    align: value.align,
                    offsets,
                },
                ..Default::default()
            };
            return Some(obj);
        }
    }

    // floatvalue (SkinFloat)
    for fv in &sk.floatvalue {
        if dst_id == fv.id.as_deref().unwrap_or("") {
            let offsets = fv.offset.as_ref().map(|ofs| {
                ofs.iter()
                    .map(|o| SkinNumberOffset {
                        x: o.x,
                        y: o.y,
                        w: o.w,
                        h: o.h,
                    })
                    .collect()
            });
            let obj = SkinObjectData {
                name: fv.id.clone(),
                object_type: SkinObjectType::Float {
                    src: fv.src.clone(),
                    x: fv.x,
                    y: fv.y,
                    w: fv.w,
                    h: fv.h,
                    divx: fv.divx,
                    divy: fv.divy,
                    timer: fv.timer,
                    cycle: fv.cycle,
                    iketa: fv.iketa,
                    fketa: fv.fketa,
                    is_signvisible: fv.is_signvisible,
                    align: fv.align,
                    zeropadding: fv.zeropadding,
                    space: fv.space,
                    ref_id: fv.ref_id,
                    value: fv.value,
                    gain: fv.gain,
                    offsets,
                },
                ..Default::default()
            };
            return Some(obj);
        }
    }

    // text
    for text in &sk.text {
        if dst_id == text.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: text.id.clone(),
                object_type: SkinObjectType::Text {
                    font: text.font.clone(),
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
            };
            return Some(obj);
        }
    }

    // slider
    for slider in &sk.slider {
        if dst_id == slider.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: slider.id.clone(),
                object_type: SkinObjectType::Slider {
                    src: slider.src.clone(),
                    x: slider.x,
                    y: slider.y,
                    w: slider.w,
                    h: slider.h,
                    divx: slider.divx,
                    divy: slider.divy,
                    timer: slider.timer,
                    cycle: slider.cycle,
                    angle: slider.angle,
                    range: slider.range,
                    slider_type: slider.slider_type,
                    changeable: slider.changeable,
                    value: slider.value,
                    event: slider.event,
                    is_ref_num: slider.is_ref_num,
                    min: slider.min,
                    max: slider.max,
                },
                ..Default::default()
            };
            return Some(obj);
        }
    }

    // graph
    for graph in &sk.graph {
        if dst_id == graph.id.as_deref().unwrap_or("") {
            if graph.graph_type < 0 {
                // SkinDistributionGraph
                let obj = SkinObjectData {
                    name: graph.id.clone(),
                    object_type: SkinObjectType::DistributionGraph {
                        src: graph.src.clone(),
                        x: graph.x,
                        y: graph.y,
                        w: graph.w,
                        h: graph.h,
                        divx: graph.divx,
                        divy: graph.divy,
                        timer: graph.timer,
                        cycle: graph.cycle,
                        graph_type: graph.graph_type,
                    },
                    ..Default::default()
                };
                return Some(obj);
            } else {
                // SkinGraph
                let obj = SkinObjectData {
                    name: graph.id.clone(),
                    object_type: SkinObjectType::Graph {
                        src: graph.src.clone(),
                        x: graph.x,
                        y: graph.y,
                        w: graph.w,
                        h: graph.h,
                        divx: graph.divx,
                        divy: graph.divy,
                        timer: graph.timer,
                        cycle: graph.cycle,
                        angle: graph.angle,
                        graph_type: graph.graph_type,
                        value: graph.value,
                        is_ref_num: graph.is_ref_num,
                        min: graph.min,
                        max: graph.max,
                    },
                    ..Default::default()
                };
                return Some(obj);
            }
        }
    }

    // gaugegraph
    for ggraph in &sk.gaugegraph {
        if dst_id == ggraph.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: ggraph.id.clone(),
                object_type: SkinObjectType::GaugeGraph {
                    color: ggraph.color.clone(),
                    assist_clear_bg_color: ggraph.assist_clear_bg_color.clone(),
                    assist_and_easy_fail_bg_color: ggraph.assist_and_easy_fail_bg_color.clone(),
                    groove_fail_bg_color: ggraph.groove_fail_bg_color.clone(),
                    groove_clear_and_hard_bg_color: ggraph.groove_clear_and_hard_bg_color.clone(),
                    ex_hard_bg_color: ggraph.ex_hard_bg_color.clone(),
                    hazard_bg_color: ggraph.hazard_bg_color.clone(),
                    assist_clear_line_color: ggraph.assist_clear_line_color.clone(),
                    assist_and_easy_fail_line_color: ggraph.assist_and_easy_fail_line_color.clone(),
                    groove_fail_line_color: ggraph.groove_fail_line_color.clone(),
                    groove_clear_and_hard_line_color: ggraph
                        .groove_clear_and_hard_line_color
                        .clone(),
                    ex_hard_line_color: ggraph.ex_hard_line_color.clone(),
                    hazard_line_color: ggraph.hazard_line_color.clone(),
                    borderline_color: ggraph.borderline_color.clone(),
                    border_color: ggraph.border_color.clone(),
                },
                ..Default::default()
            };
            return Some(obj);
        }
    }

    // judgegraph
    for ggraph in &sk.judgegraph {
        if dst_id == ggraph.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: ggraph.id.clone(),
                object_type: SkinObjectType::JudgeGraph {
                    graph_type: ggraph.graph_type,
                    delay: ggraph.delay,
                    back_tex_off: ggraph.back_tex_off,
                    order_reverse: ggraph.order_reverse,
                    no_gap: ggraph.no_gap,
                    no_gap_x: ggraph.no_gap_x,
                },
                ..Default::default()
            };
            // Java uses break here (not return), so we break out of this loop
            return Some(obj);
        }
    }

    // bpmgraph
    for ggraph in &sk.bpmgraph {
        if dst_id == ggraph.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: ggraph.id.clone(),
                object_type: SkinObjectType::BpmGraph {
                    delay: ggraph.delay,
                    line_width: ggraph.line_width,
                    main_bpm_color: ggraph.main_bpm_color.clone(),
                    min_bpm_color: ggraph.min_bpm_color.clone(),
                    max_bpm_color: ggraph.max_bpm_color.clone(),
                    other_bpm_color: ggraph.other_bpm_color.clone(),
                    stop_line_color: ggraph.stop_line_color.clone(),
                    transition_line_color: ggraph.transition_line_color.clone(),
                },
                ..Default::default()
            };
            return Some(obj);
        }
    }

    // hiterrorvisualizer
    for hev in &sk.hiterrorvisualizer {
        if dst_id == hev.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: hev.id.clone(),
                object_type: SkinObjectType::HitErrorVisualizer {
                    width: hev.width,
                    judge_width_millis: hev.judge_width_millis,
                    line_width: hev.line_width,
                    color_mode: hev.color_mode,
                    hiterror_mode: hev.hiterror_mode,
                    ema_mode: hev.ema_mode,
                    line_color: hev.line_color.clone(),
                    center_color: hev.center_color.clone(),
                    pg_color: hev.pg_color.clone(),
                    gr_color: hev.gr_color.clone(),
                    gd_color: hev.gd_color.clone(),
                    bd_color: hev.bd_color.clone(),
                    pr_color: hev.pr_color.clone(),
                    ema_color: hev.ema_color.clone(),
                    alpha: hev.alpha,
                    window_length: hev.window_length,
                    transparent: hev.transparent,
                    draw_decay: hev.draw_decay,
                },
                ..Default::default()
            };
            return Some(obj);
        }
    }

    // timingvisualizer
    for tv in &sk.timingvisualizer {
        if dst_id == tv.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: tv.id.clone(),
                object_type: SkinObjectType::TimingVisualizer {
                    width: tv.width,
                    judge_width_millis: tv.judge_width_millis,
                    line_width: tv.line_width,
                    line_color: tv.line_color.clone(),
                    center_color: tv.center_color.clone(),
                    pg_color: tv.pg_color.clone(),
                    gr_color: tv.gr_color.clone(),
                    gd_color: tv.gd_color.clone(),
                    bd_color: tv.bd_color.clone(),
                    pr_color: tv.pr_color.clone(),
                    transparent: tv.transparent,
                    draw_decay: tv.draw_decay,
                },
                ..Default::default()
            };
            return Some(obj);
        }
    }

    // timingdistributiongraph
    for td in &sk.timingdistributiongraph {
        if dst_id == td.id.as_deref().unwrap_or("") {
            let obj = SkinObjectData {
                name: td.id.clone(),
                object_type: SkinObjectType::TimingDistributionGraph {
                    width: td.width,
                    line_width: td.line_width,
                    graph_color: td.graph_color.clone(),
                    average_color: td.average_color.clone(),
                    dev_color: td.dev_color.clone(),
                    pg_color: td.pg_color.clone(),
                    gr_color: td.gr_color.clone(),
                    gd_color: td.gd_color.clone(),
                    bd_color: td.bd_color.clone(),
                    pr_color: td.pr_color.clone(),
                    draw_average: td.draw_average,
                    draw_dev: td.draw_dev,
                },
                ..Default::default()
            };
            return Some(obj);
        }
    }

    // gauge
    if let Some(ref gauge) = sk.gauge
        && dst_id == gauge.id.as_deref().unwrap_or("")
    {
        let obj = SkinObjectData {
            name: gauge.id.clone(),
            object_type: SkinObjectType::Gauge {
                nodes: gauge.nodes.clone(),
                parts: gauge.parts,
                gauge_type: gauge.gauge_type,
                range: gauge.range,
                cycle: gauge.cycle,
                starttime: gauge.starttime,
                endtime: gauge.endtime,
            },
            ..Default::default()
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::json_skin;
    use crate::json::json_skin_loader::SkinObjectType;

    fn make_loader() -> JSONSkinLoader {
        JSONSkinLoader::new()
    }

    fn make_skin() -> SkinData {
        SkinData::new()
    }

    fn make_sk() -> json_skin::Skin {
        json_skin::Skin {
            w: 1920,
            h: 1080,
            ..Default::default()
        }
    }

    fn make_dst(id: &str) -> json_skin::Destination {
        json_skin::Destination {
            id: Some(id.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_load_image_no_source() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.image.push(json_skin::Image {
            id: Some("img1".to_string()),
            src: Some("src1".to_string()),
            ..Default::default()
        });
        let dst = make_dst("img1");
        let p = std::path::Path::new("/fake/skin.json");

        // No source data loaded, so get_source returns None
        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_imageset() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.imageset.push(json_skin::ImageSet {
            id: Some("imgset1".to_string()),
            ref_id: 42,
            value: Some(100),
            images: vec!["a".to_string(), "b".to_string()],
            act: Some(10),
            click: 1,
        });
        let dst = make_dst("imgset1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        let obj = result.unwrap();
        assert_eq!(obj.name, Some("imgset1".to_string()));
        match &obj.object_type {
            SkinObjectType::ImageSet {
                images,
                ref_id,
                value,
                act,
                click,
            } => {
                assert_eq!(images, &vec!["a".to_string(), "b".to_string()]);
                assert_eq!(*ref_id, 42);
                assert_eq!(*value, Some(100));
                assert_eq!(*act, Some(10));
                assert_eq!(*click, 1);
            }
            _ => panic!("Expected ImageSet"),
        }
    }

    #[test]
    fn test_load_value_number() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.value.push(json_skin::Value {
            id: Some("num1".to_string()),
            src: Some("src1".to_string()),
            digit: 5,
            padding: 1,
            zeropadding: 1,
            space: 2,
            ref_id: 10,
            value: Some(200),
            align: 1,
            divx: 10,
            divy: 1,
            ..Default::default()
        });
        let dst = make_dst("num1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        let obj = result.unwrap();
        match &obj.object_type {
            SkinObjectType::Number {
                digit,
                padding,
                ref_id,
                value,
                align,
                ..
            } => {
                assert_eq!(*digit, 5);
                assert_eq!(*padding, 1);
                assert_eq!(*ref_id, 10);
                assert_eq!(*value, Some(200));
                assert_eq!(*align, 1);
            }
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn test_load_float_value() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.floatvalue.push(json_skin::FloatValue {
            id: Some("fv1".to_string()),
            iketa: 3,
            fketa: 2,
            gain: 1.5,
            is_signvisible: true,
            ..Default::default()
        });
        let dst = make_dst("fv1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        match &result.unwrap().object_type {
            SkinObjectType::Float {
                iketa,
                fketa,
                gain,
                is_signvisible,
                ..
            } => {
                assert_eq!(*iketa, 3);
                assert_eq!(*fketa, 2);
                assert!((gain - 1.5).abs() < f32::EPSILON);
                assert!(*is_signvisible);
            }
            _ => panic!("Expected Float"),
        }
    }

    #[test]
    fn test_load_text() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.text.push(json_skin::Text {
            id: Some("txt1".to_string()),
            font: Some("font1".to_string()),
            size: 24,
            align: 2,
            ref_id: 5,
            constant_text: Some("Hello".to_string()),
            wrapping: true,
            ..Default::default()
        });
        let dst = make_dst("txt1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        match &result.unwrap().object_type {
            SkinObjectType::Text {
                font,
                size,
                align,
                constant_text,
                wrapping,
                ..
            } => {
                assert_eq!(*font, Some("font1".to_string()));
                assert_eq!(*size, 24);
                assert_eq!(*align, 2);
                assert_eq!(*constant_text, Some("Hello".to_string()));
                assert!(*wrapping);
            }
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_load_slider() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.slider.push(json_skin::Slider {
            id: Some("sl1".to_string()),
            angle: 1,
            range: 100,
            slider_type: 2,
            changeable: false,
            value: Some(50),
            ..Default::default()
        });
        let dst = make_dst("sl1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        match &result.unwrap().object_type {
            SkinObjectType::Slider {
                angle,
                range,
                slider_type,
                changeable,
                value,
                ..
            } => {
                assert_eq!(*angle, 1);
                assert_eq!(*range, 100);
                assert_eq!(*slider_type, 2);
                assert!(!changeable);
                assert_eq!(*value, Some(50));
            }
            _ => panic!("Expected Slider"),
        }
    }

    #[test]
    fn test_load_graph_positive_type() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.graph.push(json_skin::Graph {
            id: Some("gr1".to_string()),
            graph_type: 0,
            angle: 1,
            value: Some(300),
            ..Default::default()
        });
        let dst = make_dst("gr1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        match &result.unwrap().object_type {
            SkinObjectType::Graph {
                graph_type,
                angle,
                value,
                ..
            } => {
                assert_eq!(*graph_type, 0);
                assert_eq!(*angle, 1);
                assert_eq!(*value, Some(300));
            }
            _ => panic!("Expected Graph"),
        }
    }

    #[test]
    fn test_load_graph_negative_type_distribution() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.graph.push(json_skin::Graph {
            id: Some("dgr1".to_string()),
            graph_type: -1,
            ..Default::default()
        });
        let dst = make_dst("dgr1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        match &result.unwrap().object_type {
            SkinObjectType::DistributionGraph { graph_type, .. } => {
                assert_eq!(*graph_type, -1);
            }
            _ => panic!("Expected DistributionGraph"),
        }
    }

    #[test]
    fn test_load_gauge_graph() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.gaugegraph.push(json_skin::GaugeGraph {
            id: Some("gg1".to_string()),
            color: Some(vec!["ff0000".to_string(); 24]),
            ..Default::default()
        });
        let dst = make_dst("gg1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        match &result.unwrap().object_type {
            SkinObjectType::GaugeGraph { color, .. } => {
                assert!(color.is_some());
                assert_eq!(color.as_ref().unwrap().len(), 24);
            }
            _ => panic!("Expected GaugeGraph"),
        }
    }

    #[test]
    fn test_load_judge_graph() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.judgegraph.push(json_skin::JudgeGraph {
            id: Some("jg1".to_string()),
            graph_type: 1,
            delay: 500,
            ..Default::default()
        });
        let dst = make_dst("jg1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        match &result.unwrap().object_type {
            SkinObjectType::JudgeGraph {
                graph_type, delay, ..
            } => {
                assert_eq!(*graph_type, 1);
                assert_eq!(*delay, 500);
            }
            _ => panic!("Expected JudgeGraph"),
        }
    }

    #[test]
    fn test_load_bpm_graph() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.bpmgraph.push(json_skin::BPMGraph {
            id: Some("bg1".to_string()),
            delay: 100,
            line_width: 3,
            ..Default::default()
        });
        let dst = make_dst("bg1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        match &result.unwrap().object_type {
            SkinObjectType::BpmGraph {
                delay, line_width, ..
            } => {
                assert_eq!(*delay, 100);
                assert_eq!(*line_width, 3);
            }
            _ => panic!("Expected BpmGraph"),
        }
    }

    #[test]
    fn test_load_hit_error_visualizer() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.hiterrorvisualizer.push(json_skin::HitErrorVisualizer {
            id: Some("hev1".to_string()),
            ..Default::default()
        });
        let dst = make_dst("hev1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        matches!(
            result.unwrap().object_type,
            SkinObjectType::HitErrorVisualizer { .. }
        );
    }

    #[test]
    fn test_load_timing_visualizer() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.timingvisualizer.push(json_skin::TimingVisualizer {
            id: Some("tv1".to_string()),
            ..Default::default()
        });
        let dst = make_dst("tv1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        matches!(
            result.unwrap().object_type,
            SkinObjectType::TimingVisualizer { .. }
        );
    }

    #[test]
    fn test_load_timing_distribution_graph() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.timingdistributiongraph
            .push(json_skin::TimingDistributionGraph {
                id: Some("td1".to_string()),
                ..Default::default()
            });
        let dst = make_dst("td1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        matches!(
            result.unwrap().object_type,
            SkinObjectType::TimingDistributionGraph { .. }
        );
    }

    #[test]
    fn test_load_gauge() {
        let mut loader = make_loader();
        let skin = make_skin();
        let mut sk = make_sk();
        sk.gauge = Some(json_skin::Gauge {
            id: Some("gauge1".to_string()),
            nodes: vec!["n1".to_string(), "n2".to_string()],
            parts: 50,
            gauge_type: 0,
            range: 3,
            cycle: 33,
            starttime: 0,
            endtime: 500,
        });
        let dst = make_dst("gauge1");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_some());
        match &result.unwrap().object_type {
            SkinObjectType::Gauge {
                nodes,
                parts,
                gauge_type,
                range,
                cycle,
                starttime,
                endtime,
            } => {
                assert_eq!(nodes.len(), 2);
                assert_eq!(*parts, 50);
                assert_eq!(*gauge_type, 0);
                assert_eq!(*range, 3);
                assert_eq!(*cycle, 33);
                assert_eq!(*starttime, 0);
                assert_eq!(*endtime, 500);
            }
            _ => panic!("Expected Gauge"),
        }
    }

    #[test]
    fn test_load_no_match_returns_none() {
        let mut loader = make_loader();
        let skin = make_skin();
        let sk = make_sk();
        let dst = make_dst("nonexistent");
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_no_id_returns_none() {
        let mut loader = make_loader();
        let skin = make_skin();
        let sk = make_sk();
        let dst = json_skin::Destination::default(); // id is None
        let p = std::path::Path::new("/fake/skin.json");

        let result = load_base_skin_object(&mut loader, &skin, &sk, &dst, p);
        assert!(result.is_none());
    }
}
