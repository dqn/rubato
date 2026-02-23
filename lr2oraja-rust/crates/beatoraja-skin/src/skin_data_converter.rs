// SkinData -> Skin converter (Phase 26b)
// Converts the intermediate SkinData representation into the runtime Skin object.

use std::collections::HashMap;
use std::path::Path;

use log::warn;

use crate::custom_event::CustomEvent;
use crate::custom_timer::CustomTimer;
use crate::json::json_skin_loader::{
    CustomCategoryData, CustomFileData, CustomItemData, CustomOffsetData, CustomOptionData,
    DestinationData, SkinData, SkinHeaderData, SkinNumberOffset, SkinObjectType, SourceData,
    SourceDataType,
};
use crate::json::json_skin_object_loader::get_source_image;
use crate::property::boolean_property_factory;
use crate::property::event_factory;
use crate::property::string_property_factory;
use crate::property::timer_property_factory;
use crate::skin::{Skin, SkinObject};
use crate::skin_bar_object::SkinBarObject;
use crate::skin_bpm_graph::SkinBPMGraph;
use crate::skin_graph::SkinGraph;
use crate::skin_header::{
    CustomCategory, CustomFile, CustomItemEnum, CustomOffset, CustomOption, SkinHeader,
};
use crate::skin_hit_error_visualizer::SkinHitErrorVisualizer;
use crate::skin_image::SkinImage;
use crate::skin_judge_object::SkinJudgeObject;
use crate::skin_note_distribution_graph::SkinNoteDistributionGraph;
use crate::skin_note_object::SkinNoteObject;
use crate::skin_number::SkinNumber;
use crate::skin_slider::SkinSlider;
use crate::skin_text_font::SkinTextFont;
use crate::skin_timing_distribution_graph::SkinTimingDistributionGraph;
use crate::skin_timing_visualizer::SkinTimingVisualizer;
use crate::skin_type::SkinType;
use crate::stubs::{Resolution, SkinConfigOffset, SkinOffset, TextureRegion};

/// Converts SkinHeaderData into a SkinHeader.
pub fn convert_header_data(
    data: &SkinHeaderData,
    src: &Resolution,
    dst: &Resolution,
) -> SkinHeader {
    let mut header = SkinHeader::new();

    // Map skin_type integer to SkinType enum
    if let Some(skin_type) = SkinType::get_skin_type_by_id(data.skin_type) {
        header.set_skin_type(skin_type);
    }

    header.set_name(data.name.clone());
    header.set_author(data.author.clone());
    header.set_path(data.path.clone());
    header.set_type(data.header_type);

    // Set resolutions
    header.set_resolution(Resolution {
        width: src.width,
        height: src.height,
    });
    header.set_source_resolution(src.clone());
    header.set_destination_resolution(dst.clone());

    // Convert custom options
    let options: Vec<CustomOption> = data
        .custom_options
        .iter()
        .map(|o| {
            let mut opt = if let Some(ref def) = o.def {
                CustomOption::new_with_def(
                    o.name.clone(),
                    o.option.clone(),
                    o.names.clone(),
                    def.clone(),
                )
            } else {
                CustomOption::new(o.name.clone(), o.option.clone(), o.names.clone())
            };
            // Set selected index based on selected_option
            for (i, &op_val) in o.option.iter().enumerate() {
                if op_val == o.selected_option {
                    opt.selected_index = i as i32;
                }
            }
            opt
        })
        .collect();
    header.set_custom_options(options);

    // Convert custom files
    let files: Vec<CustomFile> = data
        .custom_files
        .iter()
        .map(|f| {
            let mut cf = CustomFile::new(f.name.clone(), f.path.clone(), f.def.clone());
            cf.filename = f.selected_filename.clone();
            cf
        })
        .collect();
    header.set_custom_files(files);

    // Convert custom offsets
    let offsets: Vec<CustomOffset> = data
        .custom_offsets
        .iter()
        .map(|o| CustomOffset::new(o.name.clone(), o.id, o.x, o.y, o.w, o.h, o.r, o.a))
        .collect();
    header.set_custom_offsets(offsets);

    // Convert custom categories
    let categories: Vec<CustomCategory> = data
        .custom_categories
        .iter()
        .map(convert_category_data)
        .collect();
    header.set_custom_categories(categories);

    header
}

fn convert_category_data(cat: &CustomCategoryData) -> CustomCategory {
    let items: Vec<CustomItemEnum> = cat
        .items
        .iter()
        .map(|item| match item {
            CustomItemData::Option(o) => {
                let mut opt = if let Some(ref def) = o.def {
                    CustomOption::new_with_def(
                        o.name.clone(),
                        o.option.clone(),
                        o.names.clone(),
                        def.clone(),
                    )
                } else {
                    CustomOption::new(o.name.clone(), o.option.clone(), o.names.clone())
                };
                for (i, &op_val) in o.option.iter().enumerate() {
                    if op_val == o.selected_option {
                        opt.selected_index = i as i32;
                    }
                }
                CustomItemEnum::Option(opt)
            }
            CustomItemData::File(f) => {
                let mut cf = CustomFile::new(f.name.clone(), f.path.clone(), f.def.clone());
                cf.filename = f.selected_filename.clone();
                CustomItemEnum::File(cf)
            }
            CustomItemData::Offset(o) => CustomItemEnum::Offset(CustomOffset::new(
                o.name.clone(),
                o.id,
                o.x,
                o.y,
                o.w,
                o.h,
                o.r,
                o.a,
            )),
        })
        .collect();
    CustomCategory::new(cat.name.clone(), items)
}

/// Converts SkinData into a runtime Skin object.
///
/// Corresponds to Java JSONSkinLoader.loadJsonSkin()
pub fn convert_skin_data(
    header_data: &SkinHeaderData,
    data: SkinData,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    dstr: &Resolution,
) -> Option<Skin> {
    // Determine source resolution
    let src = header_data.source_resolution.clone().unwrap_or(Resolution {
        width: 1280.0,
        height: 720.0,
    });

    let header = convert_header_data(header_data, &src, dstr);
    let mut skin = Skin::new(header);

    // Wire options: for each custom option, create op map
    let mut op: HashMap<i32, i32> = HashMap::new();
    for option in &header_data.custom_options {
        for &op_val in &option.option {
            op.insert(
                op_val,
                if op_val == option.selected_option {
                    1
                } else {
                    0
                },
            );
        }
    }
    skin.set_option(op);

    // Wire offsets: for each custom offset, get the SkinConfigOffset
    let mut offset: HashMap<i32, SkinConfigOffset> = HashMap::new();
    for of in &header_data.custom_offsets {
        // Use a default offset (the actual value is set by setSkinConfigProperty)
        offset.insert(
            of.id,
            SkinConfigOffset {
                name: of.name.clone(),
                ..SkinConfigOffset::default()
            },
        );
    }
    skin.set_offset(offset);

    // Set skin timing
    skin.set_fadeout(data.fadeout);
    skin.set_input(data.input);
    skin.set_scene(data.scene);

    // Convert each SkinObjectData to a SkinObject
    for obj_data in &data.objects {
        let skin_obj = convert_skin_object(&obj_data.object_type, source_map, skin_path, usecim);

        if let Some(mut obj) = skin_obj {
            // Set name on the underlying SkinObjectData
            if let Some(ref name) = obj_data.name {
                obj.data_mut().name = Some(name.clone());
            }

            // Set click event from object type
            set_click_event_from_type(&mut obj, &obj_data.object_type);

            // Add the object to the skin
            skin.add(obj);
            let obj_index = skin.get_all_skin_objects_count() - 1;

            // Set destinations
            for dst in &obj_data.destinations {
                let timer_id = dst.timer.unwrap_or(0);

                // Handle draw condition
                if let Some(draw_id) = dst.draw
                    && draw_id != 0
                {
                    let timer_prop = if timer_id > 0 {
                        timer_property_factory::get_timer_property(timer_id)
                    } else {
                        None
                    };
                    if let Some(draw_prop) = boolean_property_factory::get_boolean_property(draw_id)
                    {
                        skin.set_destination_with_timer_draw(
                            obj_index,
                            dst.time as i64,
                            dst.x as f32,
                            dst.y as f32,
                            dst.w as f32,
                            dst.h as f32,
                            dst.acc,
                            dst.a,
                            dst.r,
                            dst.g,
                            dst.b,
                            dst.blend,
                            dst.filter,
                            dst.angle,
                            dst.center,
                            dst.loop_val,
                            timer_prop,
                            draw_prop,
                        );
                        continue;
                    }
                }

                // Handle op-based destination
                if !dst.op.is_empty() {
                    let timer_prop = if timer_id > 0 {
                        timer_property_factory::get_timer_property(timer_id)
                    } else {
                        None
                    };
                    skin.set_destination_with_timer(
                        obj_index,
                        dst.time as i64,
                        dst.x as f32,
                        dst.y as f32,
                        dst.w as f32,
                        dst.h as f32,
                        dst.acc,
                        dst.a,
                        dst.r,
                        dst.g,
                        dst.b,
                        dst.blend,
                        dst.filter,
                        dst.angle,
                        dst.center,
                        dst.loop_val,
                        timer_prop,
                        &dst.op,
                    );
                } else {
                    // Simple destination with offsets (from obj_data)
                    skin.set_destination(
                        obj_index,
                        dst.time as i64,
                        dst.x as f32,
                        dst.y as f32,
                        dst.w as f32,
                        dst.h as f32,
                        dst.acc,
                        dst.a,
                        dst.r,
                        dst.g,
                        dst.b,
                        dst.blend,
                        dst.filter,
                        dst.angle,
                        dst.center,
                        dst.loop_val,
                        timer_id,
                        0,
                        0,
                        0,
                        &[],
                    );
                }
            }

            // Set mouse rect if present
            if let Some(ref mr) = obj_data.mouse_rect {
                skin.set_mouse_rect_on_object(
                    obj_index,
                    mr.x as f32,
                    mr.y as f32,
                    mr.w as f32,
                    mr.h as f32,
                );
            }

            // Set offset IDs
            if !obj_data.offset_ids.is_empty()
                && let Some(obj) = skin.get_objects_mut().get_mut(obj_index)
            {
                obj.data_mut().set_offset_id(&obj_data.offset_ids);
            }

            // Set stretch
            if obj_data.stretch >= 0
                && let Some(obj) = skin.get_objects_mut().get_mut(obj_index)
            {
                obj.data_mut().set_stretch_by_id(obj_data.stretch);
            }
        }
    }

    // Add custom events
    for event_data in &data.custom_events {
        let action = event_data.action.and_then(event_factory::get_event_by_id);
        let condition = event_data
            .condition
            .and_then(boolean_property_factory::get_boolean_property);
        if let Some(action) = action {
            let event = CustomEvent::new(event_data.id, action, condition, event_data.min_interval);
            skin.add_custom_event(event);
        }
    }

    // Add custom timers
    for timer_data in &data.custom_timers {
        let timer_func = timer_data
            .timer
            .and_then(timer_property_factory::get_timer_property);
        let timer = CustomTimer::new(timer_data.id, timer_func);
        skin.add_custom_timer(timer);
    }

    Some(skin)
}

/// Sets click event on a SkinObject based on its SkinObjectType.
fn set_click_event_from_type(obj: &mut SkinObject, obj_type: &SkinObjectType) {
    match obj_type {
        SkinObjectType::Image {
            act: Some(act_id),
            click,
            ..
        } => {
            obj.data_mut().set_clickevent_by_id(*act_id);
            obj.data_mut().set_clickevent_type(*click);
        }
        SkinObjectType::ImageSet {
            act: Some(act_id),
            click,
            ..
        } => {
            obj.data_mut().set_clickevent_by_id(*act_id);
            obj.data_mut().set_clickevent_type(*click);
        }
        _ => {}
    }
}

/// Converts a SkinObjectType into a SkinObject.
fn convert_skin_object(
    obj_type: &SkinObjectType,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
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
        } => {
            if *is_movie {
                // Movie sources: create SkinImage with SkinSourceMovie
                let movie_source = crate::skin_source_movie::SkinSourceMovie::new("");
                return Some(SkinObject::Image(SkinImage::new_with_movie(movie_source)));
            }

            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
            let srcimg = get_source_image(&tex, *x, *y, *w, *h, *divx, *divy);

            if *len > 1 {
                // Multiple reference images
                let imgs_per_ref = srcimg.len() / (*len as usize);
                let mut tr: Vec<Vec<TextureRegion>> = Vec::with_capacity(*len as usize);
                for i in 0..(*len as usize) {
                    let mut row: Vec<TextureRegion> = Vec::with_capacity(imgs_per_ref);
                    for j in 0..imgs_per_ref {
                        row.push(srcimg[i * imgs_per_ref + j].clone());
                    }
                    tr.push(row);
                }
                let timer_val = timer.unwrap_or(0);
                Some(SkinObject::Image(SkinImage::new_with_int_timer_ref_id(
                    tr, timer_val, *cycle, *ref_id,
                )))
            } else {
                let timer_val = timer.unwrap_or(0);
                Some(SkinObject::Image(SkinImage::new_with_int_timer(
                    srcimg, timer_val, *cycle,
                )))
            }
        }

        SkinObjectType::ImageSet {
            images: _,
            ref_id: _,
            value: _,
            act: _,
            click: _,
        } => {
            // ImageSet requires looking up each image ID in the skin's image list,
            // which needs sk (json_skin::Skin). For now, create a placeholder image.
            // Full implementation requires passing sk to the converter.
            warn!("ImageSet conversion not fully implemented yet");
            Some(SkinObject::Image(SkinImage::new_with_image_id(0)))
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
        } => {
            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
            let images = get_source_image(&tex, *x, *y, *w, *h, *divx, *divy);
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
                if let Some(val) = value {
                    SkinNumber::new_with_int_timer(
                        pn,
                        Some(mn),
                        timer_val,
                        *cycle,
                        *digit,
                        *zeropadding,
                        *space,
                        *val,
                        *align,
                    )
                } else {
                    SkinNumber::new_with_int_timer(
                        pn,
                        Some(mn),
                        timer_val,
                        *cycle,
                        *digit,
                        *zeropadding,
                        *space,
                        *ref_id,
                        *align,
                    )
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
                let actual_padding = if d > 10 { 2 } else { *padding };
                if let Some(val) = value {
                    SkinNumber::new_with_int_timer(
                        nimages,
                        None,
                        timer_val,
                        *cycle,
                        *digit,
                        actual_padding,
                        *space,
                        *val,
                        *align,
                    )
                } else {
                    SkinNumber::new_with_int_timer(
                        nimages,
                        None,
                        timer_val,
                        *cycle,
                        *digit,
                        actual_padding,
                        *space,
                        *ref_id,
                        *align,
                    )
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
        } => {
            // SkinFloat construction requires complex image splitting.
            // For now, create a stub that won't crash but won't render either.
            warn!("Float conversion creates placeholder (full SkinFloat image splitting deferred)");
            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim);
            tex.as_ref()?;
            let tex = tex.unwrap();
            let images = get_source_image(&tex, *x, *y, *w, *h, *divx, *divy);
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

            if let Some(val) = value {
                let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
                    image_opts,
                    timer_val,
                    *cycle,
                    *iketa,
                    *fketa,
                    *is_signvisible,
                    *align,
                    *zeropadding,
                    *space,
                    *val,
                    *gain,
                );
                // SkinFloat is not in the SkinObject enum; wrap as a Number placeholder
                // Full SkinFloat support requires adding it to the SkinObject enum (Phase 29a)
                warn!("SkinFloat not in SkinObject enum; skipping");
                None
            } else {
                warn!("SkinFloat not in SkinObject enum; skipping");
                None
            }
        }

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
        } => {
            if let Some(font_path) = font {
                let text_id = value.unwrap_or(*ref_id);
                let property = if text_id >= 0 {
                    string_property_factory::get_string_property_by_id(text_id)
                } else {
                    None
                };
                let stf = SkinTextFont::new_with_property(font_path, 0, *size, 0, property);
                Some(SkinObject::TextFont(stf))
            } else {
                warn!("Text object without font path, skipping");
                None
            }
        }

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
        } => {
            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
            let images = get_source_image(&tex, *x, *y, *w, *h, *divx, *divy);
            let timer_val = timer.unwrap_or(0);
            let type_id = value.unwrap_or(*slider_type);
            let slider = SkinSlider::new_with_int_timer(
                images,
                timer_val,
                *cycle,
                *angle,
                *range,
                type_id,
                *changeable,
            );
            Some(SkinObject::Slider(slider))
        }

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
        } => {
            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
            let images = get_source_image(&tex, *x, *y, *w, *h, *divx, *divy);
            let timer_val = timer.unwrap_or(0);
            if let Some(val) = value {
                Some(SkinObject::Graph(SkinGraph::new_with_int_timer(
                    images, timer_val, *cycle, *val, *angle,
                )))
            } else if *is_ref_num {
                Some(SkinObject::Graph(SkinGraph::new_with_int_timer_minmax(
                    images,
                    timer_val,
                    *cycle,
                    *graph_type,
                    *min,
                    *max,
                    *angle,
                )))
            } else {
                Some(SkinObject::Graph(SkinGraph::new_with_int_timer(
                    images,
                    timer_val,
                    *cycle,
                    *graph_type,
                    *angle,
                )))
            }
        }

        SkinObjectType::DistributionGraph { graph_type, .. } => {
            // SkinNoteDistributionGraph with TYPE_NORMAL
            let graph = SkinNoteDistributionGraph::new(*graph_type, 0, 0, 0, 0, 0);
            Some(SkinObject::NoteDistributionGraph(graph))
        }

        SkinObjectType::GaugeGraph { .. } => {
            warn!("GaugeGraph conversion deferred to Phase 29a");
            None
        }

        SkinObjectType::JudgeGraph {
            graph_type,
            delay,
            back_tex_off,
            order_reverse,
            no_gap,
            no_gap_x,
        } => {
            let graph = SkinNoteDistributionGraph::new(
                *graph_type,
                *delay,
                *back_tex_off,
                *order_reverse,
                *no_gap,
                *no_gap_x,
            );
            Some(SkinObject::NoteDistributionGraph(graph))
        }

        SkinObjectType::BpmGraph {
            delay,
            line_width,
            main_bpm_color,
            min_bpm_color,
            max_bpm_color,
            other_bpm_color,
            stop_line_color,
            transition_line_color,
        } => {
            let graph = SkinBPMGraph::new(
                *delay,
                *line_width,
                main_bpm_color,
                min_bpm_color,
                max_bpm_color,
                other_bpm_color,
                stop_line_color,
                transition_line_color,
            );
            Some(SkinObject::BpmGraph(graph))
        }

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
        } => {
            let viz = SkinHitErrorVisualizer::new(
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
            );
            Some(SkinObject::HitErrorVisualizer(viz))
        }

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
        } => {
            let viz = SkinTimingVisualizer::new(
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
            );
            Some(SkinObject::TimingVisualizer(viz))
        }

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
        } => {
            let graph = SkinTimingDistributionGraph::new(
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
            );
            Some(SkinObject::TimingDistributionGraph(graph))
        }

        // Stub types deferred to Phase 29a
        SkinObjectType::Gauge { .. } => {
            warn!("Gauge conversion deferred to Phase 29a");
            None
        }
        SkinObjectType::Note => {
            // Default lane count; lanes are configured later via set_lane_region
            let note = SkinNoteObject::new(0);
            Some(SkinObject::Note(note))
        }
        SkinObjectType::HiddenCover { .. } => {
            warn!("HiddenCover conversion deferred to Phase 29a");
            None
        }
        SkinObjectType::LiftCover { .. } => {
            warn!("LiftCover conversion deferred to Phase 29a");
            None
        }
        SkinObjectType::Bga { .. } => {
            warn!("Bga conversion deferred to Phase 29a");
            None
        }
        SkinObjectType::Judge { index, shift } => {
            let judge = SkinJudgeObject::new(*index, *shift);
            Some(SkinObject::Judge(judge))
        }
        SkinObjectType::PmChara { .. } => {
            warn!("PmChara conversion deferred to Phase 29a");
            None
        }
        SkinObjectType::SongList { center, .. } => {
            let bar = SkinBarObject::new(*center);
            Some(SkinObject::Bar(bar))
        }
        SkinObjectType::SearchTextRegion { .. } => {
            warn!("SearchTextRegion conversion deferred to Phase 29a");
            None
        }
    }
}

/// Loads a texture from the source map, resolving the source ID path.
fn get_texture_for_src(
    src_id: Option<&str>,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    _usecim: bool,
) -> Option<crate::stubs::Texture> {
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
    let image_path = format!("{}/{}", parent, data_path);

    let result = if std::path::Path::new(&image_path).exists() {
        Some(SourceDataType::Texture(crate::stubs::Texture::new(
            &image_path,
        )))
    } else {
        None
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::json_skin_loader::{
        CustomEventData, CustomTimerData, DestinationData, RectData, SkinData, SkinHeaderData,
        SkinObjectData as DataSkinObjectData,
    };
    use crate::stubs::Resolution;

    fn make_test_header_data() -> SkinHeaderData {
        SkinHeaderData {
            skin_type: 1, // Play7Keys
            name: "Test Skin".to_string(),
            author: "Test Author".to_string(),
            path: std::path::PathBuf::from("/test/skin.json"),
            header_type: 0,
            custom_options: vec![],
            custom_files: vec![],
            custom_offsets: vec![],
            custom_categories: vec![],
            source_resolution: Some(Resolution {
                width: 1920.0,
                height: 1080.0,
            }),
            destination_resolution: None,
        }
    }

    fn make_test_dst() -> Resolution {
        Resolution {
            width: 1920.0,
            height: 1080.0,
        }
    }

    // -- Test: header conversion --

    #[test]
    fn test_convert_header_data_basic() {
        let header_data = make_test_header_data();
        let src = Resolution {
            width: 1920.0,
            height: 1080.0,
        };
        let dst = make_test_dst();

        let header = convert_header_data(&header_data, &src, &dst);

        assert_eq!(header.get_name(), Some("Test Skin"));
        assert_eq!(header.get_author(), Some("Test Author"));
        assert_eq!(header.get_source_resolution().width, 1920.0);
        assert_eq!(header.get_source_resolution().height, 1080.0);
        assert_eq!(header.get_destination_resolution().width, 1920.0);
        assert_eq!(header.get_destination_resolution().height, 1080.0);
    }

    #[test]
    fn test_convert_header_with_options() {
        let mut header_data = make_test_header_data();
        header_data.custom_options = vec![CustomOptionData {
            name: "Option1".to_string(),
            option: vec![100, 101, 102],
            names: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            def: None,
            selected_option: 101,
        }];

        let src = Resolution {
            width: 1920.0,
            height: 1080.0,
        };
        let dst = make_test_dst();
        let header = convert_header_data(&header_data, &src, &dst);

        assert_eq!(header.get_custom_options().len(), 1);
        assert_eq!(header.get_custom_options()[0].name, "Option1");
        assert_eq!(header.get_custom_options()[0].option, vec![100, 101, 102]);
        assert_eq!(header.get_custom_options()[0].selected_index, 1);
    }

    #[test]
    fn test_convert_header_with_offsets() {
        let mut header_data = make_test_header_data();
        header_data.custom_offsets = vec![CustomOffsetData {
            name: "Offset1".to_string(),
            id: 900,
            x: true,
            y: true,
            w: false,
            h: false,
            r: false,
            a: false,
        }];

        let src = Resolution {
            width: 1920.0,
            height: 1080.0,
        };
        let dst = make_test_dst();
        let header = convert_header_data(&header_data, &src, &dst);

        assert_eq!(header.get_custom_offsets().len(), 1);
        assert_eq!(header.get_custom_offsets()[0].name, "Offset1");
        assert_eq!(header.get_custom_offsets()[0].id, 900);
    }

    // -- Test: empty SkinData -> Skin --

    #[test]
    fn test_convert_empty_skin_data() {
        let header_data = make_test_header_data();
        let data = SkinData::new();
        let mut source_map = HashMap::new();
        let dst = make_test_dst();

        let skin = convert_skin_data(
            &header_data,
            data,
            &mut source_map,
            Path::new("/test/skin.json"),
            false,
            &dst,
        );

        assert!(skin.is_some());
        let skin = skin.unwrap();
        assert_eq!(skin.get_all_skin_objects_count(), 0);
        assert_eq!(skin.get_custom_events_count(), 0);
        assert_eq!(skin.get_custom_timers_count(), 0);
    }

    // -- Test: skin with ImageById object --

    #[test]
    fn test_convert_skin_data_with_image_by_id() {
        let header_data = make_test_header_data();
        let mut data = SkinData::new();
        data.objects.push(DataSkinObjectData {
            name: Some("-1".to_string()),
            object_type: SkinObjectType::ImageById(1),
            destinations: vec![DestinationData {
                time: 0,
                x: 100,
                y: 200,
                w: 300,
                h: 400,
                acc: 0,
                a: 255,
                r: 255,
                g: 255,
                b: 255,
                blend: 0,
                filter: 0,
                angle: 0,
                center: 0,
                loop_val: 0,
                timer: None,
                op: vec![],
                draw: None,
            }],
            offset_ids: vec![],
            stretch: -1,
            mouse_rect: None,
        });

        let mut source_map = HashMap::new();
        let dst = make_test_dst();

        let skin = convert_skin_data(
            &header_data,
            data,
            &mut source_map,
            Path::new("/test/skin.json"),
            false,
            &dst,
        );

        assert!(skin.is_some());
        let skin = skin.unwrap();
        assert_eq!(skin.get_all_skin_objects_count(), 1);
        assert_eq!(skin.get_objects()[0].get_type_name(), "Image");
    }

    // -- Test: option wiring --

    #[test]
    fn test_option_wiring() {
        let mut header_data = make_test_header_data();
        header_data.custom_options = vec![CustomOptionData {
            name: "TestOpt".to_string(),
            option: vec![200, 201],
            names: vec!["Off".to_string(), "On".to_string()],
            def: None,
            selected_option: 201,
        }];

        let data = SkinData::new();
        let mut source_map = HashMap::new();
        let dst = make_test_dst();

        let skin = convert_skin_data(
            &header_data,
            data,
            &mut source_map,
            Path::new("/test/skin.json"),
            false,
            &dst,
        );

        let skin = skin.unwrap();
        let option = skin.get_option();
        // 200 is not selected => 0, 201 is selected => 1
        assert_eq!(option.get(&200), Some(&0));
        assert_eq!(option.get(&201), Some(&1));
    }

    // -- Test: offset wiring --

    #[test]
    fn test_offset_wiring() {
        let mut header_data = make_test_header_data();
        header_data.custom_offsets = vec![CustomOffsetData {
            name: "TestOffset".to_string(),
            id: 42,
            x: true,
            y: true,
            w: false,
            h: false,
            r: false,
            a: false,
        }];

        let data = SkinData::new();
        let mut source_map = HashMap::new();
        let dst = make_test_dst();

        let skin = convert_skin_data(
            &header_data,
            data,
            &mut source_map,
            Path::new("/test/skin.json"),
            false,
            &dst,
        );

        let skin = skin.unwrap();
        let offset = skin.get_offset();
        assert!(offset.contains_key(&42));
        assert_eq!(offset.get(&42).unwrap().name, "TestOffset");
    }

    // -- Test: fadeout/input/scene wiring --

    #[test]
    fn test_fadeout_input_scene() {
        let header_data = make_test_header_data();
        let mut data = SkinData::new();
        data.fadeout = 500;
        data.input = 100;
        data.scene = 60000;

        let mut source_map = HashMap::new();
        let dst = make_test_dst();

        let skin = convert_skin_data(
            &header_data,
            data,
            &mut source_map,
            Path::new("/test/skin.json"),
            false,
            &dst,
        );

        let skin = skin.unwrap();
        assert_eq!(skin.get_fadeout(), 500);
        assert_eq!(skin.get_input(), 100);
        assert_eq!(skin.get_scene(), 60000);
    }

    // -- Test: custom event/timer registration --

    #[test]
    fn test_custom_timer_registration() {
        let header_data = make_test_header_data();
        let mut data = SkinData::new();
        data.custom_timers.push(CustomTimerData {
            id: 10,
            timer: None,
        });
        data.custom_timers.push(CustomTimerData {
            id: 20,
            timer: Some(42),
        });

        let mut source_map = HashMap::new();
        let dst = make_test_dst();

        let skin = convert_skin_data(
            &header_data,
            data,
            &mut source_map,
            Path::new("/test/skin.json"),
            false,
            &dst,
        );

        let skin = skin.unwrap();
        assert_eq!(skin.get_custom_timers_count(), 2);
    }

    // -- Test: conversion with destinations --

    #[test]
    fn test_convert_with_destinations() {
        let header_data = make_test_header_data();
        let mut data = SkinData::new();
        data.objects.push(DataSkinObjectData {
            name: Some("-5".to_string()),
            object_type: SkinObjectType::ImageById(5),
            destinations: vec![
                DestinationData {
                    time: 0,
                    x: 0,
                    y: 0,
                    w: 100,
                    h: 100,
                    acc: 0,
                    a: 255,
                    r: 255,
                    g: 255,
                    b: 255,
                    blend: 0,
                    filter: 0,
                    angle: 0,
                    center: 0,
                    loop_val: 0,
                    timer: None,
                    op: vec![],
                    draw: None,
                },
                DestinationData {
                    time: 1000,
                    x: 100,
                    y: 100,
                    w: 200,
                    h: 200,
                    acc: 0,
                    a: 128,
                    r: 255,
                    g: 255,
                    b: 255,
                    blend: 0,
                    filter: 0,
                    angle: 0,
                    center: 0,
                    loop_val: 0,
                    timer: None,
                    op: vec![],
                    draw: None,
                },
            ],
            offset_ids: vec![],
            stretch: -1,
            mouse_rect: None,
        });

        let mut source_map = HashMap::new();
        let dst = make_test_dst();

        let skin = convert_skin_data(
            &header_data,
            data,
            &mut source_map,
            Path::new("/test/skin.json"),
            false,
            &dst,
        );

        let skin = skin.unwrap();
        assert_eq!(skin.get_all_skin_objects_count(), 1);
        // The object should have 2 destinations set via set_destination
        // We can verify the object data has destinations
        let obj = &skin.get_objects()[0];
        assert_eq!(obj.data().dst.len(), 2);
    }

    // -- Test: mouse rect --

    #[test]
    fn test_convert_with_mouse_rect() {
        let header_data = make_test_header_data();
        let mut data = SkinData::new();
        data.objects.push(DataSkinObjectData {
            name: Some("-1".to_string()),
            object_type: SkinObjectType::ImageById(1),
            destinations: vec![DestinationData {
                time: 0,
                x: 0,
                y: 0,
                w: 100,
                h: 100,
                acc: 0,
                a: 255,
                r: 255,
                g: 255,
                b: 255,
                blend: 0,
                filter: 0,
                angle: 0,
                center: 0,
                loop_val: 0,
                timer: None,
                op: vec![],
                draw: None,
            }],
            offset_ids: vec![],
            stretch: -1,
            mouse_rect: Some(RectData {
                x: 10,
                y: 20,
                w: 30,
                h: 40,
            }),
        });

        let mut source_map = HashMap::new();
        let dst = make_test_dst();

        let skin = convert_skin_data(
            &header_data,
            data,
            &mut source_map,
            Path::new("/test/skin.json"),
            false,
            &dst,
        );

        let skin = skin.unwrap();
        assert_eq!(skin.get_all_skin_objects_count(), 1);
        // Mouse rect is set — verify via the object's mouse_rect field
        let obj = &skin.get_objects()[0];
        assert!(obj.data().mouse_rect.is_some());
    }

    // -- Test: stub types return None --

    #[test]
    fn test_stub_types_return_none() {
        let mut source_map = HashMap::new();
        let path = Path::new("/test/skin.json");

        assert!(
            convert_skin_object(
                &SkinObjectType::Bga { bga_expand: 0 },
                &mut source_map,
                path,
                false
            )
            .is_none()
        );
        assert!(
            convert_skin_object(
                &SkinObjectType::GaugeGraph {
                    color: None,
                    assist_clear_bg_color: String::new(),
                    assist_and_easy_fail_bg_color: String::new(),
                    groove_fail_bg_color: String::new(),
                    groove_clear_and_hard_bg_color: String::new(),
                    ex_hard_bg_color: String::new(),
                    hazard_bg_color: String::new(),
                    assist_clear_line_color: String::new(),
                    assist_and_easy_fail_line_color: String::new(),
                    groove_fail_line_color: String::new(),
                    groove_clear_and_hard_line_color: String::new(),
                    ex_hard_line_color: String::new(),
                    hazard_line_color: String::new(),
                    borderline_color: String::new(),
                    border_color: String::new(),
                },
                &mut source_map,
                path,
                false,
            )
            .is_none()
        );
    }

    #[test]
    fn test_note_judge_songlist_return_some() {
        let mut source_map = HashMap::new();
        let path = Path::new("/test/skin.json");

        let note = convert_skin_object(&SkinObjectType::Note, &mut source_map, path, false);
        assert!(note.is_some());
        assert_eq!(note.unwrap().get_type_name(), "SkinNote");

        let judge = convert_skin_object(
            &SkinObjectType::Judge {
                index: 0,
                shift: false,
            },
            &mut source_map,
            path,
            false,
        );
        assert!(judge.is_some());
        assert_eq!(judge.unwrap().get_type_name(), "SkinJudge");

        let bar = convert_skin_object(
            &SkinObjectType::SongList {
                center: 5,
                clickable: vec![],
            },
            &mut source_map,
            path,
            false,
        );
        assert!(bar.is_some());
        assert_eq!(bar.unwrap().get_type_name(), "SkinBar");
    }
}
