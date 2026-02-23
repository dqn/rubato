use std::collections::HashSet;

use crate::bar::bar::Bar;
use crate::bar_manager::BarManager;
use crate::music_select_key_property::{MusicSelectKey, MusicSelectKeyProperty};
use crate::skin_bar::SkinBar;
use crate::stubs::*;

/// Bar area data for rendering
struct BarArea {
    pub sd: Option<usize>, // index into currentsongs
    pub x: f32,
    pub y: f32,
    pub value: i32,
    pub text: usize,
}

impl BarArea {
    fn new() -> Self {
        Self {
            sd: None,
            x: 0.0,
            y: 0.0,
            value: -1,
            text: 0,
        }
    }
}

/// Bar renderer for song bar display
/// Translates: bms.player.beatoraja.select.BarRenderer
pub struct BarRenderer {
    pub trophy: [&'static str; 3],

    pub durationlow: i32,
    pub durationhigh: i32,
    /// Bar movement counter
    pub duration: i64,
    /// Bar movement direction
    pub angle: i32,
    pub keyinput: bool,

    /// Analog scroll buffer
    pub analog_scroll_buffer: i32,
    pub analog_ticks_per_scroll: i32,

    pub barlength: usize,
    bararea: Vec<BarArea>,

    pub bartextupdate: bool,
    bartextcharset: HashSet<char>,

    time: i64,
}

impl BarRenderer {
    pub fn new(durationlow: i32, durationhigh: i32, analog_ticks_per_scroll: i32) -> Self {
        let barlength = 60;
        let bararea = (0..barlength).map(|_| BarArea::new()).collect();

        Self {
            trophy: ["bronzemedal", "silvermedal", "goldmedal"],
            durationlow,
            durationhigh,
            duration: 0,
            angle: 0,
            keyinput: false,
            analog_scroll_buffer: 0,
            analog_ticks_per_scroll,
            barlength,
            bararea,
            bartextupdate: false,
            bartextcharset: HashSet::with_capacity(1024),
            time: 0,
        }
    }

    pub fn mouse_pressed(&self, _baro: &SkinBar, _button: i32, _x: i32, _y: i32) -> bool {
        // In Java: iterates clickable bars, checks bounds, calls select.select() or manager.close()
        log::warn!(
            "not yet implemented: BarRenderer.mousePressed - requires MusicSelector context"
        );
        false
    }

    pub fn prepare(&mut self, _baro: &SkinBar, time: i64) {
        self.time = time;
        // In Java: calculates bar positions, determines bar types and text indices
        // Requires MusicSelectSkin, BarManager with currentsongs
        // Stubbed since it needs full rendering context
    }

    pub fn render(&self, _sprite: &mut SkinObjectRenderer, _baro: &mut SkinBar) {
        // In Java: draws all bar elements (images, text, trophies, lamps, levels, labels)
        // Two-phase pattern: prepare() computes layout, render() draws using that layout.
        // render needs &mut SkinBar because child draw methods (SkinImage::draw, etc.)
        // require &mut self for scratch-space fields (tmp_rect, tmp_image).
        // render needs &mut SkinObjectRenderer for color/blend state changes.
    }

    pub fn input(&mut self) {
        // In Java: handles scroll input via keyboard/analog/mouse wheel
        // Requires MusicSelector and BMSPlayerInputProcessor context
        log::warn!("not yet implemented: BarRenderer.input - requires MusicSelector context");
    }

    pub fn reset_input(&mut self) {
        let l = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        if l > self.duration {
            self.duration = 0;
        }
    }

    pub fn update_bar_text(&mut self) {
        self.bartextupdate = true;
    }

    pub fn dispose(&self) {
        // In Java: no-op (commented out favorite writing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_renderer_new() {
        let renderer = BarRenderer::new(300, 100, 5);
        assert_eq!(renderer.durationlow, 300);
        assert_eq!(renderer.durationhigh, 100);
        assert_eq!(renderer.analog_ticks_per_scroll, 5);
        assert_eq!(renderer.barlength, 60);
        assert_eq!(renderer.duration, 0);
        assert_eq!(renderer.angle, 0);
        assert!(!renderer.keyinput);
        assert!(!renderer.bartextupdate);
    }

    #[test]
    fn test_bar_renderer_two_phase_prepare_render() {
        // Phase 40a: verify BarRenderer follows the two-phase pattern:
        //   prepare(&mut self, &SkinBar, time) — reads SkinBar, mutates self
        //   render(&self, &mut sprite, &mut SkinBar) — reads self, mutates SkinBar/sprite
        let mut renderer = BarRenderer::new(300, 100, 5);
        let mut bar = SkinBar::new(0);

        // Phase 1: prepare — BarRenderer takes immutable ref to SkinBar
        renderer.prepare(&bar, 1000);
        assert_eq!(renderer.time, 1000);

        // Phase 2: render — BarRenderer takes mutable refs to sprite and SkinBar
        let mut sprite = SkinObjectRenderer;
        renderer.render(&mut sprite, &mut bar);
        // No panic = success (render is a stub)
    }

    #[test]
    fn test_bar_renderer_prepare_stores_time() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        let bar = SkinBar::new(0);

        renderer.prepare(&bar, 5000);
        assert_eq!(renderer.time, 5000);

        renderer.prepare(&bar, 10000);
        assert_eq!(renderer.time, 10000);
    }

    #[test]
    fn test_bar_renderer_update_bar_text() {
        let mut renderer = BarRenderer::new(300, 100, 5);
        assert!(!renderer.bartextupdate);
        renderer.update_bar_text();
        assert!(renderer.bartextupdate);
    }

    #[test]
    fn test_bar_renderer_mouse_pressed_stub() {
        let renderer = BarRenderer::new(300, 100, 5);
        let bar = SkinBar::new(0);
        // Stub returns false
        assert!(!renderer.mouse_pressed(&bar, 0, 100, 200));
    }
}
