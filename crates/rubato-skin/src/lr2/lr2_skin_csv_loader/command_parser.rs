use std::collections::HashMap;
use std::path::Path;

use log::warn;

use crate::lr2::lr2_font_loader::LR2FontLoader;
use crate::lr2::lr2_skin_loader::{self, LR2SkinLoaderState};
use crate::reexports::{MainState, Resolution, Texture, TextureRegion};
use crate::safe_div_f32;
use crate::skin::SkinObject;
use crate::skin_gauge::SkinGauge;
use crate::skin_image::SkinImage;
use crate::skin_object::DestinationParams;
use crate::skin_text_image::SkinTextImageSource;

use super::{ImageListEntry, LR2SkinCSVLoaderState};

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
    pub fn source_image(&self, values: &[i32; 22]) -> Option<Vec<TextureRegion>> {
        let gr = values[2] as usize;
        if gr < self.imagelist.len()
            && let ImageListEntry::TextureEntry(ref tex) = self.imagelist[gr]
        {
            return Some(Self::source_image_from_texture(
                tex, values[3], values[4], values[5], values[6], values[7], values[8],
            ));
        }
        warn!("IMAGE is not defined or failed to load: {:?}", self.line);
        None
    }

    /// Get source image regions from texture with coordinates
    pub fn source_image_from_texture(
        image: &Texture,
        x: i32,
        y: i32,
        mut w: i32,
        mut h: i32,
        mut divx: i32,
        mut divy: i32,
    ) -> Vec<TextureRegion> {
        if w == -1 {
            w = image.width;
        }
        if h == -1 {
            h = image.height;
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
    pub fn process_csv_command(
        &mut self,
        cmd: &str,
        str_parts: &[String],
        state: Option<&dyn MainState>,
    ) {
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
                if str_parts.len() <= 1 {
                    warn!("IMAGE command missing path argument");
                    self.imagelist.push(ImageListEntry::Null);
                    return;
                }
                let imagefile =
                    lr2_skin_loader::lr2_path(&self.skinpath, &str_parts[1], &self.filemap);
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
                if str_parts.len() <= 1 {
                    warn!("LR2FONT command missing path argument");
                    self.fontlist.push(None);
                    return;
                }
                let imagefile =
                    lr2_skin_loader::lr2_path(&self.skinpath, &str_parts[1], &self.filemap);
                let path = Path::new(&imagefile);
                if path.exists() {
                    let mut loader = LR2FontLoader::new(self.usecim);
                    match loader.load_font(path) {
                        Ok(data) => {
                            let mut source = SkinTextImageSource::new(data.usecim);
                            source.size = data.size;
                            source.margin = data.margin;
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
                if str_parts.len() <= 1 {
                    warn!("INCLUDE command missing path argument");
                    return;
                }
                let imagefile =
                    lr2_skin_loader::lr2_path(&self.skinpath, &str_parts[1], &self.filemap);
                let path = Path::new(&imagefile);
                if path.exists() {
                    match std::fs::read(path) {
                        Ok(raw_bytes) => {
                            let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_bytes);
                            let content = decoded.into_owned();
                            for line in content.lines() {
                                self.line = Some(line.to_string());
                                if let Some((cmd, parts)) =
                                    self.base.process_line_directives(line, state)
                                {
                                    self.process_csv_command(&cmd, &parts, state);
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
                        let src_images = self.source_image(&values);
                        match src_images {
                            Some(imgs) => {
                                // Each source image becomes its own frame (single-element vec)
                                imgs.into_iter().map(|img| vec![img]).collect()
                            }
                            None => Vec::new(),
                        }
                    } else {
                        // Split source images into `length` groups
                        match self.source_image(&values) {
                            Some(srcimg) => {
                                let len = length as usize;
                                let group_size = srcimg.len() / len;
                                if group_size == 0 {
                                    Vec::new()
                                } else {
                                    (0..len)
                                        .map(|i| {
                                            (0..group_size)
                                                .map(|j| srcimg[i * group_size + j].clone())
                                                .collect()
                                        })
                                        .collect()
                                }
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
                            btn.data.clickevent_type = click_type;
                        }
                        self.button = Some(btn);
                    }
                }
            }
            "DST_BUTTON" => {
                if let Some(ref mut button) = self.button {
                    let values = Self::parse_int(str_parts);
                    let dstw = safe_div_f32(self.dst.width, self.src.width);
                    let dsth = safe_div_f32(self.dst.height, self.src.height);
                    let offsets = Self::read_offset(str_parts, 21);
                    button.data.set_destination_with_int_timer_ops(
                        &DestinationParams {
                            time: values[2] as i64,
                            x: values[3] as f32 * dstw,
                            y: self.dst.height - (values[4] + values[6]) as f32 * dsth,
                            w: values[5] as f32 * dstw,
                            h: values[6] as f32 * dsth,
                            acc: values[7],
                            a: values[8],
                            r: values[9],
                            g: values[10],
                            b: values[11],
                            blend: values[12],
                            filter: values[13],
                            angle: values[14],
                            center: values[15],
                            loop_val: values[16],
                        },
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
                    if let Some(images) = self.source_image(&values) {
                        let mut om = SkinImage::new_with_int_timer(images, values[10], values[9]);
                        // Set mouse hitbox rectangle (Java: skin.setMouseRect applies dw/dh scaling)
                        let dstw = safe_div_f32(self.dst.width, self.src.width);
                        let dsth = safe_div_f32(self.dst.height, self.src.height);
                        let rect_x = values[12] as f32 * dstw;
                        let rect_y = (values[6] - values[13] - values[15]) as f32 * dsth;
                        let rect_w = values[14] as f32 * dstw;
                        let rect_h = values[15] as f32 * dsth;
                        om.data.set_mouse_rect(rect_x, rect_y, rect_w, rect_h);
                        self.onmouse = Some(om);
                    }
                }
            }
            "DST_ONMOUSE" => {
                if let Some(ref mut onmouse) = self.onmouse {
                    let values = Self::parse_int(str_parts);
                    let dstw = safe_div_f32(self.dst.width, self.src.width);
                    let dsth = safe_div_f32(self.dst.height, self.src.height);
                    let offsets = Self::read_offset(str_parts, 21);
                    onmouse.data.set_destination_with_int_timer_ops(
                        &DestinationParams {
                            time: values[2] as i64,
                            x: values[3] as f32 * dstw,
                            y: self.dst.height - (values[4] + values[6]) as f32 * dsth,
                            w: values[5] as f32 * dstw,
                            h: values[6] as f32 * dsth,
                            acc: values[7],
                            a: values[8],
                            r: values[9],
                            g: values[10],
                            b: values[11],
                            blend: values[12],
                            filter: values[13],
                            angle: values[14],
                            center: values[15],
                            loop_val: values[16],
                        },
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
                    g.starttime = values[17];
                    g.endtime = values[18];
                    self.gauger = Some(g);
                }
            }
            "DST_GROOVEGAUGE" => {
                if let Some(ref mut gauger) = self.gauger {
                    let values = Self::parse_int(str_parts);
                    let dstw = safe_div_f32(self.dst.width, self.src.width);
                    let dsth = safe_div_f32(self.dst.height, self.src.height);
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
                        &DestinationParams {
                            time: values[2] as i64,
                            x,
                            y,
                            w: width,
                            h: height,
                            acc: values[7],
                            a: values[8],
                            r: values[9],
                            g: values[10],
                            b: values[11],
                            blend: values[12],
                            filter: values[13],
                            angle: values[14],
                            center: values[15],
                            loop_val: values[16],
                        },
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
    pub(super) fn build_gauge_image_array(
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
            skin.input = input;
        }
        if let Some(scene) = self.skin_scene {
            skin.scene = scene;
        }
        if let Some(fadeout) = self.skin_fadeout {
            skin.fadeout = fadeout;
        }
    }

    /// Load skin from file (corresponds to loadSkin0)
    pub fn load_skin0(&mut self, path: &Path, state: Option<&dyn MainState>) -> anyhow::Result<()> {
        let raw_bytes = std::fs::read(path)?;
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_bytes);
        let content = decoded.into_owned();

        for line in content.lines() {
            self.line = Some(line.to_string());
            if let Some((cmd, str_parts)) = self.base.process_line_directives(line, state) {
                self.process_csv_command(&cmd, &str_parts, state);
            }
        }

        // Flush any remaining active objects
        self.finalize_active_objects();

        Ok(())
    }
}
