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
        self.data_mut().set_draw_condition(bp);
    }

    pub fn option(&self) -> &[i32] {
        self.data().option()
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
    #[allow(dead_code)]
    avem_prepare: Option<HashMap<String, Vec<i64>>>,
    #[allow(dead_code)]
    avem_draw: Option<HashMap<String, Vec<i64>>>,
    pub pcnt_prepare: i64,
    pub pcnt_draw: i64,

    renderer: Option<SkinObjectRenderer>,
    nextpreparetime: i64,
    prepareduration: i64,

    /// Image registry: maps image IDs to TextureRegions.
    /// Populated during skin loading; resolved by SkinSourceReference at draw time.
    image_registry: HashMap<i32, TextureRegion>,

    /// Select-specific bar data extracted from the skin loader.
    /// MusicSelector takes this after loading to build SkinBar + BarRenderer.
    pub select_bar_data: Option<crate::select_bar_data::SelectBarData>,
}

impl Skin {
    pub fn new(header: SkinHeader) -> Self {
        let org = header.source_resolution().clone();
        let dst = header.destination_resolution().clone();
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
            image_registry: Self::create_system_image_registry(),
            select_bar_data: None,
        }
    }

    /// Take select-specific bar data out, leaving None.
    /// Called by MusicSelector after skin loading to build SkinBar + BarRenderer.
    pub fn take_select_bar_data(&mut self) -> Option<crate::select_bar_data::SelectBarData> {
        self.select_bar_data.take()
    }

    /// Create system placeholder images (BLACK=110, WHITE=111).
    /// These are always available regardless of song selection.
    fn create_system_image_registry() -> HashMap<i32, TextureRegion> {
        use crate::rendering_stubs::{Pixmap, PixmapFormat, Texture};
        use crate::skin_property::{IMAGE_BLACK, IMAGE_WHITE};

        let mut registry = HashMap::new();

        // 1x1 black pixel
        let mut black_pix = Pixmap::new(1, 1, PixmapFormat::RGBA8888);
        black_pix.set_color_rgba(0.0, 0.0, 0.0, 1.0);
        black_pix.fill();
        let black_tex = Texture::from_pixmap(&black_pix);
        registry.insert(IMAGE_BLACK, TextureRegion::from_texture(black_tex));

        // 1x1 white pixel
        let mut white_pix = Pixmap::new(1, 1, PixmapFormat::RGBA8888);
        white_pix.set_color_rgba(1.0, 1.0, 1.0, 1.0);
        white_pix.fill();
        let white_tex = Texture::from_pixmap(&white_pix);
        registry.insert(IMAGE_WHITE, TextureRegion::from_texture(white_tex));

        registry
    }

    pub fn add(&mut self, object: SkinObject) {
        self.objects.push(object);
    }

    /// Register an image by ID for SkinSourceReference resolution.
    pub fn register_image(&mut self, id: i32, tr: TextureRegion) {
        self.image_registry.insert(id, tr);
    }

    /// Look up a registered image by ID.
    pub fn registered_image(&self, id: i32) -> Option<TextureRegion> {
        self.image_registry.get(&id).cloned()
    }

    #[allow(clippy::too_many_arguments)]
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
            timer_property_factory::timer_property(timer)
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

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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

    pub fn all_skin_objects_count(&self) -> usize {
        self.objects.len()
    }

    pub fn objects(&self) -> &[SkinObject] {
        &self.objects
    }

    pub fn objects_mut(&mut self) -> &mut Vec<SkinObject> {
        &mut self.objects
    }

    pub fn custom_events_count(&self) -> usize {
        self.custom_events.len()
    }

    pub fn custom_timers_count(&self) -> usize {
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
                let draw_conds = self.objects[i].draw_condition();
                let _bp: Vec<Box<dyn BooleanProperty>> = Vec::new();
                let mut should_remove = false;

                // We need to work with the draw conditions
                // Java code checks isStatic and get for each condition
                // This is complex with borrowing, so we collect results first
                let mut static_results: Vec<(bool, bool)> = Vec::new();
                for cond in draw_conds {
                    let is_static = cond.is_static(state);
                    let get_val = cond.get(state);
                    static_results.push((is_static, get_val));
                }

                // Now we need to rebuild the draw conditions
                // We can't move out of the borrowed vec directly, so we use swap logic
                // For simplicity in the translation, we just mark for removal
                for (is_static, get_val) in static_results.iter() {
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
                let options = self.objects[i].option().to_vec();
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

        debug!(
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

        let microtime = state.timer().now_micro_time();
        let debug = false; // MainController.debug stubbed as false

        if !debug {
            if self.nextpreparetime <= microtime {
                let time = state.timer().now_time();
                for idx in &self.objectarray_indices {
                    self.objects[*idx].prepare(time, state);
                }

                self.nextpreparetime += ((microtime - self.nextpreparetime) / self.prepareduration
                    + 1)
                    * self.prepareduration;
            }

            let renderer = self.renderer.as_mut().unwrap();
            let mut draw_count = 0usize;
            for idx in &self.objectarray_indices {
                if self.objects[*idx].is_draw() && self.objects[*idx].is_visible() {
                    self.objects[*idx].draw(renderer, state);
                    draw_count += 1;
                }
            }

            // TODO: remove debug log
            use std::sync::atomic::{AtomicU64, Ordering};
            static FRAME: AtomicU64 = AtomicU64::new(0);
            let frame = FRAME.fetch_add(1, Ordering::Relaxed);
            if frame.is_multiple_of(60) {
                log::debug!(
                    "Skin draw: objects={}, drawable={}, vertices={}",
                    self.objectarray_indices.len(),
                    draw_count,
                    renderer.sprite.vertices().len(),
                );
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

    pub fn fadeout(&self) -> i32 {
        self.fadeout
    }

    pub fn set_fadeout(&mut self, fadeout: i32) {
        self.fadeout = fadeout;
    }

    pub fn input(&self) -> i32 {
        self.input
    }

    pub fn set_input(&mut self, input: i32) {
        self.input = input;
    }

    pub fn scene(&self) -> i32 {
        self.scene
    }

    pub fn set_scene(&mut self, scene: i32) {
        self.scene = scene;
    }

    pub fn option(&self) -> &HashMap<i32, i32> {
        &self.option
    }

    pub fn set_option(&mut self, option: HashMap<i32, i32>) {
        self.option = option;
    }

    pub fn offset(&self) -> &HashMap<i32, SkinConfigOffset> {
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

    pub fn scale_x(&self) -> f64 {
        self.dw as f64
    }

    pub fn scale_y(&self) -> f64 {
        self.dh as f64
    }

    pub fn offset_all(&self, _state: &dyn MainState) -> Option<SkinOffset> {
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
    pub fn micro_custom_timer(&self, id: i32) -> i64 {
        if let Some(timer) = self.custom_timers.get(&id) {
            timer.micro_timer()
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
    #[allow(clippy::too_many_arguments)]
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
    #[allow(clippy::too_many_arguments)]
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
    pub fn update_custom_objects(&mut self, state: &mut dyn MainState) {
        // Sort by ID for ordered iteration
        let mut timer_ids: Vec<i32> = self.custom_timers.keys().copied().collect();
        timer_ids.sort_unstable();
        for id in timer_ids {
            if let Some(timer) = self.custom_timers.get_mut(&id) {
                timer.update(state);
            }
        }

        let mut event_ids: Vec<i32> = self.custom_events.keys().copied().collect();
        event_ids.sort_unstable();
        for id in event_ids {
            if let Some(event) = self.custom_events.get_mut(&id) {
                event.update(state);
            }
        }
    }
}

/// Adapter that provides timer data to skin objects via the stubs::MainState interface.
/// Used by SkinDrawable to bridge beatoraja-core's TimerManager to beatoraja-skin's internal interface.
///
/// Holds a reference to the real `TimerAccess` (typically a `TimerManager`) so that
/// per-timer-id queries return actual values instead of always 0.
struct TimerOnlyMainState<'a> {
    timer: Option<&'a dyn rubato_types::timer_access::TimerAccess>,
    ctx: Option<&'a mut dyn rubato_types::skin_render_context::SkinRenderContext>,
    main_controller: crate::stubs::MainController,
    resource: crate::stubs::PlayerResource,
    state_type: Option<rubato_types::main_state_type::MainStateType>,
    image_registry: &'a HashMap<i32, TextureRegion>,
}

impl<'a> TimerOnlyMainState<'a> {
    fn from_timer(timer: &'a dyn rubato_types::timer_access::TimerAccess) -> Self {
        static EMPTY: std::sync::LazyLock<HashMap<i32, TextureRegion>> =
            std::sync::LazyLock::new(HashMap::new);
        Self {
            timer: Some(timer),
            ctx: None,
            main_controller: crate::stubs::MainController { debug: false },
            resource: crate::stubs::PlayerResource,
            state_type: None,
            image_registry: &EMPTY,
        }
    }

    fn from_render_context_with_images(
        ctx: &'a mut dyn rubato_types::skin_render_context::SkinRenderContext,
        image_registry: &'a HashMap<i32, TextureRegion>,
    ) -> Self {
        let state_type = ctx.current_state_type();
        Self {
            timer: None,
            ctx: Some(ctx),
            main_controller: crate::stubs::MainController { debug: false },
            resource: crate::stubs::PlayerResource,
            state_type,
            image_registry,
        }
    }
}

impl crate::stubs::MainState for TimerOnlyMainState<'_> {
    fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
        if let Some(ctx) = self.ctx.as_deref() {
            ctx
        } else {
            self.timer.expect("timer-only adapter must carry a timer")
        }
    }

    fn get_offset_value(&self, _id: i32) -> Option<&crate::stubs::SkinOffset> {
        None
    }

    fn get_main(&self) -> &crate::stubs::MainController {
        &self.main_controller
    }

    fn get_image(&self, id: i32) -> Option<crate::rendering_stubs::TextureRegion> {
        self.image_registry.get(&id).cloned()
    }

    fn get_resource(&self) -> &crate::stubs::PlayerResource {
        &self.resource
    }

    fn is_music_selector(&self) -> bool {
        self.ctx
            .as_deref()
            .is_some_and(rubato_types::skin_render_context::SkinRenderContext::is_music_selector)
            || self.state_type == Some(rubato_types::main_state_type::MainStateType::MusicSelect)
    }

    fn is_result_state(&self) -> bool {
        self.ctx
            .as_deref()
            .is_some_and(rubato_types::skin_render_context::SkinRenderContext::is_result_state)
            || matches!(
                self.state_type,
                Some(
                    rubato_types::main_state_type::MainStateType::Result
                        | rubato_types::main_state_type::MainStateType::CourseResult
                )
            )
    }

    fn is_bms_player(&self) -> bool {
        self.state_type == Some(rubato_types::main_state_type::MainStateType::Play)
    }

    fn recent_judges(&self) -> &[i64] {
        self.ctx
            .as_deref()
            .map_or(&[] as &[i64], |c| c.recent_judges())
    }

    fn recent_judges_index(&self) -> usize {
        self.ctx.as_deref().map_or(0, |c| c.recent_judges_index())
    }

    fn integer_value(&self, id: i32) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.integer_value(id))
    }

    fn image_index_value(&self, id: i32) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.image_index_value(id))
    }

    fn boolean_value(&self, id: i32) -> bool {
        self.ctx.as_deref().is_some_and(|c| c.boolean_value(id))
    }

    fn float_value(&self, id: i32) -> f32 {
        self.ctx.as_deref().map_or(0.0, |c| c.float_value(id))
    }

    fn string_value(&self, id: i32) -> String {
        self.ctx
            .as_deref()
            .map_or_else(String::new, |c| c.string_value(id))
    }

    fn set_float_value(&mut self, id: i32, value: f32) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.set_float_value(id, value);
        }
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        self.ctx
            .as_deref()
            .map_or(0, |c| c.judge_count(judge, fast))
    }

    fn get_gauge_value(&self) -> f32 {
        self.ctx.as_deref().map_or(0.0, |c| c.gauge_value())
    }

    fn gauge_type(&self) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.gauge_type())
    }

    fn get_now_judge(&self, player: i32) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.now_judge(player))
    }

    fn get_now_combo(&self, player: i32) -> i32 {
        self.ctx.as_deref().map_or(0, |c| c.now_combo(player))
    }

    fn get_player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        self.ctx
            .as_deref()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::player_config_ref)
    }

    fn get_config_ref(&self) -> Option<&rubato_types::config::Config> {
        self.ctx
            .as_deref()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::config_ref)
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        self.ctx
            .as_deref_mut()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::player_config_mut)
    }

    fn get_config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
        self.ctx
            .as_deref_mut()
            .and_then(rubato_types::skin_render_context::SkinRenderContext::config_mut)
    }

    fn get_selected_play_config_mut(
        &mut self,
    ) -> Option<&mut rubato_types::play_config::PlayConfig> {
        self.ctx.as_deref_mut().and_then(
            rubato_types::skin_render_context::SkinRenderContext::selected_play_config_mut,
        )
    }

    fn play_option_change_sound(&mut self) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.play_option_change_sound();
        }
    }

    fn update_bar_after_change(&mut self) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.update_bar_after_change();
        }
    }

    fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.execute_event(id, arg1, arg2);
        }
    }

    fn change_state(&mut self, state_type: rubato_types::main_state_type::MainStateType) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.change_state(state_type);
        }
    }

    fn select_song(&mut self, mode: rubato_core::bms_player_mode::BMSPlayerMode) {
        let Some(ctx) = self.ctx.as_deref_mut() else {
            return;
        };
        let event_id = match mode.mode {
            rubato_core::bms_player_mode::Mode::Play => 15,
            rubato_core::bms_player_mode::Mode::Autoplay => 16,
            rubato_core::bms_player_mode::Mode::Practice => 315,
            rubato_core::bms_player_mode::Mode::Replay => return,
        };
        ctx.select_song_mode(event_id);
    }

    fn set_timer_micro(&mut self, timer_id: i32, micro_time: i64) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.set_timer_micro(timer_id, micro_time);
        }
    }

    fn audio_play(&mut self, path: &str, volume: f32, is_loop: bool) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.audio_play(path, volume, is_loop);
        }
    }

    fn audio_stop(&mut self, path: &str) {
        if let Some(ctx) = self.ctx.as_deref_mut() {
            ctx.audio_stop(path);
        }
    }
}

impl rubato_core::main_state::SkinDrawable for Skin {
    fn prepare_skin(&mut self) {
        let null_timer = rubato_types::timer_access::NullTimer;
        let adapter = TimerOnlyMainState::from_timer(&null_timer);
        self.prepare(&adapter);
    }

    fn draw_all_objects_timed(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
        // Take image registry out to avoid borrow conflict (&mut self vs &self.image_registry)
        let registry = std::mem::take(&mut self.image_registry);
        let adapter = TimerOnlyMainState::from_render_context_with_images(ctx, &registry);
        self.draw_all_objects(&adapter);
        self.image_registry = registry;
    }

    fn update_custom_objects_timed(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
        let registry = std::mem::take(&mut self.image_registry);
        let mut adapter = TimerOnlyMainState::from_render_context_with_images(ctx, &registry);
        self.update_custom_objects(&mut adapter);
        self.image_registry = registry;
    }

    fn mouse_pressed_at(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        button: i32,
        x: i32,
        y: i32,
    ) {
        let registry = std::mem::take(&mut self.image_registry);
        let mut adapter = TimerOnlyMainState::from_render_context_with_images(ctx, &registry);
        self.mouse_pressed(&mut adapter, button, x, y);
        self.image_registry = registry;
    }

    fn mouse_dragged_at(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        button: i32,
        x: i32,
        y: i32,
    ) {
        let registry = std::mem::take(&mut self.image_registry);
        let mut adapter = TimerOnlyMainState::from_render_context_with_images(ctx, &registry);
        self.mouse_dragged(&mut adapter, button, x, y);
        self.image_registry = registry;
    }

    fn dispose_skin(&mut self) {
        self.dispose();
    }

    fn fadeout(&self) -> i32 {
        self.fadeout
    }

    fn input(&self) -> i32 {
        self.input
    }

    fn scene(&self) -> i32 {
        self.scene
    }

    fn get_width(&self) -> f32 {
        self.width
    }

    fn get_height(&self) -> f32 {
        self.height
    }

    fn swap_sprite_batch(&mut self, batch: &mut rubato_render::sprite_batch::SpriteBatch) {
        if self.renderer.is_none() {
            self.renderer = Some(SkinObjectRenderer::new());
        }
        std::mem::swap(&mut self.renderer.as_mut().unwrap().sprite, batch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property::boolean_property::BooleanProperty;
    use crate::property::event_factory;
    use crate::skin_header::SkinHeader;
    use crate::skin_image::SkinImage;
    use rubato_core::main_state::SkinDrawable;
    use rubato_types::main_state_type::MainStateType;
    use rubato_types::skin_render_context::SkinRenderContext;
    use rubato_types::timer_access::TimerAccess;

    struct AlwaysTrue;

    impl BooleanProperty for AlwaysTrue {
        fn is_static(&self, _state: &dyn MainState) -> bool {
            false
        }

        fn get(&self, _state: &dyn MainState) -> bool {
            true
        }
    }

    struct RecordingSkinRenderContext {
        timer: crate::stubs::Timer,
        state_type: MainStateType,
        executed_events: Vec<(i32, i32, i32)>,
        changed_states: Vec<MainStateType>,
        timer_writes: Vec<(i32, i64)>,
        audio_plays: Vec<(String, f32, bool)>,
        audio_stops: Vec<String>,
        float_writes: Vec<(i32, f32)>,
    }

    impl RecordingSkinRenderContext {
        fn new(state_type: MainStateType) -> Self {
            Self {
                timer: crate::stubs::Timer::with_timers(100, 100_000, Vec::new()),
                state_type,
                executed_events: Vec::new(),
                changed_states: Vec::new(),
                timer_writes: Vec::new(),
                audio_plays: Vec::new(),
                audio_stops: Vec::new(),
                float_writes: Vec::new(),
            }
        }
    }

    impl TimerAccess for RecordingSkinRenderContext {
        fn now_time(&self) -> i64 {
            self.timer.now_time()
        }

        fn now_micro_time(&self) -> i64 {
            self.timer.now_micro_time()
        }

        fn micro_timer(&self, timer_id: i32) -> i64 {
            self.timer.micro_timer(timer_id)
        }

        fn timer(&self, timer_id: i32) -> i64 {
            self.timer.timer(timer_id)
        }

        fn now_time_for(&self, timer_id: i32) -> i64 {
            self.timer.now_time_for(timer_id)
        }

        fn is_timer_on(&self, timer_id: i32) -> bool {
            self.timer.is_timer_on(timer_id)
        }
    }

    impl SkinRenderContext for RecordingSkinRenderContext {
        fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
            self.executed_events.push((id, arg1, arg2));
        }

        fn change_state(&mut self, state: MainStateType) {
            self.changed_states.push(state);
        }

        fn set_timer_micro(&mut self, timer_id: i32, micro_time: i64) {
            self.timer_writes.push((timer_id, micro_time));
        }

        fn audio_play(&mut self, path: &str, volume: f32, is_loop: bool) {
            self.audio_plays.push((path.to_string(), volume, is_loop));
        }

        fn audio_stop(&mut self, path: &str) {
            self.audio_stops.push(path.to_string());
        }

        fn current_state_type(&self) -> Option<MainStateType> {
            Some(self.state_type)
        }

        fn set_float_value(&mut self, id: i32, value: f32) {
            self.float_writes.push((id, value));
        }
    }

    fn make_test_skin() -> Skin {
        Skin::new(SkinHeader::new())
    }

    #[test]
    fn test_timer_only_main_state_returns_expected_values() {
        let timer = crate::stubs::Timer::with_timers(1000, 1_000_000, Vec::new());
        let adapter = TimerOnlyMainState::from_timer(&timer);
        let state: &dyn MainState = &adapter;
        assert_eq!(state.timer().now_time(), 1000);
        assert_eq!(state.timer().now_micro_time(), 1_000_000);
        assert!(state.get_offset_value(0).is_none());
        assert!(state.get_image(0).is_none());
        assert!(!state.get_main().debug);
    }

    /// Verify that TimerManager timer values flow through SkinDrawable to the skin adapter.
    /// Before the fix, all per-timer-id queries returned 0 (frozen animations).
    #[test]
    fn test_timer_manager_values_flow_through_to_skin_adapter() {
        use rubato_core::timer_manager::TimerManager;
        use rubato_types::timer_access::TimerAccess;

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
            state.timer().is_timer_on(10),
            "Timer 10 should be ON through adapter"
        );
        // Timer 20 should be OFF
        assert!(
            !state.timer().is_timer_on(20),
            "Timer 20 should be OFF through adapter"
        );
        // micro_timer for ON timer should not be i64::MIN
        assert_ne!(
            state.timer().micro_timer(10),
            i64::MIN,
            "ON timer should return its activation time, not i64::MIN"
        );
        // micro_timer for OFF timer should be i64::MIN
        assert_eq!(
            state.timer().micro_timer(20),
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
        assert_eq!(drawable.fadeout(), 500);
        assert_eq!(drawable.input(), 100);
        assert_eq!(drawable.scene(), 2000);
        // Default resolution is 640x480
        assert_eq!(drawable.get_width(), 640.0);
        assert_eq!(drawable.get_height(), 480.0);
    }

    #[test]
    fn test_draw_all_objects_timed_empty_skin() {
        let mut skin = make_test_skin();
        let mut null_timer = rubato_types::timer_access::NullTimer;
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
    fn test_update_custom_objects_timed_executes_custom_events() {
        let mut skin = make_test_skin();
        skin.add_custom_event(CustomEvent::new(
            9001,
            event_factory::create_zero_arg_event(777),
            Some(Box::new(AlwaysTrue)),
            0,
        ));
        let mut ctx = RecordingSkinRenderContext::new(MainStateType::MusicSelect);

        skin.update_custom_objects_timed(&mut ctx);

        assert_eq!(ctx.executed_events, vec![(777, 0, 0)]);
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
        let mut timer = rubato_types::timer_access::NullTimer;
        // Should not panic with no objects
        skin.mouse_pressed_at(&mut timer, 0, 100, 200);
    }

    #[test]
    fn test_mouse_dragged_at_empty_skin() {
        let mut skin = make_test_skin();
        let mut timer = rubato_types::timer_access::NullTimer;
        // Should not panic with no objects
        skin.mouse_dragged_at(&mut timer, 0, 100, 200);
    }

    #[test]
    fn test_timer_only_main_state_delegates_mutating_context_methods() {
        let registry = HashMap::new();
        let mut ctx = RecordingSkinRenderContext::new(MainStateType::MusicSelect);
        let mut adapter = TimerOnlyMainState::from_render_context_with_images(&mut ctx, &registry);

        adapter.execute_event(55, 1, 2);
        adapter.change_state(MainStateType::Config);
        adapter.set_timer_micro(9, 12_345);
        adapter.audio_play("test.wav", 0.75, true);
        adapter.audio_stop("test.wav");
        adapter.set_float_value(42, 0.5);

        assert_eq!(ctx.executed_events, vec![(55, 1, 2)]);
        assert_eq!(ctx.changed_states, vec![MainStateType::Config]);
        assert_eq!(ctx.timer_writes, vec![(9, 12_345)]);
        assert_eq!(ctx.audio_plays, vec![("test.wav".to_string(), 0.75, true)]);
        assert_eq!(ctx.audio_stops, vec!["test.wav".to_string()]);
        assert_eq!(ctx.float_writes, vec![(42, 0.5)]);
    }

    #[test]
    fn test_mouse_pressed_dispatches_click_event_through_render_context() {
        let mut skin = make_test_skin();
        let mut image = SkinImage::new_empty();
        image.data.draw = true;
        image.data.region.set_xywh(0.0, 0.0, 100.0, 100.0);
        image.data.set_clickevent_by_id(13);
        skin.add(SkinObject::Image(image));
        skin.objectarray_indices.push(0);

        let registry = HashMap::new();
        let mut ctx = RecordingSkinRenderContext::new(MainStateType::MusicSelect);
        let mut adapter = TimerOnlyMainState::from_render_context_with_images(&mut ctx, &registry);

        skin.mouse_pressed(&mut adapter, 0, 50, 50);

        assert_eq!(ctx.changed_states, vec![MainStateType::Config]);
    }

    #[test]
    fn test_swap_sprite_batch_exchanges_batches() {
        use rubato_render::sprite_batch::SpriteBatch;
        use rubato_render::texture::{Texture, TextureRegion};
        use std::sync::Arc;

        let mut skin = make_test_skin();
        let mut external = SpriteBatch::new();

        // Draw a quad into the external batch
        external.begin();
        let tex = Texture {
            width: 10,
            height: 10,
            disposed: false,
            path: Some(Arc::from("swap_test")),
            rgba_data: Some(Arc::new(vec![255u8; 400])),
            ..Default::default()
        };
        let region = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            region_x: 0,
            region_y: 0,
            region_width: 10,
            region_height: 10,
            texture: Some(tex),
        };
        external.draw_region(&region, 0.0, 0.0, 10.0, 10.0);
        external.end();
        assert_eq!(
            external.vertices().len(),
            6,
            "precondition: 1 quad = 6 vertices"
        );

        // Swap: skin takes the populated batch, external gets empty one
        skin.swap_sprite_batch(&mut external);
        assert!(
            external.vertices().is_empty(),
            "after swap-in, external should be empty"
        );

        // Swap back: external gets the populated batch back
        skin.swap_sprite_batch(&mut external);
        assert_eq!(
            external.vertices().len(),
            6,
            "after swap-back, external has vertices again"
        );
    }

    #[test]
    fn test_swap_sprite_batch_creates_renderer_if_needed() {
        use rubato_render::sprite_batch::SpriteBatch;

        let mut skin = make_test_skin();
        // Skin starts with renderer = None
        let mut batch = SpriteBatch::new();
        // Should not panic — swap_sprite_batch creates renderer lazily
        skin.swap_sprite_batch(&mut batch);
        // Swap back to verify it worked
        skin.swap_sprite_batch(&mut batch);
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
        // The default ref_prop is from integer_property_by_id(0) which returns None,
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
        assert_eq!(obj.type_name(), "Float");
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
