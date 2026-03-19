/// Main skin class
pub struct Skin {
    pub header: SkinHeader,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
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
    pub input: i32,
    /// Scene time (ms)
    pub scene: i32,
    /// Fadeout time (ms)
    pub fadeout: i32,
    /// Rank animation time (ms) -- result/course-result skins only.
    /// Controls score animation timing. Set by LR2 STARTINPUT str[2].
    pub ranktime: i32,

    pub option: HashMap<i32, i32>,
    pub offset: HashMap<i32, SkinConfigOffset>,

    custom_events: HashMap<i32, CustomEvent>,
    custom_timers: HashMap<i32, CustomTimer>,

    /// Debug maps (None when not in debug mode)
    pub tempmap: Option<HashMap<String, [i64; 7]>>,
    pub pcntmap: Option<HashMap<String, [i64; 7]>>,
    _avem_prepare: Option<HashMap<String, Vec<i64>>>,
    _avem_draw: Option<HashMap<String, Vec<i64>>>,
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
        let dw = crate::safe_div_f32(dst.width, org.width);
        let dh = crate::safe_div_f32(dst.height, org.height);

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
            ranktime: 0,
            option: HashMap::new(),
            offset: HashMap::new(),
            custom_events: HashMap::new(),
            custom_timers: HashMap::new(),
            tempmap,
            pcntmap,
            _avem_prepare: avem_prepare,
            _avem_draw: avem_draw,
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
        use crate::render_reexports::{Pixmap, PixmapFormat, Texture};
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

    pub fn set_destination(
        &mut self,
        obj_index: usize,
        params: &DestinationParams,
        timer: i32,
        ops: &[i32],
        offset: &[i32],
    ) {
        let scaled = self.scale_params(params);
        let timer_prop = if timer > 0 {
            timer_property_factory::timer_property(timer)
        } else {
            None
        };
        if let Some(obj) = self.objects.get_mut(obj_index) {
            obj.set_destination(&scaled, timer_prop, ops, offset);
        }
    }

    pub fn set_destination_with_timer(
        &mut self,
        obj_index: usize,
        params: &DestinationParams,
        timer: Option<TimerPropertyEnum>,
        op: &[i32],
    ) {
        let scaled = self.scale_params(params);
        if let Some(obj) = self.objects.get_mut(obj_index) {
            obj.set_destination_with_timer_ops(&scaled, timer, op);
        }
    }

    pub fn set_destination_with_timer_draw(
        &mut self,
        obj_index: usize,
        params: &DestinationParams,
        timer: Option<TimerPropertyEnum>,
        draw: Box<dyn BooleanProperty>,
    ) {
        let scaled = self.scale_params(params);
        if let Some(obj) = self.objects.get_mut(obj_index) {
            obj.set_destination_with_timer_draw(&scaled, timer, draw);
        }
    }

    /// Scale destination params by the skin's dw/dh ratios.
    fn scale_params(&self, params: &DestinationParams) -> DestinationParams {
        DestinationParams {
            time: params.time,
            x: params.x * self.dw,
            y: params.y * self.dh,
            w: params.w * self.dw,
            h: params.h * self.dh,
            acc: params.acc,
            a: params.a,
            r: params.r,
            g: params.g,
            b: params.b,
            blend: params.blend,
            filter: params.filter,
            angle: params.angle,
            center: params.center,
            loop_val: params.loop_val,
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

        if !remove_indices.is_empty() {
            let mut type_counts: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for &i in &remove_indices {
                *type_counts
                    .entry(self.objects[i].type_name())
                    .or_default() += 1;
            }
            debug!(
                "Skin::prepare removing {} / {} objects: {:?}",
                remove_indices.len(),
                self.objects.len(),
                type_counts
            );
        }

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

        // Prepare frame rate throttle: microseconds per prepare cycle.
        // Java: prepareduration = fps > 0 ? 1_000_000 / fps : 1
        let fps = state.prepare_fps();
        self.prepareduration = if fps > 0 { 1_000_000 / fps as i64 } else { 1 };
        self.nextpreparetime = -1;
    }

    pub fn draw_all_objects(&mut self, state: &dyn MainState) {
        if self.renderer.is_none() {
            let mut renderer = SkinObjectRenderer::new();
            // Apply OFFSET_ALL transform matrix (Java: Skin.drawAllObjects)
            let offset_all = self.offset_all();
            let transform = if let Some(ref oa) = offset_all {
                rubato_render::color::TransformComponents {
                    tx: self.width * oa.x / 100.0,
                    ty: self.height * oa.y / 100.0,
                    tz: 0.0,
                    qx: 0.0,
                    qy: 0.0,
                    qz: 0.0,
                    qw: 0.0,
                    sx: (oa.w + 100.0) / 100.0,
                    sy: (oa.h + 100.0) / 100.0,
                    sz: 1.0,
                }
            } else {
                rubato_render::color::TransformComponents {
                    tx: 0.0,
                    ty: 0.0,
                    tz: 0.0,
                    qx: 0.0,
                    qy: 0.0,
                    qz: 0.0,
                    qw: 0.0,
                    sx: 1.0,
                    sy: 1.0,
                    sz: 1.0,
                }
            };
            let mut matrix = rubato_render::color::Matrix4::new();
            matrix.set(&transform);
            renderer.sprite.set_transform_matrix(&matrix);
            self.renderer = Some(renderer);
        }

        let microtime = state.now_micro_time();
        let debug = false; // MainController.debug stubbed as false

        if !debug {
            if self.nextpreparetime <= microtime {
                let time = state.now_time();
                for idx in &self.objectarray_indices {
                    self.objects[*idx].prepare(time, state);
                }

                self.nextpreparetime += ((microtime - self.nextpreparetime) / self.prepareduration
                    + 1)
                    * self.prepareduration;
            }

            // Periodic diagnostic: log draw gate results every 300 frames
            static FRAME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let frame_num = FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if frame_num.is_multiple_of(300) && log::log_enabled!(log::Level::Info) {
                let mut drawable: std::collections::HashMap<&str, usize> =
                    std::collections::HashMap::new();
                let mut skipped: std::collections::HashMap<&str, usize> =
                    std::collections::HashMap::new();
                let mut no_dst_count = 0usize;
                let mut has_timer_count = 0usize;
                let mut no_timer_no_dst = 0usize;
                for idx in &self.objectarray_indices {
                    let obj = &self.objects[*idx];
                    if obj.is_draw() && obj.is_visible() {
                        *drawable.entry(obj.type_name()).or_default() += 1;
                    } else {
                        *skipped.entry(obj.type_name()).or_default() += 1;
                        let data = obj.data();
                        if data.dst.is_empty() && data.fixr.is_none() {
                            no_dst_count += 1;
                        }
                        if data.dsttimer.is_some() {
                            has_timer_count += 1;
                        } else if data.dst.is_empty() && data.fixr.is_none() {
                            no_timer_no_dst += 1;
                        }
                    }
                }
                log::info!(
                    "draw_all_objects frame {}: drawable={:?}, skipped={:?}, skipped_with_timer={}, no_dst={}, no_timer_no_dst={}",
                    frame_num,
                    drawable,
                    skipped,
                    has_timer_count,
                    no_dst_count,
                    no_timer_no_dst,
                );
            }

            let renderer = self.renderer.as_mut().expect("renderer is Some");
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

    pub fn fadeout(&self) -> i32 {
        self.fadeout
    }

    pub fn input(&self) -> i32 {
        self.input
    }

    pub fn scene(&self) -> i32 {
        self.scene
    }

    pub fn ranktime(&self) -> i32 {
        self.ranktime
    }

    pub fn option(&self) -> &HashMap<i32, i32> {
        &self.option
    }

    pub fn offset(&self) -> &HashMap<i32, SkinConfigOffset> {
        &self.offset
    }

    pub fn offset_mut(&mut self) -> &mut HashMap<i32, SkinConfigOffset> {
        &mut self.offset
    }


    pub fn scale_x(&self) -> f64 {
        self.dw as f64
    }

    pub fn scale_y(&self) -> f64 {
        self.dh as f64
    }

    /// Returns the OFFSET_ALL skin offset for play skins (non-battle).
    ///
    /// In Java, this checked `state instanceof BMSPlayer` and the skin type.
    /// In Rust, we read from `self.offset` which is populated from PlayerConfig
    /// at load time, and check the skin type from the header.
    pub fn offset_all(&self) -> Option<SkinOffset> {
        let skin_type = self.header.skin_type()?;
        // Only non-battle play skins support OFFSET_ALL (matches Java's switch cases)
        if !skin_type.is_play() || skin_type.is_battle() {
            return None;
        }
        let cfg = self.offset.get(&crate::skin_property::OFFSET_ALL)?;
        // Return None if all values are zero (no offset configured)
        if cfg.x == 0.0 && cfg.y == 0.0 && cfg.w == 0.0 && cfg.h == 0.0 {
            return None;
        }
        Some(SkinOffset {
            x: cfg.x,
            y: cfg.y,
            w: cfg.w,
            h: cfg.h,
            r: cfg.r,
            a: cfg.a,
        })
    }

    pub fn add_custom_event(&mut self, event: CustomEvent) {
        let id = event.id;
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
        let id = timer.id;
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
    pub fn add_number(
        &mut self,
        number: SkinNumber,
        params: &DestinationParams,
        timer: Option<TimerPropertyEnum>,
        ops: &[i32],
        offset: i32,
    ) {
        let scaled = self.scale_params(params);
        let mut obj = SkinObject::Number(number);
        obj.data_mut()
            .set_destination_with_timer_ops_and_single_offset(&scaled, timer, ops, offset);
        self.objects.push(obj);
    }

    /// Add a SkinImage from a TextureRegion with destination and register it.
    /// Corresponds to Java Skin.addImage(21 params)
    pub fn add_image(
        &mut self,
        tr: TextureRegion,
        params: &DestinationParams,
        timer: Option<TimerPropertyEnum>,
        ops: &[i32],
        offset: i32,
    ) -> usize {
        let scaled = self.scale_params(params);
        let si = SkinImage::new_with_single(tr);
        let mut obj = SkinObject::Image(si);
        obj.data_mut()
            .set_destination_with_timer_ops_and_single_offset(&scaled, timer, ops, offset);
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
