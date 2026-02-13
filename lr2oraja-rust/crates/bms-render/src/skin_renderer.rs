// Main skin render system.
//
// Each frame, iterates over Skin.objects in order, resolves draw conditions,
// interpolates animations, applies offsets, and updates Bevy entities.

use std::hash::{Hash, Hasher};

use bevy::prelude::*;
use bevy::render::mesh::Mesh2d;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::MeshMaterial2d;

use bms_skin::skin::Skin;
use bms_skin::skin_object_type::SkinObjectType;
use bms_skin::skin_text::FontType;

use crate::coord::skin_to_bevy_transform;
use crate::distance_field_material::DistanceFieldMaterial;
use crate::draw;
use crate::draw::bmfont_text::layout_bmfont_text;
use crate::eval;
use crate::font_map::FontMap;
use crate::state_provider::SkinStateProvider;
use crate::texture_map::TextureMap;

// ---------------------------------------------------------------------------
// Marker components for skin object entities
// ---------------------------------------------------------------------------

/// Marker component for entities managed by the skin renderer.
#[derive(Component)]
pub struct SkinObjectEntity {
    /// Index into Skin.objects Vec.
    pub object_index: usize,
}

/// Marker component for TTF text entities (rendered via Bevy Text2d).
#[derive(Component)]
pub struct TtfTextMarker;

/// Marker component for BMFont text entities (rendered via glyph sprites).
#[derive(Component)]
pub struct BitmapTextMarker;

/// Marker component for child glyph sprites under a BMFont text entity.
#[derive(Component)]
pub struct BmFontGlyphChild;

/// Caches the last rendered text to avoid re-spawning glyph children every frame.
#[derive(Component, Default)]
pub struct CachedBmFontText(pub String);

/// Marker component for TTF shadow text entities.
#[derive(Component)]
pub struct TtfShadowMarker;

/// Marker component for multi-entity skin objects (Number, Gauge, Judge, Float, DistributionGraph).
/// These objects spawn child sprite entities for rendering.
#[derive(Component)]
pub struct MultiEntityMarker;

/// Marker component for child sprites under a multi-entity skin object.
#[derive(Component)]
pub struct MultiEntityChild;

/// Caches a hash of the last rendered state to avoid unnecessary child re-spawning.
#[derive(Component, Default)]
pub struct CachedMultiEntityHash(pub u64);

/// Marker component for procedural texture skin objects (BpmGraph, HitErrorVisualizer, etc.).
/// These render CPU-generated pixel buffers as Bevy Image textures.
#[derive(Component)]
pub struct ProceduralTextureMarker;

/// Tracks the Bevy Image handle and content hash for a procedural texture.
#[derive(Component, Default)]
pub struct ProceduralTextureState {
    pub handle: Option<Handle<Image>>,
    pub hash: u64,
}

// ---------------------------------------------------------------------------
// Type aliases for complex query types
// ---------------------------------------------------------------------------

type SpriteQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SkinObjectEntity,
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut Sprite,
    ),
    (
        Without<TtfTextMarker>,
        Without<BitmapTextMarker>,
        Without<TtfShadowMarker>,
        Without<MultiEntityMarker>,
        Without<ProceduralTextureMarker>,
    ),
>;

type TtfTextQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SkinObjectEntity,
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut Text2d,
        &'static mut TextFont,
        &'static mut TextColor,
    ),
    (
        With<TtfTextMarker>,
        Without<BitmapTextMarker>,
        Without<TtfShadowMarker>,
    ),
>;

type BitmapTextQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static SkinObjectEntity,
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut CachedBmFontText,
    ),
    (With<BitmapTextMarker>, Without<TtfTextMarker>),
>;

type TtfShadowQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SkinObjectEntity,
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut Text2d,
        &'static mut TextFont,
        &'static mut TextColor,
    ),
    (
        With<TtfShadowMarker>,
        Without<TtfTextMarker>,
        Without<BitmapTextMarker>,
    ),
>;

type MultiEntityQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static SkinObjectEntity,
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut CachedMultiEntityHash,
    ),
    With<MultiEntityMarker>,
>;

type ProceduralTextureQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SkinObjectEntity,
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut Sprite,
        &'static mut ProceduralTextureState,
    ),
    With<ProceduralTextureMarker>,
>;

// ---------------------------------------------------------------------------
// Bevy Resource holding the skin render state
// ---------------------------------------------------------------------------

/// Bevy Resource holding all skin rendering state.
#[derive(Resource)]
pub struct SkinRenderState {
    pub skin: Skin,
    pub texture_map: TextureMap,
    pub font_map: FontMap,
    pub state_provider: Box<dyn SkinStateProvider>,
}

// ---------------------------------------------------------------------------
// Setup: spawn entities for each skin object
// ---------------------------------------------------------------------------

/// Spawns one Bevy entity per skin object and inserts the SkinRenderState resource.
pub fn setup_skin(
    commands: &mut Commands,
    skin: Skin,
    texture_map: TextureMap,
    font_map: FontMap,
    state_provider: Box<dyn SkinStateProvider>,
) {
    let count = skin.objects.len();

    // Spawn one entity per skin object (initially invisible)
    for i in 0..count {
        let marker = SkinObjectEntity { object_index: i };

        match &skin.objects[i] {
            SkinObjectType::Text(text) => match &text.font_type {
                FontType::Bitmap { .. } => {
                    commands.spawn((
                        Transform::default(),
                        Visibility::Hidden,
                        marker,
                        BitmapTextMarker,
                        CachedBmFontText::default(),
                    ));
                }
                FontType::Ttf(_) | FontType::Default => {
                    // Spawn TTF shadow entity first (renders behind main text)
                    if text.shadow.is_some() {
                        commands.spawn((
                            Text2d::new(""),
                            TextFont::default(),
                            TextColor(Color::WHITE),
                            TextLayout::default(),
                            Transform::default(),
                            Visibility::Hidden,
                            SkinObjectEntity { object_index: i },
                            TtfShadowMarker,
                        ));
                    }

                    // TTF text: use Bevy Text2d for native font rendering.
                    // Text2d is spawned with a placeholder; updated each frame.
                    commands.spawn((
                        Text2d::new(""),
                        TextFont::default(),
                        TextColor(Color::WHITE),
                        TextLayout::default(),
                        Transform::default(),
                        Visibility::Hidden,
                        marker,
                        TtfTextMarker,
                    ));
                }
            },
            // Multi-entity types: Number, Gauge, Judge, Float, DistributionGraph
            // These spawn child sprites dynamically each frame.
            SkinObjectType::Number(_)
            | SkinObjectType::Float(_)
            | SkinObjectType::Gauge(_)
            | SkinObjectType::Judge(_)
            | SkinObjectType::DistributionGraph(_) => {
                commands.spawn((
                    Transform::default(),
                    Visibility::Hidden,
                    marker,
                    MultiEntityMarker,
                    CachedMultiEntityHash::default(),
                ));
            }
            // Procedural texture types: rendered from CPU pixel buffers.
            SkinObjectType::BpmGraph(_)
            | SkinObjectType::HitErrorVisualizer(_)
            | SkinObjectType::NoteDistributionGraph(_)
            | SkinObjectType::TimingDistributionGraph(_)
            | SkinObjectType::TimingVisualizer(_)
            | SkinObjectType::GaugeGraph(_) => {
                commands.spawn((
                    Sprite::default(),
                    Transform::default(),
                    Visibility::Hidden,
                    marker,
                    ProceduralTextureMarker,
                    ProceduralTextureState::default(),
                ));
            }
            _ => {
                commands.spawn((
                    Sprite::default(),
                    Transform::default(),
                    Visibility::Hidden,
                    marker,
                ));
            }
        }
    }

    commands.insert_resource(SkinRenderState {
        skin,
        texture_map,
        font_map,
        state_provider,
    });
}

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
    mut sprite_query: SpriteQuery,
    mut ttf_query: TtfTextQuery,
    mut bitmap_query: BitmapTextQuery,
    mut shadow_query: TtfShadowQuery,
    mut multi_entity_query: MultiEntityQuery,
    mut procedural_query: ProceduralTextureQuery,
    mut meshes: ResMut<Assets<Mesh>>,
    mut df_materials: ResMut<Assets<DistanceFieldMaterial>>,
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

        let Some((rect, color, final_angle, final_alpha)) = eval::resolve_common(base, provider)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

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
    for (marker, mut transform, mut visibility, mut text2d, mut text_font, mut text_color) in
        &mut ttf_query
    {
        let idx = marker.object_index;
        if idx >= skin.objects.len() {
            *visibility = Visibility::Hidden;
            continue;
        }

        let object = &skin.objects[idx];
        let base = object.base();

        let Some((rect, color, final_angle, final_alpha)) = eval::resolve_common(base, provider)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

        if let SkinObjectType::Text(skin_text) = object {
            // Resolve text content
            let content = eval::resolve_text_content(skin_text, provider);

            // Update Text2d content
            **text2d = content;

            // Update font size
            text_font.font_size = skin_text.font_size;

            // If a TTF font is loaded, set the font handle
            if let FontType::Ttf(path) = &skin_text.font_type
                && let Some(entry) = state.font_map.get_ttf(path)
            {
                text_font.font = entry.handle.clone();
            }

            // Update color
            *text_color = TextColor(Color::srgba(color.r, color.g, color.b, final_alpha));

            // Update transform
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

            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    // --- TTF shadow entities ---
    for (marker, mut transform, mut visibility, mut text2d, mut text_font, mut text_color) in
        &mut shadow_query
    {
        let idx = marker.object_index;
        if idx >= skin.objects.len() {
            *visibility = Visibility::Hidden;
            continue;
        }

        let object = &skin.objects[idx];
        let base = object.base();

        let Some((rect, color, final_angle, final_alpha)) = eval::resolve_common(base, provider)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

        if let SkinObjectType::Text(skin_text) = object
            && let Some(shadow) = &skin_text.shadow
        {
            let content = eval::resolve_text_content(skin_text, provider);
            **text2d = content;

            text_font.font_size = skin_text.font_size;
            if let FontType::Ttf(path) = &skin_text.font_type
                && let Some(entry) = state.font_map.get_ttf(path)
            {
                text_font.font = entry.handle.clone();
            }

            // Shadow color: RGB halved, same alpha (Java pattern)
            let (sr, sg, sb, sa) =
                eval::shadow_color_from_main(color.r, color.g, color.b, final_alpha);
            *text_color = TextColor(Color::srgba(sr, sg, sb, sa));

            // Shadow transform: same position + shadow offset, slightly behind main
            let shadow_z_order = idx as f32 * 0.001 - 0.0005;
            *transform = skin_to_bevy_transform(
                crate::coord::SkinRect {
                    x: rect.x + shadow.offset_x,
                    y: rect.y + shadow.offset_y,
                    w: rect.w,
                    h: rect.h,
                },
                crate::coord::ScreenSize {
                    w: skin.width,
                    h: skin.height,
                },
                shadow_z_order,
                crate::coord::RotationParams {
                    angle_deg: final_angle,
                    center_x: base.center_x,
                    center_y: base.center_y,
                },
            );
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    // --- BMFont text entities ---
    let font_map = &state.font_map;
    for (entity, marker, mut transform, mut visibility, mut cached) in &mut bitmap_query {
        let idx = marker.object_index;
        if idx >= skin.objects.len() {
            *visibility = Visibility::Hidden;
            continue;
        }

        let object = &skin.objects[idx];
        let base = object.base();

        let Some((rect, color, final_angle, final_alpha)) = eval::resolve_common(base, provider)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

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

        if let SkinObjectType::Text(skin_text) = object {
            let content = eval::resolve_text_content(skin_text, provider);

            // Only rebuild glyph children when text content changes
            if content != cached.0 {
                commands.entity(entity).despawn_descendants();

                if let FontType::Bitmap { path, bitmap_type } = &skin_text.font_type
                    && let Some(entry) = font_map.get_bitmap(path)
                {
                    let glyph_region = bms_skin::skin_object::Rect::new(0.0, 0.0, rect.w, rect.h);
                    let glyph_cmds = layout_bmfont_text(
                        &content,
                        &entry.data,
                        skin_text.font_size,
                        &glyph_region,
                        skin_text.align,
                        skin_text.overflow,
                    );
                    let glyph_color = Color::srgba(color.r, color.g, color.b, final_alpha);

                    let is_distance_field = *bitmap_type == 1 || *bitmap_type == 2;

                    if is_distance_field {
                        // Distance field glyphs: use Mesh2d + DistanceFieldMaterial
                        spawn_df_glyph_children(
                            &mut commands,
                            entity,
                            &glyph_cmds,
                            entry,
                            skin_text,
                            glyph_color,
                            rect.w,
                            rect.h,
                            &mut meshes,
                            &mut df_materials,
                        );
                    } else {
                        // Standard bitmap: use Sprite children with optional shadow
                        spawn_standard_glyph_children(
                            &mut commands,
                            entity,
                            &glyph_cmds,
                            entry,
                            skin_text,
                            glyph_color,
                            rect.w,
                            rect.h,
                        );
                    }
                }

                cached.0 = content;
            }
        }

        *visibility = Visibility::Visible;
    }

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
        let new_hash = compute_multi_entity_hash(object, provider, time, &rect);

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

        let Some((rect, color, final_angle, final_alpha)) = eval::resolve_common(base, provider)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

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

        let pixels = generate_procedural_pixels(object, provider, width, height);

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

/// Computes a hash of the current multi-entity state for change detection.
fn compute_multi_entity_hash(
    object: &SkinObjectType,
    provider: &dyn SkinStateProvider,
    time: i64,
    rect: &bms_skin::skin_object::Rect,
) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    time.hash(&mut hasher);
    rect.x.to_bits().hash(&mut hasher);
    rect.y.to_bits().hash(&mut hasher);
    rect.w.to_bits().hash(&mut hasher);
    rect.h.to_bits().hash(&mut hasher);

    match object {
        SkinObjectType::Number(num) => {
            0u8.hash(&mut hasher);
            let value = num.ref_id.map(|id| provider.integer_value(id)).unwrap_or(0);
            value.hash(&mut hasher);
        }
        SkinObjectType::Float(f) => {
            1u8.hash(&mut hasher);
            let value = f.ref_id.map(|id| provider.float_value(id)).unwrap_or(0.0);
            value.to_bits().hash(&mut hasher);
        }
        SkinObjectType::Gauge(g) => {
            2u8.hash(&mut hasher);
            // Gauge value from float provider (groove gauge ref)
            let value = provider.float_value(bms_skin::property_id::FloatId(107));
            value.to_bits().hash(&mut hasher);
            g.nodes.hash(&mut hasher);
        }
        SkinObjectType::Judge(j) => {
            3u8.hash(&mut hasher);
            j.player.hash(&mut hasher);
            // Current judge type
            let judge_type =
                provider.integer_value(bms_skin::property_id::IntegerId(if j.player == 0 {
                    75
                } else {
                    175
                }));
            judge_type.hash(&mut hasher);
            // Combo count
            let combo =
                provider.integer_value(bms_skin::property_id::IntegerId(if j.player == 0 {
                    71
                } else {
                    171
                }));
            combo.hash(&mut hasher);
        }
        SkinObjectType::DistributionGraph(dg) => {
            4u8.hash(&mut hasher);
            dg.graph_type.hash(&mut hasher);
        }
        _ => {}
    }

    hasher.finish()
}

/// Spawns child sprites for a SkinNumber.
#[allow(clippy::too_many_arguments)]
fn spawn_number_children(
    commands: &mut Commands,
    parent: Entity,
    num: &bms_skin::skin_number::SkinNumber,
    provider: &dyn SkinStateProvider,
    tex_map: &TextureMap,
    time: i64,
    rect: &bms_skin::skin_object::Rect,
    obj_color: bevy::prelude::Color,
) {
    let value = num.ref_id.map(|id| provider.integer_value(id)).unwrap_or(0);

    // Java: (value >= 0 || mimage == null) ? this.image : mimage
    let digit_images = if value < 0 {
        num.minus_digit_sources
            .as_ref()
            .and_then(|s| s.get_images(time))
            .or_else(|| num.digit_sources.get_images(time))
    } else {
        num.digit_sources.get_images(time)
    };
    let Some(digit_images) = digit_images else {
        return;
    };

    let digit_w = if num.keta > 0 {
        rect.w / num.keta as f32
    } else {
        rect.w
    };

    let config = draw::number::NumberConfig {
        keta: num.keta,
        zero_padding: num.zero_padding,
        align: num.align,
        space: num.space,
        digit_w,
        negative: num.minus_digit_sources.is_some(),
    };

    let dst = bms_skin::skin_object::Rect::new(0.0, 0.0, rect.w, rect.h);
    let cmds = draw::number::compute_number_draw(value, &dst, config);

    for cmd in &cmds {
        let src_idx = cmd.source_index as usize;
        if src_idx >= digit_images.len() {
            continue;
        }
        let region = &digit_images[src_idx];
        let Some(entry) = tex_map.get(region.handle) else {
            continue;
        };

        let local_x = cmd.dst_rect.x + cmd.dst_rect.w / 2.0 - rect.w / 2.0;
        let local_y = -(cmd.dst_rect.y + cmd.dst_rect.h / 2.0 - rect.h / 2.0);

        let texture_rect = if region.w > 0.0 && region.h > 0.0 {
            Some(bevy::math::Rect::new(
                region.x,
                region.y,
                region.x + region.w,
                region.y + region.h,
            ))
        } else {
            None
        };

        commands.entity(parent).with_child((
            Sprite {
                image: entry.handle.clone(),
                custom_size: Some(Vec2::new(cmd.dst_rect.w, cmd.dst_rect.h)),
                color: obj_color,
                rect: texture_rect,
                ..default()
            },
            Transform::from_xyz(local_x, local_y, 0.0001),
            MultiEntityChild,
        ));
    }
}

/// Spawns child sprites for a SkinFloat.
#[allow(clippy::too_many_arguments)]
fn spawn_float_children(
    commands: &mut Commands,
    parent: Entity,
    float_obj: &bms_skin::skin_float::SkinFloat,
    provider: &dyn SkinStateProvider,
    tex_map: &TextureMap,
    time: i64,
    rect: &bms_skin::skin_object::Rect,
    obj_color: bevy::prelude::Color,
) {
    let value = float_obj
        .ref_id
        .map(|id| provider.float_value(id))
        .unwrap_or(0.0)
        * float_obj.gain;

    // Java: (mimage == null || v >= 0.0f) ? this.image : mimage
    let digit_images = if value < 0.0 {
        float_obj
            .minus_digit_sources
            .as_ref()
            .and_then(|s| s.get_images(time))
            .or_else(|| float_obj.digit_sources.get_images(time))
    } else {
        float_obj.digit_sources.get_images(time)
    };
    let Some(digit_images) = digit_images else {
        return;
    };

    let total_keta = float_obj.iketa + float_obj.fketa + 1; // +1 for decimal point
    let digit_w = if total_keta > 0 {
        rect.w / total_keta as f32
    } else {
        rect.w
    };

    let cmds = draw::float::compute_float_draw(value, rect, float_obj, digit_w);

    for cmd in &cmds {
        let src_idx = cmd.source_index as usize;
        if src_idx >= digit_images.len() {
            continue;
        }
        let region = &digit_images[src_idx];
        let Some(entry) = tex_map.get(region.handle) else {
            continue;
        };

        let local_x = cmd.dst_rect.x + cmd.dst_rect.w / 2.0 - rect.w / 2.0;
        let local_y = -(cmd.dst_rect.y + cmd.dst_rect.h / 2.0 - rect.h / 2.0);

        let texture_rect = if region.w > 0.0 && region.h > 0.0 {
            Some(bevy::math::Rect::new(
                region.x,
                region.y,
                region.x + region.w,
                region.y + region.h,
            ))
        } else {
            None
        };

        commands.entity(parent).with_child((
            Sprite {
                image: entry.handle.clone(),
                custom_size: Some(Vec2::new(cmd.dst_rect.w, cmd.dst_rect.h)),
                color: obj_color,
                rect: texture_rect,
                ..default()
            },
            Transform::from_xyz(local_x, local_y, 0.0001),
            MultiEntityChild,
        ));
    }
}

/// Spawns child sprites for a SkinGauge.
#[allow(clippy::too_many_arguments)]
fn spawn_gauge_children(
    commands: &mut Commands,
    parent: Entity,
    gauge: &bms_skin::skin_gauge::SkinGauge,
    provider: &dyn SkinStateProvider,
    tex_map: &TextureMap,
    time: i64,
    rect: &bms_skin::skin_object::Rect,
    obj_color: bevy::prelude::Color,
) {
    let gauge_value = provider.float_value(bms_skin::property_id::FloatId(107));

    let parts: Vec<_> = gauge
        .parts
        .iter()
        .map(|p| (p.part_type, p.images.clone(), p.timer, p.cycle))
        .collect();

    let dst = bms_skin::skin_object::Rect::new(0.0, 0.0, rect.w, rect.h);
    let cmds = draw::gauge::compute_gauge_draw(gauge.nodes, gauge_value, &parts, time, &dst);

    for cmd in &cmds {
        let region = &cmd.image_region;
        let Some(entry) = tex_map.get(region.handle) else {
            continue;
        };

        let local_x = cmd.dst_rect.x + cmd.dst_rect.w / 2.0 - rect.w / 2.0;
        let local_y = -(cmd.dst_rect.y + cmd.dst_rect.h / 2.0 - rect.h / 2.0);

        let texture_rect = if region.w > 0.0 && region.h > 0.0 {
            Some(bevy::math::Rect::new(
                region.x,
                region.y,
                region.x + region.w,
                region.y + region.h,
            ))
        } else {
            None
        };

        commands.entity(parent).with_child((
            Sprite {
                image: entry.handle.clone(),
                custom_size: Some(Vec2::new(cmd.dst_rect.w, cmd.dst_rect.h)),
                color: obj_color,
                rect: texture_rect,
                ..default()
            },
            Transform::from_xyz(local_x, local_y, 0.0001),
            MultiEntityChild,
        ));
    }
}

/// Spawns child sprites for a SkinJudge.
#[allow(clippy::too_many_arguments)]
fn spawn_judge_children(
    commands: &mut Commands,
    parent: Entity,
    judge: &bms_skin::skin_judge::SkinJudge,
    provider: &dyn SkinStateProvider,
    tex_map: &TextureMap,
    time: i64,
    rect: &bms_skin::skin_object::Rect,
    obj_color: bevy::prelude::Color,
) {
    let cmds = draw::judge::compute_judge_draw(judge, provider, tex_map, time, rect);

    for cmd in &cmds {
        let region = &cmd.image_region;
        let Some(entry) = tex_map.get(region.handle) else {
            continue;
        };

        let local_x = cmd.dst_rect.x + cmd.dst_rect.w / 2.0 - rect.w / 2.0;
        let local_y = -(cmd.dst_rect.y + cmd.dst_rect.h / 2.0 - rect.h / 2.0);

        let texture_rect = if region.w > 0.0 && region.h > 0.0 {
            Some(bevy::math::Rect::new(
                region.x,
                region.y,
                region.x + region.w,
                region.y + region.h,
            ))
        } else {
            None
        };

        commands.entity(parent).with_child((
            Sprite {
                image: entry.handle.clone(),
                custom_size: Some(Vec2::new(cmd.dst_rect.w, cmd.dst_rect.h)),
                color: obj_color,
                rect: texture_rect,
                ..default()
            },
            Transform::from_xyz(local_x, local_y, 0.0001),
            MultiEntityChild,
        ));
    }
}

/// Spawns child sprites for a SkinDistributionGraph.
fn spawn_distribution_children(
    commands: &mut Commands,
    parent: Entity,
    dg: &bms_skin::skin_distribution_graph::SkinDistributionGraph,
    provider: &dyn SkinStateProvider,
    tex_map: &TextureMap,
    rect: &bms_skin::skin_object::Rect,
    obj_color: bevy::prelude::Color,
) {
    let cmds = draw::distribution::compute_distribution_draw(dg, provider, tex_map, rect);

    for cmd in &cmds {
        let Some(entry) = tex_map.get(cmd.image_handle) else {
            continue;
        };

        let local_x = cmd.dst_rect.x + cmd.dst_rect.w / 2.0 - rect.w / 2.0;
        let local_y = -(cmd.dst_rect.y + cmd.dst_rect.h / 2.0 - rect.h / 2.0);

        commands.entity(parent).with_child((
            Sprite {
                image: entry.handle.clone(),
                custom_size: Some(Vec2::new(cmd.dst_rect.w, cmd.dst_rect.h)),
                color: obj_color,
                ..default()
            },
            Transform::from_xyz(local_x, local_y, 0.0001),
            MultiEntityChild,
        ));
    }
}

/// Generates pixel data for procedural texture skin objects.
fn generate_procedural_pixels(
    object: &SkinObjectType,
    provider: &dyn SkinStateProvider,
    width: u32,
    height: u32,
) -> Option<Vec<u8>> {
    match object {
        SkinObjectType::BpmGraph(_) => {
            let events = provider.bpm_events();
            Some(draw::visualizer::compute_bpm_graph_pixels(
                events, width, height,
            ))
        }
        SkinObjectType::HitErrorVisualizer(_) => {
            let timings = provider.recent_judge_timings();
            Some(draw::visualizer::compute_hit_error_pixels(
                timings, width, height,
            ))
        }
        SkinObjectType::NoteDistributionGraph(_) => {
            let counts = provider.note_distribution();
            Some(draw::visualizer::compute_note_distribution_pixels(
                counts, width, height,
            ))
        }
        SkinObjectType::TimingDistributionGraph(_) => {
            let counts = provider.timing_distribution();
            Some(draw::visualizer::compute_timing_distribution_pixels(
                counts, width, height,
            ))
        }
        SkinObjectType::TimingVisualizer(_) => {
            let data = provider.timing_visualizer_data();
            Some(draw::visualizer::compute_timing_visualizer_pixels(
                data, width, height,
            ))
        }
        SkinObjectType::GaugeGraph(gg) => {
            let history = provider.gauge_history();
            let gauge_type = provider.gauge_type();
            Some(draw::visualizer::compute_gauge_graph_pixels(
                history,
                gauge_type,
                &gg.colors,
                gg.line_width,
                width,
                height,
            ))
        }
        _ => None,
    }
}

/// Spawns standard (bitmap_type=0) glyph sprite children with optional shadow.
#[allow(clippy::too_many_arguments)]
fn spawn_standard_glyph_children(
    commands: &mut Commands,
    parent: Entity,
    glyph_cmds: &[draw::bmfont_text::GlyphDrawCommand],
    entry: &crate::font_map::BmFontEntry,
    skin_text: &bms_skin::skin_text::SkinText,
    main_color: Color,
    region_w: f32,
    region_h: f32,
) {
    let has_shadow = skin_text
        .shadow
        .as_ref()
        .is_some_and(|s| s.offset_x != 0.0 || s.offset_y != 0.0);

    // Shadow glyphs first (rendered behind main glyphs)
    if has_shadow {
        let shadow = skin_text.shadow.as_ref().unwrap();
        let main_srgba: Srgba = main_color.into();
        let (sr, sg, sb, sa) = eval::shadow_color_from_main(
            main_srgba.red,
            main_srgba.green,
            main_srgba.blue,
            main_srgba.alpha,
        );
        let shadow_color = Color::srgba(sr, sg, sb, sa);

        for cmd in glyph_cmds {
            let page_idx = cmd.page as usize;
            let tex_handle = match entry.page_textures.get(page_idx) {
                Some(h) => h.clone(),
                None => continue,
            };

            let local_x = cmd.dst_x + cmd.dst_w / 2.0 - region_w / 2.0 + shadow.offset_x;
            let local_y = -(cmd.dst_y + cmd.dst_h / 2.0 - region_h / 2.0) - shadow.offset_y;

            commands.entity(parent).with_child((
                Sprite {
                    image: tex_handle,
                    custom_size: Some(Vec2::new(cmd.dst_w, cmd.dst_h)),
                    rect: Some(bevy::math::Rect::new(
                        cmd.src_x,
                        cmd.src_y,
                        cmd.src_x + cmd.src_w,
                        cmd.src_y + cmd.src_h,
                    )),
                    color: shadow_color,
                    ..default()
                },
                Transform::from_xyz(local_x, local_y, 0.0),
                BmFontGlyphChild,
            ));
        }
    }

    // Main glyphs
    for cmd in glyph_cmds {
        let page_idx = cmd.page as usize;
        let tex_handle = match entry.page_textures.get(page_idx) {
            Some(h) => h.clone(),
            None => continue,
        };

        let local_x = cmd.dst_x + cmd.dst_w / 2.0 - region_w / 2.0;
        let local_y = -(cmd.dst_y + cmd.dst_h / 2.0 - region_h / 2.0);

        commands.entity(parent).with_child((
            Sprite {
                image: tex_handle,
                custom_size: Some(Vec2::new(cmd.dst_w, cmd.dst_h)),
                rect: Some(bevy::math::Rect::new(
                    cmd.src_x,
                    cmd.src_y,
                    cmd.src_x + cmd.src_w,
                    cmd.src_y + cmd.src_h,
                )),
                color: main_color,
                ..default()
            },
            Transform::from_xyz(local_x, local_y, 0.0001),
            BmFontGlyphChild,
        ));
    }
}

/// Spawns distance field (bitmap_type=1,2) glyph children using Mesh2d + DistanceFieldMaterial.
/// Shadow and outline are handled entirely in the shader (no double-draw needed).
#[allow(clippy::too_many_arguments)]
fn spawn_df_glyph_children(
    commands: &mut Commands,
    parent: Entity,
    glyph_cmds: &[draw::bmfont_text::GlyphDrawCommand],
    entry: &crate::font_map::BmFontEntry,
    skin_text: &bms_skin::skin_text::SkinText,
    main_color: Color,
    region_w: f32,
    region_h: f32,
    meshes: &mut Assets<Mesh>,
    df_materials: &mut Assets<DistanceFieldMaterial>,
) {
    let main_linear: LinearRgba = main_color.into();

    // Outline parameters
    let outline_distance = if skin_text.outline_color.is_some() && skin_text.outline_width > 0.0 {
        crate::distance_field_material::compute_outline_distance(skin_text.outline_width)
    } else {
        0.5 // No outline
    };
    let outline_linear: LinearRgba = skin_text
        .outline_color
        .as_ref()
        .map(|c| Color::srgba(c.r, c.g, c.b, c.a).into())
        .unwrap_or(LinearRgba::NONE);

    // Shadow parameters
    let (shadow_color, shadow_offset, shadow_smoothing) = if let Some(shadow) = &skin_text.shadow {
        let sc: LinearRgba = Color::srgba(
            shadow.color.r,
            shadow.color.g,
            shadow.color.b,
            shadow.color.a,
        )
        .into();
        // Compute UV-space offset using the first page dimensions
        let (pw, ph) = entry.page_dimensions.first().copied().unwrap_or((1.0, 1.0));
        let offset = crate::distance_field_material::compute_shadow_offset(
            shadow.offset_x,
            shadow.offset_y,
            pw,
            ph,
        );
        let smoothing = crate::distance_field_material::compute_shadow_smoothing(shadow.smoothness);
        (sc, offset, smoothing)
    } else {
        (LinearRgba::NONE, Vec4::ZERO, 0.0)
    };

    for cmd in glyph_cmds {
        let page_idx = cmd.page as usize;
        let tex_handle = match entry.page_textures.get(page_idx) {
            Some(h) => h.clone(),
            None => continue,
        };

        let mesh = Rectangle::new(cmd.dst_w, cmd.dst_h);
        let mesh_handle = meshes.add(mesh);

        let material = df_materials.add(DistanceFieldMaterial {
            color: main_linear,
            outline_color: outline_linear,
            shadow_color,
            params: Vec4::new(outline_distance, shadow_smoothing, 0.0, 0.0),
            shadow_offset,
            texture: tex_handle,
        });

        let local_x = cmd.dst_x + cmd.dst_w / 2.0 - region_w / 2.0;
        let local_y = -(cmd.dst_y + cmd.dst_h / 2.0 - region_h / 2.0);

        commands.entity(parent).with_child((
            Mesh2d(mesh_handle),
            MeshMaterial2d(material),
            Transform::from_xyz(local_x, local_y, 0.0001),
            BmFontGlyphChild,
        ));
    }
}

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
                            return (Some(entry.handle.clone()), None);
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
        // Multi-entity and procedural types are handled by dedicated queries.
        // Text is handled separately via TTF/BMFont queries.
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use crate::state_provider::StaticStateProvider;

    use super::*;

    #[test]
    fn skin_render_state_can_hold_static_provider() {
        let skin = Skin::new(bms_skin::skin_header::SkinHeader::default());
        let tex_map = TextureMap::new();
        let font_map = FontMap::new();
        let provider = Box::new(StaticStateProvider::default());

        let state = SkinRenderState {
            skin,
            texture_map: tex_map,
            font_map,
            state_provider: provider,
        };

        assert_eq!(state.skin.objects.len(), 0);
    }
}
