use std::collections::HashMap;
use std::path::Path;

use log::warn;

use crate::lr2::lr2_skin_loader::{self, LR2SkinLoaderState};
use crate::skin::SkinObject;
use crate::skin_gauge::SkinGauge;
use crate::skin_image::SkinImage;
use crate::stubs::{MainState, Rectangle, Resolution, Texture, TextureRegion};

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
    pub fontlist: Vec<Option<()>>, // SkinTextImageSource placeholder

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
                    // LR2FontLoader would load the font here
                    self.fontlist.push(Some(()));
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
                    let divx = if values[7] > 0 { values[7] } else { 1 };
                    let divy = if values[8] > 0 { values[8] } else { 1 };
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
