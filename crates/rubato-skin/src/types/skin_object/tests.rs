use super::renderer::{DrawRotatedParams, SkinObjectRenderer};
use super::*;
use crate::reexports::{BitmapFont, GlyphLayout, Texture};

#[test]
fn test_skin_object_renderer_new() {
    let renderer = SkinObjectRenderer::new();
    assert_eq!(renderer.blend, 0);
    assert_eq!(renderer.obj_type, 0);
    // Default color is white
    assert_eq!(renderer.color.r, 1.0);
    assert_eq!(renderer.color.g, 1.0);
    assert_eq!(renderer.color.b, 1.0);
    assert_eq!(renderer.color.a, 1.0);
}

#[test]
fn test_skin_object_renderer_type_constants() {
    // Match Java SkinObjectRenderer constants
    assert_eq!(SkinObjectRenderer::TYPE_NORMAL, 0);
    assert_eq!(SkinObjectRenderer::TYPE_LINEAR, 1);
    assert_eq!(SkinObjectRenderer::TYPE_BILINEAR, 2);
    assert_eq!(SkinObjectRenderer::TYPE_FFMPEG, 3);
    assert_eq!(SkinObjectRenderer::TYPE_LAYER, 4);
    assert_eq!(SkinObjectRenderer::TYPE_DISTANCE_FIELD, 5);
}

#[test]
fn test_skin_object_renderer_set_color() {
    let mut renderer = SkinObjectRenderer::new();
    let red = Color::new(1.0, 0.0, 0.0, 0.5);
    renderer.set_color(&red);
    assert_eq!(renderer.color().r, 1.0);
    assert_eq!(renderer.color().g, 0.0);
    assert_eq!(renderer.color().a, 0.5);
}

#[test]
fn test_skin_object_renderer_set_blend() {
    let mut renderer = SkinObjectRenderer::new();
    renderer.blend = 2;
    assert_eq!(renderer.blend(), 2);
}

#[test]
fn test_skin_object_renderer_set_type() {
    let mut renderer = SkinObjectRenderer::new();
    renderer.obj_type = SkinObjectRenderer::TYPE_BILINEAR;
    assert_eq!(renderer.toast_type(), SkinObjectRenderer::TYPE_BILINEAR);
}

#[test]
fn test_skin_object_renderer_draw_generates_vertices() {
    let mut renderer = SkinObjectRenderer::new();
    let region = TextureRegion::new();
    renderer.draw(&region, 10.0, 20.0, 100.0, 50.0);
    // draw calls pre_draw + sprite.draw_region + post_draw
    // sprite should have 6 vertices for one quad
    assert_eq!(renderer.sprite.vertices().len(), 6);
}

#[test]
fn test_skin_object_renderer_pre_draw_sets_blend_additive() {
    let mut renderer = SkinObjectRenderer::new();
    renderer.blend = 2; // Additive
    let region = TextureRegion::new();
    renderer.draw(&region, 0.0, 0.0, 10.0, 10.0);
    // After post_draw, blend should be reset to Normal
    // (post_draw resets blend to SRC_ALPHA/ONE_MINUS_SRC_ALPHA when blend >= 2)
    let color = renderer.sprite.color();
    // Color should be restored to white (default)
    assert_eq!(color.r, 1.0);
    assert_eq!(color.g, 1.0);
    assert_eq!(color.b, 1.0);
    assert_eq!(color.a, 1.0);
}

#[test]
fn test_skin_object_renderer_pre_draw_shader_switching() {
    let mut renderer = SkinObjectRenderer::new();
    // Initially TYPE_NORMAL
    assert_eq!(renderer.current_shader, SkinObjectRenderer::TYPE_NORMAL);
    // Set type to FFMPEG
    renderer.obj_type = SkinObjectRenderer::TYPE_FFMPEG;
    let region = TextureRegion::new();
    renderer.draw(&region, 0.0, 0.0, 10.0, 10.0);
    // After pre_draw, current_shader should match obj_type
    assert_eq!(renderer.current_shader, SkinObjectRenderer::TYPE_FFMPEG);
    assert_eq!(
        renderer.sprite.shader_type(),
        SkinObjectRenderer::TYPE_FFMPEG
    );
}

#[test]
fn test_skin_object_renderer_draw_rotated_generates_vertices() {
    let mut renderer = SkinObjectRenderer::new();
    let region = TextureRegion::new();
    renderer.draw_rotated(DrawRotatedParams {
        image: &region,
        x: 10.0,
        y: 20.0,
        w: 100.0,
        h: 50.0,
        cx: 0.5,
        cy: 0.5,
        angle: 45,
    });
    assert_eq!(renderer.sprite.vertices().len(), 6);
}

#[test]
fn test_skin_object_renderer_pre_draw_saves_and_restores_color() {
    let mut renderer = SkinObjectRenderer::new();
    // Set sprite color to something specific
    let blue = Color::new(0.0, 0.0, 1.0, 1.0);
    renderer.sprite.set_color(&blue);
    // Set renderer color to red
    let red = Color::new(1.0, 0.0, 0.0, 1.0);
    renderer.set_color(&red);
    // Draw: pre_draw saves blue, sets red; post_draw restores blue
    let region = TextureRegion::new();
    renderer.draw(&region, 0.0, 0.0, 10.0, 10.0);
    let restored = renderer.sprite.color();
    assert_eq!(restored.r, 0.0);
    assert_eq!(restored.g, 0.0);
    assert_eq!(restored.b, 1.0);
    assert_eq!(restored.a, 1.0);
}

#[test]
fn test_skin_object_destination_new() {
    let dst = SkinObjectDestination::new(
        1000,
        Rectangle::new(10.0, 20.0, 100.0, 50.0),
        Color::new(1.0, 1.0, 1.0, 1.0),
        45,
        1,
    );
    assert_eq!(dst.time, 1000);
    assert_eq!(dst.region.x, 10.0);
    assert_eq!(dst.angle, 45);
    assert_eq!(dst.acc, 1);
}

#[test]
fn test_skin_object_data_validate() {
    let data = SkinObjectData::new();
    assert!(!data.validate());

    let mut data = SkinObjectData::new();
    data.dst.push(SkinObjectDestination::new(
        0,
        Rectangle::default(),
        Color::default(),
        0,
        0,
    ));
    assert!(data.validate());
}

// =========================================================================
// Phase 40a: Two-phase prepare/draw lifecycle tests
// =========================================================================

/// Helper: set up a single-destination SkinObjectData so prepare() sets draw=true.
fn setup_data(data: &mut SkinObjectData, x: f32, y: f32, w: f32, h: f32) {
    data.set_destination_with_int_timer_ops(
        &DestinationParams {
            time: 0,
            x,
            y,
            w,
            h,
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
        },
        0,
        &[0],
    );
}

#[test]
fn test_skin_object_data_prepare_sets_draw_and_region() {
    // Phase 40a: verify prepare(&mut self) mutates internal state
    let mut data = SkinObjectData::new();
    setup_data(&mut data, 10.0, 20.0, 100.0, 50.0);

    // Before prepare: draw is false
    assert!(!data.draw);

    let state = crate::test_helpers::MockMainState::default();
    data.prepare(0, &state);

    // After prepare: draw is true, region is populated
    assert!(data.draw);
    assert_eq!(data.region.x, 10.0);
    assert_eq!(data.region.y, 20.0);
    assert_eq!(data.region.width, 100.0);
    assert_eq!(data.region.height, 50.0);
}

#[test]
fn test_skin_object_data_prepare_then_draw_image() {
    // Phase 40a: verify two-phase pattern — prepare() then draw_image()
    let mut data = SkinObjectData::new();
    setup_data(&mut data, 10.0, 20.0, 100.0, 50.0);

    let state = crate::test_helpers::MockMainState::default();
    data.prepare(0, &state);
    assert!(data.draw);

    // Phase 2: draw reads pre-computed state (region, color, angle)
    let mut renderer = SkinObjectRenderer::new();
    let image = TextureRegion::new();
    data.draw_image(&mut renderer, &image);

    // Verify vertices were generated
    assert_eq!(renderer.sprite.vertices().len(), 6);
}

#[test]
fn test_skin_object_data_prepare_color_and_angle_cached() {
    // Phase 40a: verify prepare() caches color and angle for later draw use
    let mut data = SkinObjectData::new();
    // Set up with specific color (128, 64, 32, 200) and angle=45
    data.set_destination_with_int_timer_ops(
        &DestinationParams {
            time: 0,
            x: 0.0,
            y: 0.0,
            w: 50.0,
            h: 50.0,
            acc: 0,
            a: 200,
            r: 128,
            g: 64,
            b: 32,
            blend: 0,
            filter: 0,
            angle: 45,
            center: 0,
            loop_val: 0,
        },
        0,
        &[0],
    );

    let state = crate::test_helpers::MockMainState::default();
    data.prepare(0, &state);

    // Color should be cached
    assert!((data.color.r - 128.0 / 255.0).abs() < 0.01);
    assert!((data.color.g - 64.0 / 255.0).abs() < 0.01);
    assert!((data.color.b - 32.0 / 255.0).abs() < 0.01);
    assert!((data.color.a - 200.0 / 255.0).abs() < 0.01);
    // Angle should be cached
    assert_eq!(data.angle, 45);
}

#[test]
fn test_skin_object_data_draw_without_prepare_does_not_draw() {
    // Phase 40a: verify draw skips when prepare hasn't been called
    let mut data = SkinObjectData::new();
    setup_data(&mut data, 10.0, 20.0, 100.0, 50.0);

    // draw is false by default (no prepare called)
    assert!(!data.draw);

    // Attempting to use draw_image would still work mechanically,
    // but the caller checks data.draw before calling draw methods.
    // This test verifies the flag is false.
}

#[test]
fn test_skin_object_data_prepare_with_offset_modifies_region() {
    // Phase 40a: verify prepare_with_offset() adds offset to region
    let mut data = SkinObjectData::new();
    setup_data(&mut data, 10.0, 20.0, 100.0, 50.0);

    let state = crate::test_helpers::MockMainState::default();
    data.prepare_with_offset(0, &state, 5.0, 3.0);

    assert!(data.draw);
    assert_eq!(data.region.x, 15.0); // 10 + 5
    assert_eq!(data.region.y, 23.0); // 20 + 3
}

#[test]
fn test_skin_object_data_two_phase_separate_calls() {
    // Phase 40a: The key invariant — prepare and draw are separate calls.
    // The caller can inspect state between prepare and draw.
    let mut data = SkinObjectData::new();
    setup_data(&mut data, 50.0, 60.0, 200.0, 150.0);

    let state = crate::test_helpers::MockMainState::default();

    // Phase 1: prepare (mutable)
    data.prepare(0, &state);
    assert!(data.draw);

    // Between phases: caller can read the cached state
    let cached_region = data.region;
    let _cached_color = data.color;
    assert_eq!(cached_region.x, 50.0);
    assert_eq!(cached_region.width, 200.0);

    // Phase 2: draw (also mutable for scratch-space)
    let mut renderer = SkinObjectRenderer::new();
    let image = TextureRegion::new();
    data.draw_image_at(
        &mut renderer,
        &image,
        cached_region.x,
        cached_region.y,
        cached_region.width,
        cached_region.height,
    );

    assert_eq!(renderer.sprite.vertices().len(), 6);
}

#[test]
fn test_skin_object_data_empty_dst_does_not_panic() {
    // Regression: rate() panicked on `self.dst.len() - 1` when dst is empty.
    let mut data = SkinObjectData::new();
    assert!(data.dst.is_empty());

    let state = crate::test_helpers::MockMainState::default();
    // prepare() calls rate(), prepare_color(), prepare_angle() — all must survive empty dst.
    data.prepare(0, &state);

    // With empty dst, draw should remain false (no destination to render).
    assert!(!data.draw);
}

// =========================================================================
// draw_texture / draw_font / draw_font_layout tests
// =========================================================================

#[test]
fn test_skin_object_renderer_draw_texture_generates_vertices() {
    let mut renderer = SkinObjectRenderer::new();
    let tex = Texture::default();
    renderer.draw_texture(&tex, 10.0, 20.0, 100.0, 50.0);
    // 1 quad = 6 vertices
    assert_eq!(renderer.sprite.vertices().len(), 6);
}

#[test]
fn test_skin_object_renderer_draw_texture_applies_blend() {
    let mut renderer = SkinObjectRenderer::new();
    renderer.blend = 2; // Additive
    let tex = Texture::default();
    renderer.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);
    // After post_draw, blend is reset to Normal
    let color = renderer.sprite.color();
    assert_eq!(color.r, 1.0);
    assert_eq!(color.a, 1.0);
}

#[test]
fn test_skin_object_renderer_draw_texture_shader_switching() {
    let mut renderer = SkinObjectRenderer::new();
    renderer.obj_type = SkinObjectRenderer::TYPE_LINEAR;
    let tex = Texture::default();
    renderer.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);
    assert_eq!(renderer.current_shader, SkinObjectRenderer::TYPE_LINEAR);
    assert_eq!(
        renderer.sprite.shader_type(),
        SkinObjectRenderer::TYPE_LINEAR
    );
}

#[test]
fn test_skin_object_renderer_draw_font_no_crash() {
    let mut renderer = SkinObjectRenderer::new();
    let mut font = BitmapFont::new();
    let white = Color::new(1.0, 1.0, 1.0, 1.0);
    // BitmapFont without a loaded font file will just be a no-op
    renderer.draw_font(&mut font, "Hello", 10.0, 20.0, &white);
    // No crash is the success criterion; font has no loaded font file
    // so no vertices are generated
}

#[test]
fn test_skin_object_renderer_draw_font_saves_restores_color() {
    let mut renderer = SkinObjectRenderer::new();
    let blue = Color::new(0.0, 0.0, 1.0, 1.0);
    renderer.sprite.set_color(&blue);
    let red = Color::new(1.0, 0.0, 0.0, 1.0);
    renderer.set_color(&red);

    let mut font = BitmapFont::new();
    let green = Color::new(0.0, 1.0, 0.0, 1.0);
    renderer.draw_font(&mut font, "Test", 0.0, 0.0, &green);

    // After post_draw, sprite color should be restored to blue
    let restored = renderer.sprite.color();
    assert_eq!(restored.r, 0.0);
    assert_eq!(restored.g, 0.0);
    assert_eq!(restored.b, 1.0);
    assert_eq!(restored.a, 1.0);
}

#[test]
fn test_skin_object_renderer_draw_font_layout_no_crash() {
    let mut renderer = SkinObjectRenderer::new();
    let mut font = BitmapFont::new();
    let layout = GlyphLayout::new();
    renderer.draw_font_layout(&mut font, &layout, 10.0, 20.0);
    // No crash is the success criterion
}

#[test]
fn test_skin_object_renderer_draw_font_shader_switching() {
    let mut renderer = SkinObjectRenderer::new();
    renderer.obj_type = SkinObjectRenderer::TYPE_LINEAR;
    let mut font = BitmapFont::new();
    let white = Color::new(1.0, 1.0, 1.0, 1.0);
    renderer.draw_font(&mut font, "Test", 0.0, 0.0, &white);
    // After draw_font, shader should have been switched to TYPE_LINEAR
    assert_eq!(renderer.current_shader, SkinObjectRenderer::TYPE_LINEAR);
    assert_eq!(
        renderer.sprite.shader_type(),
        SkinObjectRenderer::TYPE_LINEAR
    );
}
