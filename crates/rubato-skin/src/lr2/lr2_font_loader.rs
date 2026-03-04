use std::path::Path;

use crate::lr2::lr2_skin_loader::{self, LR2SkinLoaderState};

/// LR2 font loader
///
/// Translated from LR2FontLoader.java
/// Loads LR2 font definition files (.lr2font) which define
/// bitmap font mappings using S (size), M (margin), T (texture), R (reference) commands.
///
/// Font image source data
pub struct SkinTextImageSourceData {
    pub size: i32,
    pub margin: i32,
    pub paths: Vec<Option<String>>,
    pub images: Vec<FontImageEntry>,
    pub usecim: bool,
}

/// Single font image entry
pub struct FontImageEntry {
    pub code: i32,
    pub texture_index: i32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl SkinTextImageSourceData {
    pub fn new(usecim: bool) -> Self {
        Self {
            size: 0,
            margin: 0,
            paths: Vec::new(),
            images: Vec::new(),
            usecim,
        }
    }

    pub fn set_size(&mut self, size: i32) {
        self.size = size;
    }

    pub fn set_margin(&mut self, margin: i32) {
        self.margin = margin;
    }

    pub fn set_path(&mut self, index: i32, path: String) {
        let idx = index as usize;
        while self.paths.len() <= idx {
            self.paths.push(None);
        }
        self.paths[idx] = Some(path);
    }

    pub fn get_path(&self, index: i32) -> Option<&str> {
        self.paths.get(index as usize).and_then(|p| p.as_deref())
    }

    pub fn set_image(&mut self, code: i32, texture_index: i32, x: i32, y: i32, w: i32, h: i32) {
        self.images.push(FontImageEntry {
            code,
            texture_index,
            x,
            y,
            w,
            h,
        });
    }
}

/// LR2 font loader
pub struct LR2FontLoader {
    pub textimage: SkinTextImageSourceData,
    pub path: Option<std::path::PathBuf>,
    usecim: bool,
}

impl LR2FontLoader {
    pub fn new(usecim: bool) -> Self {
        Self {
            textimage: SkinTextImageSourceData::new(usecim),
            path: None,
            usecim,
        }
    }

    pub fn load_font(&mut self, p: &Path) -> anyhow::Result<&SkinTextImageSourceData> {
        self.textimage = SkinTextImageSourceData::new(self.usecim);
        self.path = Some(p.to_path_buf());

        let raw_bytes = std::fs::read(p)?;
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_bytes);
        let content = decoded.into_owned();

        let mut base = LR2SkinLoaderState::new();

        for line in content.lines() {
            if !line.starts_with('#') {
                continue;
            }
            let str_parts: Vec<String> = line.split(',').map(|s| s.to_string()).collect();
            if str_parts.is_empty() {
                continue;
            }

            // process_line_directives for #IF etc.
            if let Some((cmd, parts)) = base.process_line_directives(line, None) {
                self.process_font_command(&cmd, &parts);
            }
        }

        Ok(&self.textimage)
    }

    fn process_font_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            "S" => {
                // size
                if str_parts.len() > 1
                    && let Ok(v) = str_parts[1].trim().parse::<i32>()
                {
                    self.textimage.set_size(v);
                }
            }
            "M" => {
                // margin
                if str_parts.len() > 1
                    && let Ok(v) = str_parts[1].trim().parse::<i32>()
                {
                    self.textimage.set_margin(v);
                }
            }
            "T" => {
                // texture
                if str_parts.len() > 2
                    && let Some(ref p) = self.path
                    && let Some(parent) = p.parent()
                {
                    let imagefile = parent.join(&str_parts[2]);
                    if imagefile.exists()
                        && let Ok(index) = str_parts[1].trim().parse::<i32>()
                    {
                        self.textimage
                            .set_path(index, imagefile.to_string_lossy().into_owned());
                    }
                }
            }
            "R" => {
                // reference
                let values = Self::parse_int_font(str_parts);
                if self.textimage.get_path(values[2]).is_some() {
                    let codes = Self::map_code(values[1]);
                    for code in codes {
                        self.textimage
                            .set_image(code, values[2], values[3], values[4], values[5], values[6]);
                    }
                }
            }
            _ => {}
        }
    }

    fn parse_int_font(s: &[String]) -> [i32; 22] {
        lr2_skin_loader::parse_int(s)
    }

    /// Map LR2 font code to Unicode code point(s)
    fn map_code(code: i32) -> Vec<i32> {
        if code == 288 {
            return vec![0x0000301c, 0x0000ff5e];
        }

        let sjisbyte: Vec<u8>;
        if code >= 8127 {
            let sjiscode = (code + 49281) as u16;
            sjisbyte = vec![(sjiscode >> 8) as u8, (sjiscode & 0xff) as u8];
        } else if code >= 256 {
            let sjiscode = (code + 32832) as u16;
            sjisbyte = vec![(sjiscode >> 8) as u8, (sjiscode & 0xff) as u8];
        } else {
            sjisbyte = vec![(code & 0xff) as u8];
        }

        // Decode Shift_JIS bytes to UTF-16LE
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&sjisbyte);
        let s = decoded.into_owned();

        // Convert to UTF-16LE bytes
        let utf16: Vec<u16> = s.encode_utf16().collect();
        if utf16.is_empty() {
            return vec![];
        }

        // Reconstruct the code point from UTF-16LE bytes
        let mut b = Vec::new();
        for u in &utf16 {
            b.push((*u & 0xff) as u8);
            b.push(((*u >> 8) & 0xff) as u8);
        }

        let mut utfcode: i32 = 0;
        for (i, byte) in b.iter().enumerate() {
            utfcode |= (*byte as i32) << (8 * i);
        }
        vec![utfcode]
    }
}
