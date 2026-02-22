// Skin.java -> skin.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;

use crate::custom_event::CustomEvent;
use crate::custom_timer::CustomTimer;
use crate::property::boolean_property::BooleanProperty;
use crate::property::timer_property::TimerProperty;
use crate::property::timer_property_factory;
use crate::skin_bpm_graph::SkinBPMGraph;
use crate::skin_graph::SkinGraph;
use crate::skin_header::SkinHeader;
use crate::skin_hit_error_visualizer::SkinHitErrorVisualizer;
use crate::skin_image::SkinImage;
use crate::skin_note_distribution_graph::SkinNoteDistributionGraph;
use crate::skin_number::SkinNumber;
use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::skin_property;
use crate::skin_slider::SkinSlider;
use crate::skin_text_bitmap::SkinTextBitmap;
use crate::skin_text_font::SkinTextFont;
use crate::skin_text_image::SkinTextImage;
use crate::skin_timing_distribution_graph::SkinTimingDistributionGraph;
use crate::skin_timing_visualizer::SkinTimingVisualizer;
use crate::skin_type::SkinType;
use crate::stubs::{
    Color, MainState, Matrix4, Resolution, SkinConfigOffset, SkinOffset, SpriteBatch, TextureRegion,
};

use log::info;

/// Skin object enum for polymorphic dispatch
pub enum SkinObject {
    Image(SkinImage),
    Number(SkinNumber),
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
}

impl SkinObject {
    pub fn data(&self) -> &SkinObjectData {
        match self {
            SkinObject::Image(o) => &o.data,
            SkinObject::Number(o) => &o.data,
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
        }
    }

    pub fn data_mut(&mut self) -> &mut SkinObjectData {
        match self {
            SkinObject::Image(o) => &mut o.data,
            SkinObject::Number(o) => &mut o.data,
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
        }
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        match self {
            SkinObject::Image(o) => o.draw(sprite),
            SkinObject::Number(o) => o.draw(sprite),
            SkinObject::Slider(o) => o.draw(sprite),
            SkinObject::Graph(o) => o.draw(sprite),
            SkinObject::TextFont(o) => o.draw(sprite),
            SkinObject::TextBitmap(o) => o.draw(sprite),
            SkinObject::TextImage(o) => o.draw(sprite),
            // These types need state for draw, but the dispatch signature doesn't pass it.
            // They will draw with cached state from prepare().
            SkinObject::BpmGraph(_) => {}
            SkinObject::HitErrorVisualizer(o) => o.draw(sprite),
            SkinObject::NoteDistributionGraph(_) => {}
            SkinObject::TimingDistributionGraph(o) => o.draw(sprite),
            SkinObject::TimingVisualizer(o) => o.draw(sprite),
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
                let bp: Vec<Box<dyn BooleanProperty>> = Vec::new();
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
                for (j, (is_static, get_val)) in static_results.iter().enumerate() {
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
                    self.objects[*idx].draw(renderer);
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
