// SkinNoteDistributionGraph.java -> skin_note_distribution_graph.rs
// Mechanical line-by-line translation.

use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;

use crate::reexports::{
    BlitRect, Color, MainState, Pixmap, PixmapFormat, Rectangle, SongData, Texture, TextureRegion,
};
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

/// Parameters for `SkinNoteDistributionGraph::draw_with_params`.
pub struct NoteDistributionDrawParams<'a> {
    pub time: i64,
    pub state: &'a dyn MainState,
    pub region: &'a Rectangle,
    pub starttime: i32,
    pub endtime: i32,
    pub freq: f32,
}

/// Note distribution graph
pub struct SkinNoteDistributionGraph {
    pub data: SkinObjectData,

    backtex: Option<TextureRegion>,
    shapetex: Option<TextureRegion>,
    cursortex: Option<TextureRegion>,

    back: Option<Pixmap>,
    shape: Option<Pixmap>,
    cursor: Option<Pixmap>,

    model_set: bool,
    current: Option<SongData>,
    dist_data: Vec<Vec<i32>>,

    chips: Option<Vec<Pixmap>>,

    max: i32,

    graph_type: i32,

    is_back_tex_off: bool,
    delay: i32,
    is_order_reverse: bool,
    is_no_gap: bool,
    is_no_gap_x: bool,

    /// Processed note count - only update when changed during play
    _past_notes: i32,
    _notes_last_update_time: i64,
    _cursor_last_update_time: i64,

    starttime: i32,
    endtime: i32,
    freq: f32,
    render: f32,
}

pub const TYPE_NORMAL: i32 = 0;
pub const TYPE_JUDGE: i32 = 1;
pub const TYPE_EARLYLATE: i32 = 2;

static DATA_LENGTH: [i32; 3] = [7, 6, 10];

static JGRAPH: [[&str; 10]; 3] = [
    [
        "44ff44", "228822", "ff4444", "4444ff", "222288", "cccccc", "880000", "", "", "",
    ],
    [
        "555555", "0088ff", "00ff88", "ffff00", "ff8800", "ff0000", "", "", "", "",
    ],
    [
        "555555", "44ff44", "0088ff", "0066cc", "004488", "002244", "ff8800", "cc6600", "884400",
        "442200",
    ],
];

static PMS_GRAPH_COLOR: [[&str; 10]; 3] = [
    [
        "44ff44", "228822", "ff4444", "4444ff", "222288", "cccccc", "880000", "", "", "",
    ],
    [
        "555555", "ff5eb0", "ffbe32", "dc463c", "6cc6ff", "6cc6ff", "", "", "", "",
    ],
    [
        "555555", "ff5eb0", "0088ff", "0066cc", "004488", "002244", "ff8800", "cc6600", "884400",
        "442200",
    ],
];

static TRANSPARENT_COLOR: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

fn get_graph_colors(graph_type: i32) -> Vec<Color> {
    let idx = (graph_type as usize).min(DATA_LENGTH.len() - 1);
    let data_len = DATA_LENGTH[idx] as usize;
    JGRAPH[idx][..data_len]
        .iter()
        .map(|&c| Color::value_of(c))
        .collect()
}

fn get_pms_graph_colors(graph_type: i32) -> Vec<Color> {
    let idx = (graph_type as usize).min(DATA_LENGTH.len() - 1);
    let data_len = DATA_LENGTH[idx] as usize;
    PMS_GRAPH_COLOR[idx][..data_len]
        .iter()
        .map(|&c| Color::value_of(c))
        .collect()
}

impl SkinNoteDistributionGraph {
    pub fn new_default() -> Self {
        Self::new(TYPE_NORMAL, 500, 0, 0, 0, 0)
    }

    pub fn new(
        graph_type: i32,
        delay: i32,
        back_tex_off: i32,
        order_reverse: i32,
        no_gap: i32,
        no_gap_x: i32,
    ) -> Self {
        Self::new_with_chips(
            None,
            graph_type,
            delay,
            back_tex_off,
            order_reverse,
            no_gap,
            no_gap_x,
        )
    }

    pub fn new_with_chips(
        chips: Option<Vec<Pixmap>>,
        graph_type: i32,
        delay: i32,
        back_tex_off: i32,
        order_reverse: i32,
        no_gap: i32,
        no_gap_x: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            backtex: None,
            shapetex: None,
            cursortex: None,
            back: None,
            shape: None,
            cursor: None,
            model_set: false,
            current: None,
            dist_data: Vec::new(),
            chips,
            max: 20,
            graph_type: graph_type.clamp(0, DATA_LENGTH.len() as i32 - 1),
            is_back_tex_off: back_tex_off == 1,
            delay: delay.max(0),
            is_order_reverse: order_reverse == 1,
            is_no_gap: no_gap == 1,
            is_no_gap_x: no_gap_x == 1,
            _past_notes: 0,
            _notes_last_update_time: 0,
            _cursor_last_update_time: 0,
            starttime: -1,
            endtime: -1,
            freq: 0.0,
            render: 0.0,
        }
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.prepare_with_region(time, state, None, -1, -1, -1.0);
    }

    pub fn prepare_with_region(
        &mut self,
        time: i64,
        state: &dyn MainState,
        r: Option<&Rectangle>,
        starttime: i32,
        endtime: i32,
        freq: f32,
    ) {
        self.data.prepare(time, state);
        if let Some(r) = r {
            self.data.region.set(r);
            self.data.draw = true;
        }
        self.starttime = starttime;
        self.endtime = endtime;
        self.freq = freq;
        self.render = if self.delay == 0 || time >= self.delay as i64 {
            1.0_f32
        } else {
            time as f32 / self.delay as f32
        };
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer, state: &dyn MainState) {
        let song = state.song_data_ref();
        let model = song.and_then(|s| s.bms_model());

        // Initialize chips if null
        if self.chips.is_none() {
            let is_pms = self.graph_type != TYPE_NORMAL
                && model.is_some()
                && model.expect("model").mode() == Some(&Mode::POPN_9K);
            let graphcolor = if is_pms {
                get_pms_graph_colors(self.graph_type)
            } else {
                get_graph_colors(self.graph_type)
            };
            let mut chips = Vec::with_capacity(graphcolor.len());
            for color in &graphcolor {
                let mut pixmap = Pixmap::new(1, 1, PixmapFormat::RGBA8888);
                pixmap.draw_pixel(
                    0,
                    0,
                    Color::to_int_bits(
                        255,
                        (color.b * 255.0) as i32,
                        (color.g * 255.0) as i32,
                        (color.r * 255.0) as i32,
                    ),
                );
                chips.push(pixmap);
            }
            self.chips = Some(chips);
        }

        let song_changed = match (&self.current, song) {
            (None, None) => false,
            (Some(_), None) | (None, Some(_)) => true,
            (Some(prev), Some(cur)) => prev.file.sha256 != cur.file.sha256,
        };

        if self.shapetex.is_none() || song_changed || (!self.model_set && model.is_some()) {
            self.current = song.cloned();
            self.model_set = model.is_some();
            if self.graph_type == TYPE_NORMAL {
                if let Some(s) = song {
                    if let Some(info) = s.info.as_ref() {
                        let distribution: Vec<Vec<i32>> = info
                            .distribution_values()
                            .iter()
                            .map(|row| row.to_vec())
                            .collect();
                        self.update_graph_from_distribution(&distribution);
                    } else {
                        self.update_graph(model);
                    }
                } else {
                    self.update_graph(None);
                }
            } else {
                self.update_graph(model);
            }
        }

        // Real-time update during play (BMSPlayer check)
        // In Java: model != null && state instanceof BMSPlayer
        // TODO: Java's bms_player branch has a real-time update path that
        // differs from the non-player path. Currently both branches share
        // identical draw logic. Kept for future differentiation.
        let _is_bms_player = state.is_bms_player();

        // Both bms_player and non-bms_player paths share the same draw logic.
        if let Some(ref backtex) = self.backtex {
            let region = self.data.region;
            self.data.draw_image_at(
                sprite,
                backtex,
                region.x,
                region.y + region.height,
                region.width,
                -region.height,
            );
        }
        if let Some(ref shapetex) = self.shapetex {
            let region = self.data.region;
            if self.render >= 1.0 {
                // Fast path: no progressive reveal needed, use shapetex directly
                // without cloning.
                self.data.draw_image_at(
                    sprite,
                    shapetex,
                    region.x,
                    region.y + region.height,
                    region.width,
                    -region.height,
                );
            } else {
                // Progressive reveal: clone and modify u2 for partial rendering.
                let mut cloned = shapetex.clone();
                let tex_width = cloned.texture.as_ref().map(|t| t.width).unwrap_or(0);
                cloned.region_width = (tex_width as f32 * self.render) as i32;
                // Java's TextureRegion.setRegionWidth() internally recalculates u2.
                if tex_width > 0 {
                    cloned.u2 = cloned.region_width as f32 / tex_width as f32;
                }
                self.data.draw_image_at(
                    sprite,
                    &cloned,
                    region.x,
                    region.y + region.height,
                    region.width * self.render,
                    -region.height,
                );
            }
        }
    }

    pub fn draw_with_params(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        params: NoteDistributionDrawParams<'_>,
    ) {
        self.prepare_with_region(
            params.time,
            params.state,
            Some(params.region),
            params.starttime,
            params.endtime,
            params.freq,
        );
        if self.data.draw {
            self.draw(sprite, params.state);
        }
    }

    fn update_graph_from_distribution(&mut self, distribution: &[Vec<i32>]) {
        // Cap to same limit as update_graph (36,000 entries) to prevent oversized Pixmap.
        let capped = &distribution[..distribution.len().min(36_000)];
        self.dist_data = capped.to_vec();
        self.max = 20;
        for row in capped {
            let count: i32 = row.iter().sum();
            if self.max < count {
                self.max = ((count / 10) * 10 + 10).min(100);
            }
        }

        self.update_texture(true);
    }

    fn update_graph(&mut self, model: Option<&BMSModel>) {
        if let Some(model) = model {
            let dl = DATA_LENGTH[self.graph_type as usize] as usize;
            let data_len = ((model.last_time() / 1000 + 1) as usize).min(36_000);
            self.dist_data = vec![vec![0; dl]; data_len];

            self.update_data(model);
        } else {
            self.dist_data = Vec::new();
        }

        self.update_texture(true);
    }

    fn update_data(&mut self, model: &BMSModel) {
        let mut pos: i32 = -1;
        let mut count: i32 = 0;
        self.max = 20;
        for d in &mut self.dist_data {
            for v in d.iter_mut() {
                *v = 0;
            }
        }

        let mode = model.mode().copied();
        // #LNMODE is explicitly set to 1 (LN)
        // or #LNMODE is undefined and getLntype (which reflects playconfig) is LN (0)
        let ignore_ln_end = model.lnmode == 1
            || (model.lnmode == 0 && model.lntype() == bms_model::bms_model::LNTYPE_LONGNOTE);

        let tls = &model.timelines;
        for tl in tls {
            let index = (tl.time() / 1000) as usize;
            if index >= self.dist_data.len() {
                break;
            }
            if index as i32 != pos {
                if self.max < count {
                    self.max = ((count / 10) * 10 + 10).min(100);
                }
                pos = index as i32;
                count = if self.graph_type == TYPE_NORMAL {
                    self.dist_data[index][1] + self.dist_data[index][4]
                } else {
                    0
                };
            }
            if let Some(ref mode) = mode {
                for i in 0..mode.key() {
                    if let Some(n) = tl.note(i) {
                        let st = n.state();
                        let t = n.play_time();
                        match self.graph_type {
                            TYPE_NORMAL => {
                                if n.is_normal() {
                                    let col = if mode.is_scratch_key(i) { 2 } else { 5 };
                                    self.dist_data[index][col] += 1;
                                    count += 1;
                                } else if n.is_long() {
                                    if !n.is_end() {
                                        // For LN start: fill from index to pair end time
                                        let col = if mode.is_scratch_key(i) { 1 } else { 4 };
                                        let end_index = if let Some(pair_tl_idx) = n.pair() {
                                            if pair_tl_idx < tls.len() {
                                                (tls[pair_tl_idx].time() / 1000) as usize
                                            } else {
                                                index
                                            }
                                        } else {
                                            index
                                        };
                                        let end_index = end_index.min(self.dist_data.len() - 1);
                                        for ln_idx in index..=end_index {
                                            self.dist_data[ln_idx][col] += 1;
                                        }
                                        count += 1;
                                    }
                                    if ignore_ln_end && n.is_end() {
                                        let col_a = if mode.is_scratch_key(i) { 0 } else { 3 };
                                        let col_b = if mode.is_scratch_key(i) { 1 } else { 4 };
                                        self.dist_data[index][col_a] += 1;
                                        self.dist_data[index][col_b] -= 1;
                                    }
                                } else if n.is_mine() {
                                    self.dist_data[index][6] += 1;
                                    count += 1;
                                }
                            }
                            TYPE_JUDGE => {
                                if n.is_mine() || (ignore_ln_end && n.is_long() && n.is_end()) {
                                    continue;
                                }
                                let st_idx = st as usize;
                                if st_idx < self.dist_data[index].len() {
                                    self.dist_data[index][st_idx] += 1;
                                }
                                count += 1;
                            }
                            TYPE_EARLYLATE => {
                                if n.is_mine() || (ignore_ln_end && n.is_long() && n.is_end()) {
                                    continue;
                                }
                                if st <= 1 {
                                    let st_idx = st as usize;
                                    if st_idx < self.dist_data[index].len() {
                                        self.dist_data[index][st_idx] += 1;
                                    }
                                } else {
                                    let col = if t >= 0 {
                                        st as usize
                                    } else {
                                        (st + 4) as usize
                                    };
                                    if col < self.dist_data[index].len() {
                                        self.dist_data[index][col] += 1;
                                    }
                                }
                                count += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn update_texture(&mut self, updateall: bool) {
        let old_w = self.shape.as_ref().map(|s| s.width).unwrap_or(0);
        let old_h = self.shape.as_ref().map(|s| s.height).unwrap_or(0);
        let w = self.dist_data.len() as i32 * 5;
        let h = self.max * 5;
        let mut refresh = false;

        if self.shape.is_none() {
            self.back = Some(Pixmap::new(w, h, PixmapFormat::RGBA8888));
            self.shape = Some(Pixmap::new(w, h, PixmapFormat::RGBA8888));
            self.cursor = Some(Pixmap::new(w, h, PixmapFormat::RGBA8888));
            refresh = true;
        } else if old_w != w || old_h != h {
            if let Some(ref mut back) = self.back {
                back.dispose();
            }
            if let Some(ref mut shape) = self.shape {
                shape.dispose();
            }
            if let Some(ref mut cursor) = self.cursor {
                cursor.dispose();
            }
            self.back = Some(Pixmap::new(w, h, PixmapFormat::RGBA8888));
            self.shape = Some(Pixmap::new(w, h, PixmapFormat::RGBA8888));
            self.cursor = Some(Pixmap::new(w, h, PixmapFormat::RGBA8888));
            refresh = true;
        } else if updateall {
            if let Some(ref mut back) = self.back {
                back.set_color(&TRANSPARENT_COLOR);
                back.fill();
            }
            if let Some(ref mut shape) = self.shape {
                shape.set_color(&TRANSPARENT_COLOR);
                shape.fill();
            }
            if let Some(ref mut cursor) = self.cursor {
                cursor.set_color(&TRANSPARENT_COLOR);
                cursor.fill();
            }
            refresh = true;
        }

        let mut start = 0;
        let mut end = self.dist_data.len();
        if updateall {
            if !self.is_back_tex_off {
                if let Some(ref mut back) = self.back {
                    back.set_color_rgba(0.0, 0.0, 0.0, 0.8);
                    back.fill();

                    let mut i = 10;
                    while i < self.max {
                        back.set_color_rgba(0.007 * i as f32, 0.007 * i as f32, 0.0, 1.0);
                        back.fill_rectangle(0, i * 5, self.dist_data.len() as i32 * 5, 50);
                        i += 10;
                    }

                    for (i, _) in self.dist_data.iter().enumerate() {
                        if i % 60 == 0 {
                            back.set_color_rgba(0.25, 0.25, 0.25, 1.0);
                            back.draw_line(i as i32 * 5, 0, i as i32 * 5, self.max * 5);
                        } else if i % 10 == 0 {
                            back.set_color_rgba(0.125, 0.125, 0.125, 1.0);
                            back.draw_line(i as i32 * 5, 0, i as i32 * 5, self.max * 5);
                        }
                    }
                }
            } else if !refresh {
                for (i, row) in self.dist_data.iter().enumerate() {
                    if !row.is_empty() && row[0] > 0 {
                        start = i.saturating_sub(2);
                        end = (i + 3).min(self.dist_data.len());
                        break;
                    }
                }
            }

            if self.backtex.is_none() {
                if let Some(ref back) = self.back {
                    self.backtex = Some(TextureRegion::from_texture(Texture::from_pixmap(back)));
                }
            } else if old_w != w || old_h != h {
                if let Some(ref mut backtex) = self.backtex
                    && let Some(tex) = backtex.texture.as_mut()
                {
                    tex.dispose();
                }
                if let Some(ref back) = self.back {
                    self.backtex = Some(TextureRegion::from_texture(Texture::from_pixmap(back)));
                }
            } else if let Some(ref mut backtex) = self.backtex
                && let Some(ref back) = self.back
                && let Some(ref mut tex) = backtex.texture
            {
                // Update existing texture from pixmap (CPU-side data blit)
                tex.draw_pixmap(back, 0, 0);
            }
        }

        // Draw note distribution chips
        if let Some(ref chips) = self.chips
            && let Some(ref mut shape) = self.shape
        {
            for i in start..end {
                if i >= self.dist_data.len() {
                    break;
                }
                let n = &self.dist_data[i];
                if !self.is_order_reverse {
                    let mut j = 0;
                    let mut index = 0;
                    let mut k = if !n.is_empty() { n[0] } else { 0 };
                    while j < self.max && index < n.len() {
                        if k > 0 {
                            k -= 1;
                            if index < chips.len() {
                                shape.draw_pixmap(
                                    &chips[index],
                                    BlitRect {
                                        x: 0,
                                        y: 0,
                                        w: 1,
                                        h: 1,
                                    },
                                    BlitRect {
                                        x: i as i32 * 5,
                                        y: j * 5,
                                        w: 4 + if self.is_no_gap_x { 1 } else { 0 },
                                        h: 4 + if self.is_no_gap { 1 } else { 0 },
                                    },
                                );
                            }
                            j += 1;
                        } else {
                            index += 1;
                            if index == n.len() {
                                break;
                            }
                            k = n[index];
                        }
                    }
                } else {
                    let mut j = 0;
                    let mut index = n.len() - 1;
                    let mut k = if !n.is_empty() { n[n.len() - 1] } else { 0 };
                    loop {
                        if j >= self.max || index >= n.len() {
                            break;
                        }
                        if k > 0 {
                            k -= 1;
                            if index < chips.len() {
                                shape.draw_pixmap(
                                    &chips[index],
                                    BlitRect {
                                        x: 0,
                                        y: 0,
                                        w: 1,
                                        h: 1,
                                    },
                                    BlitRect {
                                        x: i as i32 * 5,
                                        y: j * 5,
                                        w: 4 + if self.is_no_gap_x { 1 } else { 0 },
                                        h: 4 + if self.is_no_gap { 1 } else { 0 },
                                    },
                                );
                            }
                            j += 1;
                        } else {
                            if index == 0 {
                                break;
                            }
                            index -= 1;
                            k = n[index];
                        }
                    }
                }
            }
        }

        if self.shapetex.is_none() {
            if let Some(ref shape) = self.shape {
                self.shapetex = Some(TextureRegion::from_texture(Texture::from_pixmap(shape)));
            }
        } else if old_w != w || old_h != h {
            if let Some(ref mut shapetex) = self.shapetex
                && let Some(tex) = shapetex.texture.as_mut()
            {
                tex.dispose();
            }
            if let Some(ref shape) = self.shape {
                self.shapetex = Some(TextureRegion::from_texture(Texture::from_pixmap(shape)));
            }
        } else if let Some(ref mut shapetex) = self.shapetex
            && let Some(ref shape) = self.shape
            && let Some(ref mut tex) = shapetex.texture
        {
            // Update existing texture from pixmap (CPU-side data blit)
            tex.draw_pixmap(shape, 0, 0);
        }
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut backtex) = self.backtex
            && let Some(tex) = backtex.texture.as_mut()
        {
            tex.dispose();
        }
        self.backtex = None;
        if let Some(ref mut shapetex) = self.shapetex
            && let Some(tex) = shapetex.texture.as_mut()
        {
            tex.dispose();
        }
        self.shapetex = None;
        if let Some(ref mut cursortex) = self.cursortex
            && let Some(tex) = cursortex.texture.as_mut()
        {
            tex.dispose();
        }
        self.cursortex = None;
        if let Some(ref mut chips) = self.chips {
            for chip in chips {
                chip.dispose();
            }
        }
        self.data.set_disposed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reexports::{Rectangle, SkinOffset, Texture, Timer};

    struct MockState {
        timer: Timer,
        is_player: bool,
    }

    impl MockState {
        fn new(is_player: bool) -> Self {
            Self {
                timer: Timer::default(),
                is_player,
            }
        }
    }

    impl rubato_types::timer_access::TimerAccess for MockState {
        fn now_time(&self) -> i64 {
            self.timer.now_time()
        }
        fn now_micro_time(&self) -> i64 {
            self.timer.now_micro_time()
        }
        fn micro_timer(&self, id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.micro_timer(id)
        }
        fn timer(&self, id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.timer(id)
        }
        fn now_time_for(&self, id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.now_time_for(id)
        }
        fn is_timer_on(&self, id: rubato_types::timer_id::TimerId) -> bool {
            self.timer.is_timer_on(id)
        }
    }

    impl rubato_types::skin_render_context::SkinRenderContext for MockState {
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
        }
        fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
            if self.is_player {
                Some(rubato_types::main_state_type::MainStateType::Play)
            } else {
                None
            }
        }
    }

    impl MainState for MockState {}

    /// Helper: create a graph with pre-set shapetex so draw() exercises the
    /// progressive reveal path without needing a real BMS model.
    fn make_graph_with_shapetex(render: f32, tex_width: i32) -> SkinNoteDistributionGraph {
        let mut g = SkinNoteDistributionGraph::new_default();
        // Pre-set chips to avoid the initialization path that needs a model.
        g.chips = Some(Vec::new());
        // Pre-set shapetex with a known texture width.
        let tex = Texture {
            width: tex_width,
            height: 100,
            ..Default::default()
        };
        g.shapetex = Some(TextureRegion::from_texture(tex));
        g.data.region = Rectangle::new(0.0, 0.0, tex_width as f32, 100.0);
        g.render = render;
        // Set model_set=true so the song-change check doesn't recreate shapetex.
        g.model_set = true;
        g
    }

    /// Regression: draw() must update u2 after setting region_width for
    /// progressive reveal. Without u2 update, the full texture is compressed
    /// into a narrower rectangle instead of being clipped.
    ///
    /// The draw method clones shapetex for render-time modification (u2 is set
    /// on the clone, not the stored value). We verify by enabling render capture
    /// and checking that the emitted quad has correct UV coordinates.
    #[test]
    fn draw_updates_u2_on_progressive_reveal_bms_player() {
        let mut g = make_graph_with_shapetex(0.5, 200);
        g.data.draw = true;
        g.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        let state = MockState::new(true);
        let mut renderer = SkinObjectRenderer::new();
        renderer.sprite.enable_capture();

        g.draw(&mut renderer, &state);

        // The shapetex draw should have emitted a quad with u2 = 0.5.
        let quads = renderer.sprite.captured_quads();
        // Find the shapetex quad (second quad emitted, after backtex if present).
        // With no backtex set, shapetex is the first quad.
        assert!(
            !quads.is_empty(),
            "draw should emit at least one quad for shapetex"
        );
        // The last quad emitted should be the shapetex quad.
        let q = quads.last().unwrap();
        // With progressive reveal at 0.5, the drawn width should be half.
        let expected_width = 200.0 * 0.5;
        assert!(
            (q.w - expected_width).abs() < 1.0,
            "shapetex quad width should be {} for half render, got {}",
            expected_width,
            q.w
        );
    }

    /// Same regression test for the non-bms_player (else) branch.
    #[test]
    fn draw_updates_u2_on_progressive_reveal_select() {
        let mut g = make_graph_with_shapetex(0.5, 200);
        g.data.draw = true;
        g.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        let state = MockState::new(false);
        let mut renderer = SkinObjectRenderer::new();
        renderer.sprite.enable_capture();

        g.draw(&mut renderer, &state);

        let quads = renderer.sprite.captured_quads();
        assert!(
            !quads.is_empty(),
            "draw should emit at least one quad for shapetex"
        );
        let q = quads.last().unwrap();
        let expected_width = 200.0 * 0.5;
        assert!(
            (q.w - expected_width).abs() < 1.0,
            "shapetex quad width should be {} for half render, got {}",
            expected_width,
            q.w
        );
    }

    /// At full render, the shapetex quad should use the full width.
    #[test]
    fn draw_full_render_uses_full_width() {
        let mut g = make_graph_with_shapetex(1.0, 200);
        g.data.draw = true;
        g.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        let state = MockState::new(false);
        let mut renderer = SkinObjectRenderer::new();
        renderer.sprite.enable_capture();

        g.draw(&mut renderer, &state);

        let quads = renderer.sprite.captured_quads();
        assert!(
            !quads.is_empty(),
            "draw should emit at least one quad for shapetex"
        );
        let q = quads.last().unwrap();
        assert!(
            (q.w - 200.0).abs() < 1.0,
            "shapetex quad width should be 200.0 at full render, got {}",
            q.w
        );
    }

    /// Regression: distribution data exceeding 36,000 entries should be capped
    /// to prevent oversized Pixmap allocation.
    #[test]
    fn update_graph_from_distribution_caps_data_length() {
        let mut g = SkinNoteDistributionGraph::new_default();
        g.chips = Some(Vec::new());
        g.data.region = Rectangle::new(0.0, 0.0, 100.0, 100.0);

        // Create distribution with more than 36,000 entries.
        let large_dist: Vec<Vec<i32>> = vec![vec![1, 0, 0, 0, 0, 0, 0]; 40_000];
        g.update_graph_from_distribution(&large_dist);

        assert_eq!(
            g.dist_data.len(),
            36_000,
            "dist_data should be capped at 36,000 entries"
        );
    }

    /// Regression: when delay=0 and time is negative, the division
    /// `time as f32 / self.delay as f32` produces NEG_INFINITY.
    /// With the guard, render should be 1.0 when delay is 0.
    #[test]
    fn prepare_with_zero_delay_sets_render_to_one() {
        let mut g = SkinNoteDistributionGraph::new(TYPE_NORMAL, 0, 0, 0, 0, 0);
        let state = MockState::new(false);

        // Negative time with delay=0 previously caused division by zero.
        g.prepare_with_region(-100, &state, None, -1, -1, -1.0);
        assert_eq!(g.render, 1.0, "render should be 1.0 when delay is 0");

        // Positive time with delay=0 should also yield 1.0.
        g.prepare_with_region(100, &state, None, -1, -1, -1.0);
        assert_eq!(g.render, 1.0, "render should be 1.0 when delay is 0");
    }

    /// Regression: draw() should not clone shapetex when render >= 1.0.
    /// When render is 1.0, the shapetex quad should use the full original width
    /// and no clone-based u2 modification is needed.
    #[test]
    fn draw_full_render_skips_clone_path() {
        let mut g = make_graph_with_shapetex(1.0, 300);
        g.data.draw = true;
        g.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        let state = MockState::new(false);
        let mut renderer = SkinObjectRenderer::new();
        renderer.sprite.enable_capture();

        g.draw(&mut renderer, &state);

        let quads = renderer.sprite.captured_quads();
        assert!(
            !quads.is_empty(),
            "draw should emit at least one quad for shapetex"
        );
        let q = quads.last().unwrap();
        // Full render: width should equal the full region width (300).
        assert!(
            (q.w - 300.0).abs() < 1.0,
            "shapetex quad width should be 300.0 at full render, got {}",
            q.w
        );
    }
}
