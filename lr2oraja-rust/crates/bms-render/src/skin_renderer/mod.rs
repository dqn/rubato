// Main skin render system.
//
// Each frame, iterates over Skin.objects in order, resolves draw conditions,
// interpolates animations, applies offsets, and updates Bevy entities.

mod child_spawners;
pub mod components;
mod queries;
mod setup;
mod text_renderer;

pub use components::*;
pub use setup::setup_skin;

use std::hash::{Hash, Hasher};

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use bms_skin::skin_object_type::SkinObjectType;

use crate::bga_layer_material::BgaLayerMaterial;
use crate::coord::skin_to_bevy_transform;
use crate::distance_field_material::DistanceFieldMaterial;
use crate::draw;
use crate::eval;
use crate::state_provider::SkinStateProvider;
use crate::texture_map::TextureMap;

use child_spawners::{
    compute_multi_entity_hash, generate_procedural_pixels, spawn_bar_children, spawn_bga_children,
    spawn_distribution_children, spawn_float_children, spawn_gauge_children, spawn_judge_children,
    spawn_number_children,
};

// ---------------------------------------------------------------------------
// Per-frame render system
// ---------------------------------------------------------------------------

/// Per-frame system that updates all skin object entities.
///
/// Uses three query sets:
/// - Sprite entities (images, sliders, graphs, etc.)
/// - TTF text entities (Text2d-based)
/// - BMFont text entities (glyph sprite children)
#[allow(clippy::too_many_arguments)]
pub fn skin_render_system(
    mut commands: Commands,
    render_state: Option<Res<SkinRenderState>>,
    mut sprite_query: queries::SpriteQuery,
    mut ttf_query: queries::TtfTextQuery,
    mut bitmap_query: queries::BitmapTextQuery,
    mut shadow_query: queries::TtfShadowQuery,
    mut multi_entity_query: queries::MultiEntityQuery,
    mut procedural_query: queries::ProceduralTextureQuery,
    mut meshes: ResMut<Assets<Mesh>>,
    mut df_materials: ResMut<Assets<DistanceFieldMaterial>>,
    mut bga_layer_materials: ResMut<Assets<BgaLayerMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(state) = render_state else {
        return;
    };

    let skin = &state.skin;
    let provider = &*state.state_provider;
    let tex_map = &state.texture_map;

    // --- Sprite entities ---
    for (marker, mut transform, mut visibility, mut sprite) in &mut sprite_query {
        let idx = marker.object_index;
        if idx >= skin.objects.len() {
            *visibility = Visibility::Hidden;
            continue;
        }

        let object = &skin.objects[idx];
        let base = object.base();

        if !eval::check_option_conditions(base, skin, provider) {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some((rect, color, final_angle, final_alpha)) = eval::resolve_common(base, provider)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

        // Bevy sprite picking panics on non-positive half_size
        if rect.w <= 0.0 || rect.h <= 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }

        // Object-type-specific dispatch
        let time = eval::resolve_timer_time(base, provider).unwrap_or(0);
        let (tex_handle, src_rect_uv) = resolve_object_texture(object, provider, tex_map, time);

        // Update entity
        *transform = skin_to_bevy_transform(
            crate::coord::SkinRect {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: rect.h,
            },
            crate::coord::ScreenSize {
                w: skin.width,
                h: skin.height,
            },
            idx as f32 * 0.001,
            crate::coord::RotationParams {
                angle_deg: final_angle,
                center_x: base.center_x,
                center_y: base.center_y,
            },
        );

        sprite.custom_size = Some(Vec2::new(rect.w, rect.h));
        sprite.color = Color::srgba(color.r, color.g, color.b, final_alpha);

        if let Some(handle) = tex_handle {
            sprite.image = handle;
        }

        if let Some(uv_rect) = src_rect_uv {
            sprite.rect = Some(uv_rect);
        } else {
            sprite.rect = None;
        }

        *visibility = Visibility::Visible;
    }

    // --- TTF text entities ---
    text_renderer::render_ttf_text(&mut ttf_query, skin, provider, &state);

    // --- TTF shadow entities ---
    text_renderer::render_ttf_shadow(&mut shadow_query, skin, provider, &state);

    // --- BMFont text entities ---
    text_renderer::render_bitmap_text(
        &mut commands,
        &mut bitmap_query,
        skin,
        provider,
        &state.font_map,
        &mut meshes,
        &mut df_materials,
    );

    // --- Multi-entity objects (Number, Gauge, Judge, Float, DistributionGraph) ---
    for (entity, marker, mut transform, mut visibility, mut cached_hash) in &mut multi_entity_query
    {
        let idx = marker.object_index;
        if idx >= skin.objects.len() {
            *visibility = Visibility::Hidden;
            continue;
        }

        let object = &skin.objects[idx];
        let base = object.base();

        if !eval::check_option_conditions(base, skin, provider) {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some((rect, color, final_angle, final_alpha)) = eval::resolve_common(base, provider)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let time = eval::resolve_timer_time(base, provider).unwrap_or(0);

        *transform = skin_to_bevy_transform(
            crate::coord::SkinRect {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: rect.h,
            },
            crate::coord::ScreenSize {
                w: skin.width,
                h: skin.height,
            },
            idx as f32 * 0.001,
            crate::coord::RotationParams {
                angle_deg: final_angle,
                center_x: base.center_x,
                center_y: base.center_y,
            },
        );

        let obj_color = bevy::prelude::Color::srgba(color.r, color.g, color.b, final_alpha);

        // Compute a hash of the current state for change detection
        let new_hash = compute_multi_entity_hash(
            object,
            provider,
            time,
            &rect,
            state.bar_scroll_state.as_ref(),
        );

        if new_hash != cached_hash.0 {
            commands.entity(entity).despawn_descendants();

            match object {
                SkinObjectType::Number(num) => {
                    spawn_number_children(
                        &mut commands,
                        entity,
                        num,
                        provider,
                        tex_map,
                        time,
                        &rect,
                        obj_color,
                    );
                }
                SkinObjectType::Float(float_obj) => {
                    spawn_float_children(
                        &mut commands,
                        entity,
                        float_obj,
                        provider,
                        tex_map,
                        time,
                        &rect,
                        obj_color,
                    );
                }
                SkinObjectType::Gauge(gauge) => {
                    spawn_gauge_children(
                        &mut commands,
                        entity,
                        gauge,
                        provider,
                        tex_map,
                        time,
                        &rect,
                        obj_color,
                    );
                }
                SkinObjectType::Judge(judge) => {
                    spawn_judge_children(
                        &mut commands,
                        entity,
                        judge,
                        provider,
                        tex_map,
                        time,
                        &rect,
                        obj_color,
                    );
                }
                SkinObjectType::DistributionGraph(dg) => {
                    spawn_distribution_children(
                        &mut commands,
                        entity,
                        dg,
                        provider,
                        tex_map,
                        &rect,
                        obj_color,
                    );
                }
                SkinObjectType::Bar(bar) => {
                    if let Some(bar_state) = &state.bar_scroll_state {
                        spawn_bar_children(
                            &mut commands,
                            entity,
                            bar,
                            bar_state,
                            provider,
                            tex_map,
                            time,
                            &rect,
                            obj_color,
                            skin.width,
                            skin.height,
                        );
                    }
                }
                SkinObjectType::Bga(_) => {
                    spawn_bga_children(
                        &mut commands,
                        entity,
                        provider,
                        &rect,
                        obj_color,
                        &mut meshes,
                        &mut bga_layer_materials,
                    );
                }
                _ => {}
            }

            cached_hash.0 = new_hash;
        }

        *visibility = Visibility::Visible;
    }

    // --- Procedural texture objects (BpmGraph, HitErrorVisualizer, etc.) ---
    for (marker, mut transform, mut visibility, mut sprite, mut proc_state) in &mut procedural_query
    {
        let idx = marker.object_index;
        if idx >= skin.objects.len() {
            *visibility = Visibility::Hidden;
            continue;
        }

        let object = &skin.objects[idx];
        let base = object.base();

        if !eval::check_option_conditions(base, skin, provider) {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some((rect, color, final_angle, final_alpha)) = eval::resolve_common(base, provider)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

        // Bevy sprite picking panics on non-positive half_size
        if rect.w <= 0.0 || rect.h <= 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }

        *transform = skin_to_bevy_transform(
            crate::coord::SkinRect {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: rect.h,
            },
            crate::coord::ScreenSize {
                w: skin.width,
                h: skin.height,
            },
            idx as f32 * 0.001,
            crate::coord::RotationParams {
                angle_deg: final_angle,
                center_x: base.center_x,
                center_y: base.center_y,
            },
        );

        sprite.custom_size = Some(Vec2::new(rect.w, rect.h));
        sprite.color = bevy::prelude::Color::srgba(color.r, color.g, color.b, final_alpha);

        let width = rect.w.max(1.0) as u32;
        let height = rect.h.max(1.0) as u32;

        let pixels = generate_procedural_pixels(
            object,
            provider,
            width,
            height,
            &state.bpm_events,
            &state.note_distribution,
        );

        if let Some(pixels) = pixels {
            let mut hasher = std::hash::DefaultHasher::new();
            pixels.hash(&mut hasher);
            let new_hash = hasher.finish();

            if new_hash != proc_state.hash {
                let bevy_image = Image::new(
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    pixels,
                    TextureFormat::Rgba8UnormSrgb,
                    bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
                );
                let handle = images.add(bevy_image);
                sprite.image = handle.clone();
                proc_state.handle = Some(handle);
                proc_state.hash = new_hash;
            } else if let Some(ref handle) = proc_state.handle {
                sprite.image = handle.clone();
            }
        }

        *visibility = Visibility::Visible;
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Resolves the texture handle and optional UV rect for a skin object.
fn resolve_object_texture(
    object: &SkinObjectType,
    provider: &dyn SkinStateProvider,
    tex_map: &TextureMap,
    time: i64,
) -> (Option<Handle<Image>>, Option<bevy::math::Rect>) {
    match object {
        SkinObjectType::Image(img) => {
            // Select source based on ref_id
            let source_idx = img
                .ref_id
                .map(|id| provider.integer_value(id) as usize)
                .unwrap_or(0);
            let source = img.sources.get(source_idx).or(img.sources.first());

            if let Some(source) = source {
                match source {
                    bms_skin::skin_image::SkinImageSource::Frames { images, cycle, .. } => {
                        let idx = bms_skin::skin_source::image_index(images.len(), time, *cycle);
                        if let Some(handle) = images.get(idx)
                            && let Some(entry) = tex_map.get(*handle)
                        {
                            // Use per-source rect if available, fall back to shared source_rect
                            let rect = img
                                .source_rects
                                .get(source_idx)
                                .and_then(|r| r.as_ref())
                                .or(img.source_rect.as_ref());
                            let uv =
                                rect.map(|r| bevy::math::Rect::new(r.x, r.y, r.x + r.w, r.y + r.h));
                            return (Some(entry.handle.clone()), uv);
                        }
                    }
                    bms_skin::skin_image::SkinImageSource::Reference(_id) => {
                        // Reference sources need runtime image table resolution (Phase 11)
                    }
                }
            }
            (None, None)
        }
        SkinObjectType::Slider(slider) => {
            let value = slider
                .ref_id
                .map(|id| provider.float_value(id))
                .unwrap_or(0.0);
            let (ox, oy) =
                draw::slider::compute_slider_offset(slider.direction, slider.range, value);
            // Slider offset is applied via transform, texture is from source_images
            let idx = bms_skin::skin_source::image_index(
                slider.source_images.len(),
                time,
                slider.source_cycle,
            );
            if let Some(handle) = slider.source_images.get(idx)
                && let Some(entry) = tex_map.get(*handle)
            {
                return (Some(entry.handle.clone()), None);
            }
            let _ = (ox, oy); // Offset should be applied to transform in Phase 11
            (None, None)
        }
        SkinObjectType::Graph(graph) => {
            let value = graph
                .ref_id
                .map(|id| provider.float_value(id))
                .unwrap_or(0.0);
            let idx = bms_skin::skin_source::image_index(
                graph.source_images.len(),
                time,
                graph.source_cycle,
            );
            if let Some(handle) = graph.source_images.get(idx)
                && let Some(entry) = tex_map.get(*handle)
            {
                let src = bms_skin::skin_object::Rect::new(0.0, 0.0, entry.width, entry.height);
                let dst = bms_skin::skin_object::Rect::new(0.0, 0.0, entry.width, entry.height);
                let cmd = draw::graph::compute_graph_draw(graph.direction, value, &src, &dst);
                let uv = bevy::math::Rect::new(
                    cmd.src_rect.x,
                    cmd.src_rect.y,
                    cmd.src_rect.x + cmd.src_rect.w,
                    cmd.src_rect.y + cmd.src_rect.h,
                );
                return (Some(entry.handle.clone()), Some(uv));
            }
            (None, None)
        }
        SkinObjectType::Hidden(h) => {
            let idx = bms_skin::skin_source::image_index(h.images.len(), time, h.cycle);
            if let Some(handle) = h.images.get(idx)
                && let Some(entry) = tex_map.get(*handle)
            {
                return (Some(entry.handle.clone()), None);
            }
            (None, None)
        }
        SkinObjectType::LiftCover(lc) => {
            let idx = bms_skin::skin_source::image_index(lc.images.len(), time, lc.cycle);
            if let Some(handle) = lc.images.get(idx)
                && let Some(entry) = tex_map.get(*handle)
            {
                return (Some(entry.handle.clone()), None);
            }
            (None, None)
        }
        // BGA is handled by multi-entity query (spawn_bga_children).
        // Multi-entity and procedural types are handled by dedicated queries.
        // Text is handled separately via TTF/BMFont queries.
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use crate::font_map::FontMap;
    use crate::state_provider::StaticStateProvider;
    use crate::texture_map::TextureMap;

    use super::*;

    #[test]
    fn skin_render_state_can_hold_static_provider() {
        let skin = bms_skin::skin::Skin::new(bms_skin::skin_header::SkinHeader::default());
        let tex_map = TextureMap::new();
        let font_map = FontMap::new();
        let provider = Box::new(StaticStateProvider::default());

        let state = SkinRenderState {
            skin,
            texture_map: tex_map,
            font_map,
            state_provider: provider,
            bar_scroll_state: None,
            bpm_events: Vec::new(),
            note_distribution: Vec::new(),
        };

        assert_eq!(state.skin.objects.len(), 0);
    }

    #[test]
    fn resolve_object_texture_bga_returns_none() {
        // BGA is handled by multi-entity (spawn_bga_children), not resolve_object_texture
        let provider = StaticStateProvider::default();
        let tex_map = TextureMap::new();
        let bga = bms_skin::skin_bga::SkinBga::default();
        let obj = SkinObjectType::Bga(bga);

        let (handle, uv) = resolve_object_texture(&obj, &provider, &tex_map, 0);
        assert!(handle.is_none());
        assert!(uv.is_none());
    }
}
