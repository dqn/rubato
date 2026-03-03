// SkinNote wrapper for SkinObject enum (Phase 32a)
// Wraps beatoraja_play::SkinNote with SkinObjectData for the skin pipeline.
// Translated from: SkinNote.java

use beatoraja_play::lane_renderer::DrawCommand;

use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::stubs::MainState;

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
    pub inner: beatoraja_play::skin_note::SkinNote,
    /// Draw commands from the last LaneRenderer.draw_lane() call.
    /// Set by the caller before draw() is invoked.
    draw_commands: Vec<DrawCommand>,
}

impl SkinNoteObject {
    pub fn new(lane_count: usize) -> Self {
        Self {
            data: SkinObjectData::new(),
            inner: beatoraja_play::skin_note::SkinNote::new(lane_count),
            draw_commands: Vec::new(),
        }
    }

    /// Set draw commands from LaneRenderer.draw_lane().
    /// Called by the game loop (BMSPlayer) after computing lane rendering.
    pub fn set_draw_commands(&mut self, commands: Vec<DrawCommand>) {
        self.draw_commands = commands;
    }

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
        for cmd in &self.draw_commands {
            match cmd {
                DrawCommand::SetColor { r, g, b, a } => {
                    sprite.set_color_rgba(*r, *g, *b, *a);
                }
                DrawCommand::SetBlend(blend) => {
                    sprite.set_blend(*blend);
                }
                DrawCommand::SetType(t) => {
                    sprite.set_type(*t);
                }
                DrawCommand::DrawNote {
                    lane: _,
                    x,
                    y,
                    w,
                    h,
                    image_type: _,
                } => {
                    // Note drawing: requires per-lane texture images.
                    // The SkinLane holds has_note/has_longnote flags but not actual
                    // TextureRegion references in the current architecture.
                    // Until lane images are wired, emit a placeholder quad.
                    let region = crate::stubs::TextureRegion::new();
                    sprite.draw(&region, *x, *y, *w, *h);
                }
                DrawCommand::DrawLongNote {
                    lane: _,
                    x,
                    y,
                    w,
                    h,
                    image_index: _,
                } => {
                    let region = crate::stubs::TextureRegion::new();
                    sprite.draw(&region, *x, *y, *w, *h);
                }
                DrawCommand::DrawSectionLine { .. }
                | DrawCommand::DrawTimeLine { .. }
                | DrawCommand::DrawBpmLine { .. }
                | DrawCommand::DrawStopLine { .. }
                | DrawCommand::DrawTimeText { .. }
                | DrawCommand::DrawBpmText { .. }
                | DrawCommand::DrawStopText { .. }
                | DrawCommand::DrawJudgeArea { .. } => {
                    // These commands require additional skin resources (line images, fonts)
                    // that are not yet available. They will be resolved when the skin
                    // line and text rendering infrastructure is wired.
                }
            }
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
    use beatoraja_play::lane_renderer::DrawCommand;

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
        note.set_draw_commands(commands);
        assert_eq!(note.draw_commands.len(), 2);
    }

    #[test]
    fn test_draw_set_color_command() {
        let mut note = SkinNoteObject::new(7);
        note.set_draw_commands(vec![DrawCommand::SetColor {
            r: 0.5,
            g: 0.6,
            b: 0.7,
            a: 0.8,
        }]);
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
        let c = sprite.get_color();
        assert!((c.r - 0.5).abs() < f32::EPSILON);
        assert!((c.g - 0.6).abs() < f32::EPSILON);
        assert!((c.b - 0.7).abs() < f32::EPSILON);
        assert!((c.a - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_draw_set_blend_command() {
        let mut note = SkinNoteObject::new(7);
        note.set_draw_commands(vec![DrawCommand::SetBlend(3)]);
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
        assert_eq!(sprite.get_blend(), 3);
    }

    #[test]
    fn test_draw_set_type_command() {
        let mut note = SkinNoteObject::new(7);
        note.set_draw_commands(vec![DrawCommand::SetType(5)]);
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
        assert_eq!(sprite.get_type(), 5);
    }

    #[test]
    fn test_draw_note_command_does_not_panic() {
        let mut note = SkinNoteObject::new(7);
        note.set_draw_commands(vec![DrawCommand::DrawNote {
            lane: 0,
            x: 10.0,
            y: 20.0,
            w: 30.0,
            h: 5.0,
            image_type: NoteImageType::Normal,
        }]);
        let mut sprite = SkinObjectRenderer::new();
        // Should not panic even though we use a placeholder texture
        note.draw(&mut sprite);
    }

    #[test]
    fn test_draw_long_note_command_does_not_panic() {
        let mut note = SkinNoteObject::new(7);
        note.set_draw_commands(vec![DrawCommand::DrawLongNote {
            lane: 0,
            x: 10.0,
            y: 50.0,
            w: 30.0,
            h: 100.0,
            image_index: 0,
        }]);
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
    }

    #[test]
    fn test_draw_section_line_is_noop() {
        let mut note = SkinNoteObject::new(7);
        note.set_draw_commands(vec![DrawCommand::DrawSectionLine { y_offset: 100 }]);
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
        note.set_draw_commands(vec![
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
        ]);
        let mut sprite = SkinObjectRenderer::new();
        note.draw(&mut sprite);
        // After all commands, color should be green
        let c = sprite.get_color();
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
