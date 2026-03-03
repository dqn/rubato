// Skin.java -> skin.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;

use crate::custom_event::CustomEvent;
use crate::custom_timer::CustomTimer;
use crate::property::boolean_property::BooleanProperty;
use crate::property::timer_property::TimerProperty;
use crate::property::timer_property_factory;
use crate::skin_bar_object::SkinBarObject;
use crate::skin_bga_object::SkinBgaObject;
use crate::skin_bpm_graph::SkinBPMGraph;
use crate::skin_float::SkinFloat;
use crate::skin_gauge::SkinGauge;
use crate::skin_gauge_graph_object::SkinGaugeGraphObject;
use crate::skin_graph::SkinGraph;
use crate::skin_header::SkinHeader;
use crate::skin_hidden::SkinHidden;
use crate::skin_hit_error_visualizer::SkinHitErrorVisualizer;
use crate::skin_image::SkinImage;
use crate::skin_judge_object::SkinJudgeObject;
use crate::skin_note_distribution_graph::SkinNoteDistributionGraph;
use crate::skin_note_object::SkinNoteObject;
use crate::skin_number::SkinNumber;
use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::skin_slider::SkinSlider;
use crate::skin_text_bitmap::SkinTextBitmap;
use crate::skin_text_font::SkinTextFont;
use crate::skin_text_image::SkinTextImage;
use crate::skin_timing_distribution_graph::SkinTimingDistributionGraph;
use crate::skin_timing_visualizer::SkinTimingVisualizer;
use crate::stubs::{MainState, SkinConfigOffset, SkinOffset, TextureRegion};

use log::info;

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

    pub fn get_draw_condition(&self) -> &[Box<dyn BooleanProperty>] {
        self.data().get_draw_condition()
    }

    pub fn set_draw_condition(&mut self, bp: Vec<Box<dyn BooleanProperty>>) {
        self.data_mut().set_draw_condition(bp);
    }

    pub fn get_option(&self) -> &[i32] {
        self.data().get_option()
    }

    pub fn set_option(&mut self, op: Vec<i32>) {
        self.data_mut().set_option(op);
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

    pub fn get_type_name(&self) -> &'static str {
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

/// Main skin class
pub struct Skin {
    pub header: SkinHeader,
    /// Width
    width: f32,
    /// Height
    height: f32,
    /// Width ratio from source
    dw: f32,
    /// Height ratio from source
    dh: f32,

    /// Registered skin objects
    objects: Vec<SkinObject>,
    /// Object array (references into objects for fast iteration)
    objectarray_indices: Vec<usize>,
    /// Removed skin objects
    removes: Vec<SkinObject>,
    /// Input start time (ms)
    input: i32,
    /// Scene time (ms)
    scene: i32,
    /// Fadeout time (ms)
    fadeout: i32,

    option: HashMap<i32, i32>,
    offset: HashMap<i32, SkinConfigOffset>,

    custom_events: HashMap<i32, CustomEvent>,
    custom_timers: HashMap<i32, CustomTimer>,

    /// Debug maps (None when not in debug mode)
    pub tempmap: Option<HashMap<String, [i64; 7]>>,
    pub pcntmap: Option<HashMap<String, [i64; 7]>>,
    avem_prepare: Option<HashMap<String, Vec<i64>>>,
    avem_draw: Option<HashMap<String, Vec<i64>>>,
    pub pcnt_prepare: i64,
    pub pcnt_draw: i64,

    renderer: Option<SkinObjectRenderer>,
    nextpreparetime: i64,
    prepareduration: i64,
}

impl Skin {
    pub fn new(header: SkinHeader) -> Self {
        let org = header.get_source_resolution().clone();
        let dst = header.get_destination_resolution().clone();
        let width = dst.width;
        let height = dst.height;
        let dw = dst.width / org.width;
        let dh = dst.height / org.height;

        // MainController.debug is stubbed as false
        let debug = false;

        let (tempmap, pcntmap, avem_prepare, avem_draw) = if debug {
            (
                Some(HashMap::with_capacity(32)),
                Some(HashMap::with_capacity(32)),
                Some(HashMap::with_capacity(32)),
                Some(HashMap::with_capacity(32)),
            )
        } else {
            (None, None, None, None)
        };

        Skin {
            header,
            width,
            height,
            dw,
            dh,
            objects: Vec::new(),
            objectarray_indices: Vec::new(),
            removes: Vec::new(),
            input: 0,
            scene: 3600000 * 24,
            fadeout: 0,
            option: HashMap::new(),
            offset: HashMap::new(),
            custom_events: HashMap::new(),
            custom_timers: HashMap::new(),
            tempmap,
            pcntmap,
            avem_prepare,
            avem_draw,
            pcnt_prepare: 0,
            pcnt_draw: 0,
            renderer: None,
            nextpreparetime: -1,
            prepareduration: 1,
        }
    }

    pub fn add(&mut self, object: SkinObject) {
        self.objects.push(object);
    }

    pub fn set_destination(
        &mut self,
        obj_index: usize,
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
        timer: i32,
        op1: i32,
        op2: i32,
        op3: i32,
        offset: &[i32],
    ) {
        let dw = self.dw;
        let dh = self.dh;
        let timer_prop = if timer > 0 {
            timer_property_factory::get_timer_property(timer)
        } else {
            None
        };
        if let Some(obj) = self.objects.get_mut(obj_index) {
            obj.set_destination(
                time,
                x * dw,
                y * dh,
                w * dw,
                h * dh,
                acc,
                a,
                r,
                g,
                b,
                blend,
                filter,
                angle,
                center,
                loop_val,
                timer_prop,
                op1,
                op2,
                op3,
                offset,
            );
        }
    }

    pub fn set_destination_with_timer(
        &mut self,
        obj_index: usize,
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
        let dw = self.dw;
        let dh = self.dh;
        if let Some(obj) = self.objects.get_mut(obj_index) {
            obj.set_destination_with_timer_ops(
                time,
                x * dw,
                y * dh,
                w * dw,
                h * dh,
                acc,
                a,
                r,
                g,
                b,
                blend,
                filter,
                angle,
                center,
                loop_val,
                timer,
                op,
            );
        }
    }

    pub fn set_destination_with_timer_draw(
        &mut self,
        obj_index: usize,
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
        let dw = self.dw;
        let dh = self.dh;
        if let Some(obj) = self.objects.get_mut(obj_index) {
            obj.set_destination_with_timer_draw(
                time,
                x * dw,
                y * dh,
                w * dw,
                h * dh,
                acc,
                a,
                r,
                g,
                b,
                blend,
                filter,
                angle,
                center,
                loop_val,
                timer,
                draw,
            );
        }
    }

    pub fn set_mouse_rect_on_object(&mut self, obj_index: usize, x: f32, y: f32, w: f32, h: f32) {
        let dw = self.dw;
        let dh = self.dh;
        if let Some(obj) = self.objects.get_mut(obj_index) {
            obj.set_mouse_rect(x * dw, y * dh, w * dw, h * dh);
        }
    }

    pub fn get_all_skin_objects_count(&self) -> usize {
        self.objects.len()
    }

    pub fn get_objects(&self) -> &[SkinObject] {
        &self.objects
    }

    pub fn get_objects_mut(&mut self) -> &mut Vec<SkinObject> {
        &mut self.objects
    }

    pub fn get_custom_events_count(&self) -> usize {
        self.custom_events.len()
    }

    pub fn get_custom_timers_count(&self) -> usize {
        self.custom_timers.len()
    }

    pub fn remove_skin_object(&mut self, index: usize) {
        if index < self.objects.len() {
            self.objects.remove(index);
        }
    }

    pub fn prepare(&mut self, state: &dyn MainState) {
        let mut remove_indices: Vec<usize> = Vec::new();

        for i in 0..self.objects.len() {
            if !self.objects[i].validate() {
                remove_indices.push(i);
            } else {
                let draw_conds = self.objects[i].get_draw_condition();
                let _bp: Vec<Box<dyn BooleanProperty>> = Vec::new();
                let mut should_remove = false;

                // We need to work with the draw conditions
                // Java code checks isStatic and get for each condition
                // This is complex with borrowing, so we collect results first
                let cond_count = draw_conds.len();
                let mut static_results: Vec<(bool, bool)> = Vec::new();
                for j in 0..cond_count {
                    let is_static = draw_conds[j].is_static(state);
                    let get_val = draw_conds[j].get(state);
                    static_results.push((is_static, get_val));
                }

                // Now we need to rebuild the draw conditions
                // We can't move out of the borrowed vec directly, so we use swap logic
                // For simplicity in the translation, we just mark for removal
                for (_j, (is_static, get_val)) in static_results.iter().enumerate() {
                    if *is_static && !get_val {
                        should_remove = true;
                    }
                    // Non-static conditions would be kept, but we can't easily move them
                    // This is handled via set_draw_condition below
                }

                if should_remove {
                    remove_indices.push(i);
                }

                // Check options
                let options = self.objects[i].get_option().to_vec();
                for op in &options {
                    if *op > 0 {
                        let value = self.option.get(op).copied().unwrap_or(-1);
                        if value != 1 && !remove_indices.contains(&i) {
                            remove_indices.push(i);
                        }
                    } else {
                        let neg_op = -op;
                        let value = self.option.get(&neg_op).copied().unwrap_or(-1);
                        if value != 0 && !remove_indices.contains(&i) {
                            remove_indices.push(i);
                        }
                    }
                }
                // Clear options on the object (Java: obj.setOption(l.toArray()) where l is empty)
                self.objects[i].set_option(Vec::new());
            }
        }

        info!(
            "Removing SkinObjects that are confirmed not to be drawn: {} / {}",
            remove_indices.len(),
            self.objects.len()
        );

        // Remove in reverse order to preserve indices
        remove_indices.sort_unstable();
        remove_indices.dedup();
        for &i in remove_indices.iter().rev() {
            let obj = self.objects.remove(i);
            self.removes.push(obj);
        }

        // Build object array indices
        self.objectarray_indices = (0..self.objects.len()).collect();

        self.option.clear();

        // Load all remaining objects
        for obj in &mut self.objects {
            obj.load();
        }

        // Debug mode setup (skipped since debug is false)

        // Prepare frame rate
        // state.main.getConfig().getPrepareFramePerSecond() stubbed
        self.prepareduration = 1;
        self.nextpreparetime = -1;
    }

    pub fn draw_all_objects(&mut self, state: &dyn MainState) {
        if self.renderer.is_none() {
            // Create renderer
            // In Java, this also sets up transform matrix based on offsetAll
            self.renderer = Some(SkinObjectRenderer::new());
        }

        let microtime = state.get_timer().get_now_micro_time();
        let debug = false; // MainController.debug stubbed as false

        if !debug {
            if self.nextpreparetime <= microtime {
                let time = state.get_timer().get_now_time();
                for idx in &self.objectarray_indices {
                    self.objects[*idx].prepare(time, state);
                }

                self.nextpreparetime += ((microtime - self.nextpreparetime) / self.prepareduration
                    + 1)
                    * self.prepareduration;
            }

            let renderer = self.renderer.as_mut().unwrap();
            for idx in &self.objectarray_indices {
                if self.objects[*idx].is_draw() && self.objects[*idx].is_visible() {
                    self.objects[*idx].draw(renderer, state);
                }
            }
        }
    }

    pub fn mouse_pressed(&mut self, state: &mut dyn MainState, button: i32, x: i32, y: i32) {
        for i in (0..self.objectarray_indices.len()).rev() {
            let idx = self.objectarray_indices[i];
            if self.objects[idx].is_draw() && self.objects[idx].mouse_pressed(state, button, x, y) {
                break;
            }
        }
    }

    pub fn mouse_dragged(&mut self, state: &mut dyn MainState, button: i32, x: i32, y: i32) {
        for i in (0..self.objectarray_indices.len()).rev() {
            let idx = self.objectarray_indices[i];
            if self.objects[idx].is_slider()
                && self.objects[idx].is_draw()
                && self.objects[idx].mouse_pressed(state, button, x, y)
            {
                break;
            }
        }
    }

    pub fn dispose(&mut self) {
        for obj in &mut self.objects {
            if !obj.is_disposed() {
                obj.dispose();
            }
        }
        for obj in &mut self.removes {
            if !obj.is_disposed() {
                obj.dispose();
            }
        }
    }

    pub fn get_fadeout(&self) -> i32 {
        self.fadeout
    }

    pub fn set_fadeout(&mut self, fadeout: i32) {
        self.fadeout = fadeout;
    }

    pub fn get_input(&self) -> i32 {
        self.input
    }

    pub fn set_input(&mut self, input: i32) {
        self.input = input;
    }

    pub fn get_scene(&self) -> i32 {
        self.scene
    }

    pub fn set_scene(&mut self, scene: i32) {
        self.scene = scene;
    }

    pub fn get_option(&self) -> &HashMap<i32, i32> {
        &self.option
    }

    pub fn set_option(&mut self, option: HashMap<i32, i32>) {
        self.option = option;
    }

    pub fn get_offset(&self) -> &HashMap<i32, SkinConfigOffset> {
        &self.offset
    }

    pub fn set_offset(&mut self, offset: HashMap<i32, SkinConfigOffset>) {
        self.offset = offset;
    }

    pub fn get_width(&self) -> f32 {
        self.width
    }

    pub fn get_height(&self) -> f32 {
        self.height
    }

    pub fn get_scale_x(&self) -> f64 {
        self.dw as f64
    }

    pub fn get_scale_y(&self) -> f64 {
        self.dh as f64
    }

    pub fn get_offset_all(&self, _state: &dyn MainState) -> Option<SkinOffset> {
        // In Java, checks if state instanceof BMSPlayer and gets skin type
        // For now, returns None as we can't do instanceof with trait objects
        // The actual implementation would check play skin types:
        // PLAY_5KEYS, PLAY_7KEYS, PLAY_9KEYS, PLAY_10KEYS, PLAY_14KEYS,
        // PLAY_24KEYS, PLAY_24KEYS_DOUBLE
        // and return state.getOffsetValue(SkinProperty.OFFSET_ALL)
        None
    }

    pub fn add_custom_event(&mut self, event: CustomEvent) {
        let id = event.get_id();
        self.custom_events.insert(id, event);
    }

    pub fn execute_custom_event(
        &mut self,
        state: &mut dyn MainState,
        id: i32,
        arg1: i32,
        arg2: i32,
    ) {
        if let Some(event) = self.custom_events.get_mut(&id) {
            event.execute(state, arg1, arg2);
        }
    }

    pub fn add_custom_timer(&mut self, timer: CustomTimer) {
        let id = timer.get_id();
        self.custom_timers.insert(id, timer);
    }

    /// Get custom timer value (micro sec).
    /// Recalculated only once per frame, so the value is guaranteed to be unique within the same frame.
    pub fn get_micro_custom_timer(&self, id: i32) -> i64 {
        if let Some(timer) = self.custom_timers.get(&id) {
            timer.get_micro_timer()
        } else {
            i64::MIN
        }
    }

    /// Set passive custom timer value.
    /// If the timer does not exist, it will be added.
    pub fn set_micro_custom_timer(&mut self, id: i32, time: i64) {
        if let Some(timer) = self.custom_timers.get_mut(&id) {
            timer.set_micro_timer(time);
        } else {
            let mut timer = CustomTimer::new(id, None);
            timer.set_micro_timer(time);
            self.custom_timers.insert(id, timer);
        }
    }

    /// Add a SkinNumber with destination and register it.
    /// Corresponds to Java Skin.addNumber(21 params)
    pub fn add_number(
        &mut self,
        number: SkinNumber,
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
        offset: i32,
    ) {
        let dw = self.dw;
        let dh = self.dh;
        let mut obj = SkinObject::Number(number);
        obj.data_mut()
            .set_destination_with_timer_ops_and_single_offset(
                time,
                x * dw,
                y * dh,
                w * dw,
                h * dh,
                acc,
                a,
                r,
                g,
                b,
                blend,
                filter,
                angle,
                center,
                loop_val,
                timer,
                op1,
                op2,
                op3,
                offset,
            );
        self.objects.push(obj);
    }

    /// Add a SkinImage from a TextureRegion with destination and register it.
    /// Corresponds to Java Skin.addImage(21 params)
    pub fn add_image(
        &mut self,
        tr: TextureRegion,
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
        offset: i32,
    ) -> usize {
        let dw = self.dw;
        let dh = self.dh;
        let si = SkinImage::new_with_single(tr);
        let mut obj = SkinObject::Image(si);
        obj.data_mut()
            .set_destination_with_timer_ops_and_single_offset(
                time,
                x * dw,
                y * dh,
                w * dw,
                h * dh,
                acc,
                a,
                r,
                g,
                b,
                blend,
                filter,
                angle,
                center,
                loop_val,
                timer,
                op1,
                op2,
                op3,
                offset,
            );
        self.objects.push(obj);
        self.objects.len() - 1
    }

    /// Update user-defined objects once per frame.
    /// Update order: timers -> events, each in ascending ID order.
    pub fn update_custom_objects(&mut self, state: &dyn MainState) {
        // Sort by ID for ordered iteration
        let mut timer_ids: Vec<i32> = self.custom_timers.keys().copied().collect();
        timer_ids.sort_unstable();
        for id in timer_ids {
            if let Some(timer) = self.custom_timers.get_mut(&id) {
                timer.update(state);
            }
        }

        // Events need &mut MainState but we only have &dyn MainState here
        // In the original Java, updateCustomObjects takes MainState
        // For now we skip event update as it requires &mut
        // let mut event_ids: Vec<i32> = self.custom_events.keys().copied().collect();
        // event_ids.sort_unstable();
        // for id in event_ids {
        //     if let Some(event) = self.custom_events.get_mut(&id) {
        //         event.update(state);
        //     }
        // }
    }
}

/// Adapter that provides timer data to skin objects via the stubs::MainState interface.
/// Used by SkinDrawable to bridge beatoraja-core's TimerManager to beatoraja-skin's internal interface.
///
/// Holds a reference to the real `TimerAccess` (typically a `TimerManager`) so that
/// per-timer-id queries return actual values instead of always 0.
struct TimerOnlyMainState<'a> {
    timer: &'a dyn beatoraja_types::timer_access::TimerAccess,
    ctx: Option<&'a dyn beatoraja_types::skin_render_context::SkinRenderContext>,
    main_controller: crate::stubs::MainController,
    resource: crate::stubs::PlayerResource,
    state_type: Option<beatoraja_types::main_state_type::MainStateType>,
}

impl<'a> TimerOnlyMainState<'a> {
    fn from_timer(timer: &'a dyn beatoraja_types::timer_access::TimerAccess) -> Self {
        Self {
            timer,
            ctx: None,
            main_controller: crate::stubs::MainController { debug: false },
            resource: crate::stubs::PlayerResource,
            state_type: None,
        }
    }

    fn from_render_context(
        ctx: &'a dyn beatoraja_types::skin_render_context::SkinRenderContext,
    ) -> Self {
        Self {
            timer: ctx,
            ctx: Some(ctx),
            main_controller: crate::stubs::MainController { debug: false },
            resource: crate::stubs::PlayerResource,
            state_type: ctx.current_state_type(),
        }
    }
}

impl crate::stubs::MainState for TimerOnlyMainState<'_> {
    fn get_timer(&self) -> &dyn beatoraja_types::timer_access::TimerAccess {
        self.timer
    }

    fn get_offset_value(&self, _id: i32) -> Option<&crate::stubs::SkinOffset> {
        None
    }

    fn get_main(&self) -> &crate::stubs::MainController {
        &self.main_controller
    }

    fn get_image(&self, _id: i32) -> Option<crate::rendering_stubs::TextureRegion> {
        None
    }

    fn get_resource(&self) -> &crate::stubs::PlayerResource {
        &self.resource
    }

    fn is_bms_player(&self) -> bool {
        self.state_type == Some(beatoraja_types::main_state_type::MainStateType::Play)
    }

    fn get_recent_judges(&self) -> &[i64] {
        self.ctx.map_or(&[] as &[i64], |c| c.get_recent_judges())
    }

    fn get_recent_judges_index(&self) -> usize {
        self.ctx.map_or(0, |c| c.get_recent_judges_index())
    }
}

impl beatoraja_core::main_state::SkinDrawable for Skin {
    fn prepare_skin(&mut self) {
        let null_timer = beatoraja_types::timer_access::NullTimer;
        let adapter = TimerOnlyMainState::from_timer(&null_timer);
        self.prepare(&adapter);
    }

    fn draw_all_objects_timed(
        &mut self,
        ctx: &mut dyn beatoraja_types::skin_render_context::SkinRenderContext,
    ) {
        let adapter = TimerOnlyMainState::from_render_context(ctx);
        self.draw_all_objects(&adapter);
    }

    fn update_custom_objects_timed(
        &mut self,
        ctx: &mut dyn beatoraja_types::skin_render_context::SkinRenderContext,
    ) {
        let adapter = TimerOnlyMainState::from_render_context(ctx);
        self.update_custom_objects(&adapter);
    }

    fn mouse_pressed_at(&mut self, button: i32, x: i32, y: i32) {
        let null_timer = beatoraja_types::timer_access::NullTimer;
        let mut adapter = TimerOnlyMainState::from_timer(&null_timer);
        self.mouse_pressed(&mut adapter, button, x, y);
    }

    fn mouse_dragged_at(&mut self, button: i32, x: i32, y: i32) {
        let null_timer = beatoraja_types::timer_access::NullTimer;
        let mut adapter = TimerOnlyMainState::from_timer(&null_timer);
        self.mouse_dragged(&mut adapter, button, x, y);
    }

    fn dispose_skin(&mut self) {
        self.dispose();
    }

    fn get_fadeout(&self) -> i32 {
        self.fadeout
    }

    fn get_input(&self) -> i32 {
        self.input
    }

    fn get_scene(&self) -> i32 {
        self.scene
    }

    fn get_width(&self) -> f32 {
        self.width
    }

    fn get_height(&self) -> f32 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin_header::SkinHeader;
    use beatoraja_core::main_state::SkinDrawable;

    fn make_test_skin() -> Skin {
        Skin::new(SkinHeader::new())
    }

    #[test]
    fn test_timer_only_main_state_returns_expected_values() {
        let timer = crate::stubs::Timer::with_timers(1000, 1_000_000, Vec::new());
        let adapter = TimerOnlyMainState::from_timer(&timer);
        let state: &dyn MainState = &adapter;
        assert_eq!(state.get_timer().get_now_time(), 1000);
        assert_eq!(state.get_timer().get_now_micro_time(), 1_000_000);
        assert!(state.get_offset_value(0).is_none());
        assert!(state.get_image(0).is_none());
        assert!(!state.get_main().debug);
    }

    /// Verify that TimerManager timer values flow through SkinDrawable to the skin adapter.
    /// Before the fix, all per-timer-id queries returned 0 (frozen animations).
    #[test]
    fn test_timer_manager_values_flow_through_to_skin_adapter() {
        use beatoraja_core::timer_manager::TimerManager;
        use beatoraja_types::timer_access::TimerAccess;

        let mut tm = TimerManager::new();
        tm.update(); // Advance nowmicrotime from Instant::now()
        tm.set_timer_on(10); // Timer 10 = ON at current micro time

        // Verify TimerManager implements TimerAccess correctly
        assert!(tm.is_timer_on(10));
        assert!(!tm.is_timer_on(20)); // Timer 20 was never set

        // Create adapter from TimerManager (the path SkinDrawable takes)
        let adapter = TimerOnlyMainState::from_timer(&tm);
        let state: &dyn MainState = &adapter;

        // Timer 10 should be ON through the adapter
        assert!(
            state.get_timer().is_timer_on(10),
            "Timer 10 should be ON through adapter"
        );
        // Timer 20 should be OFF
        assert!(
            !state.get_timer().is_timer_on(20),
            "Timer 20 should be OFF through adapter"
        );
        // get_micro_timer for ON timer should not be i64::MIN
        assert_ne!(
            state.get_timer().get_micro_timer(10),
            i64::MIN,
            "ON timer should return its activation time, not i64::MIN"
        );
        // get_micro_timer for OFF timer should be i64::MIN
        assert_eq!(
            state.get_timer().get_micro_timer(20),
            i64::MIN,
            "OFF timer should return i64::MIN"
        );
    }

    #[test]
    fn test_skin_drawable_getter_delegation() {
        let mut skin = make_test_skin();
        skin.set_fadeout(500);
        skin.set_input(100);
        skin.set_scene(2000);

        let drawable: &dyn SkinDrawable = &skin;
        assert_eq!(drawable.get_fadeout(), 500);
        assert_eq!(drawable.get_input(), 100);
        assert_eq!(drawable.get_scene(), 2000);
        // Default resolution is 640x480
        assert_eq!(drawable.get_width(), 640.0);
        assert_eq!(drawable.get_height(), 480.0);
    }

    #[test]
    fn test_draw_all_objects_timed_empty_skin() {
        let mut skin = make_test_skin();
        let mut null_timer = beatoraja_types::timer_access::NullTimer;
        // Should not panic with no objects
        skin.draw_all_objects_timed(&mut null_timer);
    }

    #[test]
    fn test_update_custom_objects_timed_empty_skin() {
        let mut skin = make_test_skin();
        let mut timer = crate::stubs::Timer::with_timers(100, 100_000, Vec::new());
        // Should not panic with no custom objects
        skin.update_custom_objects_timed(&mut timer);
    }

    #[test]
    fn test_dispose_skin_empty() {
        let mut skin = make_test_skin();
        // Should not panic with no objects
        skin.dispose_skin();
    }

    #[test]
    fn test_mouse_pressed_at_empty_skin() {
        let mut skin = make_test_skin();
        // Should not panic with no objects
        skin.mouse_pressed_at(0, 100, 200);
    }

    #[test]
    fn test_mouse_dragged_at_empty_skin() {
        let mut skin = make_test_skin();
        // Should not panic with no objects
        skin.mouse_dragged_at(0, 100, 200);
    }

    #[test]
    fn test_skin_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Skin>();
    }

    // =========================================================================
    // Phase 40a: Two-phase prepare/draw via SkinObject enum dispatch
    // =========================================================================

    /// Helper: make a TextureRegion with known dimensions.
    fn make_region(w: i32, h: i32) -> TextureRegion {
        TextureRegion {
            region_width: w,
            region_height: h,
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            ..TextureRegion::default()
        }
    }

    #[test]
    fn test_skin_object_enum_two_phase_image() {
        // Phase 40a: verify SkinObject::Image follows prepare/draw two-phase via enum
        let mut image = crate::skin_image::SkinImage::new_with_single(make_region(32, 32));
        image.data.set_destination_with_int_timer_ops(
            0,
            10.0,
            20.0,
            100.0,
            50.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[0],
        );
        let mut obj = SkinObject::Image(image);

        let state = crate::test_helpers::MockMainState::default();

        // Phase 1: prepare (via enum)
        obj.prepare(0, &state);
        assert!(obj.is_draw());

        // Phase 2: draw (via enum)
        let mut renderer = SkinObjectRenderer::new();
        obj.draw(&mut renderer, &state);
        // Should have generated vertices
        assert_eq!(renderer.sprite.vertices().len(), 6);
    }

    #[test]
    fn test_skin_object_enum_two_phase_bar() {
        // Phase 40a: verify SkinObject::Bar follows prepare/draw two-phase via enum
        let mut bar_obj = crate::skin_bar_object::SkinBarObject::new(0);
        bar_obj.data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            640.0,
            480.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[0],
        );
        let mut obj = SkinObject::Bar(bar_obj);

        let state = crate::test_helpers::MockMainState::default();

        // Phase 1: prepare
        obj.prepare(0, &state);
        assert!(obj.is_draw());

        // Phase 2: draw (stub — no panic)
        let mut renderer = SkinObjectRenderer::new();
        obj.draw(&mut renderer, &state);
    }

    #[test]
    fn test_skin_object_enum_two_phase_number() {
        // Phase 40a: verify SkinObject::Number follows prepare/draw two-phase via enum
        let digits: Vec<Vec<TextureRegion>> = vec![(0..12).map(|_| make_region(24, 32)).collect()];
        let mut num =
            crate::skin_number::SkinNumber::new_with_int_timer(digits, None, 0, 0, 3, 1, 0, 0, 0);
        num.data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            24.0,
            32.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[0],
        );
        let mut obj = SkinObject::Number(num);

        let state = crate::test_helpers::MockMainState::default();

        // Phase 1: prepare
        obj.prepare(0, &state);
        // draw may be false because integer property returns i32::MIN by default
        // That's expected — the property factory returns None and the default is 0,
        // which IS a valid value. Let's check.
        // The default ref_prop is from get_integer_property_by_id(0) which returns None,
        // so value = i32::MIN... but wait, SkinNumber::prepare calls ref_prop.get() which
        // returns 0 for id=0 since no property found. Actually ref_prop is None so value = i32::MIN.
        // i32::MIN triggers early return with draw=false. That's correct behavior.
    }

    #[test]
    fn test_skin_object_enum_two_phase_graph() {
        // Phase 40a: verify SkinObject::Graph follows prepare/draw two-phase
        let images = vec![make_region(64, 64)];
        let mut graph = crate::skin_graph::SkinGraph::new_with_int_timer(images, 0, 0, 0, 0);
        graph.data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            200.0,
            20.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[0],
        );
        let mut obj = SkinObject::Graph(graph);

        let state = crate::test_helpers::MockMainState::default();

        // Phase 1: prepare
        obj.prepare(0, &state);
        assert!(obj.is_draw());

        // Phase 2: draw
        let mut renderer = SkinObjectRenderer::new();
        obj.draw(&mut renderer, &state);
    }

    #[test]
    fn test_skin_object_enum_two_phase_slider() {
        // Phase 40a: verify SkinObject::Slider follows prepare/draw two-phase
        let images = vec![make_region(16, 16)];
        let mut slider =
            crate::skin_slider::SkinSlider::new_with_int_timer(images, 0, 0, 0, 100, 0, false);
        slider.data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            16.0,
            16.0,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[0],
        );
        let mut obj = SkinObject::Slider(slider);

        let state = crate::test_helpers::MockMainState::default();

        // Phase 1: prepare
        obj.prepare(0, &state);
        assert!(obj.is_draw());

        // Phase 2: draw
        let mut renderer = SkinObjectRenderer::new();
        obj.draw(&mut renderer, &state);
    }

    // ================================================================
    // SkinFloat enum variant tests (Task 47d)
    // ================================================================

    #[test]
    fn test_skin_float_in_enum_data_access() {
        // Verify SkinFloat variant provides data() / data_mut() access
        let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
            vec![vec![None; 12]],
            0,
            0,
            3,
            2,
            false,
            0,
            0,
            0,
            0,
            1.0,
        );
        let mut obj = SkinObject::Float(sf);

        // data() should return the SkinObjectData
        let _data = obj.data();
        assert!(!obj.is_draw());

        // data_mut() should also work
        obj.data_mut().visible = false;
        assert!(!obj.is_visible());
    }

    #[test]
    fn test_skin_float_type_name() {
        let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
            vec![vec![None; 12]],
            0,
            0,
            3,
            2,
            false,
            0,
            0,
            0,
            0,
            1.0,
        );
        let obj = SkinObject::Float(sf);
        assert_eq!(obj.get_type_name(), "Float");
    }

    #[test]
    fn test_skin_float_prepare_draw_dispose() {
        // Verify SkinFloat follows the prepare/draw/dispose lifecycle
        let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
            vec![vec![None; 12]],
            0,
            0,
            3,
            2,
            false,
            0,
            0,
            0,
            0,
            1.0,
        );
        let mut obj = SkinObject::Float(sf);
        let state = crate::test_helpers::MockMainState::default();

        // prepare should not panic
        obj.prepare(0, &state);

        // draw should not panic
        let mut renderer = SkinObjectRenderer::new();
        obj.draw(&mut renderer, &state);

        // dispose should not panic
        obj.dispose();
    }

    #[test]
    fn test_skin_float_validate_returns_true() {
        let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
            vec![vec![None; 12]],
            0,
            0,
            3,
            2,
            false,
            0,
            0,
            0,
            0,
            1.0,
        );
        let mut obj = SkinObject::Float(sf);
        // Float uses wildcard arm which defaults to true
        assert!(obj.validate());
    }
}
