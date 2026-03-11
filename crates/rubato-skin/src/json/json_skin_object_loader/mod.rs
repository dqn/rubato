// Mechanical translation of JsonSkinObjectLoader.java
// Object loader base class (abstract in Java, trait in Rust)

mod utilities;

#[cfg(test)]
mod tests;

use std::path::Path;

use crate::json::json_skin;
use crate::json::json_skin_loader::{
    JSONSkinLoader, SkinData, SkinObjectData, SkinObjectType, SourceDataType,
};

use utilities::map_number_offsets;
pub use utilities::{
    create_text, note_texture, parse_hex_color, set_destination_on_object, source_image,
    src_id_path, texture,
};

/// Corresponds to JsonSkinObjectLoader<S extends Skin>
/// In Java this is an abstract class parameterized by skin type.
/// In Rust we use a trait.
pub trait JsonSkinObjectLoader {
    fn skin(&self, header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData;

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
///
/// Dispatches to type-specific loader functions for each skin object category.
pub fn load_base_skin_object(
    loader: &mut JSONSkinLoader,
    _skin: &SkinData,
    sk: &json_skin::Skin,
    dst: &json_skin::Destination,
    p: &Path,
) -> Option<SkinObjectData> {
    let dst_id = dst.id.as_deref()?;

    None.or_else(|| load_image_object(loader, sk, dst_id, p))
        .or_else(|| load_imageset_object(sk, dst_id))
        .or_else(|| load_value_object(sk, dst_id))
        .or_else(|| load_floatvalue_object(sk, dst_id))
        .or_else(|| load_text_object(sk, dst_id))
        .or_else(|| load_slider_object(sk, dst_id))
        .or_else(|| load_graph_object(sk, dst_id))
        .or_else(|| load_gaugegraph_object(sk, dst_id))
        .or_else(|| load_judgegraph_object(sk, dst_id))
        .or_else(|| load_bpmgraph_object(sk, dst_id))
        .or_else(|| load_hiterror_object(sk, dst_id))
        .or_else(|| load_timingvisualizer_object(sk, dst_id))
        .or_else(|| load_timingdist_object(sk, dst_id))
        .or_else(|| load_gauge_object(sk, dst_id))
}

fn load_image_object(
    loader: &mut JSONSkinLoader,
    sk: &json_skin::Skin,
    dst_id: &str,
    p: &Path,
) -> Option<SkinObjectData> {
    for img in &sk.image {
        if dst_id == img.id.as_deref().unwrap_or("") {
            let data = loader.source(img.src.as_deref().unwrap_or(""), p);
            let is_movie = matches!(&data, Some(SourceDataType::Movie(_)));
            if data.is_some() {
                return Some(SkinObjectData {
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
                });
            }
            return None;
        }
    }
    None
}

fn load_imageset_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for imgs in &sk.imageset {
        if dst_id == imgs.id.as_deref().unwrap_or("") {
            return Some(SkinObjectData {
                name: imgs.id.clone(),
                object_type: SkinObjectType::ImageSet {
                    images: imgs.images.clone(),
                    ref_id: imgs.ref_id,
                    value: imgs.value,
                    act: imgs.act,
                    click: imgs.click,
                },
                ..Default::default()
            });
        }
    }
    None
}

fn load_value_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for value in &sk.value {
        if dst_id == value.id.as_deref().unwrap_or("") {
            return Some(SkinObjectData {
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
                    offsets: map_number_offsets(value.offset.as_ref()),
                },
                ..Default::default()
            });
        }
    }
    None
}

fn load_floatvalue_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for fv in &sk.floatvalue {
        if dst_id == fv.id.as_deref().unwrap_or("") {
            return Some(SkinObjectData {
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
                    offsets: map_number_offsets(fv.offset.as_ref()),
                },
                ..Default::default()
            });
        }
    }
    None
}

fn load_text_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for text in &sk.text {
        if dst_id == text.id.as_deref().unwrap_or("") {
            // Resolve font: JSON skins express fonts by ID (e.g. "0"), not path.
            // Look up the matching sk.font entry and use its path.
            let resolved_font = text.font.as_ref().and_then(|font_str| {
                for f in &sk.font {
                    if f.id.as_deref() == Some(font_str) {
                        return f.path.clone();
                    }
                }
                // Not a font ID reference, treat as direct path
                Some(font_str.clone())
            });
            return Some(SkinObjectData {
                name: text.id.clone(),
                object_type: SkinObjectType::Text {
                    font: resolved_font,
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
    None
}

fn load_slider_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for slider in &sk.slider {
        if dst_id == slider.id.as_deref().unwrap_or("") {
            return Some(SkinObjectData {
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
            });
        }
    }
    None
}

fn load_graph_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for graph in &sk.graph {
        if dst_id == graph.id.as_deref().unwrap_or("") {
            if graph.graph_type < 0 {
                return Some(SkinObjectData {
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
                });
            } else {
                return Some(SkinObjectData {
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
                });
            }
        }
    }
    None
}

fn load_gaugegraph_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for ggraph in &sk.gaugegraph {
        if dst_id == ggraph.id.as_deref().unwrap_or("") {
            return Some(SkinObjectData {
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
            });
        }
    }
    None
}

fn load_judgegraph_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for ggraph in &sk.judgegraph {
        if dst_id == ggraph.id.as_deref().unwrap_or("") {
            return Some(SkinObjectData {
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
            });
        }
    }
    None
}

fn load_bpmgraph_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for ggraph in &sk.bpmgraph {
        if dst_id == ggraph.id.as_deref().unwrap_or("") {
            return Some(SkinObjectData {
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
            });
        }
    }
    None
}

fn load_hiterror_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for hev in &sk.hiterrorvisualizer {
        if dst_id == hev.id.as_deref().unwrap_or("") {
            return Some(SkinObjectData {
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
            });
        }
    }
    None
}

fn load_timingvisualizer_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for tv in &sk.timingvisualizer {
        if dst_id == tv.id.as_deref().unwrap_or("") {
            return Some(SkinObjectData {
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
            });
        }
    }
    None
}

fn load_timingdist_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    for td in &sk.timingdistributiongraph {
        if dst_id == td.id.as_deref().unwrap_or("") {
            return Some(SkinObjectData {
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
            });
        }
    }
    None
}

fn load_gauge_object(sk: &json_skin::Skin, dst_id: &str) -> Option<SkinObjectData> {
    if let Some(ref gauge) = sk.gauge
        && dst_id == gauge.id.as_deref().unwrap_or("")
    {
        return Some(SkinObjectData {
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
        });
    }
    None
}
