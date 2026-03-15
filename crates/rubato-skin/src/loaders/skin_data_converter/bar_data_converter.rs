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

use super::object_converter::{apply_destinations, convert_skin_object};
use super::texture_resolution::get_texture_for_src;

/// Build SelectBarData from resolved JSON SongList bar sub-objects.
/// Each sub-SkinObjectData is converted to the appropriate skin type
/// (SkinImage, SkinNumber, SkinTextFont) and stored in SelectBarData.
pub(super) fn build_select_bar_data(
    bar_data: &SongListBarData,
    center: i32,
    clickable: &[i32],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_y: f32,
) -> crate::select_bar_data::SelectBarData {
    crate::select_bar_data::SelectBarData {
        barimageon: convert_bar_sub_images(
            &bar_data.liston,
            source_map,
            skin_path,
            usecim,
            scale_y,
        ),
        barimageoff: convert_bar_sub_images(
            &bar_data.listoff,
            source_map,
            skin_path,
            usecim,
            scale_y,
        ),
        center_bar: center,
        clickable_bar: clickable.to_vec(),
        barlevel: convert_bar_sub_numbers(&bar_data.level, source_map, skin_path, usecim, scale_y),
        bartext: convert_bar_sub_text(&bar_data.text, source_map, skin_path, usecim, scale_y),
        barlamp: convert_bar_sub_images(&bar_data.lamp, source_map, skin_path, usecim, scale_y),
        barmylamp: convert_bar_sub_images(
            &bar_data.playerlamp,
            source_map,
            skin_path,
            usecim,
            scale_y,
        ),
        barrivallamp: convert_bar_sub_images(
            &bar_data.rivallamp,
            source_map,
            skin_path,
            usecim,
            scale_y,
        ),
        bartrophy: convert_bar_sub_images(&bar_data.trophy, source_map, skin_path, usecim, scale_y),
        barlabel: convert_bar_sub_images(&bar_data.label, source_map, skin_path, usecim, scale_y),
        graph_type: resolve_graph_type(bar_data.graph.as_ref()),
        graph_images: resolve_graph_images(bar_data.graph.as_ref(), source_map, skin_path, usecim),
        graph_region: resolve_graph_region(bar_data.graph.as_ref()),
    }
}

fn convert_bar_sub_images(
    objs: &[Option<LoaderSkinObjectData>],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_y: f32,
) -> Vec<Option<SkinImage>> {
    objs.iter()
        .map(|opt_obj| {
            let obj_data = opt_obj.as_ref()?;
            let skin_obj = convert_skin_object(
                &obj_data.object_type,
                source_map,
                skin_path,
                usecim,
                scale_y,
            )?;
            if let SkinObject::Image(mut img) = skin_obj {
                apply_destinations(&mut img.data, &obj_data.destinations);
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
    scale_y: f32,
) -> Vec<Option<crate::skin_text::SkinTextEnum>> {
    objs.iter()
        .map(|opt_obj| {
            let obj_data = opt_obj.as_ref()?;
            let skin_obj = convert_skin_object(
                &obj_data.object_type,
                source_map,
                skin_path,
                usecim,
                scale_y,
            )?;
            if let SkinObject::TextFont(mut stf) = skin_obj {
                apply_destinations(&mut stf.text_data.data, &obj_data.destinations);
                Some(crate::skin_text::SkinTextEnum::Font(stf))
            } else {
                None
            }
        })
        .collect()
}

fn convert_bar_sub_numbers(
    objs: &[Option<LoaderSkinObjectData>],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_y: f32,
) -> Vec<Option<SkinNumber>> {
    objs.iter()
        .map(|opt_obj| {
            let obj_data = opt_obj.as_ref()?;
            let skin_obj = convert_skin_object(
                &obj_data.object_type,
                source_map,
                skin_path,
                usecim,
                scale_y,
            )?;
            if let SkinObject::Number(mut num) = skin_obj {
                apply_destinations(&mut num.data, &obj_data.destinations);
                Some(num)
            } else {
                None
            }
        })
        .collect()
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
    let tex = get_texture_for_src(src, source_map, skin_path, usecim)?;
    Some(source_image(&tex, x, y, w, h, divx, divy))
}

/// Extract graph_region from the first destination entry of a resolved graph SkinObjectData.
fn resolve_graph_region(graph: Option<&LoaderSkinObjectData>) -> Rectangle {
    let graph = match graph {
        Some(g) => g,
        None => return Rectangle::default(),
    };
    // Use the first destination entry's x, y, w, h for the graph region
    match graph.destinations.first() {
        Some(dst) => Rectangle::new(dst.x as f32, dst.y as f32, dst.w as f32, dst.h as f32),
        None => Rectangle::default(),
    }
}
