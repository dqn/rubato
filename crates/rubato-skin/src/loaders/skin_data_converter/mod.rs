// SkinData -> Skin converter (Phase 26b)
// Converts the intermediate SkinData representation into the runtime Skin object.

use std::collections::HashMap;
use std::path::Path;

use log::{debug, warn};

use crate::core::custom_event::CustomEvent;
use crate::core::custom_timer::CustomTimer;
use crate::graphs::skin_bpm_graph::SkinBPMGraph;
use crate::graphs::skin_graph::SkinGraph;
use crate::graphs::skin_hit_error_visualizer::SkinHitErrorVisualizer;
use crate::graphs::skin_note_distribution_graph::SkinNoteDistributionGraph;
use crate::graphs::skin_timing_distribution_graph::SkinTimingDistributionGraph;
use crate::graphs::skin_timing_visualizer::SkinTimingVisualizer;
use crate::json::json_skin_loader::{
    CustomCategoryData, CustomItemData, ResolvedImageEntry, SkinData, SkinHeaderData,
    SkinObjectData as LoaderSkinObjectData, SkinObjectType, SongListBarData, SourceData,
    SourceDataType,
};
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
use crate::property::boolean_property_factory;
use crate::property::event_factory;
use crate::property::string_property_factory;
use crate::property::timer_property_factory;
use crate::stubs::{Resolution, SkinConfigOffset, SkinOffset, TextureRegion};
use crate::text::skin_text_font::SkinTextFont;
use crate::types::skin::{Skin, SkinObject};
use crate::types::skin_bar_object::SkinBarObject;
use crate::types::skin_header::{
    CustomCategory, CustomFile, CustomItemEnum, CustomOffset, CustomOption, SkinHeader,
};
use crate::types::skin_type::SkinType;

/// Converts SkinHeaderData into a SkinHeader.
pub fn convert_header_data(
    data: &SkinHeaderData,
    src: &Resolution,
    dst: &Resolution,
) -> SkinHeader {
    let mut header = SkinHeader::new();

    // Map skin_type integer to SkinType enum
    if let Some(skin_type) = SkinType::skin_type_by_id(data.skin_type) {
        header.set_skin_type(skin_type);
    }

    header.set_name(data.name.clone());
    header.set_author(data.author.clone());
    header.set_path(data.path.clone());
    header.skin_type_id = data.header_type;

    // Set resolutions
    header.resolution = Resolution {
        width: src.width,
        height: src.height,
    };
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
    header.options = options;

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
    header.files = files;

    // Convert custom offsets
    let offsets: Vec<CustomOffset> = data
        .custom_offsets
        .iter()
        .map(|o| CustomOffset::new(o.name.clone(), o.id, o.caps))
        .collect();
    header.offsets = offsets;

    // Convert custom categories
    let categories: Vec<CustomCategory> = data
        .custom_categories
        .iter()
        .map(convert_category_data)
        .collect();
    header.categories = categories;

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
            CustomItemData::Offset(o) => {
                CustomItemEnum::Offset(CustomOffset::new(o.name.clone(), o.id, o.caps))
            }
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
    skin.option = op;

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
    skin.offset = offset;

    // Set skin timing
    skin.fadeout = data.fadeout;
    skin.input = data.input;
    skin.scene = data.scene;

    // Convert each SkinObjectData to a SkinObject
    for obj_data in &data.objects {
        let scale_y = dstr.height / src.height;
        let skin_obj = convert_skin_object(
            &obj_data.object_type,
            source_map,
            skin_path,
            usecim,
            scale_y,
        );

        if let Some(mut obj) = skin_obj {
            // Set name on the underlying SkinObjectData
            if let Some(ref name) = obj_data.name {
                obj.data_mut().name = Some(name.clone());
            }

            // Set click event from object type
            set_click_event_from_type(&mut obj, &obj_data.object_type);

            // Add the object to the skin
            skin.add(obj);
            let obj_index = skin.all_skin_objects_count() - 1;

            // Set destinations
            for dst in &obj_data.destinations {
                let timer_id = dst.timer.unwrap_or(0);

                // Handle draw condition
                if let Some(draw_id) = dst.draw
                    && draw_id != 0
                {
                    let timer_prop = if timer_id > 0 {
                        timer_property_factory::timer_property(timer_id)
                    } else {
                        None
                    };
                    if let Some(draw_prop) = boolean_property_factory::boolean_property(draw_id) {
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
                        timer_property_factory::timer_property(timer_id)
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
                && let Some(obj) = skin.objects_mut().get_mut(obj_index)
            {
                obj.data_mut().set_offset_id(&obj_data.offset_ids);
            }

            // Set stretch
            if obj_data.stretch >= 0
                && let Some(obj) = skin.objects_mut().get_mut(obj_index)
            {
                obj.data_mut().set_stretch_by_id(obj_data.stretch);
            }

            // For SongList, build SelectBarData from resolved bar sub-objects
            if let SkinObjectType::SongList {
                center,
                clickable,
                bar_data: Some(bar_data),
            } = &obj_data.object_type
            {
                skin.select_bar_data = Some(build_select_bar_data(
                    bar_data, *center, clickable, source_map, skin_path, usecim, scale_y,
                ));
            }
        }
    }

    // Add custom events
    for event_data in &data.custom_events {
        let action = event_data.action.and_then(event_factory::event_by_id);
        let condition = event_data
            .condition
            .and_then(boolean_property_factory::boolean_property);
        if let Some(action) = action {
            let event = CustomEvent::new(event_data.id, action, condition, event_data.min_interval);
            skin.add_custom_event(event);
        }
    }

    // Add custom timers
    for timer_data in &data.custom_timers {
        let timer_func = timer_data
            .timer
            .and_then(timer_property_factory::timer_property);
        let timer = CustomTimer::new(timer_data.id, timer_func);
        skin.add_custom_timer(timer);
    }

    Some(skin)
}

include!("object_converter.rs");

#[cfg(test)]
mod tests;
