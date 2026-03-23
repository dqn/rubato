use std::collections::HashMap;
use std::path::Path;

use crate::json::json_skin_loader::{
    SkinObjectData as LoaderSkinObjectData, SkinObjectType, SongListBarData, SourceData,
};
use crate::json::json_skin_object_loader::source_image;
use crate::objects::skin_image::SkinImage;
use crate::objects::skin_number::SkinNumber;
use crate::reexports::{Rectangle, TextureRegion};
use crate::types::skin::SkinObject;

use super::object_converter::convert_skin_object;
use super::texture_resolution::get_texture_for_src;

/// Build SelectBarData from resolved JSON SongList bar sub-objects.
/// Each sub-SkinObjectData is converted to the appropriate skin type
/// (SkinImage, SkinNumber, SkinTextFont) and stored in SelectBarData.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_select_bar_data(
    bar_data: &SongListBarData,
    center: i32,
    clickable: &[i32],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_x: f32,
    scale_y: f32,
    filemap: &HashMap<String, String>,
) -> crate::select_bar_data::SelectBarData {
    crate::select_bar_data::SelectBarData {
        barimageon: convert_bar_sub_images(
            &bar_data.liston,
            source_map,
            skin_path,
            usecim,
            scale_x,
            scale_y,
            filemap,
        ),
        barimageoff: convert_bar_sub_images(
            &bar_data.listoff,
            source_map,
            skin_path,
            usecim,
            scale_x,
            scale_y,
            filemap,
        ),
        center_bar: center,
        clickable_bar: clickable.to_vec(),
        barlevel: convert_bar_sub_numbers(
            &bar_data.level,
            source_map,
            skin_path,
            usecim,
            scale_x,
            scale_y,
            filemap,
        ),
        bartext: convert_bar_sub_text(
            &bar_data.text,
            source_map,
            skin_path,
            usecim,
            scale_x,
            scale_y,
            filemap,
        ),
        barlamp: convert_bar_sub_images(
            &bar_data.lamp,
            source_map,
            skin_path,
            usecim,
            scale_x,
            scale_y,
            filemap,
        ),
        barmylamp: convert_bar_sub_images(
            &bar_data.playerlamp,
            source_map,
            skin_path,
            usecim,
            scale_x,
            scale_y,
            filemap,
        ),
        barrivallamp: convert_bar_sub_images(
            &bar_data.rivallamp,
            source_map,
            skin_path,
            usecim,
            scale_x,
            scale_y,
            filemap,
        ),
        bartrophy: convert_bar_sub_images(
            &bar_data.trophy,
            source_map,
            skin_path,
            usecim,
            scale_x,
            scale_y,
            filemap,
        ),
        barlabel: convert_bar_sub_images(
            &bar_data.label,
            source_map,
            skin_path,
            usecim,
            scale_x,
            scale_y,
            filemap,
        ),
        graph_type: resolve_graph_type(bar_data.graph.as_ref()),
        graph_images: resolve_graph_images(
            bar_data.graph.as_ref(),
            source_map,
            skin_path,
            usecim,
            filemap,
        ),
        graph_region: resolve_graph_region(bar_data.graph.as_ref(), scale_x, scale_y),
    }
}

fn convert_bar_sub_images(
    objs: &[Option<LoaderSkinObjectData>],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_x: f32,
    scale_y: f32,
    filemap: &HashMap<String, String>,
) -> Vec<Option<SkinImage>> {
    objs.iter()
        .map(|opt_obj| {
            let obj_data = opt_obj.as_ref()?;
            let skin_obj = convert_skin_object(
                &obj_data.object_type,
                source_map,
                skin_path,
                usecim,
                scale_x,
                scale_y,
                filemap,
            )?;
            if let SkinObject::Image(mut img) = skin_obj {
                apply_scaled_destinations_with_offsets(
                    &mut img.data,
                    &obj_data.destinations,
                    scale_x,
                    scale_y,
                    &obj_data.offset_ids,
                    obj_data.stretch,
                );
                Some(img)
            } else {
                None
            }
        })
        .collect()
}

fn convert_bar_sub_text(
    objs: &[Option<LoaderSkinObjectData>],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_x: f32,
    scale_y: f32,
    filemap: &HashMap<String, String>,
) -> Vec<Option<crate::skin_text::SkinTextEnum>> {
    objs.iter()
        .map(|opt_obj| {
            let obj_data = opt_obj.as_ref()?;
            let skin_obj = convert_skin_object(
                &obj_data.object_type,
                source_map,
                skin_path,
                usecim,
                scale_x,
                scale_y,
                filemap,
            )?;
            match skin_obj {
                SkinObject::TextFont(mut stf) => {
                    apply_scaled_destinations_with_offsets(
                        &mut stf.text_data.data,
                        &obj_data.destinations,
                        scale_x,
                        scale_y,
                        &obj_data.offset_ids,
                        obj_data.stretch,
                    );
                    Some(crate::skin_text::SkinTextEnum::Font(stf))
                }
                SkinObject::TextBitmap(mut stb) => {
                    apply_scaled_destinations_with_offsets(
                        &mut stb.text_data.data,
                        &obj_data.destinations,
                        scale_x,
                        scale_y,
                        &obj_data.offset_ids,
                        obj_data.stretch,
                    );
                    Some(crate::skin_text::SkinTextEnum::Bitmap(stb))
                }
                _ => None,
            }
        })
        .collect()
}

fn convert_bar_sub_numbers(
    objs: &[Option<LoaderSkinObjectData>],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_x: f32,
    scale_y: f32,
    filemap: &HashMap<String, String>,
) -> Vec<Option<SkinNumber>> {
    objs.iter()
        .map(|opt_obj| {
            let obj_data = opt_obj.as_ref()?;
            let skin_obj = convert_skin_object(
                &obj_data.object_type,
                source_map,
                skin_path,
                usecim,
                scale_x,
                scale_y,
                filemap,
            )?;
            if let SkinObject::Number(mut num) = skin_obj {
                apply_scaled_destinations_with_offsets(
                    &mut num.data,
                    &obj_data.destinations,
                    scale_x,
                    scale_y,
                    &obj_data.offset_ids,
                    obj_data.stretch,
                );
                Some(num)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
fn apply_scaled_destinations(
    data: &mut crate::skin_object::SkinObjectData,
    destinations: &[crate::json::json_skin_loader::DestinationData],
    scale_x: f32,
    scale_y: f32,
) {
    apply_scaled_destinations_with_offsets(data, destinations, scale_x, scale_y, &[], -1);
}

fn apply_scaled_destinations_with_offsets(
    data: &mut crate::skin_object::SkinObjectData,
    destinations: &[crate::json::json_skin_loader::DestinationData],
    scale_x: f32,
    scale_y: f32,
    offset_ids: &[i32],
    stretch: i32,
) {
    for dst in destinations {
        let timer_id = dst.timer.unwrap_or(0);
        let params = crate::skin_object::DestinationParams {
            time: dst.time as i64,
            x: dst.x as f32 * scale_x,
            y: dst.y as f32 * scale_y,
            w: dst.w as f32 * scale_x,
            h: dst.h as f32 * scale_y,
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
        };

        // Handle draw condition: replicate the logic from mod.rs for main skin objects.
        // draw_id -1 is a Lua expression sentinel (always draw); skip boolean_property lookup.
        if let Some(draw_id) = dst.draw
            && draw_id != 0
            && draw_id != -1
            && let Some(draw_prop) =
                crate::property::boolean_property_factory::boolean_property(draw_id)
        {
            data.set_destination_with_int_timer_draw(&params, timer_id, draw_prop);
            continue;
        }

        data.set_destination_with_int_timer_ops(&params, timer_id, &dst.op);
    }

    // Apply offset IDs and stretch after all destinations are set (matching mod.rs logic)
    if !offset_ids.is_empty() {
        data.set_offset_id(offset_ids);
    }
    if stretch >= 0 {
        data.set_stretch_by_id(stretch);
    }
}

/// Extract graph_type from a resolved graph SkinObjectData.
fn resolve_graph_type(graph: Option<&LoaderSkinObjectData>) -> Option<i32> {
    let graph = graph?;
    match &graph.object_type {
        SkinObjectType::Graph { graph_type, .. }
        | SkinObjectType::DistributionGraph { graph_type, .. } => Some(*graph_type),
        _ => None,
    }
}

/// Resolve graph source textures into TextureRegion vec for bar distribution graph.
fn resolve_graph_images(
    graph: Option<&LoaderSkinObjectData>,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    filemap: &HashMap<String, String>,
) -> Option<Vec<TextureRegion>> {
    let graph = graph?;
    let (src, x, y, w, h, divx, divy) = match &graph.object_type {
        SkinObjectType::Graph {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            ..
        } => (src.as_deref(), *x, *y, *w, *h, *divx, *divy),
        SkinObjectType::DistributionGraph {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            ..
        } => (src.as_deref(), *x, *y, *w, *h, *divx, *divy),
        _ => return None,
    };
    let tex = get_texture_for_src(src, source_map, skin_path, usecim, filemap)?;
    Some(source_image(&tex, x, y, w, h, divx, divy))
}

/// Extract graph_region from the first destination entry of a resolved graph SkinObjectData.
fn resolve_graph_region(
    graph: Option<&LoaderSkinObjectData>,
    scale_x: f32,
    scale_y: f32,
) -> Rectangle {
    let graph = match graph {
        Some(g) => g,
        None => return Rectangle::default(),
    };
    // Use the first destination entry's x, y, w, h for the graph region
    match graph.destinations.first() {
        Some(dst) => Rectangle::new(
            dst.x as f32 * scale_x,
            dst.y as f32 * scale_y,
            dst.w as f32 * scale_x,
            dst.h as f32 * scale_y,
        ),
        None => Rectangle::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::json_skin_loader::DestinationData;
    use crate::skin_object::SkinObjectData;

    #[test]
    fn apply_scaled_destinations_propagates_draw_condition() {
        let mut data = SkinObjectData::default();
        // draw_id=1 is a valid boolean property (OPTION_PANEL1 in the factory)
        let dst = DestinationData {
            draw: Some(1),
            a: 255,
            r: 255,
            g: 255,
            b: 255,
            ..Default::default()
        };
        apply_scaled_destinations(&mut data, &[dst], 1.0, 1.0);
        assert!(
            !data.dstdraw.is_empty(),
            "draw condition from DestinationData.draw should be set on SkinObjectData.dstdraw"
        );
    }

    #[test]
    fn apply_scaled_destinations_skips_draw_zero() {
        let mut data = SkinObjectData::default();
        let dst = DestinationData {
            draw: Some(0),
            ..Default::default()
        };
        apply_scaled_destinations(&mut data, &[dst], 1.0, 1.0);
        assert!(
            data.dstdraw.is_empty(),
            "draw_id=0 should not produce a draw condition"
        );
    }

    #[test]
    fn apply_scaled_destinations_skips_lua_sentinel() {
        let mut data = SkinObjectData::default();
        let dst = DestinationData {
            draw: Some(-1),
            ..Default::default()
        };
        apply_scaled_destinations(&mut data, &[dst], 1.0, 1.0);
        assert!(
            data.dstdraw.is_empty(),
            "draw_id=-1 (Lua sentinel) should not produce a draw condition"
        );
    }

    #[test]
    fn apply_scaled_destinations_no_draw_falls_through_to_ops() {
        let mut data = SkinObjectData::default();
        let dst = DestinationData {
            draw: None,
            op: vec![1],
            a: 255,
            r: 255,
            g: 255,
            b: 255,
            ..Default::default()
        };
        apply_scaled_destinations(&mut data, &[dst], 1.0, 1.0);
        // With draw=None, the op-based path should be taken.
        // Op ID 1 is a valid boolean property, so it goes into dstdraw via set_draw_condition_from_ops.
        assert!(
            !data.dstdraw.is_empty(),
            "without draw field, ops should be processed via set_destination_with_int_timer_ops"
        );
    }
}
