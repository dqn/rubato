// BitmapFontBatchLoader.java -> bitmap_font_batch_loader.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use log::warn;

use crate::json::json_skin;
use crate::loaders::bitmap_font_cache::{self, CacheableBitmapFont};
use crate::loaders::skin_loader;
use crate::stubs::{BitmapFont, BitmapFontData, TextureRegion};

/// Parallelized bitmap font preloader.
/// Largely adopted from SkinTextBitmap.java
///
/// Translated from BitmapFontBatchLoader.java
pub struct BitmapFontBatchLoader {
    usecim: bool,
    _use_mip_maps: bool,
    font_paths: HashMap<PathBuf, i32>,
    font_data: HashMap<PathBuf, BitmapFontData>,
}

/// Record for font sizes parsed from .fnt file header
struct FontSizes {
    size: f32,
    scale_w: f32,
    scale_h: f32,
}

impl BitmapFontBatchLoader {
    pub fn new(skin: &json_skin::Skin, skin_path: &Path, usecim: bool, use_mip_maps: bool) -> Self {
        let mut font_paths: HashMap<PathBuf, i32> = HashMap::new();

        let skin_parent = skin_path.parent().unwrap_or(Path::new(""));

        for font in &skin.font {
            match (|| -> Result<(), String> {
                let font_path_str = font.path.as_deref().unwrap_or("");
                let path = skin_parent.join(font_path_str);
                let valid_path = path
                    .to_str()
                    .map(|s| s.to_lowercase().ends_with(".fnt"))
                    .unwrap_or(false);
                let already_cached = bitmap_font_cache::has(Some(&path));
                if !valid_path || already_cached {
                    return Ok(());
                }
                font_paths.insert(path, font.font_type);
                Ok(())
            })() {
                Ok(()) => {}
                Err(e) => {
                    warn!("Skin attempted to load a font with an invalid path: {}", e);
                }
            }
        }

        Self {
            usecim,
            _use_mip_maps: use_mip_maps,
            font_paths,
            font_data: HashMap::new(),
        }
    }

    pub fn load(&mut self) {
        // In Java, this uses a thread pool for parallel loading.
        // In Rust, we translate the logic sequentially for correctness,
        // since the actual GL texture creation must happen on the main thread anyway.
        // The parallel image loading could be done with rayon in the future.

        // Parse each font description file
        for path in self.font_paths.keys() {
            let font_data = BitmapFontData::from_fnt(path).unwrap_or_default();
            self.font_data.insert(path.clone(), font_data);
        }

        // Load images for each font data page
        let mut loaded_textures: HashMap<String, TextureRegion> = HashMap::new();
        for font_data in self.font_data.values() {
            for image_path in &font_data.image_paths {
                if loaded_textures.contains_key(image_path) {
                    continue;
                }
                if let Some(tex) = skin_loader::texture(image_path, self.usecim) {
                    loaded_textures.insert(image_path.clone(), TextureRegion::from_texture(tex));
                }
            }
        }

        // Build CacheableBitmapFont for each font path
        for (path, _type_id) in &self.font_paths {
            let font_data = match self.font_data.get(path) {
                Some(fd) => fd.clone(),
                None => continue,
            };

            // Collect texture regions for each page image
            let image_regions: Vec<TextureRegion> = font_data
                .image_paths
                .iter()
                .filter_map(|ip| loaded_textures.get(ip).cloned())
                .collect();

            // Use parsed font data metrics, fall back to read_font_sizes for legacy
            let mut size = font_data.line_height;
            let mut scale_w = font_data.scale_w;
            let mut scale_h = font_data.scale_h;

            if size == 0.0
                && let Some(s) = read_font_sizes(path)
            {
                size = s.size;
                scale_w = s.scale_w;
                scale_h = s.scale_h;
            }
            if scale_w == 0.0 && !image_regions.is_empty() {
                scale_w = image_regions[0].region_width as f32;
                scale_h = image_regions[0].region_height as f32;
            }

            let font_cache = CacheableBitmapFont {
                font: BitmapFont::new(),
                font_data,
                regions: image_regions,
                type_: *_type_id,
                original_size: size,
                page_width: scale_w,
                page_height: scale_h,
            };
            bitmap_font_cache::set(path.clone(), font_cache);
        }
    }
}

/// Reads font size, scaleW, scaleH from the .fnt file header.
/// Corresponds to BitmapFontBatchLoader.readFontSizes(Path)
fn read_font_sizes(font_path: &Path) -> Option<FontSizes> {
    // size is not available from BitmapFont, so we parse it manually
    let file = match std::fs::File::open(font_path) {
        Ok(f) => f,
        Err(e) => {
            warn!("{}", e);
            return None;
        }
    };
    let reader = BufReader::with_capacity(512, file);
    let mut lines = reader.lines();

    // First line: info face="..." size=N ...
    let line1 = match lines.next() {
        Some(Ok(l)) => l,
        _ => {
            warn!("Failed to read first line of font file: {:?}", font_path);
            return None;
        }
    };

    let size = match parse_field(&line1, "size=") {
        Some(v) => v as f32,
        None => {
            warn!("Failed to parse size from font file: {:?}", font_path);
            return None;
        }
    };

    // Second line: common lineHeight=N ... scaleW=N scaleH=N
    let line2 = match lines.next() {
        Some(Ok(l)) => l,
        _ => {
            warn!("Failed to read second line of font file: {:?}", font_path);
            return None;
        }
    };

    let scale_w = match parse_field(&line2, "scaleW=") {
        Some(v) => v as f32,
        None => {
            warn!("Failed to parse scaleW from font file: {:?}", font_path);
            return None;
        }
    };

    let scale_h = match parse_field(&line2, "scaleH=") {
        Some(v) => v as f32,
        None => {
            warn!("Failed to parse scaleH from font file: {:?}", font_path);
            return None;
        }
    };

    Some(FontSizes {
        size,
        scale_w,
        scale_h,
    })
}

/// Parses an integer field value from a line like "key=value rest..."
fn parse_field(line: &str, key: &str) -> Option<i32> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    rest[..end].parse::<i32>().ok()
}
