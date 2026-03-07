// Skin.java -> skin.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;

use crate::core::custom_event::CustomEvent;
use crate::core::custom_timer::CustomTimer;
use crate::core::skin_float::SkinFloat;
use crate::graphs::skin_bpm_graph::SkinBPMGraph;
use crate::graphs::skin_graph::SkinGraph;
use crate::graphs::skin_hit_error_visualizer::SkinHitErrorVisualizer;
use crate::graphs::skin_note_distribution_graph::SkinNoteDistributionGraph;
use crate::graphs::skin_timing_distribution_graph::SkinTimingDistributionGraph;
use crate::graphs::skin_timing_visualizer::SkinTimingVisualizer;
use crate::objects::skin_bga_object::SkinBgaObject;
use crate::objects::skin_gauge::SkinGauge;
use crate::objects::skin_gauge_graph_object::SkinGaugeGraphObject;
use crate::objects::skin_hidden::SkinHidden;
use crate::objects::skin_image::SkinImage;
use crate::objects::skin_judge_object::SkinJudgeObject;
use crate::objects::skin_note_object::SkinNoteObject;
use crate::objects::skin_number::SkinNumber;
use crate::objects::skin_slider::SkinSlider;
use crate::property::boolean_property::BooleanProperty;
use crate::property::timer_property::TimerProperty;
use crate::property::timer_property_factory;
use crate::stubs::{MainState, SkinConfigOffset, SkinOffset, TextureRegion};
use crate::text::skin_text_bitmap::SkinTextBitmap;
use crate::text::skin_text_font::SkinTextFont;
use crate::text::skin_text_image::SkinTextImage;
use crate::types::skin_bar_object::SkinBarObject;
use crate::types::skin_header::SkinHeader;
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

use log::debug;

/// Skin object enum for polymorphic dispatch
// All variants mirror Java SkinObject hierarchy; NoteDistributionGraph/HitErrorVisualizer
// are intentionally large — boxing all match sites would be a disproportionate structural change.
#[allow(clippy::large_enum_variant)]
pub enum SkinObject {
    Image(SkinImage),
    Number(SkinNumber),
    Float(SkinFloat),
    Slider(SkinSlider),
    Graph(SkinGraph),
    TextFont(SkinTextFont),
    TextBitmap(SkinTextBitmap),
    TextImage(SkinTextImage),
    BpmGraph(SkinBPMGraph),
    HitErrorVisualizer(SkinHitErrorVisualizer),
    NoteDistributionGraph(SkinNoteDistributionGraph),
    TimingDistributionGraph(SkinTimingDistributionGraph),
    TimingVisualizer(SkinTimingVisualizer),
    Note(SkinNoteObject),
    Bar(SkinBarObject),
    Judge(SkinJudgeObject),
    Bga(SkinBgaObject),
    Gauge(SkinGauge),
    GaugeGraph(SkinGaugeGraphObject),
    Hidden(SkinHidden),
}

impl SkinObject {
    pub fn data(&self) -> &SkinObjectData {
        match self {
            SkinObject::Image(o) => &o.data,
            SkinObject::Number(o) => &o.data,
            SkinObject::Float(o) => &o.data,
            SkinObject::Slider(o) => &o.data,
            SkinObject::Graph(o) => &o.data,
            SkinObject::TextFont(o) => &o.text_data.data,
            SkinObject::TextBitmap(o) => &o.text_data.data,
            SkinObject::TextImage(o) => &o.text_data.data,
            SkinObject::BpmGraph(o) => &o.data,
            SkinObject::HitErrorVisualizer(o) => &o.data,
            SkinObject::NoteDistributionGraph(o) => &o.data,
            SkinObject::TimingDistributionGraph(o) => &o.data,
            SkinObject::TimingVisualizer(o) => &o.data,
            SkinObject::Note(o) => &o.data,
            SkinObject::Bar(o) => &o.data,
            SkinObject::Judge(o) => &o.data,
            SkinObject::Bga(o) => &o.data,
            SkinObject::Gauge(o) => &o.data,
            SkinObject::GaugeGraph(o) => &o.data,
            SkinObject::Hidden(o) => &o.data,
        }
    }

    pub fn data_mut(&mut self) -> &mut SkinObjectData {
        match self {
            SkinObject::Image(o) => &mut o.data,
            SkinObject::Number(o) => &mut o.data,
            SkinObject::Float(o) => &mut o.data,
            SkinObject::Slider(o) => &mut o.data,
            SkinObject::Graph(o) => &mut o.data,
            SkinObject::TextFont(o) => &mut o.text_data.data,
            SkinObject::TextBitmap(o) => &mut o.text_data.data,
            SkinObject::TextImage(o) => &mut o.text_data.data,
            SkinObject::BpmGraph(o) => &mut o.data,
            SkinObject::HitErrorVisualizer(o) => &mut o.data,
            SkinObject::NoteDistributionGraph(o) => &mut o.data,
            SkinObject::TimingDistributionGraph(o) => &mut o.data,
            SkinObject::TimingVisualizer(o) => &mut o.data,
            SkinObject::Note(o) => &mut o.data,
            SkinObject::Bar(o) => &mut o.data,
            SkinObject::Judge(o) => &mut o.data,
            SkinObject::Bga(o) => &mut o.data,
            SkinObject::Gauge(o) => &mut o.data,
            SkinObject::GaugeGraph(o) => &mut o.data,
            SkinObject::Hidden(o) => &mut o.data,
        }
    }

    pub fn validate(&mut self) -> bool {
        match self {
            SkinObject::Image(o) => o.validate(),
            SkinObject::Slider(o) => o.validate(),
            SkinObject::Graph(o) => o.validate(),
            SkinObject::TextFont(o) => o.validate(),
            // Types without validate() default to true
            _ => true,
        }
    }

    pub fn draw_condition(&self) -> &[Box<dyn BooleanProperty>] {
        self.data().draw_condition()
    }

    pub fn set_draw_condition(&mut self, bp: Vec<Box<dyn BooleanProperty>>) {
        self.data_mut().dstdraw = bp;
    }

    pub fn option(&self) -> &[i32] {
        self.data().option()
    }

    pub fn set_option(&mut self, op: Vec<i32>) {
        self.data_mut().dstop = op;
    }

    pub fn load(&mut self) {
        self.data_mut().load();
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        match self {
            SkinObject::Image(o) => o.prepare(time, state),
            SkinObject::Number(o) => o.prepare(time, state),
            SkinObject::Float(o) => o.prepare(time, state),
            SkinObject::Slider(o) => o.prepare(time, state),
            SkinObject::Graph(o) => o.prepare(time, state),
            SkinObject::TextFont(o) => o.prepare(time, state),
            SkinObject::TextBitmap(o) => o.prepare(time, state),
            SkinObject::TextImage(o) => o.prepare(time, state),
            SkinObject::BpmGraph(o) => o.prepare(time, state),
            SkinObject::HitErrorVisualizer(o) => o.prepare(time, state),
            SkinObject::NoteDistributionGraph(o) => o.prepare(time, state),
            SkinObject::TimingDistributionGraph(o) => o.prepare(time, state),
            SkinObject::TimingVisualizer(o) => o.prepare(time, state),
            SkinObject::Note(o) => o.prepare(time, state),
            SkinObject::Bar(o) => o.prepare(time, state),
            SkinObject::Judge(o) => o.prepare(time, state),
            SkinObject::Bga(o) => o.prepare(time, state),
            SkinObject::Gauge(o) => o.prepare(time, state),
            SkinObject::GaugeGraph(o) => o.prepare(time, state),
            SkinObject::Hidden(o) => o.prepare(time, state),
        }
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer, state: &dyn MainState) {
        match self {
            SkinObject::Image(o) => o.draw(sprite),
            SkinObject::Number(o) => o.draw(sprite),
            SkinObject::Float(o) => o.draw(sprite),
            SkinObject::Slider(o) => o.draw(sprite),
            SkinObject::Graph(o) => o.draw(sprite),
            SkinObject::TextFont(o) => o.draw(sprite),
            SkinObject::TextBitmap(o) => o.draw(sprite),
            SkinObject::TextImage(o) => o.draw(sprite),
            SkinObject::BpmGraph(o) => o.draw(sprite, state),
            SkinObject::HitErrorVisualizer(o) => o.draw(sprite),
            SkinObject::NoteDistributionGraph(o) => o.draw(sprite, state),
            SkinObject::TimingDistributionGraph(o) => o.draw(sprite),
            SkinObject::TimingVisualizer(o) => o.draw(sprite),
            SkinObject::Note(o) => o.draw(sprite),
            SkinObject::Bar(o) => o.draw(sprite),
            SkinObject::Judge(o) => o.draw(sprite),
            SkinObject::Bga(o) => o.draw(sprite),
            SkinObject::Gauge(o) => o.draw(sprite),
            SkinObject::GaugeGraph(o) => o.draw(sprite),
            SkinObject::Hidden(o) => o.draw(sprite),
        }
    }

    pub fn is_draw(&self) -> bool {
        self.data().draw
    }

    pub fn is_visible(&self) -> bool {
        self.data().visible
    }

    pub fn mouse_pressed(&self, state: &mut dyn MainState, button: i32, x: i32, y: i32) -> bool {
        self.data().mouse_pressed(state, button, x, y)
    }

    pub fn is_disposed(&self) -> bool {
        self.data().is_disposed()
    }

    pub fn dispose(&mut self) {
        match self {
            SkinObject::Image(o) => o.dispose(),
            SkinObject::Number(o) => o.dispose(),
            SkinObject::Float(o) => o.dispose(),
            SkinObject::Slider(o) => o.dispose(),
            SkinObject::Graph(o) => o.dispose(),
            SkinObject::TextFont(o) => o.dispose(),
            SkinObject::TextBitmap(o) => o.dispose(),
            SkinObject::TextImage(o) => o.dispose(),
            SkinObject::BpmGraph(o) => o.dispose(),
            SkinObject::HitErrorVisualizer(o) => o.dispose(),
            SkinObject::NoteDistributionGraph(o) => o.dispose(),
            SkinObject::TimingDistributionGraph(o) => o.dispose(),
            SkinObject::TimingVisualizer(o) => o.dispose(),
            SkinObject::Note(o) => o.dispose(),
            SkinObject::Bar(o) => o.dispose(),
            SkinObject::Judge(o) => o.dispose(),
            SkinObject::Bga(o) => o.dispose(),
            SkinObject::Gauge(_) => {}
            SkinObject::GaugeGraph(_) => {}
            SkinObject::Hidden(_) => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_destination(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: Option<Box<dyn TimerProperty>>,
        op1: i32,
        op2: i32,
        op3: i32,
        offset: &[i32],
    ) {
        self.data_mut().set_destination_with_timer_ops_and_offsets(
            time, x, y, w, h, acc, a, r, g, b, blend, filter, angle, center, loop_val, timer, op1,
            op2, op3, offset,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_destination_with_timer_ops(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: Option<Box<dyn TimerProperty>>,
        op: &[i32],
    ) {
        self.data_mut().set_destination_with_timer_and_ops(
            time, x, y, w, h, acc, a, r, g, b, blend, filter, angle, center, loop_val, timer, op,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_destination_with_timer_draw(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: Option<Box<dyn TimerProperty>>,
        draw: Box<dyn BooleanProperty>,
    ) {
        self.data_mut().set_destination_with_timer_draw(
            time, x, y, w, h, acc, a, r, g, b, blend, filter, angle, center, loop_val, timer, draw,
        );
    }

    pub fn set_mouse_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.data_mut().set_mouse_rect(x, y, w, h);
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            SkinObject::Image(_) => "Image",
            SkinObject::Number(_) => "Number",
            SkinObject::Float(_) => "Float",
            SkinObject::Slider(_) => "Slider",
            SkinObject::Graph(_) => "Graph",
            SkinObject::TextFont(_) => "Text",
            SkinObject::TextBitmap(_) => "Text",
            SkinObject::TextImage(_) => "Text",
            SkinObject::BpmGraph(_) => "BpmGraph",
            SkinObject::HitErrorVisualizer(_) => "HitErrorVisualizer",
            SkinObject::NoteDistributionGraph(_) => "NoteDistributionGraph",
            SkinObject::TimingDistributionGraph(_) => "TimingDistributionGraph",
            SkinObject::TimingVisualizer(_) => "TimingVisualizer",
            SkinObject::Note(_) => "SkinNote",
            SkinObject::Bar(_) => "SkinBar",
            SkinObject::Judge(_) => "SkinJudge",
            SkinObject::Bga(_) => "SkinBGA",
            SkinObject::Gauge(_) => "SkinGauge",
            SkinObject::GaugeGraph(_) => "SkinGaugeGraph",
            SkinObject::Hidden(_) => "SkinHidden",
        }
    }

    /// Returns true if this object is a SkinSlider
    pub fn is_slider(&self) -> bool {
        matches!(self, SkinObject::Slider(_))
    }
}

include!("skin_impl.rs");
include!("skin_drawable.rs");

#[cfg(test)]
mod tests;
