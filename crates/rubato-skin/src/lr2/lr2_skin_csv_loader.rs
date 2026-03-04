use std::collections::HashMap;
use std::path::Path;

use log::warn;

use crate::lr2::lr2_font_loader::LR2FontLoader;
use crate::lr2::lr2_skin_loader::{self, LR2SkinLoaderState};
use crate::skin::SkinObject;
use crate::skin_gauge::SkinGauge;
use crate::skin_image::SkinImage;
use crate::skin_text_image::SkinTextImageSource;
use crate::stubs::{MainState, Resolution, Texture, TextureRegion};

/// LR2 CSV skin loader base
///
/// Translated from LR2SkinCSVLoader.java
/// Base class for all LR2 CSV-based skin loaders.
/// Provides IMAGE, LR2FONT, SRC_IMAGE, DST_IMAGE, SRC_NUMBER, DST_NUMBER,
/// SRC_TEXT, DST_TEXT, SRC_SLIDER, DST_SLIDER, SRC_BARGRAPH, DST_BARGRAPH,
/// SRC_BUTTON, DST_BUTTON, SRC_ONMOUSE, DST_ONMOUSE, SRC_GROOVEGAUGE, DST_GROOVEGAUGE,
/// INCLUDE, STARTINPUT, SCENETIME, FADEOUT, STRETCH commands.
///
/// Image list entry (can be Texture or MovieSource)
pub enum ImageListEntry {
    TextureEntry(Texture),
    Movie(String),
    Null,
}

/// State for CSV loader
pub struct LR2SkinCSVLoaderState {
    pub base: LR2SkinLoaderState,
    pub imagelist: Vec<ImageListEntry>,
    pub fontlist: Vec<Option<SkinTextImageSource>>,

    /// Source resolution
    pub src: Resolution,
    /// Destination resolution
    pub dst: Resolution,
    pub usecim: bool,
    pub skinpath: String,

    pub filemap: HashMap<String, String>,

    // Accumulated skin property values (applied to Skin by caller)
    pub stretch: i32,
    /// Input start time (ms) — set by STARTINPUT command
    pub skin_input: Option<i32>,
    /// Scene time (ms) — set by SCENETIME command
    pub skin_scene: Option<i32>,
    /// Fadeout time (ms) — set by FADEOUT command
    pub skin_fadeout: Option<i32>,

    pub groovex: i32,
    pub groovey: i32,
    pub line: Option<String>,
    pub imagesetarray: Vec<Vec<TextureRegion>>,

    // Active skin objects (built by SRC, destination set by DST)
    pub button: Option<SkinImage>,
    pub onmouse: Option<SkinImage>,
    pub gauger: Option<SkinGauge>,
    /// Collected skin objects to add to Skin after parsing
    pub collected_objects: Vec<SkinObject>,
}

impl LR2SkinCSVLoaderState {
    pub fn new(src: Resolution, dst: Resolution, usecim: bool, skinpath: String) -> Self {
        let mut base = LR2SkinLoaderState::new();

        // Register CSV command names
        let csv_commands = [
            "STARTINPUT",
            "SCENETIME",
            "FADEOUT",
            "STRETCH",
            "INCLUDE",
            "IMAGE",
            "LR2FONT",
            "SRC_IMAGE",
            "DST_IMAGE",
            "IMAGESET",
            "SRC_IMAGESET",
            "SRC_NUMBER",
            "DST_NUMBER",
            "SRC_TEXT",
            "DST_TEXT",
            "SRC_SLIDER",
            "SRC_SLIDER_REFNUMBER",
            "DST_SLIDER",
            "SRC_BARGRAPH",
            "SRC_BARGRAPH_REFNUMBER",
            "DST_BARGRAPH",
            "SRC_BUTTON",
            "DST_BUTTON",
            "SRC_ONMOUSE",
            "DST_ONMOUSE",
            "SRC_GROOVEGAUGE",
            "SRC_GROOVEGAUGE_EX",
            "DST_GROOVEGAUGE",
        ];
        for cmd in &csv_commands {
            base.add_command_name(cmd);
        }

        Self {
            base,
            imagelist: Vec::new(),
            fontlist: Vec::new(),
            src,
            dst,
            usecim,
            skinpath,
            filemap: HashMap::new(),
            stretch: -1,
            skin_input: None,
            skin_scene: None,
            skin_fadeout: None,
            groovex: 0,
            groovey: 0,
            line: None,
            imagesetarray: Vec::new(),
            button: None,
            onmouse: None,
            gauger: None,
            collected_objects: Vec::new(),
        }
    }

    /// Parse int array from string parts
    pub fn parse_int(s: &[String]) -> [i32; 22] {
        lr2_skin_loader::parse_int(s)
    }

    /// Read offset array
    pub fn read_offset(str_parts: &[String], start_index: usize) -> Vec<i32> {
        lr2_skin_loader::read_offset(str_parts, start_index)
    }

    /// Read offset with base
    pub fn read_offset_with_base(
        str_parts: &[String],
        start_index: usize,
        offset: &[i32],
    ) -> Vec<i32> {
        lr2_skin_loader::read_offset_with_base(str_parts, start_index, offset)
    }

    /// Get source image regions from texture
    pub fn get_source_image(&self, values: &[i32; 22]) -> Option<Vec<TextureRegion>> {
        let gr = values[2] as usize;
        if gr < self.imagelist.len()
            && let ImageListEntry::TextureEntry(ref tex) = self.imagelist[gr]
        {
            return Some(Self::get_source_image_from_texture(
                tex, values[3], values[4], values[5], values[6], values[7], values[8],
            ));
        }
        warn!("IMAGE is not defined or failed to load: {:?}", self.line);
        None
    }

    /// Get source image regions from texture with coordinates
    pub fn get_source_image_from_texture(
        image: &Texture,
        x: i32,
        y: i32,
        mut w: i32,
        mut h: i32,
        mut divx: i32,
        mut divy: i32,
    ) -> Vec<TextureRegion> {
        if w == -1 {
            w = image.get_width();
        }
        if h == -1 {
            h = image.get_height();
        }
        if divx <= 0 {
            divx = 1;
        }
        if divy <= 0 {
            divy = 1;
        }
        let mut images = vec![TextureRegion::new(); (divx * divy) as usize];
        for i in 0..divx {
            for j in 0..divy {
                images[(divx * j + i) as usize] = TextureRegion::from_texture_region(
                    image.clone(),
                    x + w / divx * i,
                    y + h / divy * j,
                    w / divx,
                    h / divy,
                );
            }
        }
        images
    }

    /// Process a CSV command
    pub fn process_csv_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            "STARTINPUT" => {
                if str_parts.len() > 1 {
                    self.skin_input = str_parts[1].trim().parse().ok();
                }
            }
            "SCENETIME" => {
                if str_parts.len() > 1 {
                    self.skin_scene = str_parts[1].trim().parse().ok();
                }
            }
            "FADEOUT" => {
                if str_parts.len() > 1 {
                    self.skin_fadeout = str_parts[1].trim().parse().ok();
                }
            }
            "STRETCH" => {
                if str_parts.len() > 1 {
                    self.stretch = str_parts[1].trim().parse().unwrap_or(-1);
                }
            }
            "IMAGE" => {
                let imagefile =
                    lr2_skin_loader::get_lr2_path(&self.skinpath, &str_parts[1], &self.filemap);
                let path = Path::new(&imagefile);
                if path.exists() {
                    let is_movie = ["mpg", "mpeg", "avi", "wmv", "mp4", "m4v"]
                        .iter()
                        .any(|ext| imagefile.to_lowercase().ends_with(ext));
                    if is_movie {
                        self.imagelist.push(ImageListEntry::Movie(imagefile));
                    } else {
                        self.imagelist
                            .push(ImageListEntry::TextureEntry(Texture::new(&imagefile)));
                    }
                } else {
                    warn!(
                        "IMAGE {} : file not found : {}",
                        self.imagelist.len(),
                        imagefile
                    );
                    self.imagelist.push(ImageListEntry::Null);
                }
            }
            "LR2FONT" => {
                let imagefile =
                    lr2_skin_loader::get_lr2_path(&self.skinpath, &str_parts[1], &self.filemap);
                let path = Path::new(&imagefile);
                if path.exists() {
                    let mut loader = LR2FontLoader::new(self.usecim);
                    match loader.load_font(path) {
                        Ok(data) => {
                            let mut source = SkinTextImageSource::new(data.usecim);
                            source.set_size(data.size);
                            source.set_margin(data.margin);
                            for (i, p) in data.paths.iter().enumerate() {
                                if let Some(p) = p {
                                    source.set_path(i as i32, p.clone());
                                }
                            }
                            for entry in &data.images {
                                source.set_image(
                                    entry.code,
                                    entry.texture_index,
                                    entry.x,
                                    entry.y,
                                    entry.w,
                                    entry.h,
                                );
                            }
                            self.fontlist.push(Some(source));
                        }
                        Err(e) => {
                            warn!("LR2FONT load error: {} : {}", imagefile, e);
                            self.fontlist.push(None);
                        }
                    }
                } else {
                    warn!(
                        "IMAGE {} : file not found : {}",
                        self.imagelist.len(),
                        imagefile
                    );
                    self.fontlist.push(None);
                }
            }
            "INCLUDE" => {
                let imagefile =
                    lr2_skin_loader::get_lr2_path(&self.skinpath, &str_parts[1], &self.filemap);
                let path = Path::new(&imagefile);
                if path.exists() {
                    match std::fs::read(path) {
                        Ok(raw_bytes) => {
                            let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_bytes);
                            let content = decoded.into_owned();
                            for line in content.lines() {
                                self.line = Some(line.to_string());
                                // Note: state=None means #IF conditionals in included files
                                // won't evaluate against MainState. This matches common usage
                                // where INCLUDE files contain unconditional definitions.
                                if let Some((cmd, parts)) =
                                    self.base.process_line_directives(line, None)
                                {
                                    self.process_csv_command(&cmd, &parts);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("INCLUDE: failed to read {}: {}", imagefile, e);
                        }
                    }
                }
            }
            "SRC_BUTTON" => {
                // Finalize previous button
                if let Some(btn) = self.button.take() {
                    self.collected_objects.push(SkinObject::Image(btn));
                }
                let gr: usize = str_parts
                    .get(2)
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                if gr < self.imagelist.len()
                    && matches!(self.imagelist[gr], ImageListEntry::TextureEntry(_))
                {
                    let values = Self::parse_int(str_parts);
                    let _divx = if values[7] > 0 { values[7] } else { 1 };
                    let _divy = if values[8] > 0 { values[8] } else { 1 };
                    let length = values[15];
                    let images = if length <= 0 {
                        // Grid-based division: each cell is one animation frame
                        let src_images = self.get_source_image(&values);
                        match src_images {
                            Some(imgs) => {
                                // Each source image becomes its own frame (single-element vec)
                                imgs.into_iter().map(|img| vec![img]).collect()
                            }
                            None => Vec::new(),
                        }
                    } else {
                        // Split source images into `length` groups
                        match self.get_source_image(&values) {
                            Some(srcimg) => {
                                let len = length as usize;
                                let group_size = srcimg.len() / len;
                                (0..len)
                                    .map(|i| {
                                        (0..group_size)
                                            .map(|j| srcimg[i * group_size + j].clone())
                                            .collect()
                                    })
                                    .collect()
                            }
                            None => Vec::new(),
                        }
                    };
                    if !images.is_empty() {
                        let mut btn = SkinImage::new_with_int_timer_ref_id(
                            images, values[10], values[9], values[11],
                        );
                        if values[12] == 1 {
                            btn.data.set_clickevent_by_id(values[11]);
                            let click_type = if values[14] > 0 {
                                0
                            } else if values[14] < 0 {
                                1
                            } else {
                                2
                            };
                            btn.data.set_clickevent_type(click_type);
                        }
                        self.button = Some(btn);
                    }
                }
            }
            "DST_BUTTON" => {
                if let Some(ref mut button) = self.button {
                    let values = Self::parse_int(str_parts);
                    let dstw = self.dst.width / self.src.width;
                    let dsth = self.dst.height / self.src.height;
                    let offsets = Self::read_offset(str_parts, 21);
                    button.data.set_destination_with_int_timer_ops(
                        values[2] as i64,
                        values[3] as f32 * dstw,
                        self.dst.height - (values[4] + values[6]) as f32 * dsth,
                        values[5] as f32 * dstw,
                        values[6] as f32 * dsth,
                        values[7],
                        values[8],
                        values[9],
                        values[10],
                        values[11],
                        values[12],
                        values[13],
                        values[14],
                        values[15],
                        values[16],
                        values[17],
                        &offsets,
                    );
                }
            }
            "SRC_ONMOUSE" => {
                // Finalize previous onmouse
                if let Some(om) = self.onmouse.take() {
                    self.collected_objects.push(SkinObject::Image(om));
                }
                let gr: usize = str_parts
                    .get(2)
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                if gr < self.imagelist.len()
                    && matches!(self.imagelist[gr], ImageListEntry::TextureEntry(_))
                {
                    let values = Self::parse_int(str_parts);
                    if let Some(images) = self.get_source_image(&values) {
                        let mut om = SkinImage::new_with_int_timer(images, values[10], values[9]);
                        // Set mouse hitbox rectangle
                        let rect_x = values[12] as f32;
                        let rect_y = (values[6] - values[13] - values[15]) as f32;
                        let rect_w = values[14] as f32;
                        let rect_h = values[15] as f32;
                        om.data.set_mouse_rect(rect_x, rect_y, rect_w, rect_h);
                        self.onmouse = Some(om);
                    }
                }
            }
            "DST_ONMOUSE" => {
                if let Some(ref mut onmouse) = self.onmouse {
                    let values = Self::parse_int(str_parts);
                    let dstw = self.dst.width / self.src.width;
                    let dsth = self.dst.height / self.src.height;
                    let offsets = Self::read_offset(str_parts, 21);
                    onmouse.data.set_destination_with_int_timer_ops(
                        values[2] as i64,
                        values[3] as f32 * dstw,
                        self.dst.height - (values[4] + values[6]) as f32 * dsth,
                        values[5] as f32 * dstw,
                        values[6] as f32 * dsth,
                        values[7],
                        values[8],
                        values[9],
                        values[10],
                        values[11],
                        values[12],
                        values[13],
                        values[14],
                        values[15],
                        values[16],
                        values[17],
                        &offsets,
                    );
                }
            }
            "SRC_GROOVEGAUGE" | "SRC_GROOVEGAUGE_EX" => {
                // Finalize previous gauger
                if let Some(g) = self.gauger.take() {
                    self.collected_objects.push(SkinObject::Gauge(g));
                }
                let values = Self::parse_int(str_parts);
                let gr = values[2] as usize;
                if gr < self.imagelist.len()
                    && matches!(self.imagelist[gr], ImageListEntry::TextureEntry(_))
                {
                    let divx = if values[7] > 0 { values[7] } else { 1 };
                    let divy = if values[8] > 0 { values[8] } else { 1 };
                    let total = (divx * divy) as usize;
                    let is_ex = cmd == "SRC_GROOVEGAUGE_EX";

                    // Build gauge image array: gauge[state][slot] with 36 slots per state
                    let gauge = self.build_gauge_image_array(&values, divx, divy, total, is_ex);

                    self.groovex = values[11];
                    self.groovey = values[12];

                    let parts;
                    let anim_type;
                    let anim_range;
                    let duration;
                    if values[13] == 0 {
                        // Default values (POPN_9K check omitted — would need mode context)
                        parts = 50;
                        anim_type = 0;
                        anim_range = 3;
                        duration = 33;
                    } else {
                        parts = values[13];
                        anim_type = values[14];
                        anim_range = values[15];
                        duration = values[16] as i64;
                    }
                    let mut g = SkinGauge::new(
                        gauge, values[10], values[9], parts, anim_type, anim_range, duration,
                    );
                    g.set_starttime(values[17]);
                    g.set_endtime(values[18]);
                    self.gauger = Some(g);
                }
            }
            "DST_GROOVEGAUGE" => {
                if let Some(ref mut gauger) = self.gauger {
                    let values = Self::parse_int(str_parts);
                    let dstw = self.dst.width / self.src.width;
                    let dsth = self.dst.height / self.src.height;
                    // Java: groovex/groovey control gauge tile spacing
                    let width = if self.groovex.abs() >= 1 {
                        self.groovex as f32 * 50.0 * dstw
                    } else {
                        values[5] as f32 * dstw
                    };
                    let height = if self.groovey.abs() >= 1 {
                        self.groovey as f32 * 50.0 * dsth
                    } else {
                        values[6] as f32 * dsth
                    };
                    let x = values[3] as f32 * dstw
                        - if self.groovex < 0 {
                            self.groovex as f32 * dstw
                        } else {
                            0.0
                        };
                    let y = self.dst.height - values[4] as f32 * dsth - height;
                    let offsets = Self::read_offset(str_parts, 21);
                    gauger.data.set_destination_with_int_timer_ops(
                        values[2] as i64,
                        x,
                        y,
                        width,
                        height,
                        values[7],
                        values[8],
                        values[9],
                        values[10],
                        values[11],
                        values[12],
                        values[13],
                        values[14],
                        values[15],
                        values[16],
                        values[17],
                        &offsets,
                    );
                }
            }
            _ => {
                // Other commands handled by subclass
            }
        }
    }

    /// Build the gauge image array for SRC_GROOVEGAUGE / SRC_GROOVEGAUGE_EX.
    ///
    /// Returns Vec<Vec<Option<TextureRegion>>> with 36 slots per state.
    /// The slot layout encodes 6 gauge types x 6 visual states per type:
    ///   [lit above-border, lit below-border, unlit above, unlit below, tip above, tip below]
    fn build_gauge_image_array(
        &self,
        values: &[i32; 22],
        divx: i32,
        divy: i32,
        total: usize,
        is_ex: bool,
    ) -> Vec<Vec<Option<TextureRegion>>> {
        let gr = values[2] as usize;
        let tex = match &self.imagelist[gr] {
            ImageListEntry::TextureEntry(t) => t.clone(),
            _ => return Vec::new(),
        };
        let w = values[5];
        let h = values[6];
        let anim_type = values[14];

        let make_tr = |x_idx: i32, y_idx: i32| -> TextureRegion {
            TextureRegion::from_texture_region(
                tex.clone(),
                values[3] + w * x_idx / divx,
                values[4] + h * y_idx / divy,
                w / divx,
                h / divy,
            )
        };

        if is_ex {
            if anim_type == 3 && total.is_multiple_of(12) {
                // PMS EX: 12 images per state
                let states = total / 12;
                let mut gauge: Vec<Vec<Option<TextureRegion>>> = vec![vec![None; 36]; states];
                for x in 0..divx {
                    for y in 0..divy {
                        let idx = (y * divx + x) as usize;
                        let dx = idx / 12;
                        let dy = idx % 12;
                        if dx < states {
                            let tr = make_tr(x, y);
                            if dy < 4 {
                                for &slot in &[dy, dy + 6, dy + 12, dy + 18] {
                                    gauge[dx][slot] = Some(tr.clone());
                                }
                            } else if dy < 8 {
                                for &slot in &[dy + 20, dy + 26] {
                                    gauge[dx][slot] = Some(tr.clone());
                                }
                            } else if dy == 8 || dy == 9 {
                                for &slot in &[dy - 4, dy + 2, dy + 8, dy + 14] {
                                    gauge[dx][slot] = Some(tr.clone());
                                }
                            } else {
                                // dy == 10 || dy == 11
                                for &slot in &[dy + 18, dy + 24] {
                                    gauge[dx][slot] = Some(tr.clone());
                                }
                            }
                        }
                    }
                }
                gauge
            } else {
                // Standard EX: 8 images per state
                let states = if total >= 8 { total / 8 } else { 0 };
                let mut gauge: Vec<Vec<Option<TextureRegion>>> = vec![vec![None; 36]; states];
                for x in 0..divx {
                    for y in 0..divy {
                        let idx = (y * divx + x) as usize;
                        let dx = idx / 8;
                        let dy = idx % 8;
                        if dx < states {
                            let tr = make_tr(x, y);
                            if dy < 4 {
                                for &slot in &[dy, dy + 6, dy + 12, dy + 18] {
                                    gauge[dx][slot] = Some(tr.clone());
                                }
                                if dy < 2 {
                                    for &slot in &[dy + 4, dy + 10, dy + 16, dy + 22] {
                                        gauge[dx][slot] = Some(tr.clone());
                                    }
                                }
                            } else {
                                for &slot in &[dy + 20, dy + 26] {
                                    gauge[dx][slot] = Some(tr.clone());
                                }
                                if dy < 6 {
                                    for &slot in &[dy + 24, dy + 30] {
                                        gauge[dx][slot] = Some(tr.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                gauge
            }
        } else if anim_type == 3 && total.is_multiple_of(6) {
            // PMS: 6 images per state
            let states = total / 6;
            let mut gauge: Vec<Vec<Option<TextureRegion>>> = vec![vec![None; 36]; states];
            for x in 0..divx {
                for y in 0..divy {
                    let idx = (y * divx + x) as usize;
                    let dx = idx / 6;
                    let dy = idx % 6;
                    if dx < states {
                        let tr = make_tr(x, y);
                        for &slot in &[dy, dy + 6, dy + 12, dy + 18, dy + 24, dy + 30] {
                            gauge[dx][slot] = Some(tr.clone());
                        }
                    }
                }
            }
            gauge
        } else {
            // Standard: 4 images per state
            let states = if total >= 4 { total / 4 } else { 0 };
            let mut gauge: Vec<Vec<Option<TextureRegion>>> = vec![vec![None; 36]; states];
            for x in 0..divx {
                for y in 0..divy {
                    let idx = (y * divx + x) as usize;
                    let dx = idx / 4;
                    let dy = idx % 4;
                    if dx < states {
                        let tr = make_tr(x, y);
                        for &slot in &[dy, dy + 6, dy + 12, dy + 18, dy + 24, dy + 30] {
                            gauge[dx][slot] = Some(tr.clone());
                        }
                        if dy < 2 {
                            for &slot in &[dy + 4, dy + 10, dy + 16, dy + 22, dy + 28, dy + 34] {
                                gauge[dx][slot] = Some(tr.clone());
                            }
                        }
                    }
                }
            }
            gauge
        }
    }

    /// Finalize any active skin objects (button, onmouse, gauger) into collected_objects.
    /// Call this after CSV parsing completes.
    pub fn finalize_active_objects(&mut self) {
        if let Some(btn) = self.button.take() {
            self.collected_objects.push(SkinObject::Image(btn));
        }
        if let Some(om) = self.onmouse.take() {
            self.collected_objects.push(SkinObject::Image(om));
        }
        if let Some(g) = self.gauger.take() {
            self.collected_objects.push(SkinObject::Gauge(g));
        }
    }

    /// Apply accumulated skin properties to the given Skin.
    /// Call this after load_skin0 or process_csv_command to transfer
    /// STARTINPUT, SCENETIME, FADEOUT values to the Skin object.
    pub fn apply_to_skin(&self, skin: &mut crate::skin::Skin) {
        if let Some(input) = self.skin_input {
            skin.set_input(input);
        }
        if let Some(scene) = self.skin_scene {
            skin.set_scene(scene);
        }
        if let Some(fadeout) = self.skin_fadeout {
            skin.set_fadeout(fadeout);
        }
    }

    /// Load skin from file (corresponds to loadSkin0)
    pub fn load_skin0(
        &mut self,
        path: &Path,
        _state: Option<&dyn MainState>,
    ) -> anyhow::Result<()> {
        let raw_bytes = std::fs::read(path)?;
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_bytes);
        let content = decoded.into_owned();

        for line in content.lines() {
            self.line = Some(line.to_string());
            if let Some((cmd, str_parts)) = self.base.process_line_directives(line, _state) {
                self.process_csv_command(&cmd, &str_parts);
            }
        }

        // Flush any remaining active objects
        self.finalize_active_objects();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> LR2SkinCSVLoaderState {
        LR2SkinCSVLoaderState::new(
            Resolution {
                width: 640.0,
                height: 480.0,
            },
            Resolution {
                width: 1920.0,
                height: 1080.0,
            },
            false,
            "/tmp/test_skin".to_string(),
        )
    }

    fn str_vec(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_startinput_parses_value() {
        let mut state = make_state();
        state.process_csv_command("STARTINPUT", &str_vec(&["STARTINPUT", "1000"]));
        assert_eq!(state.skin_input, Some(1000));
    }

    #[test]
    fn test_startinput_empty_parts_no_panic() {
        let mut state = make_state();
        state.process_csv_command("STARTINPUT", &str_vec(&["STARTINPUT"]));
        assert_eq!(state.skin_input, None);
    }

    #[test]
    fn test_scenetime_parses_value() {
        let mut state = make_state();
        state.process_csv_command("SCENETIME", &str_vec(&["SCENETIME", "5000"]));
        assert_eq!(state.skin_scene, Some(5000));
    }

    #[test]
    fn test_fadeout_parses_value() {
        let mut state = make_state();
        state.process_csv_command("FADEOUT", &str_vec(&["FADEOUT", "300"]));
        assert_eq!(state.skin_fadeout, Some(300));
    }

    #[test]
    fn test_fadeout_invalid_value_returns_none() {
        let mut state = make_state();
        state.process_csv_command("FADEOUT", &str_vec(&["FADEOUT", "abc"]));
        assert_eq!(state.skin_fadeout, None);
    }

    #[test]
    fn test_stretch_parses_value() {
        let mut state = make_state();
        assert_eq!(state.stretch, -1);
        state.process_csv_command("STRETCH", &str_vec(&["STRETCH", "2"]));
        assert_eq!(state.stretch, 2);
    }

    #[test]
    fn test_apply_to_skin_transfers_values() {
        let mut state = make_state();
        state.skin_input = Some(500);
        state.skin_scene = Some(60000);
        state.skin_fadeout = Some(200);

        let mut skin = crate::skin::Skin::new(crate::skin_header::SkinHeader::new());
        state.apply_to_skin(&mut skin);
        assert_eq!(skin.get_input(), 500);
        assert_eq!(skin.get_scene(), 60000);
        assert_eq!(skin.get_fadeout(), 200);
    }

    #[test]
    fn test_apply_to_skin_none_values_not_overwritten() {
        let state = make_state();
        let mut skin = crate::skin::Skin::new(crate::skin_header::SkinHeader::new());
        skin.set_input(42);
        skin.set_scene(99);
        skin.set_fadeout(77);

        state.apply_to_skin(&mut skin);
        // None values should not overwrite existing values
        assert_eq!(skin.get_input(), 42);
        assert_eq!(skin.get_scene(), 99);
        assert_eq!(skin.get_fadeout(), 77);
    }

    #[test]
    fn test_unknown_command_no_panic() {
        let mut state = make_state();
        state.process_csv_command("NONEXISTENT", &str_vec(&["NONEXISTENT", "1"]));
        // Should not panic, no state changed
        assert_eq!(state.skin_input, None);
    }

    // --- load_skin0 file-based tests ---

    /// Helper: write content to a temp file and return the path.
    fn write_temp_csv(name: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_load_skin0_parses_directives_from_file() {
        let csv = "\
#STARTINPUT,750\n\
#SCENETIME,4000\n\
#FADEOUT,200\n\
#STRETCH,1\n";
        let path = write_temp_csv("directives.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();

        assert_eq!(state.skin_input, Some(750));
        assert_eq!(state.skin_scene, Some(4000));
        assert_eq!(state.skin_fadeout, Some(200));
        assert_eq!(state.stretch, 1);
    }

    #[test]
    fn test_load_skin0_empty_file() {
        let path = write_temp_csv("empty.lr2skin", "");
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();

        // Nothing should be set
        assert_eq!(state.skin_input, None);
        assert_eq!(state.skin_scene, None);
        assert_eq!(state.skin_fadeout, None);
        assert_eq!(state.stretch, -1);
        assert!(state.imagelist.is_empty());
    }

    #[test]
    fn test_load_skin0_nonexistent_file_returns_error() {
        let path = std::path::PathBuf::from("/nonexistent/path/skin.lr2skin");
        let mut state = make_state();
        assert!(state.load_skin0(&path, None).is_err());
    }

    #[test]
    fn test_load_skin0_lines_without_hash_are_skipped() {
        // Lines not starting with '#' are ignored by process_line_directives
        let csv = "\
This is a comment line\n\
SCENETIME,9999\n\
   indented line\n\
\n\
#SCENETIME,1234\n";
        let path = write_temp_csv("comments.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();

        // Only the #SCENETIME line should be processed
        assert_eq!(state.skin_scene, Some(1234));
        assert_eq!(state.skin_input, None);
    }

    #[test]
    fn test_load_skin0_blank_lines_are_harmless() {
        let csv = "\n\n\n#FADEOUT,100\n\n\n";
        let path = write_temp_csv("blanks.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();
        assert_eq!(state.skin_fadeout, Some(100));
    }

    // --- #IF / #ELSE / #ENDIF conditional processing ---

    #[test]
    fn test_load_skin0_if_true_branch() {
        let csv = "\
#SETOPTION,42,1\n\
#IF,42\n\
#SCENETIME,1111\n\
#ELSE\n\
#SCENETIME,2222\n\
#ENDIF\n";
        let path = write_temp_csv("if_true.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();
        assert_eq!(state.skin_scene, Some(1111));
    }

    #[test]
    fn test_load_skin0_if_false_branch() {
        let csv = "\
#SETOPTION,42,0\n\
#IF,42\n\
#SCENETIME,1111\n\
#ELSE\n\
#SCENETIME,2222\n\
#ENDIF\n";
        let path = write_temp_csv("if_false.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();
        assert_eq!(state.skin_scene, Some(2222));
    }

    #[test]
    fn test_load_skin0_if_unset_option_skips_true_branch() {
        // When the option is not set at all, #IF evaluates to false
        let csv = "\
#IF,99\n\
#SCENETIME,1111\n\
#ELSE\n\
#SCENETIME,2222\n\
#ENDIF\n";
        let path = write_temp_csv("if_unset.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();
        assert_eq!(state.skin_scene, Some(2222));
    }

    // --- IMAGE command tests ---

    #[test]
    fn test_image_command_nonexistent_file_pushes_null() {
        let mut state = make_state();
        state.process_csv_command("IMAGE", &str_vec(&["#IMAGE", "/nonexistent/image.png"]));
        assert_eq!(state.imagelist.len(), 1);
        assert!(matches!(state.imagelist[0], ImageListEntry::Null));
    }

    #[test]
    fn test_image_command_movie_extension_detection() {
        // Create a temp file with a movie extension to test classification
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let movie_path = dir.join("test.mp4");
        std::fs::write(&movie_path, b"fake movie data").unwrap();

        let mut state = make_state();
        state.process_csv_command("IMAGE", &str_vec(&["#IMAGE", movie_path.to_str().unwrap()]));
        assert_eq!(state.imagelist.len(), 1);
        assert!(matches!(state.imagelist[0], ImageListEntry::Movie(_)));
    }

    #[test]
    fn test_image_command_real_png_loads_as_texture() {
        // Create a minimal 1x1 PNG
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let png_path = dir.join("test_1x1.png");
        // Minimal valid 1x1 white PNG
        let png_data: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, // 8-bit RGB
            0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
            0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, // compressed data
            0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, // ...
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
            0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&png_path, png_data).unwrap();

        let mut state = make_state();
        state.process_csv_command("IMAGE", &str_vec(&["#IMAGE", png_path.to_str().unwrap()]));
        assert_eq!(state.imagelist.len(), 1);
        assert!(matches!(
            state.imagelist[0],
            ImageListEntry::TextureEntry(_)
        ));
    }

    #[test]
    fn test_multiple_images_grow_imagelist() {
        let mut state = make_state();
        // All nonexistent, but imagelist should still grow
        for i in 0..5 {
            state.process_csv_command(
                "IMAGE",
                &str_vec(&["#IMAGE", &format!("/nonexistent/img{}.png", i)]),
            );
        }
        assert_eq!(state.imagelist.len(), 5);
        assert!(
            state
                .imagelist
                .iter()
                .all(|e| matches!(e, ImageListEntry::Null))
        );
    }

    // --- LR2FONT command tests ---

    #[test]
    fn test_lr2font_nonexistent_file_pushes_none() {
        let mut state = make_state();
        state.process_csv_command(
            "LR2FONT",
            &str_vec(&["#LR2FONT", "/nonexistent/font.lr2font"]),
        );
        assert_eq!(state.fontlist.len(), 1);
        assert!(state.fontlist[0].is_none());
    }

    #[test]
    fn test_lr2font_existing_file_pushes_some() {
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let font_path = dir.join("test.lr2font");
        std::fs::write(&font_path, b"fake font data").unwrap();

        let mut state = make_state();
        state.process_csv_command(
            "LR2FONT",
            &str_vec(&["#LR2FONT", font_path.to_str().unwrap()]),
        );
        assert_eq!(state.fontlist.len(), 1);
        assert!(state.fontlist[0].is_some());
    }

    // --- parse_int tests ---

    #[test]
    fn test_parse_int_basic() {
        let parts = str_vec(&["#CMD", "10", "20", "30"]);
        let result = LR2SkinCSVLoaderState::parse_int(&parts);
        assert_eq!(result[1], 10);
        assert_eq!(result[2], 20);
        assert_eq!(result[3], 30);
        // Rest should be 0
        assert_eq!(result[4], 0);
    }

    #[test]
    fn test_parse_int_empty_parts() {
        let parts = str_vec(&["#CMD"]);
        let result = LR2SkinCSVLoaderState::parse_int(&parts);
        // All zeros
        assert!(result.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_parse_int_bang_as_negative() {
        // '!' is replaced with '-' in Java, so !5 becomes -5
        let parts = str_vec(&["#CMD", "!5"]);
        let result = LR2SkinCSVLoaderState::parse_int(&parts);
        assert_eq!(result[1], -5);
    }

    #[test]
    fn test_parse_int_non_numeric_becomes_zero() {
        let parts = str_vec(&["#CMD", "abc", "42"]);
        let result = LR2SkinCSVLoaderState::parse_int(&parts);
        assert_eq!(result[1], 0); // "abc" -> parse fails -> 0
        assert_eq!(result[2], 42);
    }

    #[test]
    fn test_parse_int_more_than_22_parts_truncated() {
        // parse_int only reads up to index 21
        let mut parts: Vec<&str> = vec!["#CMD"];
        for _ in 0..25 {
            parts.push("7");
        }
        let result = LR2SkinCSVLoaderState::parse_int(&str_vec(&parts));
        assert_eq!(result[1], 7);
        assert_eq!(result[21], 7);
        // Index 0 is always 0 (skipped)
        assert_eq!(result[0], 0);
    }

    // --- read_offset tests ---

    #[test]
    fn test_read_offset_basic() {
        let parts = str_vec(&[
            "#DST", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0",
            "0", "0", "0", "0", "0", "100", "200",
        ]);
        let offsets = LR2SkinCSVLoaderState::read_offset(&parts, 21);
        // Index 21 is "0", index 22 is "100", index 23 is "200"
        assert_eq!(offsets, vec![0, 100, 200]);
    }

    #[test]
    fn test_read_offset_no_extra_parts() {
        let parts = str_vec(&["#DST", "0"]);
        let offsets = LR2SkinCSVLoaderState::read_offset(&parts, 21);
        assert!(offsets.is_empty());
    }

    // --- get_source_image_from_texture tests ---

    #[test]
    fn test_get_source_image_from_texture_basic_grid() {
        let tex = Texture {
            width: 100,
            height: 100,
            ..Default::default()
        };
        let images =
            LR2SkinCSVLoaderState::get_source_image_from_texture(&tex, 0, 0, 100, 100, 2, 2);
        // 2x2 grid = 4 images
        assert_eq!(images.len(), 4);
        // First cell: (0,0) 50x50
        assert_eq!(images[0].region_x, 0);
        assert_eq!(images[0].region_y, 0);
        assert_eq!(images[0].region_width, 50);
        assert_eq!(images[0].region_height, 50);
        // Second cell: (50,0)
        assert_eq!(images[1].region_x, 50);
        assert_eq!(images[1].region_y, 0);
    }

    #[test]
    fn test_get_source_image_from_texture_w_h_minus_one_uses_full_texture() {
        let tex = Texture {
            width: 200,
            height: 150,
            ..Default::default()
        };
        let images = LR2SkinCSVLoaderState::get_source_image_from_texture(&tex, 0, 0, -1, -1, 1, 1);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].region_width, 200);
        assert_eq!(images[0].region_height, 150);
    }

    #[test]
    fn test_get_source_image_from_texture_zero_div_treated_as_one() {
        let tex = Texture {
            width: 64,
            height: 64,
            ..Default::default()
        };
        // divx=0, divy=0 should be treated as 1
        let images = LR2SkinCSVLoaderState::get_source_image_from_texture(&tex, 0, 0, 64, 64, 0, 0);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].region_width, 64);
        assert_eq!(images[0].region_height, 64);
    }

    #[test]
    fn test_get_source_image_from_texture_negative_div_treated_as_one() {
        let tex = Texture {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let images =
            LR2SkinCSVLoaderState::get_source_image_from_texture(&tex, 0, 0, 64, 64, -3, -2);
        assert_eq!(images.len(), 1);
    }

    // --- get_source_image tests ---

    #[test]
    fn test_get_source_image_out_of_bounds_index_returns_none() {
        let state = make_state();
        // imagelist is empty, gr=0 is out of bounds
        let values = [0i32; 22];
        assert!(state.get_source_image(&values).is_none());
    }

    #[test]
    fn test_get_source_image_null_entry_returns_none() {
        let mut state = make_state();
        state.imagelist.push(ImageListEntry::Null);
        let mut values = [0i32; 22];
        values[2] = 0; // gr index
        assert!(state.get_source_image(&values).is_none());
    }

    #[test]
    fn test_get_source_image_movie_entry_returns_none() {
        let mut state = make_state();
        state
            .imagelist
            .push(ImageListEntry::Movie("test.mp4".to_string()));
        let mut values = [0i32; 22];
        values[2] = 0; // gr index
        assert!(state.get_source_image(&values).is_none());
    }

    #[test]
    fn test_get_source_image_valid_texture_returns_regions() {
        let mut state = make_state();
        let tex = Texture {
            width: 128,
            height: 64,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0; // gr index
        values[3] = 0; // x
        values[4] = 0; // y
        values[5] = 128; // w
        values[6] = 64; // h
        values[7] = 2; // divx
        values[8] = 2; // divy
        let result = state.get_source_image(&values);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 4); // 2x2 grid
    }

    // --- finalize_active_objects tests ---

    #[test]
    fn test_finalize_active_objects_empty_state() {
        let mut state = make_state();
        state.finalize_active_objects();
        assert!(state.collected_objects.is_empty());
    }

    // --- STRETCH edge cases ---

    #[test]
    fn test_stretch_invalid_value_defaults_to_minus_one() {
        let mut state = make_state();
        state.process_csv_command("STRETCH", &str_vec(&["STRETCH", "abc"]));
        assert_eq!(state.stretch, -1);
    }

    #[test]
    fn test_stretch_empty_parts_unchanged() {
        let mut state = make_state();
        state.process_csv_command("STRETCH", &str_vec(&["STRETCH"]));
        assert_eq!(state.stretch, -1);
    }

    // --- Directive value edge cases ---

    #[test]
    fn test_startinput_negative_value() {
        let mut state = make_state();
        state.process_csv_command("STARTINPUT", &str_vec(&["STARTINPUT", "-100"]));
        assert_eq!(state.skin_input, Some(-100));
    }

    #[test]
    fn test_scenetime_zero_value() {
        let mut state = make_state();
        state.process_csv_command("SCENETIME", &str_vec(&["SCENETIME", "0"]));
        assert_eq!(state.skin_scene, Some(0));
    }

    #[test]
    fn test_fadeout_whitespace_trimmed() {
        let mut state = make_state();
        state.process_csv_command("FADEOUT", &str_vec(&["FADEOUT", "  500  "]));
        assert_eq!(state.skin_fadeout, Some(500));
    }

    #[test]
    fn test_multiple_commands_last_value_wins() {
        let mut state = make_state();
        state.process_csv_command("SCENETIME", &str_vec(&["SCENETIME", "1000"]));
        state.process_csv_command("SCENETIME", &str_vec(&["SCENETIME", "2000"]));
        assert_eq!(state.skin_scene, Some(2000));
    }

    // --- new() constructor tests ---

    #[test]
    fn test_new_initializes_defaults() {
        let state = make_state();
        assert_eq!(state.stretch, -1);
        assert_eq!(state.skin_input, None);
        assert_eq!(state.skin_scene, None);
        assert_eq!(state.skin_fadeout, None);
        assert_eq!(state.groovex, 0);
        assert_eq!(state.groovey, 0);
        assert!(state.imagelist.is_empty());
        assert!(state.fontlist.is_empty());
        assert!(state.filemap.is_empty());
        assert!(state.collected_objects.is_empty());
        assert!(state.button.is_none());
        assert!(state.onmouse.is_none());
        assert!(state.gauger.is_none());
        assert!(state.line.is_none());
        assert!(state.imagesetarray.is_empty());
    }

    #[test]
    fn test_new_registers_csv_command_names() {
        let state = make_state();
        // Verify key command names are registered by checking the base state
        // accepts them via process_line_directives
        let expected_commands = [
            "STARTINPUT",
            "SCENETIME",
            "FADEOUT",
            "STRETCH",
            "INCLUDE",
            "IMAGE",
            "LR2FONT",
            "SRC_IMAGE",
            "DST_IMAGE",
            "SRC_NUMBER",
            "DST_NUMBER",
            "SRC_BUTTON",
            "DST_BUTTON",
            "SRC_GROOVEGAUGE",
        ];
        // All these commands should be recognized (they won't return None for
        // skip-related reasons since skip is false initially)
        for cmd in &expected_commands {
            let mut test_state = make_state();
            let line = format!("#{},0", cmd);
            let result = test_state.base.process_line_directives(&line, None);
            assert!(result.is_some(), "Command {} should be recognized", cmd);
        }
    }

    // --- INCLUDE command tests ---

    #[test]
    fn test_include_nonexistent_file_no_panic() {
        let mut state = make_state();
        state.process_csv_command(
            "INCLUDE",
            &str_vec(&["#INCLUDE", "/nonexistent/include.lr2skin"]),
        );
        // Should silently skip
        assert_eq!(state.skin_scene, None);
    }

    #[test]
    fn test_include_processes_included_file_commands() {
        // Create an included file with directives
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let inc_path = dir.join("included.lr2skin");
        std::fs::write(&inc_path, "#SCENETIME,7777\n#FADEOUT,333\n").unwrap();

        let mut state = make_state();
        state.skinpath = dir.to_str().unwrap().to_string();
        state.process_csv_command(
            "INCLUDE",
            &str_vec(&["#INCLUDE", inc_path.to_str().unwrap()]),
        );
        assert_eq!(state.skin_scene, Some(7777));
        assert_eq!(state.skin_fadeout, Some(333));
    }

    // --- load_skin0 integration: SRC/DST pairs through full pipeline ---

    #[test]
    fn test_load_skin0_combined_directives_and_conditionals() {
        let csv = "\
#STARTINPUT,200\n\
#SETOPTION,10,1\n\
#IF,10\n\
#SCENETIME,5555\n\
#ENDIF\n\
#FADEOUT,400\n\
#STRETCH,3\n";
        let path = write_temp_csv("combined.lr2skin", csv);
        let mut state = make_state();
        state.skinpath = path.parent().unwrap().to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();

        assert_eq!(state.skin_input, Some(200));
        assert_eq!(state.skin_scene, Some(5555));
        assert_eq!(state.skin_fadeout, Some(400));
        assert_eq!(state.stretch, 3);
    }

    #[test]
    fn test_load_skin0_shift_jis_encoding() {
        // load_skin0 decodes Shift-JIS. Verify ASCII content works fine.
        let csv = b"#SCENETIME,9999\n";
        let dir = std::env::temp_dir().join("lr2_skin_csv_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sjis_ascii.lr2skin");
        std::fs::write(&path, csv).unwrap();

        let mut state = make_state();
        state.skinpath = dir.to_str().unwrap().to_string();

        state.load_skin0(&path, None).unwrap();
        assert_eq!(state.skin_scene, Some(9999));
    }

    // --- build_gauge_image_array tests ---

    #[test]
    fn test_build_gauge_standard_4_images_per_state() {
        let mut state = make_state();
        let tex = Texture {
            width: 80,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        // 4 divx, 1 divy -> total=4, standard mode: 4 per state = 1 state
        let mut values = [0i32; 22];
        values[2] = 0; // gr
        values[3] = 0; // x
        values[4] = 0; // y
        values[5] = 80; // w
        values[6] = 20; // h
        values[14] = 0; // anim_type != 3 -> standard

        let gauge = state.build_gauge_image_array(&values, 4, 1, 4, false);
        assert_eq!(gauge.len(), 1); // 1 state
        assert_eq!(gauge[0].len(), 36); // 36 slots per state
        // Slots 0-3 should be populated
        assert!(gauge[0][0].is_some());
        assert!(gauge[0][1].is_some());
        assert!(gauge[0][2].is_some());
        assert!(gauge[0][3].is_some());
    }

    #[test]
    fn test_build_gauge_too_few_images_returns_empty() {
        let mut state = make_state();
        let tex = Texture {
            width: 20,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0;
        values[5] = 20;
        values[6] = 20;

        // total=1, but standard needs at least 4 -> states=0
        let gauge = state.build_gauge_image_array(&values, 1, 1, 1, false);
        assert!(gauge.is_empty());
    }

    #[test]
    fn test_build_gauge_null_image_returns_empty() {
        let mut state = make_state();
        state.imagelist.push(ImageListEntry::Null);
        let mut values = [0i32; 22];
        values[2] = 0;
        let gauge = state.build_gauge_image_array(&values, 1, 1, 1, false);
        assert!(gauge.is_empty());
    }

    #[test]
    fn test_build_gauge_pms_mode_6_images_per_state() {
        let mut state = make_state();
        let tex = Texture {
            width: 120,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0;
        values[3] = 0;
        values[4] = 0;
        values[5] = 120;
        values[6] = 20;
        values[14] = 3; // anim_type=3 -> PMS mode

        // 6 divx, 1 divy -> total=6, PMS mode: 6 per state = 1 state
        let gauge = state.build_gauge_image_array(&values, 6, 1, 6, false);
        assert_eq!(gauge.len(), 1);
        assert_eq!(gauge[0].len(), 36);
    }

    #[test]
    fn test_build_gauge_ex_standard_8_images_per_state() {
        let mut state = make_state();
        let tex = Texture {
            width: 160,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0;
        values[3] = 0;
        values[4] = 0;
        values[5] = 160;
        values[6] = 20;
        values[14] = 0; // not PMS

        // 8 divx, 1 divy -> total=8, EX mode: 8 per state = 1 state
        let gauge = state.build_gauge_image_array(&values, 8, 1, 8, true);
        assert_eq!(gauge.len(), 1);
        assert_eq!(gauge[0].len(), 36);
    }

    #[test]
    fn test_build_gauge_ex_pms_12_images_per_state() {
        let mut state = make_state();
        let tex = Texture {
            width: 240,
            height: 20,
            ..Default::default()
        };
        state.imagelist.push(ImageListEntry::TextureEntry(tex));
        let mut values = [0i32; 22];
        values[2] = 0;
        values[3] = 0;
        values[4] = 0;
        values[5] = 240;
        values[6] = 20;
        values[14] = 3; // PMS

        // 12 divx, 1 divy -> total=12, EX+PMS: 12 per state = 1 state
        let gauge = state.build_gauge_image_array(&values, 12, 1, 12, true);
        assert_eq!(gauge.len(), 1);
        assert_eq!(gauge[0].len(), 36);
    }
}

/// Trait for LR2 skin loaders — provides access to the base CSV loader state.
pub trait LR2SkinLoaderAccess {
    /// Get mutable reference to the base CSV loader state.
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState;

    /// Assemble accumulated loader state into SkinObjects and add them to the Skin.
    /// Called after CSV parsing completes to convert parsed source data into drawable objects.
    fn assemble_objects(&mut self, skin: &mut crate::skin::Skin);
}

/// Create the appropriate LR2 skin loader for the given SkinType.
fn create_lr2_loader(
    skin_type: &crate::skin_type::SkinType,
    src: Resolution,
    dst: Resolution,
    usecim: bool,
    skinpath: String,
) -> Option<Box<dyn LR2SkinLoaderAccess>> {
    use crate::skin_type::SkinType;
    match skin_type {
        SkinType::MusicSelect => Some(Box::new(
            super::lr2_select_skin_loader::LR2SelectSkinLoaderState::new(
                src, dst, usecim, skinpath,
            ),
        )),
        SkinType::Decide => Some(Box::new(
            super::lr2_decide_skin_loader::LR2DecideSkinLoaderState::new(
                src, dst, usecim, skinpath,
            ),
        )),
        SkinType::Result => Some(Box::new(
            super::lr2_result_skin_loader::LR2ResultSkinLoaderState::new(
                src, dst, usecim, skinpath,
            ),
        )),
        SkinType::CourseResult => Some(Box::new(
            super::lr2_course_result_skin_loader::LR2CourseResultSkinLoaderState::new(
                src, dst, usecim, skinpath,
            ),
        )),
        SkinType::SkinSelect => Some(Box::new(
            super::lr2_skin_select_skin_loader::LR2SkinSelectSkinLoaderState::new(
                src, dst, usecim, skinpath,
            ),
        )),
        st if st.is_play() => Some(Box::new(
            super::lr2_play_skin_loader::LR2PlaySkinLoaderState::new(
                *st, src, dst, usecim, skinpath,
            ),
        )),
        _ => None,
    }
}

/// Load an LR2 skin from a .lr2skin file path.
///
/// Pipeline: header load → loader create → CSV parse → apply properties → assemble objects → return Skin.
pub fn load_lr2_skin(
    path: &std::path::Path,
    skin_type: &crate::skin_type::SkinType,
) -> Option<crate::skin::Skin> {
    use crate::skin_header::{self, SkinHeader};

    let skinpath = path.parent()?.to_str()?.to_string();

    // 1. Load header
    let mut header_loader = super::lr2_skin_header_loader::LR2SkinHeaderLoader::new(&skinpath);
    let header_data = header_loader.load_skin(path, None).ok()?;

    // 2. Build SkinHeader from LR2SkinHeaderData
    let mut skin_header = SkinHeader::new();
    skin_header.set_type(skin_header::TYPE_LR2SKIN);
    if let Some(st) = header_data.skin_type {
        skin_header.set_skin_type(st);
    }
    skin_header.set_name(header_data.name.clone());
    skin_header.set_author(header_data.author.clone());
    skin_header.set_path(path.to_path_buf());
    if let Some(ref res) = header_data.resolution {
        skin_header.set_resolution(res.clone());
    }
    // Convert lr2_skin_header_loader custom types → skin_header custom types
    let options: Vec<skin_header::CustomOption> = header_data
        .custom_options
        .iter()
        .map(|o| {
            skin_header::CustomOption::new(o.name.clone(), o.option.clone(), o.contents.clone())
        })
        .collect();
    skin_header.set_custom_options(options);
    let files: Vec<skin_header::CustomFile> = header_data
        .custom_files
        .iter()
        .map(|f| skin_header::CustomFile::new(f.name.clone(), f.path.clone(), f.def.clone()))
        .collect();
    skin_header.set_custom_files(files);
    let offsets: Vec<skin_header::CustomOffset> = header_data
        .custom_offsets
        .iter()
        .map(|o| skin_header::CustomOffset::new(o.name.clone(), o.id, o.x, o.y, o.w, o.h, o.r, o.a))
        .collect();
    skin_header.set_custom_offsets(offsets);

    // 3. Create Skin
    let mut skin = crate::skin::Skin::new(skin_header);

    // 4. Create appropriate loader and parse CSV
    let src = header_data.resolution.unwrap_or(Resolution {
        width: 640.0,
        height: 480.0,
    });
    let dst = Resolution {
        width: 1920.0,
        height: 1080.0,
    };
    let mut loader = create_lr2_loader(skin_type, src, dst, false, skinpath)?;

    // Transfer header options to loader's op map
    for option in &header_data.custom_options {
        for i in 0..option.option.len() {
            let val = if option.get_selected_option() == option.option[i] {
                1
            } else {
                0
            };
            loader.csv_mut().base.op.insert(option.option[i], val);
        }
    }

    // Transfer custom file mappings to loader's filemap
    for file in &header_data.custom_files {
        if let Some(filename) = file.get_selected_filename() {
            loader
                .csv_mut()
                .filemap
                .insert(file.path.clone(), filename.to_string());
        }
    }

    // Parse the CSV file
    if let Err(e) = loader.csv_mut().load_skin0(path, None) {
        log::warn!("LR2 CSV skin load failed: {}: {}", path.display(), e);
        return None;
    }

    // 5. Apply accumulated properties to skin
    loader.csv_mut().apply_to_skin(&mut skin);

    // 6. Assemble parsed source data into SkinObjects
    loader.assemble_objects(&mut skin);

    Some(skin)
}
