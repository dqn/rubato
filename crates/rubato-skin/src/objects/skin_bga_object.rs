// SkinBGA.java -> skin_bga_object.rs
// Translated from Java SkinBGA, connecting BGA display to the skin rendering system.

use std::sync::{Arc, Mutex};

use rubato_play::bga::bga_processor::{BGAProcessor, BgaRenderType, BgaRenderer};
use rubato_play::practice_configuration::{PracticeColor, PracticeDrawCommand};
use rubato_play::skin_bga::{
    BGAEXPAND_FULL, BGAEXPAND_KEEP_ASPECT_RATIO, BGAEXPAND_OFF, StretchType,
};
use rubato_render::color::Color;
use rubato_render::texture::TextureRegion;

use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::skin_property;
use crate::stubs::{MainState, Rectangle};

/// Trait for BGA drawing, bridging BGAProcessor to the skin rendering system.
/// This trait allows the skin to call BGA operations without knowing BGAProcessor internals.
/// It also enables mock-based testing of the skin BGA object.
pub trait BgaDraw: Send {
    /// Update BGA timeline to the given time (milliseconds).
    /// Pass -1 for states where BGA should not be displayed (preload, practice, ready).
    /// Corresponds to Java BGAProcessor.prepareBGA(time).
    fn prepare_bga(&mut self, time_ms: i64);

    /// Draw BGA content using the skin's renderer.
    /// The bga_expand parameter determines aspect-ratio handling.
    /// Corresponds to Java BGAProcessor.drawBGA(SkinBGA, SkinObjectRenderer, Rectangle).
    fn draw_bga(&mut self, sprite: &mut SkinObjectRenderer, region: &Rectangle, bga_expand: i32);
}

// =========================================================================
// BgaRenderer adapter: wraps SkinObjectRenderer for BGAProcessor.draw_bga()
// =========================================================================

/// Adapter that implements BgaRenderer (used by BGAProcessor.draw_bga) using SkinObjectRenderer.
struct SkinObjectRendererAdapter<'a> {
    sprite: &'a mut SkinObjectRenderer,
}

impl BgaRenderer for SkinObjectRendererAdapter<'_> {
    fn set_color_rgba(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.sprite.set_color_rgba(r, g, b, a);
    }

    fn set_blend(&mut self, blend: i32) {
        self.sprite.blend = blend;
    }

    fn set_type(&mut self, render_type: BgaRenderType) {
        let type_id = match render_type {
            BgaRenderType::Linear => SkinObjectRenderer::TYPE_LINEAR,
            BgaRenderType::Ffmpeg => SkinObjectRenderer::TYPE_FFMPEG,
            BgaRenderType::Layer => SkinObjectRenderer::TYPE_LAYER,
        };
        self.sprite.obj_type = type_id;
    }

    fn draw(&mut self, image: &TextureRegion, x: f32, y: f32, w: f32, h: f32) {
        self.sprite.draw(image, x, y, w, h);
    }
}

/// Convert bga_expand config value to StretchType.
fn bga_expand_to_stretch(bga_expand: i32) -> StretchType {
    match bga_expand {
        BGAEXPAND_FULL => StretchType::Stretch,
        BGAEXPAND_KEEP_ASPECT_RATIO => StretchType::KeepAspectRatioFitInner,
        BGAEXPAND_OFF => StretchType::KeepAspectRatioNoExpanding,
        _ => StretchType::Stretch,
    }
}

// =========================================================================
// BgaDraw implementation for BGAProcessor
// =========================================================================

impl BgaDraw for BGAProcessor {
    fn prepare_bga(&mut self, time_ms: i64) {
        BGAProcessor::prepare_bga(self, time_ms);
    }

    fn draw_bga(&mut self, sprite: &mut SkinObjectRenderer, region: &Rectangle, bga_expand: i32) {
        let stretch = bga_expand_to_stretch(bga_expand);
        let color = {
            let c = sprite.color();
            (c.r, c.g, c.b, c.a)
        };
        let blend = sprite.blend();

        let mut adapter = SkinObjectRendererAdapter { sprite };
        BGAProcessor::draw_bga(self, &mut adapter, region, stretch, color, blend);
    }
}

// =========================================================================
// SkinBgaObject — BGA skin object for the rendering pipeline
// =========================================================================

/// BGA skin object for the rendering pipeline.
/// Translated from: SkinBGA.java
///
/// In practice mode, draws PracticeConfiguration UI instead of BGA.
/// The caller sets practice draw commands via `set_practice_draw_commands()`.
pub struct SkinBgaObject {
    pub data: SkinObjectData,
    bga_expand: i32,
    /// Shared reference to the BGA drawing implementation (BGAProcessor).
    /// Set by BMSPlayer when the skin is loaded.
    bga_draw: Option<Arc<Mutex<dyn BgaDraw>>>,
    /// Practice mode draw commands (set by caller when in practice mode).
    practice_commands: Vec<PracticeDrawCommand>,
    /// Whether this BGA is currently in practice mode.
    practice_mode: bool,
}

impl SkinBgaObject {
    pub fn new(bga_expand: i32) -> Self {
        SkinBgaObject {
            data: SkinObjectData::default(),
            bga_expand,
            bga_draw: None,
            practice_commands: Vec::new(),
            practice_mode: false,
        }
    }

    /// Set the BGA drawing implementation.
    /// Called by BMSPlayer to connect the BGAProcessor to the skin system.
    pub fn set_bga_draw(&mut self, bga_draw: Arc<Mutex<dyn BgaDraw>>) {
        self.bga_draw = Some(bga_draw);
    }

    /// Get the BGA expand mode.
    pub fn bga_expand(&self) -> i32 {
        self.bga_expand
    }

    /// Check if this object has a BGA drawing implementation connected.
    pub fn has_bga_draw(&self) -> bool {
        self.bga_draw.is_some()
    }

    /// Set practice mode draw commands.
    /// Called by the game loop when in practice mode.
    pub fn set_practice_draw_commands(&mut self, commands: Vec<PracticeDrawCommand>) {
        self.practice_commands = commands;
        self.practice_mode = true;
    }

    /// Set whether this BGA is in practice mode.
    pub fn set_practice_mode(&mut self, practice: bool) {
        self.practice_mode = practice;
        if !practice {
            self.practice_commands.clear();
        }
    }

    /// Check if this BGA is in practice mode.
    pub fn is_practice_mode(&self) -> bool {
        self.practice_mode
    }

    /// Prepare BGA for rendering.
    /// Translated from: Java SkinBGA.prepare(long time, MainState state)
    ///
    /// In Java, this:
    /// 1. Sets the player reference from state
    /// 2. Calls super.prepare() to update draw/region/color
    /// 3. If draw is true, calls BGAProcessor.prepareBGA() with appropriate time:
    ///    - -1 for PRELOAD/PRACTICE/READY states
    ///    - timer.getNowTime(TIMER_PLAY) for other states
    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.data.prepare(time, state);

        if self.data.draw
            && let Some(ref bga_draw) = self.bga_draw
        {
            // Determine BGA time:
            // In Java: s == STATE_PRELOAD || s == STATE_PRACTICE || s == STATE_READY ? -1
            //          : player.timer.getNowTime(TIMER_PLAY)
            let timer = state.timer();
            let play_time = timer.now_time_for(skin_property::TIMER_PLAY);
            // If play timer is not active (returns Long.MIN_VALUE in Java, which
            // we represent as i64::MIN or a negative value), pass -1
            let bga_time = if play_time < 0 { -1 } else { play_time };

            if let Ok(mut draw) = bga_draw.lock() {
                draw.prepare_bga(bga_time);
            }
        }
    }

    /// Draw BGA content or practice configuration UI.
    ///
    /// Translated from: Java SkinBGA.draw(SkinObjectRenderer sprite)
    /// In Java:
    ///   if (PRACTICE) { player.getPracticeConfiguration().draw(...) }
    ///   else { resource.getBGAManager().drawBGA(...) }
    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if self.practice_mode {
            self.draw_practice(sprite);
        } else if let Some(ref bga_draw) = self.bga_draw {
            let region = self.data.region.clone();
            if let Ok(mut draw) = bga_draw.lock() {
                draw.draw_bga(sprite, &region, self.bga_expand);
            }
        }
    }

    /// Execute practice mode draw commands.
    /// Translated from: Java PracticeConfiguration.draw(Rectangle, SkinObjectRenderer, long, MainState)
    fn draw_practice(&mut self, sprite: &mut SkinObjectRenderer) {
        for cmd in &self.practice_commands {
            match cmd {
                PracticeDrawCommand::DrawText { text, x, y, color } => {
                    let c = match color {
                        PracticeColor::Yellow => Color::new(1.0, 1.0, 0.0, 1.0),
                        PracticeColor::Cyan => Color::new(0.0, 1.0, 1.0, 1.0),
                        PracticeColor::Orange => Color::new(1.0, 0.65, 0.0, 1.0),
                        PracticeColor::White => Color::new(1.0, 1.0, 1.0, 1.0),
                    };
                    // Draw text using sprite's font rendering.
                    // BitmapFont is not available here (it lives in BMSPlayer/PracticeConfiguration).
                    // Use a temporary BitmapFont for rendering.
                    let mut font = rubato_render::font::BitmapFont::new();
                    sprite.draw_font(&mut font, text, *x, *y, &c);
                }
                PracticeDrawCommand::DrawGraph { .. } => {
                    // Note distribution graph drawing requires SkinNoteDistributionGraph
                    // which is in beatoraja-skin. This will be wired when the full graph
                    // rendering pipeline is connected.
                }
            }
        }
    }

    pub fn dispose(&mut self) {
        // No resources to dispose in Rust translation
    }

    pub fn validate(&mut self) -> bool {
        self.data.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Mock BgaDraw for testing
    // =========================================================================

    #[derive(Default)]
    struct MockBgaDraw {
        prepare_calls: Vec<i64>,
        draw_calls: Vec<(f32, f32, f32, f32, i32)>,
    }

    impl BgaDraw for MockBgaDraw {
        fn prepare_bga(&mut self, time_ms: i64) {
            self.prepare_calls.push(time_ms);
        }

        fn draw_bga(
            &mut self,
            _sprite: &mut SkinObjectRenderer,
            region: &Rectangle,
            bga_expand: i32,
        ) {
            self.draw_calls
                .push((region.x, region.y, region.width, region.height, bga_expand));
        }
    }

    // =========================================================================
    // SkinBgaObject tests
    // =========================================================================

    #[test]
    fn test_new_skin_bga_object() {
        let bga = SkinBgaObject::new(BGAEXPAND_FULL);
        assert_eq!(bga.bga_expand(), BGAEXPAND_FULL);
        assert!(!bga.has_bga_draw());
    }

    #[test]
    fn test_new_with_different_expand_modes() {
        assert_eq!(
            SkinBgaObject::new(BGAEXPAND_FULL).bga_expand(),
            BGAEXPAND_FULL
        );
        assert_eq!(
            SkinBgaObject::new(BGAEXPAND_KEEP_ASPECT_RATIO).bga_expand(),
            BGAEXPAND_KEEP_ASPECT_RATIO
        );
        assert_eq!(
            SkinBgaObject::new(BGAEXPAND_OFF).bga_expand(),
            BGAEXPAND_OFF
        );
    }

    #[test]
    fn test_set_bga_draw() {
        let mut bga = SkinBgaObject::new(BGAEXPAND_FULL);
        assert!(!bga.has_bga_draw());

        let mock = Arc::new(Mutex::new(MockBgaDraw::default()));
        bga.set_bga_draw(mock);
        assert!(bga.has_bga_draw());
    }

    #[test]
    fn test_draw_delegates_to_bga_draw() {
        let mut bga = SkinBgaObject::new(BGAEXPAND_KEEP_ASPECT_RATIO);
        let mock = Arc::new(Mutex::new(MockBgaDraw::default()));
        bga.set_bga_draw(mock.clone());

        // Set up region on data
        bga.data.region = Rectangle::new(10.0, 20.0, 300.0, 200.0);

        let mut sprite = SkinObjectRenderer::new();
        bga.draw(&mut sprite);

        let mock_locked = mock.lock().expect("mutex poisoned");
        assert_eq!(mock_locked.draw_calls.len(), 1);
        assert_eq!(
            mock_locked.draw_calls[0],
            (10.0, 20.0, 300.0, 200.0, BGAEXPAND_KEEP_ASPECT_RATIO)
        );
    }

    #[test]
    fn test_draw_no_bga_draw_is_noop() {
        let mut bga = SkinBgaObject::new(BGAEXPAND_FULL);
        let mut sprite = SkinObjectRenderer::new();
        // Should not panic
        bga.draw(&mut sprite);
    }

    #[test]
    fn test_dispose_is_noop() {
        let mut bga = SkinBgaObject::new(BGAEXPAND_FULL);
        bga.dispose(); // Should not panic
    }

    // =========================================================================
    // BgaDraw for BGAProcessor integration tests
    // =========================================================================

    #[test]
    fn test_bga_processor_implements_bga_draw_prepare() {
        let mut proc = BGAProcessor::new();
        // BgaDraw::prepare_bga calls BGAProcessor::prepare_bga
        BgaDraw::prepare_bga(&mut proc, -1);
        // time should be -1 (blank screen)
        assert_eq!(proc.current_bga_id(), -1);
    }

    #[test]
    fn test_bga_processor_implements_bga_draw_draw() {
        let mut proc = BGAProcessor::new();
        proc.prepare_bga(1000); // set time to 1000ms

        let mut sprite = SkinObjectRenderer::new();
        let region = Rectangle::new(0.0, 0.0, 256.0, 256.0);

        // Should not panic — draws blank since no BGA data
        BgaDraw::draw_bga(&mut proc, &mut sprite, &region, BGAEXPAND_FULL);
    }

    #[test]
    fn test_bga_expand_to_stretch_conversion() {
        assert_eq!(bga_expand_to_stretch(BGAEXPAND_FULL), StretchType::Stretch);
        assert_eq!(
            bga_expand_to_stretch(BGAEXPAND_KEEP_ASPECT_RATIO),
            StretchType::KeepAspectRatioFitInner
        );
        assert_eq!(
            bga_expand_to_stretch(BGAEXPAND_OFF),
            StretchType::KeepAspectRatioNoExpanding
        );
        assert_eq!(bga_expand_to_stretch(99), StretchType::Stretch);
    }

    #[test]
    fn test_skin_bga_object_with_real_bga_processor() {
        use bms_model::bms_model::BMSModel;

        let model = BMSModel::new();
        let proc = BGAProcessor::from_model(&model);
        let shared = Arc::new(Mutex::new(proc));

        let mut bga = SkinBgaObject::new(BGAEXPAND_FULL);
        bga.set_bga_draw(shared.clone());
        assert!(bga.has_bga_draw());

        // Set up region
        bga.data.region = Rectangle::new(0.0, 0.0, 640.0, 480.0);

        let mut sprite = SkinObjectRenderer::new();
        // Should not panic — no BGA data but draws blank
        bga.draw(&mut sprite);
    }

    #[test]
    fn test_renderer_adapter_type_mapping() {
        // Test that BgaRenderType maps to correct SkinObjectRenderer type constants
        let mut sprite = SkinObjectRenderer::new();
        let mut adapter = SkinObjectRendererAdapter {
            sprite: &mut sprite,
        };

        BgaRenderer::set_type(&mut adapter, BgaRenderType::Linear);
        assert_eq!(sprite.toast_type(), SkinObjectRenderer::TYPE_LINEAR);

        let mut adapter2 = SkinObjectRendererAdapter {
            sprite: &mut sprite,
        };
        BgaRenderer::set_type(&mut adapter2, BgaRenderType::Ffmpeg);
        assert_eq!(sprite.toast_type(), SkinObjectRenderer::TYPE_FFMPEG);

        let mut adapter3 = SkinObjectRendererAdapter {
            sprite: &mut sprite,
        };
        BgaRenderer::set_type(&mut adapter3, BgaRenderType::Layer);
        assert_eq!(sprite.toast_type(), SkinObjectRenderer::TYPE_LAYER);
    }
}
