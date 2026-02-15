// Pomyu character (.chp) skin loader.
//
// Parses .chp files that define character animations for PMS (Pop'n Music Style)
// rhythm games. Characters display different motions based on judge results
// (neutral, great, bad, etc.).
//
// Ported from Java: PomyuCharaLoader.java

use std::path::{Path, PathBuf};

use crate::image_handle::ImageHandle;
use crate::loader::lr2_header_loader::decode_ms932;
use crate::property_id::{
    OPTION_1P_100, OPTION_1P_BORDER_OR_MORE, TIMER_MUSIC_END, TIMER_PM_CHARA_1P_BAD,
    TIMER_PM_CHARA_1P_FEVER, TIMER_PM_CHARA_1P_GOOD, TIMER_PM_CHARA_1P_GREAT,
    TIMER_PM_CHARA_1P_NEUTRAL, TIMER_PM_CHARA_2P_BAD, TIMER_PM_CHARA_2P_GREAT,
    TIMER_PM_CHARA_2P_NEUTRAL, TIMER_PM_CHARA_DANCE,
};
use crate::skin::Skin;
use crate::skin_image::{SkinImage, SkinImageSource};
use crate::skin_object::{Color, Destination, Rect, SkinObjectBase};

// ---------------------------------------------------------------------------
// Motion type constants (matching Java PomyuCharaLoader)
// ---------------------------------------------------------------------------

pub const MOTION_PLAY: i32 = 0;
pub const MOTION_BACKGROUND: i32 = 1;
pub const MOTION_NAME: i32 = 2;
pub const MOTION_FACE_UPPER: i32 = 3;
pub const MOTION_FACE_ALL: i32 = 4;
pub const MOTION_SELECT_CG: i32 = 5;
pub const MOTION_NEUTRAL: i32 = 6;
pub const MOTION_FEVER: i32 = 7;
pub const MOTION_GREAT: i32 = 8;
pub const MOTION_GOOD: i32 = 9;
pub const MOTION_BAD: i32 = 10;
pub const MOTION_FEVERWIN: i32 = 11;
pub const MOTION_WIN: i32 = 12;
pub const MOTION_LOSE: i32 = 13;
pub const MOTION_OJAMA: i32 = 14;
pub const MOTION_DANCE: i32 = 15;

// Image slot indices
const CHAR_BMP_INDEX: usize = 0;
const CHAR_TEX_INDEX: usize = 2;
const CHAR_FACE_INDEX: usize = 4;
const SELECT_CG_INDEX: usize = 6;

// Animation data priority
const PATTERN: usize = 0;
const TEXTURE: usize = 1;
const LAYER: usize = 2;

/// Frame interpolation threshold (60fps ~= 17ms).
const INCREASE_RATE_THRESHOLD: i32 = 17;

// ---------------------------------------------------------------------------
// ChpData — parsed .chp file content
// ---------------------------------------------------------------------------

/// Parsed data from a .chp file.
#[derive(Debug, Clone)]
pub struct ChpData {
    /// Image file paths: [CharBMP, CharBMP2P, CharTex, CharTex2P, CharFace, CharFace2P, SelectCG, SelectCG2P]
    pub image_paths: [Option<PathBuf>; 8],
    /// Base36-indexed coordinate table (up to 1296 entries of [x, y, w, h]).
    pub xywh: Vec<[i32; 4]>,
    /// CharFace upper-body region [x, y, w, h].
    pub char_face_upper_xywh: [i32; 4],
    /// CharFace full-body region [x, y, w, h].
    pub char_face_all_xywh: [i32; 4],
    /// Default animation frame time in ms.
    pub anime: i32,
    /// Canvas size [w, h].
    pub size: [i32; 2],
    /// Per-motion frame times (20 motion slots).
    pub frame: [i32; 20],
    /// Per-motion loop start points (20 motion slots).
    pub loop_points: [i32; 20],
    /// Pattern/Texture/Layer animation data lines.
    pub pattern_data: [Vec<String>; 3],
}

impl Default for ChpData {
    fn default() -> Self {
        Self {
            image_paths: Default::default(),
            xywh: vec![[0; 4]; 1296],
            char_face_upper_xywh: [0, 0, 256, 256],
            char_face_all_xywh: [320, 0, 320, 480],
            anime: 100,
            size: [0, 0],
            frame: [i32::MIN; 20],
            loop_points: [-1; 20],
            pattern_data: [Vec::new(), Vec::new(), Vec::new()],
        }
    }
}

// ---------------------------------------------------------------------------
// PomyuCharaLoader
// ---------------------------------------------------------------------------

/// Pomyu character skin loader.
pub struct PomyuCharaLoader;

impl PomyuCharaLoader {
    /// Checks if the given path is or contains a Pomyu character (.chp) skin.
    pub fn is_pomyu_chara(path: &Path) -> bool {
        find_chp_file(path).is_some()
    }

    /// Loads a pomyu character and generates SkinImage objects.
    ///
    /// Returns the generated SkinImage if successful.
    ///
    /// Parameters:
    /// - `chara_path`: Path to .chp file or directory containing one
    /// - `motion_type`: 0-15 (PLAY through DANCE)
    /// - `color`: 1=1P, 2=2P color variant
    /// - `dst_x/y/w/h`: Destination rectangle
    /// - `side`: 1=1P, 2=2P (determines timer assignment)
    /// - `dst_timer`: Timer ID override (for DST_PM_CHARA_ANIMATION)
    /// - `dst_op`: Option condition IDs [op1, op2, op3]
    /// - `dst_offset`: Offset ID
    /// - `skin`: Skin to add objects to
    /// - `next_handle_id`: Next available ImageHandle ID for extra images
    #[allow(clippy::too_many_arguments)] // Matches Java API for exact port parity
    pub fn load(
        chara_path: &Path,
        motion_type: i32,
        color: i32,
        dst_x: f32,
        dst_y: f32,
        dst_w: f32,
        dst_h: f32,
        side: i32,
        dst_timer: i32,
        dst_op: [i32; 3],
        dst_offset: i32,
        skin: &mut Skin,
        next_handle_id: &mut u32,
    ) -> Option<SkinImage> {
        if !(0..=15).contains(&motion_type) {
            return None;
        }

        let chp_file = find_chp_file(chara_path)?;
        let chp_dir = chp_file.parent()?;
        let data = parse_chp(&chp_file)?;

        // Determine effective color (1P or 2P)
        let set_color = determine_color(color, &data);

        // Map motion type to internal motion number
        let set_motion = motion_to_internal(motion_type);

        generate_skin_images(
            &data,
            motion_type,
            set_motion,
            set_color,
            dst_x,
            dst_y,
            dst_w,
            dst_h,
            side,
            dst_timer,
            dst_op,
            dst_offset,
            chp_dir,
            skin,
            next_handle_id,
        )
    }
}

// ---------------------------------------------------------------------------
// File discovery
// ---------------------------------------------------------------------------

/// Finds a .chp file from the given path.
///
/// If `path` is a .chp file, returns it directly.
/// If `path` is a directory, searches for the first .chp file in it.
pub fn find_chp_file(path: &Path) -> Option<PathBuf> {
    let path_str = path.to_string_lossy();

    if path_str.to_ascii_lowercase().ends_with(".chp") {
        if path.exists() {
            return Some(path.to_path_buf());
        }
        // .chp specified but doesn't exist — search parent directory
        let parent = path.parent()?;
        return find_chp_in_dir(parent);
    }

    // Path is a directory
    if path.is_dir() {
        return find_chp_in_dir(path);
    }

    None
}

fn find_chp_in_dir(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("chp"))
        {
            return Some(p);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parses a .chp file and returns the parsed data.
pub fn parse_chp(path: &Path) -> Option<ChpData> {
    let bytes = std::fs::read(path).ok()?;
    let text = decode_ms932(&bytes);
    let chp_dir = path.parent().unwrap_or(Path::new(""));

    let mut data = ChpData::default();

    for line in text.lines() {
        if !line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() <= 1 {
            continue;
        }

        let fields = pm_parse_str(&parts);
        if fields.is_empty() {
            continue;
        }

        let cmd = parts[0];

        if cmd.eq_ignore_ascii_case("#CharBMP") {
            if fields.len() > 1 {
                data.image_paths[CHAR_BMP_INDEX] = Some(resolve_image_path(chp_dir, fields[1]));
            }
        } else if cmd.eq_ignore_ascii_case("#CharBMP2P") {
            if fields.len() > 1 {
                data.image_paths[CHAR_BMP_INDEX + 1] = Some(resolve_image_path(chp_dir, fields[1]));
            }
        } else if cmd.eq_ignore_ascii_case("#CharTex") {
            if fields.len() > 1 {
                data.image_paths[CHAR_TEX_INDEX] = Some(resolve_image_path(chp_dir, fields[1]));
            }
        } else if cmd.eq_ignore_ascii_case("#CharTex2P") {
            if fields.len() > 1 {
                data.image_paths[CHAR_TEX_INDEX + 1] = Some(resolve_image_path(chp_dir, fields[1]));
            }
        } else if cmd.eq_ignore_ascii_case("#CharFace") {
            if fields.len() > 1 {
                data.image_paths[CHAR_FACE_INDEX] = Some(resolve_image_path(chp_dir, fields[1]));
            }
        } else if cmd.eq_ignore_ascii_case("#CharFace2P") {
            if fields.len() > 1 {
                data.image_paths[CHAR_FACE_INDEX + 1] =
                    Some(resolve_image_path(chp_dir, fields[1]));
            }
        } else if cmd.eq_ignore_ascii_case("#SelectCG") {
            if fields.len() > 1 {
                data.image_paths[SELECT_CG_INDEX] = Some(resolve_image_path(chp_dir, fields[1]));
            }
        } else if cmd.eq_ignore_ascii_case("#SelectCG2P") {
            if fields.len() > 1 {
                data.image_paths[SELECT_CG_INDEX + 1] =
                    Some(resolve_image_path(chp_dir, fields[1]));
            }
        } else if cmd.eq_ignore_ascii_case("#Patern") || cmd.eq_ignore_ascii_case("#Pattern") {
            data.pattern_data[PATTERN].push(line.to_string());
        } else if cmd.eq_ignore_ascii_case("#Texture") {
            data.pattern_data[TEXTURE].push(line.to_string());
        } else if cmd.eq_ignore_ascii_case("#Layer") {
            data.pattern_data[LAYER].push(line.to_string());
        } else if cmd.eq_ignore_ascii_case("#Flame") || cmd.eq_ignore_ascii_case("#Frame") {
            if fields.len() > 2 {
                let idx = pm_parse_int(fields[1]);
                let val = pm_parse_int(fields[2]);
                if idx >= 0 && (idx as usize) < data.frame.len() {
                    data.frame[idx as usize] = val;
                }
            }
        } else if cmd.eq_ignore_ascii_case("#Anime") {
            if fields.len() > 1 {
                data.anime = pm_parse_int(fields[1]);
            }
        } else if cmd.eq_ignore_ascii_case("#Size") {
            if fields.len() > 2 {
                data.size[0] = pm_parse_int(fields[1]);
                data.size[1] = pm_parse_int(fields[2]);
            }
        } else if cmd.len() == 3 {
            // Base36 coordinate definition: #XX
            let base36_str = &cmd[1..3];
            let idx = pm_parse_base36(base36_str);
            if idx >= 0 && (idx as usize) < data.xywh.len() && fields.len() > 4 {
                for i in 0..4 {
                    data.xywh[idx as usize][i] = pm_parse_int(fields[i + 1]);
                }
            }
        } else if cmd.eq_ignore_ascii_case("#CharFaceUpperSize") {
            if fields.len() > 4 {
                for i in 0..4 {
                    data.char_face_upper_xywh[i] = pm_parse_int(fields[i + 1]);
                }
            }
        } else if cmd.eq_ignore_ascii_case("#CharFaceAllSize") {
            if fields.len() > 4 {
                for i in 0..4 {
                    data.char_face_all_xywh[i] = pm_parse_int(fields[i + 1]);
                }
            }
        } else if cmd.eq_ignore_ascii_case("#Loop") && fields.len() > 2 {
            let idx = pm_parse_int(fields[1]);
            let val = pm_parse_int(fields[2]);
            if idx >= 0 && (idx as usize) < data.loop_points.len() {
                data.loop_points[idx as usize] = val;
            }
        }
    }

    Some(data)
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Parses a tab-delimited field array, stripping `//` comments.
/// Matching Java `PMparseStr`.
fn pm_parse_str<'a>(parts: &[&'a str]) -> Vec<&'a str> {
    let mut result = Vec::new();
    for &part in parts {
        if part.is_empty() {
            continue;
        }
        if part.starts_with('/') {
            break;
        }
        if let Some(pos) = part.find("//") {
            result.push(&part[..pos]);
            break;
        }
        result.push(part);
    }
    result
}

/// Parses an integer from a string, stripping non-numeric characters.
/// Matching Java `PMparseInt(String s)`.
fn pm_parse_int(s: &str) -> i32 {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    cleaned.parse().unwrap_or(0)
}

/// Parses a two-character base36 value.
/// Matching Java `PMparseInt(String s, 36)`.
fn pm_parse_base36(s: &str) -> i32 {
    if s.len() < 2 {
        return -1;
    }
    let chars: Vec<char> = s.chars().collect();
    let c1 = base36_digit(chars[0]);
    let c2 = base36_digit(chars[1]);
    if c1 < 0 || c2 < 0 {
        return -1;
    }
    c1 * 36 + c2
}

fn base36_digit(c: char) -> i32 {
    match c {
        '0'..='9' => (c as i32) - ('0' as i32),
        'a'..='z' => (c as i32) - ('a' as i32) + 10,
        'A'..='Z' => (c as i32) - ('A' as i32) + 10,
        _ => -1,
    }
}

/// Parses a two-character hex value, stripping non-hex characters.
fn pm_parse_hex(s: &str) -> i32 {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_hexdigit() || *c == '-')
        .collect();
    i32::from_str_radix(&cleaned, 16).unwrap_or(0)
}

fn resolve_image_path(chp_dir: &Path, relative: &str) -> PathBuf {
    chp_dir.join(relative.replace('\\', "/"))
}

// ---------------------------------------------------------------------------
// Color determination
// ---------------------------------------------------------------------------

fn determine_color(requested_color: i32, data: &ChpData) -> i32 {
    // Use 2P color if: requested=2, CharBMP2P exists, and either no Texture data or CharTex2P exists
    if requested_color == 2
        && data.image_paths[CHAR_BMP_INDEX + 1].is_some()
        && (data.pattern_data[TEXTURE].is_empty() || data.image_paths[CHAR_TEX_INDEX + 1].is_some())
    {
        2
    } else {
        1
    }
}

// ---------------------------------------------------------------------------
// Motion mapping
// ---------------------------------------------------------------------------

/// Maps external motion type (0-15) to internal motion number.
/// Returns i32::MIN for PLAY type (which processes all motions).
fn motion_to_internal(motion_type: i32) -> i32 {
    match motion_type {
        MOTION_NEUTRAL => 1,
        MOTION_FEVER => 6,
        MOTION_GREAT => 7,
        MOTION_GOOD => 8,
        MOTION_BAD => 10,
        MOTION_FEVERWIN => 17,
        MOTION_WIN => 15,
        MOTION_LOSE => 16,
        MOTION_OJAMA => 3,
        MOTION_DANCE => 14,
        _ => i32::MIN, // PLAY and static types
    }
}

/// Maps internal motion number to timer ID for a given side.
fn motion_to_timer(motion: i32, side: i32) -> Option<i32> {
    if side != 2 {
        // 1P timers
        match motion {
            1 => Some(TIMER_PM_CHARA_1P_NEUTRAL),
            6 => Some(TIMER_PM_CHARA_1P_FEVER),
            7 => Some(TIMER_PM_CHARA_1P_GREAT),
            8 => Some(TIMER_PM_CHARA_1P_GOOD),
            10 => Some(TIMER_PM_CHARA_1P_BAD),
            14 => Some(TIMER_PM_CHARA_DANCE),
            15 => Some(TIMER_MUSIC_END), // WIN
            16 => Some(TIMER_MUSIC_END), // LOSE
            17 => Some(TIMER_MUSIC_END), // FEVERWIN
            _ => None,
        }
    } else {
        // 2P timers
        match motion {
            1 => Some(TIMER_PM_CHARA_2P_NEUTRAL),
            7 => Some(TIMER_PM_CHARA_2P_GREAT),
            10 => Some(TIMER_PM_CHARA_2P_BAD),
            15 | 16 => Some(TIMER_MUSIC_END),
            _ => None,
        }
    }
}

/// Returns option conditions for WIN/LOSE/FEVERWIN motions.
fn motion_to_ops(motion: i32, side: i32) -> [i32; 3] {
    if side != 2 {
        match motion {
            15 => [OPTION_1P_BORDER_OR_MORE, -OPTION_1P_100, 0], // WIN
            16 => [-OPTION_1P_BORDER_OR_MORE, 0, 0],             // LOSE
            17 => [OPTION_1P_100, 0, 0],                         // FEVERWIN
            _ => [0, 0, 0],
        }
    } else {
        match motion {
            15 => [-OPTION_1P_BORDER_OR_MORE, 0, 0], // WIN (2P perspective)
            16 => [OPTION_1P_BORDER_OR_MORE, 0, 0],  // LOSE (2P perspective)
            _ => [0, 0, 0],
        }
    }
}

// ---------------------------------------------------------------------------
// SkinImage generation
// ---------------------------------------------------------------------------

/// Allocates an ImageHandle for a PomyuChara extra image.
fn alloc_handle(next_handle_id: &mut u32) -> ImageHandle {
    let handle = ImageHandle(*next_handle_id);
    *next_handle_id += 1;
    handle
}

#[allow(clippy::too_many_arguments)]
fn generate_skin_images(
    data: &ChpData,
    motion_type: i32,
    set_motion: i32,
    set_color: i32,
    dst_x: f32,
    dst_y: f32,
    dst_w: f32,
    dst_h: f32,
    side: i32,
    dst_timer: i32,
    dst_op: [i32; 3],
    dst_offset: i32,
    chp_dir: &Path,
    skin: &mut Skin,
    next_handle_id: &mut u32,
) -> Option<SkinImage> {
    // Check required image exists
    data.image_paths[CHAR_BMP_INDEX].as_ref()?;

    // Check Texture data requires CharTex
    if set_color == 1
        && !data.pattern_data[TEXTURE].is_empty()
        && data.image_paths[CHAR_TEX_INDEX].is_none()
    {
        return None;
    }

    match motion_type {
        MOTION_BACKGROUND => {
            let img_idx = CHAR_BMP_INDEX + (set_color - 1) as usize;
            let handle = alloc_extra_image(data, img_idx, chp_dir, skin, next_handle_id)?;
            let xywh = &data.xywh[1]; // index 1 for background
            let source = SkinImageSource::Reference(handle.0 as i32);
            let mut img = SkinImage {
                sources: vec![source],
                source_rect: Some(Rect::new(
                    xywh[0] as f32,
                    xywh[1] as f32,
                    xywh[2] as f32,
                    xywh[3] as f32,
                )),
                ..Default::default()
            };
            img.base = SkinObjectBase::default();
            skin.add(img.clone().into());
            Some(img)
        }
        MOTION_NAME => {
            let img_idx = CHAR_BMP_INDEX + (set_color - 1) as usize;
            let handle = alloc_extra_image(data, img_idx, chp_dir, skin, next_handle_id)?;
            let xywh = &data.xywh[0]; // index 0 for name
            let source = SkinImageSource::Reference(handle.0 as i32);
            let mut img = SkinImage {
                sources: vec![source],
                source_rect: Some(Rect::new(
                    xywh[0] as f32,
                    xywh[1] as f32,
                    xywh[2] as f32,
                    xywh[3] as f32,
                )),
                ..Default::default()
            };
            img.base = SkinObjectBase::default();
            skin.add(img.clone().into());
            Some(img)
        }
        MOTION_FACE_UPPER => {
            let img_idx = if set_color == 2 && data.image_paths[CHAR_FACE_INDEX + 1].is_some() {
                CHAR_FACE_INDEX + 1
            } else {
                CHAR_FACE_INDEX
            };
            let handle = alloc_extra_image(data, img_idx, chp_dir, skin, next_handle_id)?;
            let xywh = &data.char_face_upper_xywh;
            let source = SkinImageSource::Reference(handle.0 as i32);
            let mut img = SkinImage {
                sources: vec![source],
                source_rect: Some(Rect::new(
                    xywh[0] as f32,
                    xywh[1] as f32,
                    xywh[2] as f32,
                    xywh[3] as f32,
                )),
                ..Default::default()
            };
            img.base = SkinObjectBase::default();
            skin.add(img.clone().into());
            Some(img)
        }
        MOTION_FACE_ALL => {
            let img_idx = if set_color == 2 && data.image_paths[CHAR_FACE_INDEX + 1].is_some() {
                CHAR_FACE_INDEX + 1
            } else {
                CHAR_FACE_INDEX
            };
            let handle = alloc_extra_image(data, img_idx, chp_dir, skin, next_handle_id)?;
            let xywh = &data.char_face_all_xywh;
            let source = SkinImageSource::Reference(handle.0 as i32);
            let mut img = SkinImage {
                sources: vec![source],
                source_rect: Some(Rect::new(
                    xywh[0] as f32,
                    xywh[1] as f32,
                    xywh[2] as f32,
                    xywh[3] as f32,
                )),
                ..Default::default()
            };
            img.base = SkinObjectBase::default();
            skin.add(img.clone().into());
            Some(img)
        }
        MOTION_SELECT_CG => {
            let img_idx = if set_color == 2 && data.image_paths[SELECT_CG_INDEX + 1].is_some() {
                SELECT_CG_INDEX + 1
            } else {
                SELECT_CG_INDEX
            };
            let _handle = alloc_extra_image(data, img_idx, chp_dir, skin, next_handle_id)?;
            let source = SkinImageSource::Reference(_handle.0 as i32);
            // SelectCG uses the full image (no source_rect)
            let mut img = SkinImage {
                sources: vec![source],
                ..Default::default()
            };
            img.base = SkinObjectBase::default();
            skin.add(img.clone().into());
            Some(img)
        }
        // Animation types: PLAY, NEUTRAL..DANCE
        _ => {
            generate_animation(
                data,
                set_motion,
                set_color,
                dst_x,
                dst_y,
                dst_w,
                dst_h,
                side,
                dst_timer,
                dst_op,
                dst_offset,
                chp_dir,
                skin,
                next_handle_id,
            );
            None
        }
    }
}

/// Allocates an extra image handle and registers its path in the skin.
fn alloc_extra_image(
    data: &ChpData,
    img_idx: usize,
    _chp_dir: &Path,
    skin: &mut Skin,
    next_handle_id: &mut u32,
) -> Option<ImageHandle> {
    let path = data.image_paths[img_idx].as_ref()?;
    let handle = alloc_handle(next_handle_id);
    // Register in skin's extra_image_paths for deferred loading
    skin.extra_image_paths
        .insert(handle, (path.clone(), img_idx != SELECT_CG_INDEX));
    Some(handle)
}

// ---------------------------------------------------------------------------
// Animation generation
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn generate_animation(
    data: &ChpData,
    set_motion: i32,
    set_color: i32,
    dst_x: f32,
    dst_y: f32,
    dst_w: f32,
    dst_h: f32,
    side: i32,
    dst_timer: i32,
    dst_op: [i32; 3],
    dst_offset: i32,
    chp_dir: &Path,
    skin: &mut Skin,
    next_handle_id: &mut u32,
) {
    // Finalize frame times: unset -> anime default, < 1 -> 100
    let mut frame = data.frame;
    for f in &mut frame {
        if *f == i32::MIN {
            *f = data.anime;
        }
        if *f < 1 {
            *f = 100;
        }
    }
    let mut loop_points = data.loop_points;

    // Image source indices for each pattern type: Pattern uses CharBMP, Texture uses CharTex, Layer uses CharBMP
    let set_bmp_index = [CHAR_BMP_INDEX, CHAR_TEX_INDEX, CHAR_BMP_INDEX];

    for (pattern_index, pattern_lines) in data.pattern_data.iter().enumerate() {
        for pattern_line in pattern_lines {
            let parts: Vec<&str> = pattern_line.split('\t').collect();
            if parts.len() <= 1 {
                continue;
            }

            let img_idx = set_bmp_index[pattern_index] + (set_color - 1) as usize;

            // Allocate image handle for this pattern's source
            let handle = match alloc_extra_image(data, img_idx, chp_dir, skin, next_handle_id) {
                Some(h) => h,
                None => continue,
            };

            let fields = pm_parse_str(&parts);
            let motion = if fields.len() > 1 {
                pm_parse_int(fields[1])
            } else {
                continue;
            };

            // Parse dst fields (up to 4: src_frames, dst_xywh, alpha, angle)
            let mut dst = ["".to_string(), String::new(), String::new(), String::new()];
            for (i, d) in dst.iter_mut().enumerate() {
                if fields.len() > i + 2 {
                    // Strip non-alphanumeric except hyphens (matching Java replaceAll)
                    *d = fields[i + 2]
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                        .collect();
                }
            }

            // Determine timer and ops
            let (timer, op) = if set_motion != i32::MIN && set_motion == motion {
                (dst_timer, dst_op)
            } else if set_motion == i32::MIN {
                match motion_to_timer(motion, side) {
                    Some(t) => {
                        let ops = motion_to_ops(motion, side);
                        (t, ops)
                    }
                    None => continue,
                }
            } else {
                continue;
            };

            // Validate dst field lengths
            let d0_len = dst[0].len();
            if d0_len == 0 || !d0_len.is_multiple_of(2) {
                continue;
            }
            let valid = dst[1..].iter().all(|d| d.is_empty() || d.len() == d0_len);
            if !valid {
                continue;
            }

            let frame_count = d0_len / 2;

            // Clamp loop point
            if loop_points[motion as usize] >= frame_count as i32 - 1 {
                loop_points[motion as usize] = frame_count as i32 - 2;
            } else if loop_points[motion as usize] < -1 {
                loop_points[motion as usize] = -1;
            }
            let loop_point = loop_points[motion as usize];

            let cycle = frame[motion as usize] * frame_count as i32;
            let loop_time = frame[motion as usize] * (loop_point + 1);

            // Set PMcharaTime for chara timer motions
            if set_motion == i32::MIN
                && (TIMER_PM_CHARA_1P_NEUTRAL..TIMER_MUSIC_END).contains(&timer)
            {
                // Store cycle time — this will be picked up by PomyuCharaProcessor
                // via skin's pomyu_chara_times
                let time_idx = (timer - TIMER_PM_CHARA_1P_NEUTRAL) as usize;
                if time_idx < skin.pomyu_chara_times.len() {
                    skin.pomyu_chara_times[time_idx] = cycle;
                }
            }

            // Check for hyphen interpolation
            let has_hyphen = dst[1..].iter().any(|d| d.contains('-'));

            // Calculate increase rate for interpolation
            let increase_rate = if has_hyphen && frame[motion as usize] >= INCREASE_RATE_THRESHOLD {
                let mut rate = 1;
                for i in 1..=frame[motion as usize] {
                    if frame[motion as usize] / i < INCREASE_RATE_THRESHOLD
                        && frame[motion as usize] % i == 0
                    {
                        rate = i;
                        break;
                    }
                }
                // Expand dst fields
                for d in dst[1..].iter_mut() {
                    if !d.is_empty() {
                        let mut expanded = String::with_capacity(d.len() * rate as usize);
                        let chars: Vec<char> = d.chars().collect();
                        for pair in chars.chunks(2) {
                            if pair.len() == 2 {
                                for _ in 0..rate {
                                    expanded.push(pair[0]);
                                    expanded.push(pair[1]);
                                }
                            }
                        }
                        *d = expanded;
                    }
                }
                rate
            } else {
                1
            };

            let frame_time = frame[motion as usize] as f64 / increase_rate as f64;
            let loop_frame = loop_point * increase_rate;

            // Parse destination xywh for each frame
            let dst_frame_count = if !dst[1].is_empty() {
                dst[1].len() / 2
            } else {
                d0_len / 2 * increase_rate as usize
            };

            let mut dstxywh = vec![[0i32, 0, data.size[0], data.size[1]]; dst_frame_count];

            // Process dst[1] (position data) with hyphen interpolation
            if !dst[1].is_empty() {
                let chars: Vec<char> = dst[1].chars().collect();
                let mut start_xywh = [0, 0, data.size[0], data.size[1]];
                let mut i = 0;
                while i < chars.len() - 1 {
                    let pair = format!("{}{}", chars[i], chars[i + 1]);
                    if pair == "--" {
                        // Count consecutive hyphens
                        let mut count = 0;
                        let mut j = i;
                        while j < chars.len() - 1 && chars[j] == '-' && chars[j + 1] == '-' {
                            count += 1;
                            j += 2;
                        }
                        // Get end value
                        let end_xywh = if j < chars.len() - 1 {
                            let end_pair = format!("{}{}", chars[j], chars[j + 1]);
                            let idx = pm_parse_base36(&end_pair);
                            if idx >= 0 && (idx as usize) < data.xywh.len() {
                                data.xywh[idx as usize]
                            } else {
                                [0, 0, data.size[0], data.size[1]]
                            }
                        } else {
                            [0, 0, data.size[0], data.size[1]]
                        };
                        // Interpolate
                        for k in 0..count {
                            let frame_idx = (i / 2) + k;
                            if frame_idx < dstxywh.len() {
                                for dim in 0..4 {
                                    dstxywh[frame_idx][dim] = start_xywh[dim]
                                        + (end_xywh[dim] - start_xywh[dim]) * (k as i32 + 1)
                                            / (count as i32 + 1);
                                }
                            }
                        }
                        i += (count - 1) * 2;
                    } else {
                        let idx = pm_parse_base36(&pair);
                        if idx >= 0 && (idx as usize) < data.xywh.len() {
                            start_xywh = data.xywh[idx as usize];
                            let frame_idx = i / 2;
                            if frame_idx < dstxywh.len() {
                                dstxywh[frame_idx] = start_xywh;
                            }
                        }
                    }
                    i += 2;
                }
            }

            // Parse alpha and angle (dst[2] and dst[3])
            let mut alpha_angle = vec![[255i32, 0]; dstxywh.len()];
            for (index, dst_field) in dst.iter().enumerate().skip(2).take(2) {
                if dst_field.is_empty() {
                    continue;
                }
                let chars: Vec<char> = dst_field.chars().collect();
                let mut start_value = if index == 2 { 255 } else { 0 };
                let mut i = 0;
                while i < chars.len() - 1 {
                    let pair = format!("{}{}", chars[i], chars[i + 1]);
                    if pair == "--" {
                        let mut count = 0;
                        let mut j = i;
                        while j < chars.len() - 1 && chars[j] == '-' && chars[j + 1] == '-' {
                            count += 1;
                            j += 2;
                        }
                        let mut end_value = 0;
                        if j < chars.len() - 1 {
                            let end_pair = format!("{}{}", chars[j], chars[j + 1]);
                            let parsed = pm_parse_hex(&end_pair);
                            if (0..=255).contains(&parsed) {
                                end_value = parsed;
                                if index == 3 {
                                    end_value = (end_value as f32 * 360.0 / 256.0).round() as i32;
                                }
                            }
                        }
                        for k in 0..count {
                            let frame_idx = (i / 2) + k;
                            if frame_idx < alpha_angle.len() {
                                alpha_angle[frame_idx][index - 2] = start_value
                                    + (end_value - start_value) * (k as i32 + 1)
                                        / (count as i32 + 1);
                            }
                        }
                        i += (count - 1) * 2;
                    } else {
                        let parsed = pm_parse_hex(&pair);
                        if (0..=255).contains(&parsed) {
                            start_value = parsed;
                            if index == 3 {
                                start_value = (start_value as f32 * 360.0 / 256.0).round() as i32;
                            }
                            let frame_idx = i / 2;
                            if frame_idx < alpha_angle.len() {
                                alpha_angle[frame_idx][index - 2] = start_value;
                            }
                        }
                    }
                    i += 2;
                }
            }

            let size_w = data.size[0] as f32;
            let size_h = data.size[1] as f32;

            // Generate SkinImages: pre-loop + loop parts
            if (loop_frame + increase_rate) != 0 {
                // Pre-loop part: frames 0..=loop_point
                let pre_loop_count = (loop_point + 1) as usize;
                let src_chars: Vec<char> = dst[0].chars().collect();
                let mut images = Vec::with_capacity(pre_loop_count);
                for fi in 0..pre_loop_count {
                    let ci = fi * 2;
                    if ci + 1 < src_chars.len() {
                        let pair = format!("{}{}", src_chars[ci], src_chars[ci + 1]);
                        let idx = pm_parse_base36(&pair);
                        if idx >= 0
                            && (idx as usize) < data.xywh.len()
                            && data.xywh[idx as usize][2] > 0
                            && data.xywh[idx as usize][3] > 0
                        {
                            images.push(ImageHandle(handle.0));
                        } else {
                            images.push(ImageHandle::NONE);
                        }
                    }
                }

                let mut part = SkinImage {
                    sources: vec![SkinImageSource::Frames {
                        images,
                        timer: Some(timer),
                        cycle: loop_time,
                    }],
                    ..Default::default()
                };
                part.base.timer = Some(crate::property_id::TimerId(timer));

                // Add destinations for pre-loop frames
                for i in 0..(loop_frame + increase_rate) as usize {
                    if i < dstxywh.len() && size_w > 0.0 && size_h > 0.0 {
                        part.base.add_destination(Destination {
                            time: (frame_time * i as f64) as i64,
                            region: compute_region(
                                dst_x,
                                dst_y,
                                dst_w,
                                dst_h,
                                &dstxywh[i],
                                size_w,
                                size_h,
                            ),
                            color: Color::from_rgba_u8(
                                alpha_angle[i][0].clamp(0, 255) as u8,
                                255,
                                255,
                                255,
                            ),
                            angle: alpha_angle[i][1],
                            acc: 3, // discrete
                        });
                    }
                }
                // Final pre-loop destination
                let last_pre = ((loop_frame + increase_rate) as usize).saturating_sub(1);
                if last_pre < dstxywh.len() && size_w > 0.0 && size_h > 0.0 {
                    part.base.add_destination(Destination {
                        time: loop_time as i64 - 1,
                        region: compute_region(
                            dst_x,
                            dst_y,
                            dst_w,
                            dst_h,
                            &dstxywh[last_pre],
                            size_w,
                            size_h,
                        ),
                        color: Color::from_rgba_u8(
                            alpha_angle[last_pre][0].clamp(0, 255) as u8,
                            255,
                            255,
                            255,
                        ),
                        angle: alpha_angle[last_pre][1],
                        acc: 3,
                    });
                    if dst_offset != 0 {
                        part.base.set_offset_ids(&[dst_offset]);
                    }
                }
                set_option_conditions(&mut part.base, &op);
                skin.add(part.into());
            }

            // Loop part: frames after loop_point
            let loop_start = (loop_point + 1) as usize;
            let total_frames = d0_len / 2;
            if loop_start < total_frames {
                let src_chars: Vec<char> = dst[0].chars().collect();
                let loop_frame_count = total_frames - loop_start;
                let mut images = Vec::with_capacity(loop_frame_count);
                for fi in loop_start..total_frames {
                    let ci = fi * 2;
                    if ci + 1 < src_chars.len() {
                        let pair = format!("{}{}", src_chars[ci], src_chars[ci + 1]);
                        let idx = pm_parse_base36(&pair);
                        if idx >= 0
                            && (idx as usize) < data.xywh.len()
                            && data.xywh[idx as usize][2] > 0
                            && data.xywh[idx as usize][3] > 0
                        {
                            images.push(ImageHandle(handle.0));
                        } else {
                            images.push(ImageHandle::NONE);
                        }
                    }
                }

                let loop_cycle = cycle - loop_time;
                let mut part = SkinImage {
                    sources: vec![SkinImageSource::Frames {
                        images,
                        timer: Some(timer),
                        cycle: loop_cycle,
                    }],
                    ..Default::default()
                };
                part.base.timer = Some(crate::property_id::TimerId(timer));
                part.base.loop_time = loop_time;

                // Add destinations for loop frames
                for i in (loop_frame + increase_rate) as usize..dstxywh.len() {
                    if size_w > 0.0 && size_h > 0.0 {
                        part.base.add_destination(Destination {
                            time: (frame_time * i as f64) as i64,
                            region: compute_region(
                                dst_x,
                                dst_y,
                                dst_w,
                                dst_h,
                                &dstxywh[i],
                                size_w,
                                size_h,
                            ),
                            color: Color::from_rgba_u8(
                                alpha_angle[i][0].clamp(0, 255) as u8,
                                255,
                                255,
                                255,
                            ),
                            angle: alpha_angle[i][1],
                            acc: 3,
                        });
                    }
                }
                // Final loop destination
                let last = dstxywh.len().saturating_sub(1);
                if size_w > 0.0 && size_h > 0.0 {
                    part.base.add_destination(Destination {
                        time: cycle as i64,
                        region: compute_region(
                            dst_x,
                            dst_y,
                            dst_w,
                            dst_h,
                            &dstxywh[last],
                            size_w,
                            size_h,
                        ),
                        color: Color::from_rgba_u8(
                            alpha_angle[last][0].clamp(0, 255) as u8,
                            255,
                            255,
                            255,
                        ),
                        angle: alpha_angle[last][1],
                        acc: 3,
                    });
                    if dst_offset != 0 {
                        part.base.set_offset_ids(&[dst_offset]);
                    }
                }
                set_option_conditions(&mut part.base, &op);
                skin.add(part.into());
            }
        }
    }
}

fn compute_region(
    dst_x: f32,
    dst_y: f32,
    dst_w: f32,
    dst_h: f32,
    xywh: &[i32; 4],
    size_w: f32,
    size_h: f32,
) -> Rect {
    Rect {
        x: dst_x + xywh[0] as f32 * dst_w / size_w,
        y: dst_y + dst_h - (xywh[1] as f32 + xywh[3] as f32) * dst_h / size_h,
        w: xywh[2] as f32 * dst_w / size_w,
        h: xywh[3] as f32 * dst_h / size_h,
    }
}

fn set_option_conditions(base: &mut SkinObjectBase, op: &[i32; 3]) {
    let conditions: Vec<i32> = op.iter().copied().filter(|&v| v != 0).collect();
    if !conditions.is_empty() {
        base.option_conditions = conditions;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pm_parse_int() {
        assert_eq!(pm_parse_int("123"), 123);
        assert_eq!(pm_parse_int("abc123def"), 123);
        assert_eq!(pm_parse_int("-5"), -5);
        assert_eq!(pm_parse_int(""), 0);
    }

    #[test]
    fn test_pm_parse_base36() {
        assert_eq!(pm_parse_base36("00"), 0);
        assert_eq!(pm_parse_base36("01"), 1);
        assert_eq!(pm_parse_base36("0Z"), 35);
        assert_eq!(pm_parse_base36("10"), 36);
        assert_eq!(pm_parse_base36("ZZ"), 35 * 36 + 35);
        assert_eq!(pm_parse_base36("aA"), 10 * 36 + 10); // case-insensitive
    }

    #[test]
    fn test_pm_parse_hex() {
        assert_eq!(pm_parse_hex("FF"), 255);
        assert_eq!(pm_parse_hex("00"), 0);
        assert_eq!(pm_parse_hex("7F"), 127);
    }

    #[test]
    fn test_pm_parse_str() {
        let parts = ["#Pattern", "1", "0001", "0001", "FF", "00"];
        let result = pm_parse_str(&parts);
        assert_eq!(result, vec!["#Pattern", "1", "0001", "0001", "FF", "00"]);
    }

    #[test]
    fn test_pm_parse_str_with_comment() {
        let parts = ["#Frame", "1", "100", "//comment"];
        let result = pm_parse_str(&parts);
        assert_eq!(result, vec!["#Frame", "1", "100"]);
    }

    #[test]
    fn test_pm_parse_str_with_inline_comment() {
        let parts = ["#Anime", "100//default"];
        let result = pm_parse_str(&parts);
        assert_eq!(result, vec!["#Anime", "100"]);
    }

    #[test]
    fn test_is_pomyu_chara_nonexistent() {
        assert!(!PomyuCharaLoader::is_pomyu_chara(Path::new(
            "/nonexistent/path.chp"
        )));
    }

    #[test]
    fn test_motion_to_internal() {
        assert_eq!(motion_to_internal(MOTION_NEUTRAL), 1);
        assert_eq!(motion_to_internal(MOTION_FEVER), 6);
        assert_eq!(motion_to_internal(MOTION_GREAT), 7);
        assert_eq!(motion_to_internal(MOTION_GOOD), 8);
        assert_eq!(motion_to_internal(MOTION_BAD), 10);
        assert_eq!(motion_to_internal(MOTION_WIN), 15);
        assert_eq!(motion_to_internal(MOTION_LOSE), 16);
        assert_eq!(motion_to_internal(MOTION_FEVERWIN), 17);
        assert_eq!(motion_to_internal(MOTION_DANCE), 14);
        assert_eq!(motion_to_internal(MOTION_OJAMA), 3);
        assert_eq!(motion_to_internal(MOTION_PLAY), i32::MIN);
    }

    #[test]
    fn test_motion_to_timer_1p() {
        assert_eq!(motion_to_timer(1, 1), Some(TIMER_PM_CHARA_1P_NEUTRAL));
        assert_eq!(motion_to_timer(6, 1), Some(TIMER_PM_CHARA_1P_FEVER));
        assert_eq!(motion_to_timer(7, 1), Some(TIMER_PM_CHARA_1P_GREAT));
        assert_eq!(motion_to_timer(8, 1), Some(TIMER_PM_CHARA_1P_GOOD));
        assert_eq!(motion_to_timer(10, 1), Some(TIMER_PM_CHARA_1P_BAD));
        assert_eq!(motion_to_timer(14, 1), Some(TIMER_PM_CHARA_DANCE));
        assert_eq!(motion_to_timer(15, 1), Some(TIMER_MUSIC_END));
    }

    #[test]
    fn test_motion_to_timer_2p() {
        assert_eq!(motion_to_timer(1, 2), Some(TIMER_PM_CHARA_2P_NEUTRAL));
        assert_eq!(motion_to_timer(7, 2), Some(TIMER_PM_CHARA_2P_GREAT));
        assert_eq!(motion_to_timer(10, 2), Some(TIMER_PM_CHARA_2P_BAD));
    }

    #[test]
    fn test_determine_color_default() {
        let data = ChpData::default();
        assert_eq!(determine_color(1, &data), 1);
        assert_eq!(determine_color(2, &data), 1); // No 2P images
    }

    #[test]
    fn test_parse_chp_basic() {
        let dir = tempfile::tempdir().unwrap();
        let chp_path = dir.path().join("test.chp");

        // Create a basic .chp file
        let content =
            b"#Anime\t100\n#Size\t320\t240\n#Frame\t1\t50\n#00\t0\t0\t32\t32\n#Loop\t1\t3\n";
        std::fs::write(&chp_path, content).unwrap();

        let data = parse_chp(&chp_path).unwrap();
        assert_eq!(data.anime, 100);
        assert_eq!(data.size, [320, 240]);
        assert_eq!(data.frame[1], 50);
        assert_eq!(data.xywh[0], [0, 0, 32, 32]);
        assert_eq!(data.loop_points[1], 3);
    }

    #[test]
    fn test_find_chp_file_direct() {
        let dir = tempfile::tempdir().unwrap();
        let chp_path = dir.path().join("chara.chp");
        std::fs::write(&chp_path, b"#Anime\t100\n").unwrap();

        assert_eq!(find_chp_file(&chp_path), Some(chp_path.clone()));
    }

    #[test]
    fn test_find_chp_file_in_dir() {
        let dir = tempfile::tempdir().unwrap();
        let chp_path = dir.path().join("chara.chp");
        std::fs::write(&chp_path, b"#Anime\t100\n").unwrap();

        let found = find_chp_file(dir.path());
        assert!(found.is_some());
        assert_eq!(found.unwrap(), chp_path);
    }

    #[test]
    fn test_find_chp_file_nonexistent() {
        assert!(find_chp_file(Path::new("/nonexistent/dir")).is_none());
    }

    #[test]
    fn test_chp_data_defaults() {
        let data = ChpData::default();
        assert_eq!(data.anime, 100);
        assert_eq!(data.char_face_upper_xywh, [0, 0, 256, 256]);
        assert_eq!(data.char_face_all_xywh, [320, 0, 320, 480]);
        assert_eq!(data.xywh.len(), 1296);
        assert!(data.frame.iter().all(|&f| f == i32::MIN));
        assert!(data.loop_points.iter().all(|&l| l == -1));
    }
}
