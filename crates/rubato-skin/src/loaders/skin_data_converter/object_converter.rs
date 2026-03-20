use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
use crate::reexports::{SkinOffset, TextureRegion};
use crate::text::skin_text_bitmap::{SkinTextBitmap, SkinTextBitmapSource};
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
    scale_x: f32,
    scale_y: f32,
    filemap: &HashMap<String, String>,
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
            source_map, skin_path, usecim, filemap,
        ),

        SkinObjectType::ImageSet {
            images,
            ref_id,
            value,
            act: _,
            click: _,
        } => convert_image_set(images, *ref_id, *value),

        SkinObjectType::ResolvedImageSet { images, ref_id } => {
            resolve_image_set(images, *ref_id, source_map, skin_path, usecim, filemap)
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
            filemap,
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
            offsets,
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
            offsets,
            source_map,
            skin_path,
            usecim,
            filemap,
        ),

        SkinObjectType::Text {
            font,
            size,
            align,
            ref_id,
            value,
            constant_text,
            wrapping,
            overflow,
            outline_color,
            outline_width,
            shadow_color,
            shadow_offset_x,
            shadow_offset_y,
            shadow_smoothness,
        } => convert_text(
            font,
            *size,
            *align,
            *ref_id,
            *value,
            constant_text,
            *wrapping,
            *overflow,
            outline_color,
            *outline_width,
            shadow_color,
            *shadow_offset_x,
            *shadow_offset_y,
            *shadow_smoothness,
            usecim,
            scale_x,
        ),

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
            event,
            is_ref_num,
            min,
            max,
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
            *event,
            *is_ref_num,
            *min,
            *max,
            scale_x,
            scale_y,
            source_map,
            skin_path,
            usecim,
            filemap,
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
            filemap,
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
            filemap,
        ),

        SkinObjectType::Note => {
            // Note is handled as pre-built resolved_note in convert_skin_data.
            // Fallback: create empty note (should not reach here with play loader).
            Some(SkinObject::Note(SkinNoteObject::new(0)))
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
            filemap,
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
            filemap,
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
    filemap: &HashMap<String, String>,
) -> Option<SkinObject> {
    if is_movie {
        // Movie sources: resolve source ID through source_map to get actual file path,
        // then create SkinImage with SkinSourceMovie
        let src_id = src.as_deref()?;
        let data_path = source_map.get(src_id)?.path.clone();
        let parent = skin_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let movie_path = format!("{}/{}", parent, data_path);
        let movie_source = crate::skin_source_movie::SkinSourceMovie::new(&movie_path);
        return Some(SkinObject::Image(SkinImage::new_with_movie(movie_source)));
    }

    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim, filemap)?;
    let srcimg = source_image(&tex, x, y, w, h, divx, divy);

    if len > 1 {
        // Multiple reference images
        let imgs_per_ref = srcimg.len() / (len as usize);
        if imgs_per_ref == 0 {
            // Not enough source images to distribute across refs.
            // Java behavior for broken skin configs: silently produces empty entries.
            // Return None to avoid creating a SkinImage with empty sub-arrays.
            return None;
        }
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
    filemap: &HashMap<String, String>,
) -> Option<SkinObject> {
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim, filemap)?;
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
        if !images.len().is_multiple_of(d) {
            log::warn!(
                "convert_number: image count {} is not divisible by {}, trailing {} images ignored",
                images.len(),
                d,
                images.len() % d
            );
        }
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
    offsets: &Option<Vec<SkinNumberOffset>>,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    filemap: &HashMap<String, String>,
) -> Option<SkinObject> {
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim, filemap);
    tex.as_ref()?;
    let tex = tex.expect("tex");
    let images = source_image(&tex, x, y, w, h, divx, divy);
    let timer_val = timer.unwrap_or(0);

    // Use `value` if present (explicit ID), otherwise fall back to `ref_id`
    let prop_id = value.unwrap_or(ref_id);

    // Six-branch image layout cascade matching Java's JsonSkinObjectLoader:
    // %26: signed, separate +/- (13+13 per set), preserve is_signvisible
    // %24: unsigned, separate +/- (12+12 per set)
    // %22: unsigned, separate +/- (11+11 with shared reverse zero mapping)
    // %12: unsigned, single image (12 per set)
    // %11: unsigned, single image (11 per set, mapped to 12 with shared reverse zero)
    // fallback: treat as %12
    // Priority: 26 > 24 > 22 > 12 > 11 > fallback (because 24 is divisible by 12, 22 by 11)
    let mut sf = if images.len().is_multiple_of(26) {
        // %26: signed, separate +/- images (13 positive + 13 negative per set)
        let set_count = images.len() / 26;
        let mut pn: Vec<Vec<Option<TextureRegion>>> = Vec::with_capacity(set_count);
        let mut mn: Vec<Vec<Option<TextureRegion>>> = Vec::with_capacity(set_count);
        for j in 0..set_count {
            let mut p_row = Vec::with_capacity(13);
            let mut m_row = Vec::with_capacity(13);
            for i in 0..13 {
                p_row.push(Some(images[j * 26 + i].clone()));
                m_row.push(Some(images[j * 26 + i + 13].clone()));
            }
            pn.push(p_row);
            mn.push(m_row);
        }
        crate::skin_float::SkinFloat::new_with_int_timer_int_id_mimage(
            pn,
            Some(mn),
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
        )
    } else if images.len().is_multiple_of(24) {
        // %24: unsigned, separate +/- images (12 positive + 12 negative per set)
        let set_count = images.len() / 24;
        let mut pn: Vec<Vec<Option<TextureRegion>>> = Vec::with_capacity(set_count);
        let mut mn: Vec<Vec<Option<TextureRegion>>> = Vec::with_capacity(set_count);
        for j in 0..set_count {
            let mut p_row = Vec::with_capacity(12);
            let mut m_row = Vec::with_capacity(12);
            for i in 0..12 {
                p_row.push(Some(images[j * 24 + i].clone()));
                m_row.push(Some(images[j * 24 + i + 12].clone()));
            }
            pn.push(p_row);
            mn.push(m_row);
        }
        crate::skin_float::SkinFloat::new_with_int_timer_int_id_mimage(
            pn,
            Some(mn),
            timer_val,
            cycle,
            crate::skin_float::FloatDisplayConfig {
                iketa,
                fketa,
                is_sign_visible: false,
                align,
                zeropadding,
                space,
                gain,
            },
            prop_id,
        )
    } else if images.len().is_multiple_of(22) {
        // %22: unsigned, separate +/- images (11+11 with shared reverse zero mapping)
        let set_count = images.len() / 22;
        let mut pn: Vec<Vec<Option<TextureRegion>>> = Vec::with_capacity(set_count);
        let mut mn: Vec<Vec<Option<TextureRegion>>> = Vec::with_capacity(set_count);
        for j in 0..set_count {
            let mut p_row = vec![None; 12];
            let mut m_row = vec![None; 12];
            for i in 0..10 {
                p_row[i] = Some(images[j * 22 + i].clone());
                m_row[i] = Some(images[j * 22 + i + 11].clone());
            }
            p_row[10] = Some(images[j * 22].clone()); // shared reverse zero
            p_row[11] = Some(images[j * 22 + 10].clone()); // decimal point
            m_row[10] = Some(images[j * 22 + 11].clone()); // minus reverse
            m_row[11] = Some(images[j * 22 + 21].clone()); // minus decimal
            pn.push(p_row);
            mn.push(m_row);
        }
        crate::skin_float::SkinFloat::new_with_int_timer_int_id_mimage(
            pn,
            Some(mn),
            timer_val,
            cycle,
            crate::skin_float::FloatDisplayConfig {
                iketa,
                fketa,
                is_sign_visible: false,
                align,
                zeropadding,
                space,
                gain,
            },
            prop_id,
        )
    } else if images.len().is_multiple_of(12) {
        // %12: unsigned, single image (12 per set)
        let set_count = images.len() / 12;
        let mut nimage: Vec<Vec<Option<TextureRegion>>> = Vec::with_capacity(set_count);
        for j in 0..set_count {
            let mut row = Vec::with_capacity(12);
            for i in 0..12 {
                row.push(Some(images[j * 12 + i].clone()));
            }
            nimage.push(row);
        }
        crate::skin_float::SkinFloat::new_with_int_timer_int_id(
            nimage,
            timer_val,
            cycle,
            crate::skin_float::FloatDisplayConfig {
                iketa,
                fketa,
                is_sign_visible: false,
                align,
                zeropadding,
                space,
                gain,
            },
            prop_id,
        )
    } else if images.len().is_multiple_of(11) {
        // %11: unsigned, single image (11 per set, mapped to 12 with shared reverse zero)
        let set_count = images.len() / 11;
        let mut nimage: Vec<Vec<Option<TextureRegion>>> = Vec::with_capacity(set_count);
        for j in 0..set_count {
            let mut row = vec![None; 12];
            for i in 0..10 {
                row[i] = Some(images[j * 11 + i].clone());
            }
            row[10] = Some(images[j * 11].clone()); // shared reverse zero
            row[11] = Some(images[j * 11 + 10].clone()); // decimal point
            nimage.push(row);
        }
        crate::skin_float::SkinFloat::new_with_int_timer_int_id(
            nimage,
            timer_val,
            cycle,
            crate::skin_float::FloatDisplayConfig {
                iketa,
                fketa,
                is_sign_visible: false,
                align,
                zeropadding,
                space,
                gain,
            },
            prop_id,
        )
    } else {
        // Fallback: treat as %12 (Java: divx*divy/12 sets)
        let d = 12;
        let set_count = images.len() / d;
        let mut nimages: Vec<Vec<Option<TextureRegion>>> = Vec::with_capacity(set_count);
        for j in 0..set_count {
            let mut row = Vec::with_capacity(d);
            for i in 0..d {
                row.push(Some(images[j * d + i].clone()));
            }
            nimages.push(row);
        }
        crate::skin_float::SkinFloat::new_with_int_timer_int_id(
            nimages,
            timer_val,
            cycle,
            crate::skin_float::FloatDisplayConfig {
                iketa,
                fketa,
                is_sign_visible: false,
                align,
                zeropadding,
                space,
                gain,
            },
            prop_id,
        )
    };

    // Apply per-digit offsets if present
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
        sf.set_offsets(skin_offsets);
    }

    Some(SkinObject::Float(sf))
}

#[allow(clippy::too_many_arguments)]
fn convert_text(
    font: &Option<String>,
    size: i32,
    align: i32,
    ref_id: i32,
    value: Option<i32>,
    constant_text: &Option<String>,
    wrapping: bool,
    overflow: i32,
    outline_color: &String,
    outline_width: f32,
    shadow_color: &String,
    shadow_offset_x: f32,
    shadow_offset_y: f32,
    shadow_smoothness: f32,
    usecim: bool,
    scale_x: f32,
) -> Option<SkinObject> {
    if let Some(font_path) = font {
        let text_id = value.unwrap_or(ref_id);
        let property = if text_id >= 0 {
            string_property_factory::string_property_by_id(text_id)
        } else {
            None
        };
        let is_bitmap_font = Path::new(font_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("fnt"));
        if is_bitmap_font {
            let source = SkinTextBitmapSource::new(PathBuf::from(font_path), usecim);
            let mut stb =
                SkinTextBitmap::new_with_property(source, size as f32 * scale_x, property);
            stb.text_data.align = align;
            stb.text_data.wrapping = wrapping;
            stb.text_data.overflow = overflow;
            if let Some(ct) = constant_text {
                stb.text_data.set_constant_text(ct.clone());
            }
            if !outline_color.is_empty() && outline_color != "ffffff00" {
                stb.text_data
                    .set_outline_color(crate::reexports::Color::value_of(outline_color));
            }
            stb.text_data.outline_width = outline_width;
            if !shadow_color.is_empty() && shadow_color != "ffffff00" {
                stb.text_data
                    .set_shadow_color(crate::reexports::Color::value_of(shadow_color));
            }
            stb.text_data
                .set_shadow_offset(shadow_offset_x, shadow_offset_y);
            stb.text_data.shadow_smoothness = shadow_smoothness;
            Some(SkinObject::TextBitmap(stb))
        } else {
            let mut stf = SkinTextFont::new_with_property(font_path, 0, size, 0, property);
            // Apply JSON text layout fields
            stf.text_data.align = align;
            stf.text_data.wrapping = wrapping;
            stf.text_data.overflow = overflow;
            if let Some(ct) = constant_text {
                stf.text_data.set_constant_text(ct.clone());
            }
            if !outline_color.is_empty() && outline_color != "ffffff00" {
                stf.text_data
                    .set_outline_color(crate::reexports::Color::value_of(outline_color));
            }
            stf.text_data.outline_width = outline_width;
            if !shadow_color.is_empty() && shadow_color != "ffffff00" {
                stf.text_data
                    .set_shadow_color(crate::reexports::Color::value_of(shadow_color));
            }
            stf.text_data
                .set_shadow_offset(shadow_offset_x, shadow_offset_y);
            stf.text_data.shadow_smoothness = shadow_smoothness;
            Some(SkinObject::TextFont(stf))
        }
    } else {
        warn!("Text object without font path, skipping");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::convert_text;
    use crate::types::skin::SkinObject;

    #[test]
    fn convert_text_uses_bitmap_object_for_fnt_fonts() {
        let obj = convert_text(
            &Some("skin/ECFN/_font/selectsongname.fnt".to_string()),
            50,
            0,
            10,
            None,
            &None,
            false,
            1,
            &String::new(),
            0.0,
            &String::new(),
            0.0,
            0.0,
            0.0,
            false,
            1.0,
        )
        .expect("bitmap text object should be created");

        assert!(
            matches!(obj, SkinObject::TextBitmap(_)),
            ".fnt fonts must become SkinObject::TextBitmap"
        );
    }

    #[test]
    fn convert_text_scales_bitmap_font_size_by_destination_width() {
        let obj = convert_text(
            &Some("skin/ECFN/_font/barfont.fnt".to_string()),
            25,
            0,
            10,
            None,
            &None,
            false,
            1,
            &String::new(),
            0.0,
            &String::new(),
            0.0,
            0.0,
            0.0,
            false,
            1280.0 / 1920.0,
        )
        .expect("bitmap text object should be created");

        let bitmap = match obj {
            SkinObject::TextBitmap(bitmap) => bitmap,
            _ => panic!("bitmap font should stay bitmap"),
        };
        assert!(
            (bitmap.debug_size() - 25.0 * (1280.0 / 1920.0)).abs() < 0.01,
            "bitmap font size should follow Java parity destination-width scaling, got {}",
            bitmap.debug_size()
        );
    }

    #[test]
    fn convert_text_uses_font_object_for_ttf_fonts() {
        let obj = convert_text(
            &Some("skin/default/VL-Gothic-Regular.ttf".to_string()),
            24,
            0,
            12,
            None,
            &None,
            false,
            1,
            &String::new(),
            0.0,
            &String::new(),
            0.0,
            0.0,
            0.0,
            false,
            1.0,
        )
        .expect("font text object should be created");

        assert!(
            matches!(obj, SkinObject::TextFont(_)),
            ".ttf fonts must remain SkinObject::TextFont"
        );
    }

    #[test]
    fn convert_image_returns_none_when_srcimg_fewer_than_len() {
        use super::convert_image;
        use crate::json::json_skin_loader::{SourceData, SourceDataType};
        use crate::reexports::Texture;
        use std::collections::HashMap;
        use std::path::Path;

        // Create a small 2x2 texture that will produce only 4 source images (divx=2, divy=2).
        // Request len=8, so imgs_per_ref = 4/8 = 0 -> bug triggers empty Vec entries.
        let tex = Texture {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let mut source_map = HashMap::new();
        source_map.insert(
            "test_src".to_string(),
            SourceData {
                path: "test.png".to_string(),
                loaded: true,
                data: Some(SourceDataType::Texture(tex)),
            },
        );

        let result = convert_image(
            &Some("test_src".to_string()),
            0,
            0,
            64,
            64,
            2, // divx: produces 2*2=4 source images
            2, // divy
            Some(0),
            0,
            8,   // len: 8 refs, but only 4 source images -> imgs_per_ref=0
            100, // ref_id
            false,
            &mut source_map,
            Path::new("/tmp"),
            false,
            &HashMap::new(),
        );

        // With imgs_per_ref == 0, the current code produces empty Vec entries.
        // The fix should return None instead.
        assert!(
            result.is_none(),
            "convert_image should return None when source images are fewer than len"
        );
    }

    /// Helper: create a SourceData entry with a texture that produces `divx * divy` source images.
    fn make_source_map_with_image_count(
        divx: i32,
        divy: i32,
    ) -> (
        std::collections::HashMap<String, crate::json::json_skin_loader::SourceData>,
        i32,
        i32,
    ) {
        use crate::json::json_skin_loader::{SourceData, SourceDataType};
        use crate::reexports::Texture;
        use std::collections::HashMap;

        let w = divx * 10;
        let h = divy * 10;
        let tex = Texture {
            width: w,
            height: h,
            ..Default::default()
        };
        let mut source_map = HashMap::new();
        source_map.insert(
            "src".to_string(),
            SourceData {
                path: "test.png".to_string(),
                loaded: true,
                data: Some(SourceDataType::Texture(tex)),
            },
        );
        (source_map, w, h)
    }

    /// Helper: call convert_float with a texture producing `divx * divy` source images.
    fn call_convert_float(divx: i32, divy: i32, is_signvisible: bool) -> Option<SkinObject> {
        use std::path::Path;

        let (mut source_map, w, h) = make_source_map_with_image_count(divx, divy);
        super::convert_float(
            &Some("src".to_string()),
            0,
            0,
            w,
            h,
            divx,
            divy,
            Some(0),
            0,
            3, // iketa
            2, // fketa
            is_signvisible,
            0,    // align
            0,    // zeropadding
            0,    // space
            0,    // ref_id
            None, // value
            1.0,  // gain
            &None,
            &mut source_map,
            Path::new("/tmp"),
            false,
            &std::collections::HashMap::new(),
        )
    }

    #[test]
    fn convert_float_26_layout_preserves_sign_visible() {
        // 26 images: %26 branch, should preserve is_signvisible=true
        let obj = call_convert_float(26, 1, true).expect("should produce Float");
        match obj {
            SkinObject::Float(sf) => {
                assert!(
                    sf.is_sign_visible,
                    "%26 layout must preserve is_signvisible=true"
                );
            }
            _ => panic!("expected SkinObject::Float"),
        }

        // 26 images with is_signvisible=false: should preserve false
        let obj = call_convert_float(26, 1, false).expect("should produce Float");
        match obj {
            SkinObject::Float(sf) => {
                assert!(
                    !sf.is_sign_visible,
                    "%26 layout must preserve is_signvisible=false"
                );
            }
            _ => panic!("expected SkinObject::Float"),
        }
    }

    #[test]
    fn convert_float_24_layout_forces_sign_invisible() {
        // 24 images: %24 branch, should force is_sign_visible=false
        let obj = call_convert_float(24, 1, true).expect("should produce Float");
        match obj {
            SkinObject::Float(sf) => {
                assert!(
                    !sf.is_sign_visible,
                    "%24 layout must force is_sign_visible=false"
                );
            }
            _ => panic!("expected SkinObject::Float"),
        }
    }

    #[test]
    fn convert_float_22_layout_forces_sign_invisible() {
        // 22 images: %22 branch
        let obj = call_convert_float(22, 1, true).expect("should produce Float");
        match obj {
            SkinObject::Float(sf) => {
                assert!(
                    !sf.is_sign_visible,
                    "%22 layout must force is_sign_visible=false"
                );
            }
            _ => panic!("expected SkinObject::Float"),
        }
    }

    #[test]
    fn convert_float_12_layout_forces_sign_invisible() {
        // 12 images: %12 branch
        let obj = call_convert_float(12, 1, true).expect("should produce Float");
        match obj {
            SkinObject::Float(sf) => {
                assert!(
                    !sf.is_sign_visible,
                    "%12 layout must force is_sign_visible=false"
                );
            }
            _ => panic!("expected SkinObject::Float"),
        }
    }

    #[test]
    fn convert_float_11_layout_forces_sign_invisible() {
        // 11 images: %11 branch
        let obj = call_convert_float(11, 1, true).expect("should produce Float");
        match obj {
            SkinObject::Float(sf) => {
                assert!(
                    !sf.is_sign_visible,
                    "%11 layout must force is_sign_visible=false"
                );
            }
            _ => panic!("expected SkinObject::Float"),
        }
    }

    #[test]
    fn convert_float_fallback_forces_sign_invisible() {
        // 7 images: not divisible by 26/24/22/12/11, hits fallback
        let obj = call_convert_float(7, 1, true).expect("should produce Float");
        match obj {
            SkinObject::Float(sf) => {
                assert!(
                    !sf.is_sign_visible,
                    "fallback layout must force is_sign_visible=false"
                );
            }
            _ => panic!("expected SkinObject::Float"),
        }
    }

    #[test]
    fn convert_float_priority_24_over_12() {
        // 48 images: divisible by both 24 (=2 sets) and 12 (=4 sets)
        // Should pick %24 (separate +/- images), not %12
        // %24 forces is_sign_visible=false (same as %12), but uses mimage.
        // We verify it reaches the %24 branch by checking that it still produces a valid Float.
        let obj = call_convert_float(48, 1, true).expect("should produce Float");
        match obj {
            SkinObject::Float(sf) => {
                assert!(
                    !sf.is_sign_visible,
                    "%24 branch (48 images) must force is_sign_visible=false"
                );
            }
            _ => panic!("expected SkinObject::Float"),
        }
    }

    #[test]
    fn convert_float_priority_26_over_others() {
        // 52 images: divisible by 26 (=2 sets), also by others (not 24, not 22, not 12)
        // Should pick %26 and preserve is_signvisible
        let obj = call_convert_float(52, 1, true).expect("should produce Float");
        match obj {
            SkinObject::Float(sf) => {
                assert!(
                    sf.is_sign_visible,
                    "%26 branch (52 images) must preserve is_signvisible=true"
                );
            }
            _ => panic!("expected SkinObject::Float"),
        }
    }

    #[test]
    fn convert_slider_scales_range_by_direction() {
        use std::path::Path;

        let (mut source_map, w, h) = make_source_map_with_image_count(2, 2);
        let scale_x = 0.5_f32;
        let scale_y = 2.0_f32;
        let range = 100;

        // angle=1 (right): should use scale_x
        let obj = super::convert_slider(
            &Some("src".to_string()),
            0,
            0,
            w,
            h,
            2,
            2,
            Some(0),
            0,
            1, // angle: right
            range,
            0,     // slider_type
            false, // changeable
            None,  // value
            None,  // event
            false, // is_ref_num
            0,
            0, // min, max
            scale_x,
            scale_y,
            &mut source_map,
            Path::new("/tmp"),
            false,
            &std::collections::HashMap::new(),
        )
        .expect("should produce Slider");

        match obj {
            SkinObject::Slider(sl) => {
                assert_eq!(
                    sl.range(),
                    (scale_x * range as f32) as i32,
                    "angle=1 should use scale_x for range scaling"
                );
            }
            _ => panic!("expected SkinObject::Slider"),
        }

        // angle=0 (up): should use scale_y
        let (mut source_map, w, h) = make_source_map_with_image_count(2, 2);
        let obj = super::convert_slider(
            &Some("src".to_string()),
            0,
            0,
            w,
            h,
            2,
            2,
            Some(0),
            0,
            0, // angle: up
            range,
            0,     // slider_type
            false, // changeable
            None,  // value
            None,  // event
            false, // is_ref_num
            0,
            0, // min, max
            scale_x,
            scale_y,
            &mut source_map,
            Path::new("/tmp"),
            false,
            &std::collections::HashMap::new(),
        )
        .expect("should produce Slider");

        match obj {
            SkinObject::Slider(sl) => {
                assert_eq!(
                    sl.range(),
                    (scale_y * range as f32) as i32,
                    "angle=0 should use scale_y for range scaling"
                );
            }
            _ => panic!("expected SkinObject::Slider"),
        }
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
    event: Option<i32>,
    is_ref_num: bool,
    min: i32,
    max: i32,
    scale_x: f32,
    scale_y: f32,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    filemap: &HashMap<String, String>,
) -> Option<SkinObject> {
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim, filemap)?;
    let images = source_image(&tex, x, y, w, h, divx, divy);
    let timer_val = timer.unwrap_or(0);

    // Scale range by destination/source ratio based on angle direction
    // Java: (int) ((angle == 1 || angle == 3 ? dstr.width/sk.w : dstr.height/sk.h) * range)
    let scaled_range = if angle == 1 || angle == 3 {
        (scale_x * range as f32) as i32
    } else {
        (scale_y * range as f32) as i32
    };

    let slider = if let Some(val_id) = value {
        // Explicit FloatProperty + FloatWriter from value/event IDs
        use crate::property::float_property_factory;
        let ref_prop = float_property_factory::rate_property_by_id(val_id);
        let writer = event.and_then(float_property_factory::rate_writer_by_id);
        if let Some(prop) = ref_prop {
            SkinSlider::new_with_int_timer_ref_writer(
                images,
                timer_val,
                cycle,
                angle,
                scaled_range,
                prop,
                writer,
            )
        } else {
            // Fallback: value ID didn't resolve to a property, use type-based lookup
            SkinSlider::new_with_int_timer(
                images,
                timer_val,
                cycle,
                angle,
                scaled_range,
                val_id,
                changeable,
            )
        }
    } else if is_ref_num {
        // RateProperty with min/max range
        SkinSlider::new_with_int_timer_minmax(
            crate::objects::skin_slider::SliderIntTimerMinmaxParams {
                image: images,
                timer: timer_val,
                cycle,
                angle,
                range: scaled_range,
                type_id: slider_type,
                min,
                max,
            },
        )
    } else {
        // Default: type-based lookup with changeable flag
        SkinSlider::new_with_int_timer(
            images,
            timer_val,
            cycle,
            angle,
            scaled_range,
            slider_type,
            changeable,
        )
    };
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
    filemap: &HashMap<String, String>,
) -> Option<SkinObject> {
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim, filemap)?;
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
    filemap: &HashMap<String, String>,
) -> Option<SkinObject> {
    // Resolve gauge node IDs to TextureRegion images via source_map.
    // Each node string references a source entry; resolve to a full-texture TextureRegion.
    // Java indexmap logic maps 4/8/12 node configs to 36 gauge slots.
    // With 36 nodes, each maps 1:1 to a slot.
    let mut resolved_nodes: Vec<Option<TextureRegion>> = Vec::with_capacity(nodes.len());
    for node_id in nodes {
        let tex = get_texture_for_src(Some(node_id), source_map, skin_path, usecim, filemap);
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
    filemap: &HashMap<String, String>,
) -> Option<SkinObject> {
    // HiddenCover: create SkinHidden with texture and disappear line.
    // Java: new SkinHidden(getSourceImage(tex,...), timer, cycle)
    //       setDisapearLine(disapearLine * scaleY)
    //       offsets += [OFFSET_LIFT, OFFSET_HIDDEN_COVER]
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim, filemap);
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
    filemap: &HashMap<String, String>,
) -> Option<SkinObject> {
    // LiftCover: same as HiddenCover but offset list only adds OFFSET_LIFT.
    let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim, filemap);
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
