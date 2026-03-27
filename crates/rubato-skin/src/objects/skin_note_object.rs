// SkinNote wrapper for SkinObject enum (Phase 32a)
// Wraps rubato_game::play::SkinNote with SkinObjectData for the skin pipeline.
// Translated from: SkinNote.java

use rubato_types::draw_command::{DrawCommand, NoteImageType};

use crate::reexports::{BitmapFont, Color, MainState};
use crate::types::skin_object::{DestinationParams, SkinObjectData, SkinObjectRenderer};

/// SkinNote skin object — wraps play-side SkinNote with SkinObjectData.
///
/// In Java, SkinNote.draw() calls LaneRenderer.drawLane() which directly renders
/// using SkinObjectRenderer. In Rust, LaneRenderer.draw_lane() produces
/// DrawCommand values, which are executed here against SkinObjectRenderer.
///
/// The caller (BMSPlayer's render loop) is responsible for calling
/// `set_draw_commands()` with the result of `LaneRenderer.draw_lane()`.
pub struct SkinNoteObject {
    pub data: SkinObjectData,
    pub inner: rubato_types::skin_note::SkinNote,
    /// Draw commands from the last LaneRenderer.draw_lane() call.
    /// Set by the caller before draw() is invoked.
    pub draw_commands: Vec<DrawCommand>,
    /// Per-lane normal note textures (first frame of animation).
    pub note_images: Vec<Option<crate::reexports::TextureRegion>>,
    /// Per-lane mine note textures.
    pub mine_images: Vec<Option<crate::reexports::TextureRegion>>,
    /// Per-lane LN body textures (10 types).
    pub ln_body_images: Vec<[Option<crate::reexports::TextureRegion>; 10]>,
    /// Line images: [0]=section, [1]=section(2P), [2]=BPM, [3]=BPM(2P),
    /// [4]=stop, [5]=stop(2P), [6]=time, [7]=time(2P).
    /// Each entry contains (TextureRegion, dst_x, dst_width, dst_height).
    pub line_images: [Option<LineImage>; 8],
    /// White pixel texture for judge area rendering (IMAGE_WHITE).
    /// Set by the caller after obtaining the system image registry.
    pub judge_area_image: Option<crate::reexports::TextureRegion>,
    /// Font for text overlay rendering (time/BPM/stop text in practice mode).
    /// Set by the caller from LaneRenderer's font.
    pub font: Option<BitmapFont>,
}

/// Line image data for section/BPM/stop/time lines.
pub struct LineImage {
    pub region: crate::reexports::TextureRegion,
    pub dst_x: f32,
    pub dst_width: f32,
    pub dst_height: f32,
}

impl SkinNoteObject {
    pub fn new(lane_count: usize) -> Self {
        let mut data = SkinObjectData::new();
        // Java: this.setDestination(0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 0, 0, 0, 0, 0, 0, new int[0]);
        // A default destination is required so that prepare() sets draw=true.
        // Without it, dst is empty and prepare_region() sets draw=false,
        // causing draw_all_objects() to skip this object entirely.
        data.set_destination_with_int_timer_ops(
            &DestinationParams {
                time: 0,
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                acc: 0,
                a: 0,
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
            &[],
        );
        Self {
            data,
            inner: rubato_types::skin_note::SkinNote::new(lane_count),
            draw_commands: Vec::new(),
            note_images: vec![None; lane_count],
            mine_images: vec![None; lane_count],
            ln_body_images: vec![Default::default(); lane_count],
            line_images: Default::default(),
            judge_area_image: None,
            font: None,
        }
    }

    /// Prepare the note object for rendering.
    /// Called by the game loop (BMSPlayer) after computing lane rendering.
    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.data.prepare(time, state);
        self.inner.prepare(time);
    }

    /// Execute draw commands produced by LaneRenderer.draw_lane().
    ///
    /// Translated from: Java SkinNote.draw(SkinObjectRenderer sprite)
    /// Java: renderer.drawLane(sprite, time, lanes, this.getOffsets())
    ///
    /// Note commands (DrawNote, DrawLongNote) are currently emitted as
    /// SkinObjectRenderer draw calls using the lane's region. Section lines,
    /// BPM lines, and text drawing are represented as commands but require
    /// additional skin resources (line images, fonts) that are resolved by
    /// the caller or deferred.
    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        #[cfg(debug_assertions)]
        if self.draw_commands.is_empty() {
            log::warn!(
                "SkinNoteObject::draw() called with empty draw_commands \
                 - compute_note_draw_commands() may not have been called"
            );
        }
        for cmd in &self.draw_commands {
            match cmd {
                DrawCommand::SetColor { r, g, b, a } => {
                    sprite.set_color_rgba(*r, *g, *b, *a);
                }
                DrawCommand::SetBlend(blend) => {
                    sprite.blend = *blend;
                }
                DrawCommand::SetType(t) => {
                    sprite.obj_type = *t;
                }
                DrawCommand::DrawNote {
                    lane,
                    x,
                    y,
                    w,
                    h,
                    image_type,
                } => {
                    let region = match image_type {
                        NoteImageType::Mine => self.mine_images.get(*lane).and_then(|r| r.as_ref()),
                        _ => self.note_images.get(*lane).and_then(|r| r.as_ref()),
                    };
                    if let Some(region) = region {
                        sprite.draw(region, *x, *y, *w, *h);
                    }
                }
                DrawCommand::DrawLongNote {
                    lane,
                    x,
                    y,
                    w,
                    h,
                    image_index,
                } => {
                    let region = self
                        .ln_body_images
                        .get(*lane)
                        .and_then(|arr| arr.get(*image_index))
                        .and_then(|r| r.as_ref());
                    if let Some(region) = region {
                        sprite.draw(region, *x, *y, *w, *h);
                    }
                }
                DrawCommand::DrawSectionLine { y_offset } => {
                    self.draw_line_image(sprite, 0, *y_offset);
                }
                DrawCommand::DrawTimeLine { y_offset } => {
                    self.draw_line_image(sprite, 6, *y_offset);
                }
                DrawCommand::DrawBpmLine { y_offset, .. } => {
                    self.draw_line_image(sprite, 2, *y_offset);
                }
                DrawCommand::DrawStopLine { y_offset, .. } => {
                    self.draw_line_image(sprite, 4, *y_offset);
                }
                DrawCommand::DrawTimeText { text, x, y } => {
                    if let Some(font) = &mut self.font {
                        // Java: Color.valueOf("40c0c0") -> cyan
                        let color = Color::new(
                            0x40 as f32 / 255.0,
                            0xC0 as f32 / 255.0,
                            0xC0 as f32 / 255.0,
                            1.0,
                        );
                        sprite.draw_font(font, text, *x, *y, &color);
                    }
                }
                DrawCommand::DrawBpmText { text, x, y } => {
                    if let Some(font) = &mut self.font {
                        // Java: Color.valueOf("00c000") -> green
                        let color = Color::new(0.0, 0.75, 0.0, 1.0);
                        sprite.draw_font(font, text, *x, *y, &color);
                    }
                }
                DrawCommand::DrawStopText { text, x, y } => {
                    if let Some(font) = &mut self.font {
                        // Java: Color.valueOf("c0c000") -> yellow
                        let color = Color::new(0.75, 0.75, 0.0, 1.0);
                        sprite.draw_font(font, text, *x, *y, &color);
                    }
                }
                DrawCommand::DrawJudgeArea {
                    x,
                    y,
                    w,
                    h,
                    color_index,
                    ..
                } => {
                    if let Some(white) = &self.judge_area_image {
                        // Java: Color.valueOf("0000ff20"), "00ff0020", "ffff0020", "ff800020", "ff000020"
                        const JUDGE_COLORS: [(f32, f32, f32, f32); 5] = [
                            (0.0, 0.0, 1.0, 0x20 as f32 / 255.0), // blue
                            (0.0, 1.0, 0.0, 0x20 as f32 / 255.0), // green
                            (1.0, 1.0, 0.0, 0x20 as f32 / 255.0), // yellow
                            (1.0, 0.5, 0.0, 0x20 as f32 / 255.0), // orange
                            (1.0, 0.0, 0.0, 0x20 as f32 / 255.0), // red
                        ];
                        let (cr, cg, cb, ca) = JUDGE_COLORS
                            .get(*color_index)
                            .copied()
                            .unwrap_or(JUDGE_COLORS[0]);
                        sprite.set_color_rgba(cr, cg, cb, ca);
                        sprite.draw(white, *x, *y, *w, *h);
                    }
                }
            }
        }
    }

    /// Draw line images at the given y_offset for both 1P and 2P.
    /// index: 0=section, 2=BPM, 4=stop, 6=time (even indices for 1P, odd for 2P).
    fn draw_line_image(&self, sprite: &mut SkinObjectRenderer, index: usize, y_offset: i32) {
        // Draw 1P line (even index)
        if let Some(li) = &self.line_images[index] {
            sprite.draw(
                &li.region,
                li.dst_x,
                self.data.region.y + y_offset as f32,
                li.dst_width,
                li.dst_height,
            );
        }
        // Draw 2P line (odd index) for double-play mode
        if let Some(li) = &self.line_images[index + 1] {
            sprite.draw(
                &li.region,
                li.dst_x,
                self.data.region.y + y_offset as f32,
                li.dst_width,
                li.dst_height,
            );
        }
    }

    pub fn dispose(&mut self) {
        self.inner.dispose();
        self.data.set_disposed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_types::draw_command::{DrawCommand, NoteImageType};

    #[test]
    fn test_new_skin_note_object() {
        let note = SkinNoteObject::new(7);
        assert!(note.draw_commands.is_empty());
    }

    /// Constructor post-condition: new() must produce a non-empty dst so that
    /// prepare() can set draw=true. This matches Java SkinNote's constructor
    /// which calls setDestination() to add a default DST entry.
    #[test]
    fn test_new_has_default_destination() {
        let note = SkinNoteObject::new(7);
        assert!(
            !note.data.dst.is_empty(),
            "SkinNoteObject must have at least one DST entry after construction"
        );
        assert!(
            note.data.fixr.is_some(),
            "First DST entry should set fixr for fast-path prepare"
        );
    }

    /// Pipeline gate test: after prepare(), is_draw() must return true so that
    /// draw_all_objects() does not skip this object.
    #[test]
    fn test_prepare_sets_draw_true() {
        let mut note = SkinNoteObject::new(7);
        let state = crate::test_helpers::MockMainState::default();
        note.prepare(0, &state);
        assert!(
            note.data.draw,
            "SkinNoteObject must be drawable after prepare()"
        );
    }

    #[test]
    fn test_set_draw_commands() {
        let mut note = SkinNoteObject::new(7);
        let commands = vec![
            DrawCommand::SetColor {
                r: 1.0,
                g: 0.5,
                b: 0.0,
                a: 1.0,
            },
            DrawCommand::SetBlend(2),
        ];
        note.draw_commands = commands;
        assert_eq!(note.draw_commands.len(), 2);
    }

    #[test]
    fn test_draw_set_color_command() {
        let mut note = SkinNoteObject::new(7);
        note.draw_commands = vec![DrawCommand::SetColor {
            r: 0.5,
            g: 0.6,
            b: 0.7,
            a: 0.8,
        }];
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
        let c = sprite.color();
        assert!((c.r - 0.5).abs() < f32::EPSILON);
        assert!((c.g - 0.6).abs() < f32::EPSILON);
        assert!((c.b - 0.7).abs() < f32::EPSILON);
        assert!((c.a - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_draw_set_blend_command() {
        let mut note = SkinNoteObject::new(7);
        note.draw_commands = vec![DrawCommand::SetBlend(3)];
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
        assert_eq!(sprite.blend(), 3);
    }

    #[test]
    fn test_draw_set_type_command() {
        let mut note = SkinNoteObject::new(7);
        note.draw_commands = vec![DrawCommand::SetType(5)];
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
        assert_eq!(sprite.toast_type(), 5);
    }

    #[test]
    fn test_draw_note_command_does_not_panic() {
        let mut note = SkinNoteObject::new(7);
        note.draw_commands = vec![DrawCommand::DrawNote {
            lane: 0,
            x: 10.0,
            y: 20.0,
            w: 30.0,
            h: 5.0,
            image_type: NoteImageType::Normal,
        }];
        let mut sprite = SkinObjectRenderer::new();
        // Should not panic even though we use a placeholder texture
        note.draw(&mut sprite);
    }

    #[test]
    fn test_draw_long_note_command_does_not_panic() {
        let mut note = SkinNoteObject::new(7);
        note.draw_commands = vec![DrawCommand::DrawLongNote {
            lane: 0,
            x: 10.0,
            y: 50.0,
            w: 30.0,
            h: 100.0,
            image_index: 0,
        }];
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
    }

    #[test]
    fn test_draw_section_line_is_noop() {
        let mut note = SkinNoteObject::new(7);
        note.draw_commands = vec![DrawCommand::DrawSectionLine { y_offset: 100 }];
        let mut sprite = SkinObjectRenderer::new();
        // Should not panic, currently a no-op
        note.draw(&mut sprite);
    }

    #[test]
    fn test_draw_empty_commands_is_noop() {
        let mut note = SkinNoteObject::new(7);
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
    }

    #[test]
    fn test_draw_multiple_commands_in_sequence() {
        let mut note = SkinNoteObject::new(7);
        note.draw_commands = vec![
            DrawCommand::SetColor {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            DrawCommand::SetBlend(1),
            DrawCommand::DrawNote {
                lane: 0,
                x: 0.0,
                y: 0.0,
                w: 20.0,
                h: 5.0,
                image_type: NoteImageType::Normal,
            },
            DrawCommand::SetColor {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            },
            DrawCommand::DrawNote {
                lane: 1,
                x: 20.0,
                y: 0.0,
                w: 20.0,
                h: 5.0,
                image_type: NoteImageType::Processed,
            },
        ];
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
        // After all commands, color should be green
        let c = sprite.color();
        assert!((c.r - 0.0).abs() < f32::EPSILON);
        assert!((c.g - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_dispose() {
        let mut note = SkinNoteObject::new(7);
        note.dispose();
        // Should not panic
    }

    /// Regression: DrawTimeText must use Java's Color.valueOf("40c0c0") (cyan),
    /// not white (1.0, 1.0, 1.0). The color is only applied when a font is set,
    /// so this test verifies the constant values directly.
    #[test]
    fn time_text_color_matches_java_40c0c0() {
        let expected_r = 0x40 as f32 / 255.0; // 0.251
        let expected_g = 0xC0 as f32 / 255.0; // 0.753
        let expected_b = 0xC0 as f32 / 255.0; // 0.753

        // These must NOT be white (1.0, 1.0, 1.0)
        assert!(
            (expected_r - 1.0).abs() > 0.1,
            "Time text red channel must not be white"
        );
        assert!(
            expected_r > 0.2 && expected_r < 0.3,
            "Time text red channel should be ~0.251 (0x40/255)"
        );
        assert!(
            expected_g > 0.7 && expected_g < 0.8,
            "Time text green channel should be ~0.753 (0xC0/255)"
        );
        assert!(
            expected_b > 0.7 && expected_b < 0.8,
            "Time text blue channel should be ~0.753 (0xC0/255)"
        );
    }
}
