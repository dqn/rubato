use std::collections::HashMap;
use std::path::Path;

use log::warn;

use crate::lr2::lr2_skin_loader::{self, LR2SkinLoaderState};
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
            _ => {
                // Other commands handled by subclass
            }
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

/// Get SkinLoader for given SkinType
pub fn get_skin_loader(
    _skin_type: &crate::skin_type::SkinType,
    _src: Resolution,
    _config: &beatoraja_core::config::Config,
) -> Option<Box<dyn std::any::Any>> {
    // Concrete loaders exist (LR2SelectSkinLoader, LR2PlaySkinLoader, etc.)
    // but produce loader-specific state structs, not SkinData.
    // Requires per-SkinType dispatch + state→Skin converter.
    None
}
