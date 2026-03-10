use std::collections::HashMap;
use std::path::Path;

use log::{debug, warn};

use crate::graphs::skin_bpm_graph::SkinBPMGraph;
use crate::graphs::skin_graph::SkinGraph;
use crate::graphs::skin_hit_error_visualizer::SkinHitErrorVisualizer;
use crate::graphs::skin_note_distribution_graph::SkinNoteDistributionGraph;
use crate::graphs::skin_timing_distribution_graph::SkinTimingDistributionGraph;
use crate::graphs::skin_timing_visualizer::SkinTimingVisualizer;
use crate::json::json_skin_loader::{SkinNumberOffset, SkinObjectType, SourceData};
use crate::json::json_skin_object_loader::source_image;
use crate::objects::skin_bga_object::SkinBgaObject;
use crate::objects::skin_gauge::SkinGauge;
use crate::objects::skin_gauge_graph_object::SkinGaugeGraphObject;
use crate::objects::skin_hidden::SkinHidden;
use crate::objects::skin_image::SkinImage;
use crate::objects::skin_judge_object::SkinJudgeObject;
use crate::objects::skin_note_object::SkinNoteObject;
use crate::objects::skin_number::{NumberDisplayConfig, SkinNumber};
use crate::objects::skin_slider::SkinSlider;
use crate::property::string_property_factory;
use crate::stubs::{SkinOffset, TextureRegion};
use crate::text::skin_text_font::SkinTextFont;
use crate::types::skin::SkinObject;
use crate::types::skin_bar_object::SkinBarObject;

use super::texture_resolution::{get_texture_for_src, resolve_image_set};

pub(super) fn set_click_event_from_type(obj: &mut SkinObject, obj_type: &SkinObjectType) {
    match obj_type {
        SkinObjectType::Image {
            act: Some(act_id),
            click,
            ..
        } => {
            obj.data_mut().set_clickevent_by_id(*act_id);
            obj.data_mut().clickevent_type = *click;
        }
        SkinObjectType::ImageSet {
            act: Some(act_id),
            click,
            ..
        } => {
            obj.data_mut().set_clickevent_by_id(*act_id);
            obj.data_mut().clickevent_type = *click;
        }
        _ => {}
    }
}

/// Converts a SkinObjectType into a SkinObject.
pub(super) fn convert_skin_object(
    obj_type: &SkinObjectType,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_y: f32,
) -> Option<SkinObject> {
    match obj_type {
        SkinObjectType::Unknown => None,

        SkinObjectType::ImageById(id) => Some(SkinObject::Image(SkinImage::new_with_image_id(*id))),

        SkinObjectType::Image {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            len,
            ref_id,
            act: _,
            click: _,
            is_movie,
        } => convert_image(
            src, *x, *y, *w, *h, *divx, *divy, *timer, *cycle, *len, *ref_id, *is_movie,
            source_map, skin_path, usecim,
        ),

        SkinObjectType::ImageSet {
            images,
            ref_id,
            value,
            act: _,
            click: _,
        } => convert_image_set(images, *ref_id, *value),

        SkinObjectType::ResolvedImageSet { images, ref_id } => {
            resolve_image_set(images, *ref_id, source_map, skin_path, usecim)
        }

        SkinObjectType::Number {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            digit,
            padding,
            zeropadding,
            space,
            ref_id,
            value,
            align,
            offsets,
        } => convert_number(
            src,
            *x,
            *y,
            *w,
            *h,
            *divx,
            *divy,
            *timer,
            *cycle,
            *digit,
            *padding,
            *zeropadding,
            *space,
            *ref_id,
            *value,
            *align,
            offsets,
            source_map,
            skin_path,
            usecim,
        ),

        SkinObjectType::Float {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            iketa,
            fketa,
            is_signvisible,
            align,
            zeropadding,
            space,
            ref_id,
            value,
            gain,
            offsets: _,
        } => convert_float(
            src,
            *x,
            *y,
            *w,
            *h,
            *divx,
            *divy,
            *timer,
            *cycle,
            *iketa,
            *fketa,
            *is_signvisible,
            *align,
            *zeropadding,
            *space,
            *ref_id,
            *value,
            *gain,
            source_map,
            skin_path,
            usecim,
        ),

        SkinObjectType::Text {
            font,
            size,
            align: _,
            ref_id,
            value,
            constant_text: _,
            wrapping: _,
            overflow: _,
            outline_color: _,
            outline_width: _,
            shadow_color: _,
            shadow_offset_x: _,
            shadow_offset_y: _,
            shadow_smoothness: _,
        } => convert_text(font, *size, *ref_id, *value),

        SkinObjectType::Slider {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            angle,
            range,
            slider_type,
            changeable,
            value,
            event: _,
            is_ref_num: _,
            min: _,
            max: _,
        } => convert_slider(
            src,
            *x,
            *y,
            *w,
            *h,
            *divx,
            *divy,
            *timer,
            *cycle,
            *angle,
            *range,
            *slider_type,
            *changeable,
            *value,
            source_map,
            skin_path,
            usecim,
        ),

        SkinObjectType::Graph {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            angle,
            graph_type,
            value,
            is_ref_num,
            min,
            max,
        } => convert_graph(
            src,
            *x,
            *y,
            *w,
            *h,
            *divx,
            *divy,
            *timer,
            *cycle,
            *angle,
            *graph_type,
            *value,
            *is_ref_num,
            *min,
            *max,
            source_map,
            skin_path,
            usecim,
        ),

        SkinObjectType::DistributionGraph { graph_type, .. } => {
            // SkinNoteDistributionGraph with TYPE_NORMAL
            let graph = SkinNoteDistributionGraph::new(*graph_type, 0, 0, 0, 0, 0);
            Some(SkinObject::NoteDistributionGraph(graph))
        }

        SkinObjectType::GaugeGraph {
            color,
            assist_clear_bg_color,
            assist_and_easy_fail_bg_color,
            groove_fail_bg_color,
            groove_clear_and_hard_bg_color,
            ex_hard_bg_color,
            hazard_bg_color,
            assist_clear_line_color,
            assist_and_easy_fail_line_color,
            groove_fail_line_color,
            groove_clear_and_hard_line_color,
            ex_hard_line_color,
            hazard_line_color,
            borderline_color,
            border_color,
        } => convert_gauge_graph(
            color,
            assist_clear_bg_color,
            assist_and_easy_fail_bg_color,
            groove_fail_bg_color,
            groove_clear_and_hard_bg_color,
            ex_hard_bg_color,
            hazard_bg_color,
            assist_clear_line_color,
            assist_and_easy_fail_line_color,
            groove_fail_line_color,
            groove_clear_and_hard_line_color,
            ex_hard_line_color,
            hazard_line_color,
            borderline_color,
            border_color,
        ),

        SkinObjectType::JudgeGraph {
            graph_type,
            delay,
            back_tex_off,
            order_reverse,
            no_gap,
            no_gap_x,
        } => convert_judge_graph(
            *graph_type,
            *delay,
            *back_tex_off,
            *order_reverse,
            *no_gap,
            *no_gap_x,
        ),

        SkinObjectType::BpmGraph {
            delay,
            line_width,
            main_bpm_color,
            min_bpm_color,
            max_bpm_color,
            other_bpm_color,
            stop_line_color,
            transition_line_color,
        } => convert_bpm_graph(
            *delay,
            *line_width,
            main_bpm_color,
            min_bpm_color,
            max_bpm_color,
            other_bpm_color,
            stop_line_color,
            transition_line_color,
        ),

        SkinObjectType::HitErrorVisualizer {
            width,
            judge_width_millis,
            line_width,
            color_mode,
            hiterror_mode,
            ema_mode,
            line_color,
            center_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            ema_color,
            alpha,
            window_length,
            transparent,
            draw_decay,
        } => convert_hit_error_visualizer(
            *width,
            *judge_width_millis,
            *line_width,
            *color_mode,
            *hiterror_mode,
            *ema_mode,
            line_color,
            center_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            ema_color,
            *alpha,
            *window_length,
            *transparent,
            *draw_decay,
        ),

        SkinObjectType::TimingVisualizer {
            width,
            judge_width_millis,
            line_width,
            line_color,
            center_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            transparent,
            draw_decay,
        } => convert_timing_visualizer(
            *width,
            *judge_width_millis,
            *line_width,
            line_color,
            center_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            *transparent,
            *draw_decay,
        ),

        SkinObjectType::TimingDistributionGraph {
            width,
            line_width,
            graph_color,
            average_color,
            dev_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            draw_average,
            draw_dev,
        } => convert_timing_distribution_graph(
            *width,
            *line_width,
            graph_color,
            average_color,
            dev_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            *draw_average,
            *draw_dev,
        ),

        SkinObjectType::Gauge {
            nodes,
            parts,
            gauge_type,
            range,
            cycle,
            starttime,
            endtime,
        } => convert_gauge(
            nodes,
            *parts,
            *gauge_type,
            *range,
            *cycle,
            *starttime,
            *endtime,
            source_map,
            skin_path,
            usecim,
        ),

        SkinObjectType::Note => {
            // Default lane count; lanes are configured later via set_lane_region
            let note = SkinNoteObject::new(0);
            Some(SkinObject::Note(note))
        }
        SkinObjectType::HiddenCover {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            disapear_line,
            is_disapear_line_link_lift,
        } => convert_hidden_cover(
            src,
            *x,
            *y,
            *w,
            *h,
            *divx,
            *divy,
            *timer,
            *cycle,
            *disapear_line,
            *is_disapear_line_link_lift,
            scale_y,
            source_map,
            skin_path,
            usecim,
        ),

        SkinObjectType::LiftCover {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            disapear_line,
            is_disapear_line_link_lift,
        } => convert_lift_cover(
            src,
            *x,
            *y,
            *w,
            *h,
            *divx,
            *divy,
            *timer,
            *cycle,
            *disapear_line,
            *is_disapear_line_link_lift,
            scale_y,
            source_map,
            skin_path,
            usecim,
        ),

        SkinObjectType::Bga { bga_expand } => {
            let bga = SkinBgaObject::new(*bga_expand);
            Some(SkinObject::Bga(bga))
        }
        SkinObjectType::Judge { index, shift } => {
            let judge = SkinJudgeObject::new(*index, *shift);
            Some(SkinObject::Judge(judge))
        }
        SkinObjectType::PmChara {
            src,
            color,
            chara_type,
            side: _,
        } => convert_pm_chara(src, *color, *chara_type),

        SkinObjectType::SongList { center, .. } => {
            let bar = SkinBarObject::new(*center);
            Some(SkinObject::Bar(bar))
        }
        SkinObjectType::SearchTextRegion { x, y, w, h } => {
            // SearchTextRegion: In Java, this sets a Rectangle on MusicSelectSkin.
            // It's not a SkinObject itself but a property of the select skin.
            // Since we don't have MusicSelectSkin in the converter, we log and skip.
            debug!(
                "SearchTextRegion: ({}, {}, {}, {}) -- stored as skin property, not a SkinObject",
                x, y, w, h
            );
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn convert_image(
    src: &Option<String>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    divx: i32,
    divy: i32,
    timer: Option<i32>,
    cycle: i32,
    len: i32,
    ref_id: i32,
    is_movie: bool,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
) -> Option<SkinObject> {
    if is_movie {
        // Movie sources: create SkinImage with SkinSourceMovie
        let movie_path = src.as_deref().unwrap_or("");
        let movie_source = crate::skin_source_movie::SkinSourceMovie::new(movie_path);
        return Some(SkinObject::Image(SkinImage::new_with_movie(movie_source)));
    }

    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
    let srcimg = source_image(&tex, x, y, w, h, divx, divy);

    if len > 1 {
        // Multiple reference images
        let imgs_per_ref = srcimg.len() / (len as usize);
        let mut tr: Vec<Vec<TextureRegion>> = Vec::with_capacity(len as usize);
        for i in 0..(len as usize) {
            let mut row: Vec<TextureRegion> = Vec::with_capacity(imgs_per_ref);
            for j in 0..imgs_per_ref {
                row.push(srcimg[i * imgs_per_ref + j].clone());
            }
            tr.push(row);
        }
        let timer_val = timer.unwrap_or(0);
        Some(SkinObject::Image(SkinImage::new_with_int_timer_ref_id(
            tr, timer_val, cycle, ref_id,
        )))
    } else {
        let timer_val = timer.unwrap_or(0);
        Some(SkinObject::Image(SkinImage::new_with_int_timer(
            srcimg, timer_val, cycle,
        )))
    }
}

fn convert_image_set(images: &[String], ref_id: i32, value: Option<i32>) -> Option<SkinObject> {
    // ImageSet: each image ID in `images` references an entry in sk.image[].
    // The converter doesn't have access to sk, so we create a SkinImage
    // bound to the value/ref property. The actual image sources will be empty
    // (rendering deferred until sk is threaded through the converter).
    if images.is_empty() {
        warn!("ImageSet has no image entries");
        return None;
    }
    let binding_id = value.unwrap_or(ref_id);
    debug!(
        "ImageSet: creating placeholder with {} image refs, binding={}",
        images.len(),
        binding_id
    );
    Some(SkinObject::Image(SkinImage::new_with_image_id(binding_id)))
}

#[allow(clippy::too_many_arguments)]
fn convert_number(
    src: &Option<String>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    divx: i32,
    divy: i32,
    timer: Option<i32>,
    cycle: i32,
    digit: i32,
    padding: i32,
    zeropadding: i32,
    space: i32,
    ref_id: i32,
    value: Option<i32>,
    align: i32,
    offsets: &Option<Vec<SkinNumberOffset>>,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
) -> Option<SkinObject> {
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
    let images = source_image(&tex, x, y, w, h, divx, divy);
    let timer_val = timer.unwrap_or(0);

    let num = if images.len().is_multiple_of(24) {
        // +-12 digit images
        let set_count = images.len() / 24;
        let mut pn: Vec<Vec<TextureRegion>> = Vec::with_capacity(set_count);
        let mut mn: Vec<Vec<TextureRegion>> = Vec::with_capacity(set_count);
        for j in 0..set_count {
            let mut p_row = Vec::with_capacity(12);
            let mut m_row = Vec::with_capacity(12);
            for i in 0..12 {
                p_row.push(images[j * 24 + i].clone());
                m_row.push(images[j * 24 + i + 12].clone());
            }
            pn.push(p_row);
            mn.push(m_row);
        }
        let config = NumberDisplayConfig {
            keta: digit,
            zeropadding,
            space,
            align,
        };
        if let Some(val) = value {
            SkinNumber::new_with_int_timer(pn, Some(mn), timer_val, cycle, config, val)
        } else {
            SkinNumber::new_with_int_timer(pn, Some(mn), timer_val, cycle, config, ref_id)
        }
    } else {
        // 10 or 11 digit images
        let d = if images.len().is_multiple_of(10) {
            10
        } else {
            11
        };
        let set_count = images.len() / d;
        let mut nimages: Vec<Vec<TextureRegion>> = Vec::with_capacity(set_count);
        for j in 0..set_count {
            let mut row = Vec::with_capacity(d);
            for i in 0..d {
                row.push(images[j * d + i].clone());
            }
            nimages.push(row);
        }
        let actual_padding = if d > 10 { 2 } else { padding };
        let config = NumberDisplayConfig {
            keta: digit,
            zeropadding: actual_padding,
            space,
            align,
        };
        if let Some(val) = value {
            SkinNumber::new_with_int_timer(nimages, None, timer_val, cycle, config, val)
        } else {
            SkinNumber::new_with_int_timer(nimages, None, timer_val, cycle, config, ref_id)
        }
    };

    // Apply per-digit offsets if present
    let mut num = num;
    if let Some(ofs) = offsets {
        let skin_offsets: Vec<SkinOffset> = ofs
            .iter()
            .map(|o| SkinOffset {
                x: o.x as f32,
                y: o.y as f32,
                w: o.w as f32,
                h: o.h as f32,
                r: 0.0,
                a: 0.0,
            })
            .collect();
        num.set_offsets(skin_offsets);
    }

    Some(SkinObject::Number(num))
}

#[allow(clippy::too_many_arguments)]
fn convert_float(
    src: &Option<String>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    divx: i32,
    divy: i32,
    timer: Option<i32>,
    cycle: i32,
    iketa: i32,
    fketa: i32,
    is_signvisible: bool,
    align: i32,
    zeropadding: i32,
    space: i32,
    ref_id: i32,
    value: Option<i32>,
    gain: f32,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
) -> Option<SkinObject> {
    // SkinFloat construction requires complex image splitting.
    // For now, create a stub that won't crash but won't render either.
    warn!("Float conversion creates placeholder (full SkinFloat image splitting deferred)");
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim);
    tex.as_ref()?;
    let tex = tex.expect("tex");
    let images = source_image(&tex, x, y, w, h, divx, divy);
    let timer_val = timer.unwrap_or(0);

    // Create as SkinFloat using the available constructor
    let image_opts: Vec<Vec<Option<TextureRegion>>> = if images.len().is_multiple_of(12) {
        let set_count = images.len() / 12;
        let mut result = Vec::with_capacity(set_count);
        for j in 0..set_count {
            let mut row = Vec::with_capacity(12);
            for i in 0..12 {
                row.push(Some(images[j * 12 + i].clone()));
            }
            result.push(row);
        }
        result
    } else {
        vec![images.into_iter().map(Some).collect()]
    };

    // Use `value` if present (explicit ID), otherwise fall back to `ref_id`
    let prop_id = value.unwrap_or(ref_id);
    let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
        image_opts,
        timer_val,
        cycle,
        crate::skin_float::FloatDisplayConfig {
            iketa,
            fketa,
            is_sign_visible: is_signvisible,
            align,
            zeropadding,
            space,
            gain,
        },
        prop_id,
    );
    Some(SkinObject::Float(sf))
}

fn convert_text(
    font: &Option<String>,
    size: i32,
    ref_id: i32,
    value: Option<i32>,
) -> Option<SkinObject> {
    if let Some(font_path) = font {
        let text_id = value.unwrap_or(ref_id);
        let property = if text_id >= 0 {
            string_property_factory::string_property_by_id(text_id)
        } else {
            None
        };
        let stf = SkinTextFont::new_with_property(font_path, 0, size, 0, property);
        Some(SkinObject::TextFont(stf))
    } else {
        warn!("Text object without font path, skipping");
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn convert_slider(
    src: &Option<String>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    divx: i32,
    divy: i32,
    timer: Option<i32>,
    cycle: i32,
    angle: i32,
    range: i32,
    slider_type: i32,
    changeable: bool,
    value: Option<i32>,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
) -> Option<SkinObject> {
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
    let images = source_image(&tex, x, y, w, h, divx, divy);
    let timer_val = timer.unwrap_or(0);
    let type_id = value.unwrap_or(slider_type);
    let slider =
        SkinSlider::new_with_int_timer(images, timer_val, cycle, angle, range, type_id, changeable);
    Some(SkinObject::Slider(slider))
}

#[allow(clippy::too_many_arguments)]
fn convert_graph(
    src: &Option<String>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    divx: i32,
    divy: i32,
    timer: Option<i32>,
    cycle: i32,
    angle: i32,
    graph_type: i32,
    value: Option<i32>,
    is_ref_num: bool,
    min: i32,
    max: i32,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
) -> Option<SkinObject> {
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
    let images = source_image(&tex, x, y, w, h, divx, divy);
    let timer_val = timer.unwrap_or(0);
    if let Some(val) = value {
        Some(SkinObject::Graph(SkinGraph::new_with_int_timer(
            images, timer_val, cycle, val, angle,
        )))
    } else if is_ref_num {
        Some(SkinObject::Graph(SkinGraph::new_with_int_timer_minmax(
            images, timer_val, cycle, graph_type, min, max, angle,
        )))
    } else {
        Some(SkinObject::Graph(SkinGraph::new_with_int_timer(
            images, timer_val, cycle, graph_type, angle,
        )))
    }
}

#[allow(clippy::too_many_arguments)]
fn convert_gauge_graph(
    color: &Option<Vec<String>>,
    assist_clear_bg_color: &str,
    assist_and_easy_fail_bg_color: &str,
    groove_fail_bg_color: &str,
    groove_clear_and_hard_bg_color: &str,
    ex_hard_bg_color: &str,
    hazard_bg_color: &str,
    assist_clear_line_color: &str,
    assist_and_easy_fail_line_color: &str,
    groove_fail_line_color: &str,
    groove_clear_and_hard_line_color: &str,
    ex_hard_line_color: &str,
    hazard_line_color: &str,
    borderline_color: &str,
    border_color: &str,
) -> Option<SkinObject> {
    let gg = if let Some(colors) = color {
        SkinGaugeGraphObject::new_from_colors(colors)
    } else {
        SkinGaugeGraphObject::new_from_color_strings(
            &crate::objects::skin_gauge_graph_object::GaugeGraphColorStrings {
                assist_clear_bg: assist_clear_bg_color,
                assist_easy_fail_bg: assist_and_easy_fail_bg_color,
                groove_fail_bg: groove_fail_bg_color,
                groove_clear_hard_bg: groove_clear_and_hard_bg_color,
                ex_hard_bg: ex_hard_bg_color,
                hazard_bg: hazard_bg_color,
                assist_clear_line: assist_clear_line_color,
                assist_easy_fail_line: assist_and_easy_fail_line_color,
                groove_fail_line: groove_fail_line_color,
                groove_clear_hard_line: groove_clear_and_hard_line_color,
                ex_hard_line: ex_hard_line_color,
                hazard_line: hazard_line_color,
                borderline_color,
                border_color,
            },
        )
    };
    Some(SkinObject::GaugeGraph(gg))
}

fn convert_judge_graph(
    graph_type: i32,
    delay: i32,
    back_tex_off: i32,
    order_reverse: i32,
    no_gap: i32,
    no_gap_x: i32,
) -> Option<SkinObject> {
    let graph = SkinNoteDistributionGraph::new(
        graph_type,
        delay,
        back_tex_off,
        order_reverse,
        no_gap,
        no_gap_x,
    );
    Some(SkinObject::NoteDistributionGraph(graph))
}

#[allow(clippy::too_many_arguments)]
fn convert_bpm_graph(
    delay: i32,
    line_width: i32,
    main_bpm_color: &str,
    min_bpm_color: &str,
    max_bpm_color: &str,
    other_bpm_color: &str,
    stop_line_color: &str,
    transition_line_color: &str,
) -> Option<SkinObject> {
    let graph = SkinBPMGraph::new(crate::skin_bpm_graph::BpmGraphConfig {
        delay,
        line_width,
        main_bpm_color,
        min_bpm_color,
        max_bpm_color,
        other_bpm_color,
        stop_line_color,
        transition_line_color,
    });
    Some(SkinObject::BpmGraph(graph))
}

#[allow(clippy::too_many_arguments)]
fn convert_hit_error_visualizer(
    width: i32,
    judge_width_millis: i32,
    line_width: i32,
    color_mode: i32,
    hiterror_mode: i32,
    ema_mode: i32,
    line_color: &str,
    center_color: &str,
    pg_color: &str,
    gr_color: &str,
    gd_color: &str,
    bd_color: &str,
    pr_color: &str,
    ema_color: &str,
    alpha: f32,
    window_length: i32,
    transparent: i32,
    draw_decay: i32,
) -> Option<SkinObject> {
    let viz =
        SkinHitErrorVisualizer::new(crate::skin_hit_error_visualizer::HitErrorVisualizerConfig {
            width,
            judge_width_millis,
            line_width,
            color_mode,
            hiterror_mode,
            ema_mode,
            line_color,
            center_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            ema_color,
            alpha,
            window_length,
            transparent,
            draw_decay,
        });
    Some(SkinObject::HitErrorVisualizer(viz))
}

#[allow(clippy::too_many_arguments)]
fn convert_timing_visualizer(
    width: i32,
    judge_width_millis: i32,
    line_width: i32,
    line_color: &str,
    center_color: &str,
    pg_color: &str,
    gr_color: &str,
    gd_color: &str,
    bd_color: &str,
    pr_color: &str,
    transparent: i32,
    draw_decay: i32,
) -> Option<SkinObject> {
    let viz = SkinTimingVisualizer::new(crate::skin_timing_visualizer::TimingVisualizerConfig {
        width,
        judge_width_millis,
        line_width,
        line_color,
        center_color,
        pg_color,
        gr_color,
        gd_color,
        bd_color,
        pr_color,
        transparent,
        draw_decay,
    });
    Some(SkinObject::TimingVisualizer(viz))
}

#[allow(clippy::too_many_arguments)]
fn convert_timing_distribution_graph(
    width: i32,
    line_width: i32,
    graph_color: &str,
    average_color: &str,
    dev_color: &str,
    pg_color: &str,
    gr_color: &str,
    gd_color: &str,
    bd_color: &str,
    pr_color: &str,
    draw_average: i32,
    draw_dev: i32,
) -> Option<SkinObject> {
    let graph = SkinTimingDistributionGraph::new(
        crate::skin_timing_distribution_graph::TimingDistributionGraphConfig {
            width,
            line_width,
            graph_color,
            average_color,
            dev_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            draw_average,
            draw_dev,
        },
    );
    Some(SkinObject::TimingDistributionGraph(graph))
}

#[allow(clippy::too_many_arguments)]
fn convert_gauge(
    nodes: &[String],
    parts: i32,
    gauge_type: i32,
    range: i32,
    cycle: i32,
    starttime: i32,
    endtime: i32,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
) -> Option<SkinObject> {
    // Resolve gauge node IDs to TextureRegion images via source_map.
    // Each node string references a source entry; resolve to a full-texture TextureRegion.
    // Java indexmap logic maps 4/8/12 node configs to 36 gauge slots.
    // With 36 nodes, each maps 1:1 to a slot.
    let mut resolved_nodes: Vec<Option<TextureRegion>> = Vec::with_capacity(nodes.len());
    for node_id in nodes {
        let tex = get_texture_for_src(Some(node_id), source_map, skin_path, usecim);
        resolved_nodes.push(tex.map(TextureRegion::from_texture));
    }

    // Build gauge_images: 36 slots, each containing a single-element vec
    // (no animation frames from JSON sources).
    let gauge_images: Vec<Vec<Option<TextureRegion>>> = if resolved_nodes.len() == 36 {
        // 1:1 mapping
        resolved_nodes.into_iter().map(|tr| vec![tr]).collect()
    } else if !resolved_nodes.is_empty() {
        // Expand fewer nodes to 36 slots by repeating.
        // The pattern repeats: each node set represents one gauge visual state.
        let mut images = Vec::with_capacity(36);
        for i in 0..36 {
            let idx = i % resolved_nodes.len();
            images.push(vec![resolved_nodes[idx].clone()]);
        }
        images
    } else {
        Vec::new()
    };
    debug!(
        "Gauge: creating with {} nodes ({} resolved images), parts={}, type={}",
        nodes.len(),
        gauge_images
            .iter()
            .filter(|v| v.iter().any(|t| t.is_some()))
            .count(),
        parts,
        gauge_type
    );
    let mut gauge = SkinGauge::new(
        gauge_images,
        0,
        cycle,
        parts,
        gauge_type,
        range,
        cycle as i64,
    );
    gauge.starttime = starttime;
    gauge.endtime = endtime;
    Some(SkinObject::Gauge(gauge))
}

#[allow(clippy::too_many_arguments)]
fn convert_hidden_cover(
    src: &Option<String>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    divx: i32,
    divy: i32,
    timer: Option<i32>,
    cycle: i32,
    disapear_line: i32,
    is_disapear_line_link_lift: bool,
    scale_y: f32,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
) -> Option<SkinObject> {
    // HiddenCover: create SkinHidden with texture and disappear line.
    // Java: new SkinHidden(getSourceImage(tex,...), timer, cycle)
    //       setDisapearLine(disapearLine * scaleY)
    //       offsets += [OFFSET_LIFT, OFFSET_HIDDEN_COVER]
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim);
    if let Some(tex) = tex {
        let srcimg = source_image(&tex, x, y, w, h, divx, divy);
        let timer_val = timer.unwrap_or(0);
        let mut hidden = SkinHidden::new_with_int_timer(srcimg, timer_val, cycle);
        hidden.set_disapear_line(disapear_line as f32 * scale_y);
        hidden.is_disapear_line_link_lift = is_disapear_line_link_lift;
        Some(SkinObject::Hidden(hidden))
    } else {
        warn!("HiddenCover: texture source {:?} not found", src);
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn convert_lift_cover(
    src: &Option<String>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    divx: i32,
    divy: i32,
    timer: Option<i32>,
    cycle: i32,
    disapear_line: i32,
    is_disapear_line_link_lift: bool,
    scale_y: f32,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
) -> Option<SkinObject> {
    // LiftCover: same as HiddenCover but offset list only adds OFFSET_LIFT.
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim);
    if let Some(tex) = tex {
        let srcimg = source_image(&tex, x, y, w, h, divx, divy);
        let timer_val = timer.unwrap_or(0);
        let mut hidden = SkinHidden::new_with_int_timer(srcimg, timer_val, cycle);
        hidden.set_disapear_line(disapear_line as f32 * scale_y);
        hidden.is_disapear_line_link_lift = is_disapear_line_link_lift;
        Some(SkinObject::Hidden(hidden))
    } else {
        warn!("LiftCover: texture source {:?} not found", src);
        None
    }
}

fn convert_pm_chara(src: &Option<String>, color: i32, chara_type: i32) -> Option<SkinObject> {
    // PmChara: Pomyu character rendering.
    // In Java, this uses PomyuCharaLoader to load character sprite sheets.
    // The loader needs file system access via getSrcIdPath and dst coordinates.
    // We create a placeholder SkinImage since PomyuCharaLoader produces SkinImage.
    debug!(
        "PmChara: type={}, color={}, src={:?} (image loading deferred)",
        chara_type, color, src
    );
    Some(SkinObject::Image(SkinImage::new_with_image_id(0)))
}

/// Apply destination data from loader DestinationData to a runtime SkinObjectData.
/// Sets the initial position/size from the destination keyframes.
pub(super) fn apply_destinations(
    data: &mut crate::skin_object::SkinObjectData,
    destinations: &[crate::json::json_skin_loader::DestinationData],
) {
    for dst in destinations {
        data.set_destination_with_int_timer_ops(
            &crate::skin_object::DestinationParams {
                time: dst.time as i64,
                x: dst.x as f32,
                y: dst.y as f32,
                w: dst.w as f32,
                h: dst.h as f32,
                acc: dst.acc,
                a: dst.a,
                r: dst.r,
                g: dst.g,
                b: dst.b,
                blend: dst.blend,
                filter: dst.filter,
                angle: dst.angle,
                center: dst.center,
                loop_val: dst.loop_val,
            },
            dst.timer.unwrap_or(0),
            &dst.op,
        );
    }
}
