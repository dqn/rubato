// SkinNote wrapper for SkinObject enum (Phase 32a)
// Wraps rubato_play::SkinNote with SkinObjectData for the skin pipeline.
// Translated from: SkinNote.java

use rubato_play::lane_renderer::{DrawCommand, NoteImageType};

use crate::reexports::MainState;
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

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
    pub inner: rubato_play::skin_note::SkinNote,
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
}

/// Line image data for section/BPM/stop/time lines.
pub struct LineImage {
    pub region: crate::reexports::TextureRegion,
    pub dst_width: f32,
    pub dst_height: f32,
}

impl SkinNoteObject {
    pub fn new(lane_count: usize) -> Self {
        Self {
            data: SkinObjectData::new(),
            inner: rubato_play::skin_note::SkinNote::new(lane_count),
            draw_commands: Vec::new(),
            note_images: vec![None; lane_count],
            mine_images: vec![None; lane_count],
            ln_body_images: vec![Default::default(); lane_count],
            line_images: Default::default(),
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
                DrawCommand::DrawTimeText { .. }
                | DrawCommand::DrawBpmText { .. }
                | DrawCommand::DrawStopText { .. } => {
                    // Text rendering requires a wired BitmapFont; skipped until
                    // the play-skin font pipeline is connected.
                }
                DrawCommand::DrawJudgeArea { .. } => {
                    // Judge area rendering requires a solid-color fill primitive;
                    // skipped until SpriteBatch supports fill_rect.
                }
            }
        }
    }

    /// Draw a line image at the given y_offset.
    /// index: 0=section, 2=BPM, 4=stop, 6=time (even indices for 1P).
    fn draw_line_image(&self, sprite: &mut SkinObjectRenderer, index: usize, y_offset: i32) {
        if let Some(li) = &self.line_images[index] {
            sprite.draw(
                &li.region,
                self.data.region.x,
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
    use rubato_play::lane_renderer::{DrawCommand, NoteImageType};

    #[test]
    fn test_new_skin_note_object() {
        let note = SkinNoteObject::new(7);
        assert!(note.draw_commands.is_empty());
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
}
