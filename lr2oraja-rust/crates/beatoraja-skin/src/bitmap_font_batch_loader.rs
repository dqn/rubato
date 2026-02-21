// BitmapFontBatchLoader.java -> bitmap_font_batch_loader.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use log::warn;

use crate::bitmap_font_cache::{self, CacheableBitmapFont};
use crate::json::json_skin;
use crate::skin_loader;
use crate::stubs::{BitmapFont, BitmapFontData, Pixmap, PixmapFormat, Texture, TextureRegion};

/// Parallelized bitmap font preloader.
/// Largely adopted from SkinTextBitmap.java
///
/// Translated from BitmapFontBatchLoader.java
pub struct BitmapFontBatchLoader {
    usecim: bool,
    use_mip_maps: bool,
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
            use_mip_maps,
            font_paths,
            font_data: HashMap::new(),
        }
    }

    pub fn load(&mut self) {
        // In Java, this uses a thread pool for parallel loading.
        // In Rust, we translate the logic sequentially for correctness,
        // since the actual GL texture creation must happen on the main thread anyway.
        // The parallel image loading could be done with rayon in the future.

        let mut parsed_font_data: Vec<(PathBuf, BitmapFontData)> = Vec::new();

        // Parse each font description file
        for path in self.font_paths.keys() {
            // BitmapFont.BitmapFontData fontData = new BitmapFont.BitmapFontData(new FileHandle(path.toFile()), false);
            // In our stub, BitmapFontData is empty. The actual parsing would happen here.
            let font_data = BitmapFontData;
            self.font_data.insert(path.clone(), font_data.clone());
            parsed_font_data.push((path.clone(), font_data));
        }

        // Load images for each font data
        // Each font data contains image paths that need to be loaded
        let loaded_textures: HashMap<String, TextureRegion> = HashMap::new();

        // In the Java code, imagePaths come from BitmapFontData.imagePaths
        // Since our BitmapFontData is a stub, we skip actual image loading.
        // The resource pool loading would happen here:
        //
        // for (path, font_data) in &parsed_font_data {
        //     for image_path in &font_data.image_paths {
        //         let resource = skin_loader::get_resource();
        //         if let Some(ref r) = *resource {
        //             if let Some(_pixmap) = r.get(image_path) {
        //                 // image loaded successfully
        //             }
        //         }
        //     }
        // }

        // Create textures on main thread
        // In the Java code, textures are created from loaded images:
        //
        // for image_path in loaded_images {
        //     let texture = skin_loader::get_texture(&image_path, self.usecim, self.use_mip_maps);
        //     loaded_textures.insert(image_path, TextureRegion::from_texture(texture));
        // }

        // Build CacheableBitmapFont for each font path
        for (path, _type_id) in &self.font_paths {
            let font_data = match self.font_data.get(path) {
                Some(fd) => fd.clone(),
                None => continue,
            };

            // In Java: Array<TextureRegion> imageRegions = new Array<>(fontData.imagePaths.length);
            // Since BitmapFontData is stubbed, imageRegions will be empty.
            let image_regions: Vec<TextureRegion> = Vec::new();

            // float size = fontData.lineHeight;
            // Using 0.0 as stub since BitmapFontData doesn't have lineHeight
            let mut size: f32 = 0.0;
            let mut scale_w: f32 = 0.0;
            let mut scale_h: f32 = 0.0;

            let sizes = read_font_sizes(path);
            if let Some(s) = sizes {
                size = s.size;
                scale_w = s.scale_w;
                scale_h = s.scale_h;
            } else if !image_regions.is_empty() {
                scale_w = image_regions[0].get_region_width() as f32;
                scale_h = image_regions[0].get_region_height() as f32;
            }

            // fontCache.font = new BitmapFont(fontData, imageRegions, true);
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
