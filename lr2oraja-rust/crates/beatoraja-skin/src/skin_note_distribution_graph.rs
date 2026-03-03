// SkinNoteDistributionGraph.java -> skin_note_distribution_graph.rs
// Mechanical line-by-line translation.

use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;

use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::stubs::{
    Color, MainState, Pixmap, PixmapFormat, Rectangle, SongData, Texture, TextureRegion,
};

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
    past_notes: i32,
    notes_last_update_time: i64,
    cursor_last_update_time: i64,

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
    let data_len = DATA_LENGTH[graph_type as usize] as usize;
    let mut colors = Vec::with_capacity(data_len);
    for i in 0..data_len {
        colors.push(Color::value_of(JGRAPH[graph_type as usize][i]));
    }
    colors
}

fn get_pms_graph_colors(graph_type: i32) -> Vec<Color> {
    let data_len = DATA_LENGTH[graph_type as usize] as usize;
    let mut colors = Vec::with_capacity(data_len);
    for i in 0..data_len {
        colors.push(Color::value_of(PMS_GRAPH_COLOR[graph_type as usize][i]));
    }
    colors
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
            graph_type,
            is_back_tex_off: back_tex_off == 1,
            delay,
            is_order_reverse: order_reverse == 1,
            is_no_gap: no_gap == 1,
            is_no_gap_x: no_gap_x == 1,
            past_notes: 0,
            notes_last_update_time: 0,
            cursor_last_update_time: 0,
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
        self.render = if time >= self.delay as i64 {
            1.0_f32
        } else {
            time as f32 / self.delay as f32
        };
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer, state: &dyn MainState) {
        let song = state.get_resource().get_songdata();
        let model = song.and_then(|s| s.get_bms_model());

        // Initialize chips if null
        if self.chips.is_none() {
            let is_pms = self.graph_type != TYPE_NORMAL
                && model.is_some()
                && model.unwrap().get_mode() == Some(&Mode::POPN_9K);
            let graphcolor = if is_pms {
                get_pms_graph_colors(self.graph_type)
            } else {
                get_graph_colors(self.graph_type)
            };
            let mut chips = Vec::with_capacity(graphcolor.len());
            for i in 0..graphcolor.len() {
                let mut pixmap = Pixmap::new(1, 1, PixmapFormat::RGBA8888);
                pixmap.draw_pixel(
                    0,
                    0,
                    Color::to_int_bits(
                        255,
                        (graphcolor[i].b * 255.0) as i32,
                        (graphcolor[i].g * 255.0) as i32,
                        (graphcolor[i].r * 255.0) as i32,
                    ),
                );
                chips.push(pixmap);
            }
            self.chips = Some(chips);
        }

        let song_changed = match (&self.current, song) {
            (None, None) => false,
            (Some(_), None) | (None, Some(_)) => true,
            (Some(_), Some(_)) => true, // simplified
        };

        if self.shapetex.is_none() || song_changed || (!self.model_set && model.is_some()) {
            self.current = song.cloned();
            self.model_set = model.is_some();
            if self.graph_type == TYPE_NORMAL {
                if let Some(s) = song {
                    if let Some(info) = s.get_song_information() {
                        let distribution: Vec<Vec<i32>> = info
                            .get_distribution_values()
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
        let is_bms_player = state.is_bms_player();

        if is_bms_player {
            // Real-time update path (stubbed)
            if let Some(ref backtex) = self.backtex.clone() {
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    backtex,
                    region.x,
                    region.y + region.height,
                    region.width,
                    -region.height,
                );
            }
            if let Some(ref mut shapetex) = self.shapetex.clone() {
                let tex_width = shapetex.get_texture().map(|t| t.get_width()).unwrap_or(0);
                shapetex.set_region_width((tex_width as f32 * self.render) as i32);
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    shapetex,
                    region.x,
                    region.y + region.height,
                    region.width * self.render,
                    -region.height,
                );
            }
        } else {
            if let Some(ref backtex) = self.backtex.clone() {
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    backtex,
                    region.x,
                    region.y + region.height,
                    region.width,
                    -region.height,
                );
            }
            if let Some(ref mut shapetex) = self.shapetex.clone() {
                let tex_width = shapetex.get_texture().map(|t| t.get_width()).unwrap_or(0);
                shapetex.set_region_width((tex_width as f32 * self.render) as i32);
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    shapetex,
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
        time: i64,
        state: &dyn MainState,
        r: &Rectangle,
        starttime: i32,
        endtime: i32,
        freq: f32,
    ) {
        self.prepare_with_region(time, state, Some(r), starttime, endtime, freq);
        if self.data.draw {
            self.draw(sprite, state);
        }
    }

    fn update_graph_from_distribution(&mut self, distribution: &[Vec<i32>]) {
        self.dist_data = distribution.to_vec();
        self.max = 20;
        for i in 0..distribution.len() {
            let mut count = 0;
            for j in 0..distribution[i].len() {
                count += distribution[i][j];
            }
            if self.max < count {
                self.max = ((count / 10) * 10 + 10).min(100);
            }
        }

        self.update_texture(true);
    }

    fn update_graph(&mut self, model: Option<&BMSModel>) {
        if let Some(model) = model {
            let dl = DATA_LENGTH[self.graph_type as usize] as usize;
            let data_len = (model.get_last_time() / 1000 + 1) as usize;
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

        let mode = model.get_mode().cloned();
        // #LNMODE is explicitly set to 1 (LN)
        // or #LNMODE is undefined and getLntype (which reflects playconfig) is LN (0)
        let ignore_ln_end = model.get_lnmode() == 1
            || (model.get_lnmode() == 0
                && model.get_lntype() == bms_model::bms_model::LNTYPE_LONGNOTE);

        let tls = model.get_all_time_lines();
        for tl in tls {
            let index = (tl.get_time() / 1000) as usize;
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
                    if let Some(n) = tl.get_note(i) {
                        let st = n.get_state();
                        let t = n.get_play_time();
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
                                        let end_index = if let Some(pair_tl_idx) = n.get_pair() {
                                            if pair_tl_idx < tls.len() {
                                                (tls[pair_tl_idx].get_time() / 1000) as usize
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
        let old_w = self.shape.as_ref().map(|s| s.get_width()).unwrap_or(0);
        let old_h = self.shape.as_ref().map(|s| s.get_height()).unwrap_or(0);
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

                    for i in 0..self.dist_data.len() {
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
                for i in 0..self.dist_data.len() {
                    if !self.dist_data[i].is_empty() && self.dist_data[i][0] > 0 {
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
            }
            // else: backtex.getTexture().draw(back, 0, 0) - texture update stub
        }

        // Draw note distribution chips
        if let Some(ref chips) = self.chips.clone()
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
                                    0,
                                    0,
                                    1,
                                    1,
                                    i as i32 * 5,
                                    j * 5,
                                    4 + if self.is_no_gap_x { 1 } else { 0 },
                                    4 + if self.is_no_gap { 1 } else { 0 },
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
                                    0,
                                    0,
                                    1,
                                    1,
                                    i as i32 * 5,
                                    j * 5,
                                    4 + if self.is_no_gap_x { 1 } else { 0 },
                                    4 + if self.is_no_gap { 1 } else { 0 },
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
        }
        // else: shapetex.getTexture().draw(shape, 0, 0) - texture update stub
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
