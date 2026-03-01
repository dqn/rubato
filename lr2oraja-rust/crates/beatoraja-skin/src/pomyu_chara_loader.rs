// PomyuCharaLoader.java -> pomyu_chara_loader.rs
// Mechanical line-by-line translation.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;

use crate::property::timer_property::TimerProperty;
use crate::skin_image::SkinImage;
use crate::skin_property::*;
use crate::stubs::{Pixmap, PixmapFormat, PlaySkinStub, SkinLoaderStub, Texture, TextureRegion};

pub const PLAY: i32 = 0;
pub const BACKGROUND: i32 = 1;
pub const NAME: i32 = 2;
pub const FACE_UPPER: i32 = 3;
pub const FACE_ALL: i32 = 4;
pub const SELECT_CG: i32 = 5;
pub const NEUTRAL: i32 = 6;
pub const FEVER: i32 = 7;
pub const GREAT: i32 = 8;
pub const GOOD: i32 = 9;
pub const BAD: i32 = 10;
pub const FEVERWIN: i32 = 11;
pub const WIN: i32 = 12;
pub const LOSE: i32 = 13;
pub const OJAMA: i32 = 14;
pub const DANCE: i32 = 15;

pub struct PomyuCharaLoader<'a> {
    skin: &'a mut PlaySkinStub,
}

impl<'a> PomyuCharaLoader<'a> {
    pub fn new(skin: &'a mut PlaySkinStub) -> Self {
        Self { skin }
    }

    pub fn load_with_timer_property(
        &mut self,
        usecim: bool,
        imagefile: &Path,
        load_type: i32,
        color: i32,
        dstx: f32,
        dsty: f32,
        dstw: f32,
        dsth: f32,
        side: i32,
        dsttimer: &dyn TimerProperty,
        dst_op1: i32,
        dst_op2: i32,
        dst_op3: i32,
        dst_offset: i32,
    ) -> Option<SkinImage> {
        self.load(
            usecim,
            imagefile,
            load_type,
            color,
            dstx,
            dsty,
            dstw,
            dsth,
            side,
            dsttimer.get_timer_id(),
            dst_op1,
            dst_op2,
            dst_op3,
            dst_offset,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(unused_assignments)]
    pub fn load(
        &mut self,
        usecim: bool,
        imagefile: &Path,
        load_type: i32,
        color: i32,
        dstx: f32,
        dsty: f32,
        dstw: f32,
        dsth: f32,
        side: i32,
        dsttimer: i32,
        dst_op1: i32,
        dst_op2: i32,
        dst_op3: i32,
        dst_offset: i32,
    ) -> Option<SkinImage> {
        // type 0:play 1:char background 2:name image 3:face(upper) 4:face(all) 5:select CG
        // 6:NEUTRAL 7:FEVER 8:GREAT 9:GOOD 10:BAD 11:FEVERWIN 12:WIN 13:LOSE 14:OJAMA 15:DANCE
        if !(0..=15).contains(&load_type) {
            return None;
        }

        let imagefile_str = imagefile.to_string_lossy().to_string();
        let mut chp: Option<String> = None;
        let mut chpdir: Option<String> = None;

        let ext4 = if imagefile_str.len() >= 4 {
            &imagefile_str[imagefile_str.len() - 4..]
        } else {
            ""
        };

        if imagefile.exists() && ext4.eq_ignore_ascii_case(".chp") {
            chp = Some(imagefile_str.clone());
        } else if !imagefile.exists() && ext4.eq_ignore_ascii_case(".chp") {
            let last_sep = imagefile_str
                .rfind('\\')
                .unwrap_or(0)
                .max(imagefile_str.rfind('/').unwrap_or(0));
            chpdir = Some(imagefile_str[..last_sep + 1].to_string());
        } else {
            let last_char = imagefile_str.chars().last().unwrap_or(' ');
            if last_char != '/' && last_char != '\\' {
                chpdir = Some(format!("{}/", imagefile_str));
            } else {
                chpdir = Some(imagefile_str.clone());
            }
        }

        if chp.is_none()
            && let Some(ref dir) = chpdir
        {
            // Search for .chp file in directory
            let dir_path = Path::new(dir);
            if let Ok(entries) = std::fs::read_dir(dir_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(path_str) = path.to_str()
                        && path_str.len() >= 4
                        && path_str[path_str.len() - 4..].eq_ignore_ascii_case(".chp")
                    {
                        chp = Some(path_str.to_string());
                        break;
                    }
                }
            }
        }

        let chp = chp?;

        // Image data: 0:#CharBMP 1:#CharBMP2P 2:#CharTex 3:#CharTex2P 4:#CharFace 5:#CharFace2P 6:#SelectCG 7:#SelectCG2P
        let mut char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
        let char_bmp_index: usize = 0;
        let char_tex_index: usize = 2;
        let char_face_index: usize = 4;
        let select_cg_index: usize = 6;
        // Transparent processing flags
        let mut transparent_flag = [false; 8];
        // Parameters
        let mut xywh = vec![[0_i32; 4]; 1296];
        let mut char_face_upper_xywh = [0, 0, 256, 256];
        let mut char_face_all_xywh = [320, 0, 320, 480];
        let mut anime = 100_i32;
        let mut size = [0_i32; 2];
        let mut frame = [i32::MIN; 20];
        let mut loop_val = [-1_i32; 20];
        // Final color
        let mut set_color = 1;
        // Frame interpolation threshold time 60FPS = 17ms
        let increase_rate_threshold = 17;
        // #Pattern, #Texture, #Layer data
        let pattern_idx = 0;
        let texture_idx = 1;
        let layer_idx = 2;
        let mut pattern_data: Vec<Vec<String>> = vec![Vec::new(), Vec::new(), Vec::new()];

        let chp_dir_prefix = {
            let last_sep = chp
                .rfind('\\')
                .unwrap_or(0)
                .max(chp.rfind('/').unwrap_or(0));
            chp[..last_sep + 1].to_string()
        };

        // Read the .chp file
        if let Ok(file) = std::fs::File::open(&chp) {
            let reader = BufReader::new(file);
            // In Java, this uses MS932 encoding. In Rust, we just read as UTF-8/lossy.
            for line_result in reader.lines() {
                let line = match line_result {
                    Ok(l) => l,
                    Err(_) => continue,
                };
                if line.starts_with('#') {
                    let str_parts: Vec<&str> = line.split('\t').collect();
                    if str_parts.len() > 1 {
                        let data = pm_parse_str(&str_parts);
                        if str_parts[0].eq_ignore_ascii_case("#CharBMP") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                char_bmp[char_bmp_index] =
                                    SkinLoaderStub::get_texture(&path, usecim);
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharBMP2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                char_bmp[char_bmp_index + 1] =
                                    SkinLoaderStub::get_texture(&path, usecim);
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharTex") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                char_bmp[char_tex_index] =
                                    SkinLoaderStub::get_texture(&path, usecim);
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharTex2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                char_bmp[char_tex_index + 1] =
                                    SkinLoaderStub::get_texture(&path, usecim);
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharFace") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                char_bmp[char_face_index] =
                                    SkinLoaderStub::get_texture(&path, usecim);
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharFace2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                char_bmp[char_face_index + 1] =
                                    SkinLoaderStub::get_texture(&path, usecim);
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#SelectCG") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                char_bmp[select_cg_index] =
                                    SkinLoaderStub::get_texture(&path, usecim);
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#SelectCG2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                char_bmp[select_cg_index + 1] =
                                    SkinLoaderStub::get_texture(&path, usecim);
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#Patern")
                            || str_parts[0].eq_ignore_ascii_case("#Pattern")
                        {
                            pattern_data[pattern_idx].push(line.clone());
                        } else if str_parts[0].eq_ignore_ascii_case("#Texture") {
                            pattern_data[texture_idx].push(line.clone());
                        } else if str_parts[0].eq_ignore_ascii_case("#Layer") {
                            pattern_data[layer_idx].push(line.clone());
                        } else if str_parts[0].eq_ignore_ascii_case("#Flame")
                            || str_parts[0].eq_ignore_ascii_case("#Frame")
                        {
                            if data.len() > 2 {
                                let idx = pm_parse_int(&data[1]);
                                if idx >= 0 && (idx as usize) < frame.len() {
                                    frame[idx as usize] = pm_parse_int(&data[2]);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#Anime") {
                            if data.len() > 1 {
                                anime = pm_parse_int(&data[1]);
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#Size") {
                            if data.len() > 2 {
                                size[0] = pm_parse_int(&data[1]);
                                size[1] = pm_parse_int(&data[2]);
                            }
                        } else if str_parts[0].len() == 3 {
                            let substr = &str_parts[0][1..3];
                            let parsed = pm_parse_int_radix(substr, 36);
                            if parsed >= 0
                                && (parsed as usize) < xywh.len()
                                && data.len() > xywh[0].len()
                            {
                                for i in 0..xywh[0].len() {
                                    xywh[parsed as usize][i] = pm_parse_int(&data[i + 1]);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharFaceUpperSize") {
                            if data.len() > char_face_upper_xywh.len() {
                                for i in 0..char_face_upper_xywh.len() {
                                    char_face_upper_xywh[i] = pm_parse_int(&data[i + 1]);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharFaceAllSize") {
                            if data.len() > char_face_all_xywh.len() {
                                for i in 0..char_face_all_xywh.len() {
                                    char_face_all_xywh[i] = pm_parse_int(&data[i + 1]);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#Loop") && data.len() > 2 {
                            let idx = pm_parse_int(&data[1]);
                            if idx >= 0 && (idx as usize) < loop_val.len() {
                                loop_val[idx as usize] = pm_parse_int(&data[2]);
                            }
                        }
                    }
                }
            }
        }

        // If #CharBMP is absent, return null
        char_bmp[char_bmp_index].as_ref()?;

        // Check 2P color availability
        if color == 2
            && char_bmp[char_bmp_index + 1].is_some()
            && (pattern_data[texture_idx].is_empty()
                || (!pattern_data[texture_idx].is_empty()
                    && char_bmp[char_tex_index + 1].is_some()))
        {
            set_color = 2;
        }

        // If #Texture definition exists but #CharTex is absent, return null
        if set_color == 1
            && !pattern_data[texture_idx].is_empty()
            && char_bmp[char_tex_index].is_none()
        {
            return None;
        }

        let mut set_motion = i32::MIN;

        match load_type {
            BACKGROUND => {
                let set_index = char_bmp_index + set_color as usize - 1;
                char_bmp[set_index] = transparent_processing(
                    char_bmp[set_index].take(),
                    set_index,
                    &mut transparent_flag,
                );
                let set_bmp = char_bmp[set_index].as_ref()?;
                let region = TextureRegion::from_texture_region(
                    set_bmp.clone(),
                    xywh[1][0],
                    xywh[1][1],
                    xywh[1][2],
                    xywh[1][3],
                );
                let pm_chara_part = SkinImage::new_with_int_timer(vec![region], 0, 0);
                self.skin.add(pm_chara_part);
                return None; // Java returns PMcharaPart, but skin.add already stores it
            }
            NAME => {
                let set_index = char_bmp_index + set_color as usize - 1;
                char_bmp[set_index] = transparent_processing(
                    char_bmp[set_index].take(),
                    set_index,
                    &mut transparent_flag,
                );
                let set_bmp = char_bmp[set_index].as_ref()?;
                let region = TextureRegion::from_texture_region(
                    set_bmp.clone(),
                    xywh[0][0],
                    xywh[0][1],
                    xywh[0][2],
                    xywh[0][3],
                );
                let pm_chara_part = SkinImage::new_with_int_timer(vec![region], 0, 0);
                self.skin.add(pm_chara_part);
                return None;
            }
            FACE_UPPER => {
                let set_index = if set_color == 2 && char_bmp[char_face_index + 1].is_some() {
                    char_face_index + 1
                } else {
                    char_face_index
                };
                char_bmp[set_index] = transparent_processing(
                    char_bmp[set_index].take(),
                    set_index,
                    &mut transparent_flag,
                );
                let set_bmp = char_bmp[set_index].as_ref()?;
                let region = TextureRegion::from_texture_region(
                    set_bmp.clone(),
                    char_face_upper_xywh[0],
                    char_face_upper_xywh[1],
                    char_face_upper_xywh[2],
                    char_face_upper_xywh[3],
                );
                let pm_chara_part = SkinImage::new_with_int_timer(vec![region], 0, 0);
                self.skin.add(pm_chara_part);
                return None;
            }
            FACE_ALL => {
                let set_index = if set_color == 2 && char_bmp[char_face_index + 1].is_some() {
                    char_face_index + 1
                } else {
                    char_face_index
                };
                char_bmp[set_index] = transparent_processing(
                    char_bmp[set_index].take(),
                    set_index,
                    &mut transparent_flag,
                );
                let set_bmp = char_bmp[set_index].as_ref()?;
                let region = TextureRegion::from_texture_region(
                    set_bmp.clone(),
                    char_face_all_xywh[0],
                    char_face_all_xywh[1],
                    char_face_all_xywh[2],
                    char_face_all_xywh[3],
                );
                let pm_chara_part = SkinImage::new_with_int_timer(vec![region], 0, 0);
                self.skin.add(pm_chara_part);
                return None;
            }
            SELECT_CG => {
                let set_bmp = if set_color == 2 && char_bmp[select_cg_index + 1].is_some() {
                    char_bmp[select_cg_index + 1].as_ref()?
                } else {
                    char_bmp[select_cg_index].as_ref()?
                };
                let w = set_bmp.get_width();
                let h = set_bmp.get_height();
                let region = TextureRegion::from_texture_region(set_bmp.clone(), 0, 0, w, h);
                let pm_chara_part = SkinImage::new_with_int_timer(vec![region], 0, 0);
                self.skin.add(pm_chara_part);
                return None;
            }
            NEUTRAL => {
                if set_motion == i32::MIN {
                    set_motion = 1;
                }
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    set_motion,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            FEVER => {
                set_motion = 6;
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    set_motion,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            GREAT => {
                set_motion = 7;
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    set_motion,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            GOOD => {
                set_motion = 8;
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    set_motion,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            BAD => {
                set_motion = 10;
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    set_motion,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            FEVERWIN => {
                set_motion = 17;
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    set_motion,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            WIN => {
                set_motion = 15;
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    set_motion,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            LOSE => {
                set_motion = 16;
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    set_motion,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            OJAMA => {
                set_motion = 3;
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    set_motion,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            DANCE => {
                set_motion = 14;
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    set_motion,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            PLAY => {
                // PLAY type: setMotion stays i32::MIN
                self.load_play_type(
                    usecim,
                    &chp,
                    &mut char_bmp,
                    &mut transparent_flag,
                    &xywh,
                    &mut frame,
                    anime,
                    &size,
                    &mut loop_val,
                    set_color,
                    increase_rate_threshold,
                    &pattern_data,
                    char_bmp_index,
                    char_tex_index,
                    i32::MIN,
                    dsttimer,
                    dst_op1,
                    dst_op2,
                    dst_op3,
                    dst_offset,
                    side,
                    dstx,
                    dsty,
                    dstw,
                    dsth,
                );
                return None;
            }
            _ => {}
        }

        None
    }

    #[allow(clippy::too_many_arguments)]
    fn load_play_type(
        &mut self,
        _usecim: bool,
        _chp: &str,
        char_bmp: &mut [Option<Texture>; 8],
        transparent_flag: &mut [bool; 8],
        xywh: &[[i32; 4]],
        frame: &mut [i32; 20],
        anime: i32,
        size: &[i32; 2],
        loop_val: &mut [i32; 20],
        set_color: i32,
        increase_rate_threshold: i32,
        pattern_data: &[Vec<String>],
        char_bmp_index: usize,
        char_tex_index: usize,
        set_motion: i32,
        dsttimer: i32,
        dst_op1: i32,
        dst_op2: i32,
        dst_op3: i32,
        dst_offset: i32,
        side: i32,
        dstx: f32,
        dsty: f32,
        dstw: f32,
        dsth: f32,
    ) {
        // Initialize frame values
        for i in 0..frame.len() {
            if frame[i] == i32::MIN {
                frame[i] = anime;
            }
            if frame[i] < 1 {
                frame[i] = 100;
            }
        }

        // Dummy transparent 1x1 texture
        let pixmap = Pixmap::new(1, 1, PixmapFormat::RGBA8888);
        let transparent_tex = Texture::from_pixmap(&pixmap);

        // #Pattern, #Texture, #Layer render order
        let set_bmp_index = [char_bmp_index, char_tex_index, char_bmp_index];
        for pattern_index in 0..3 {
            for pattern_data_index in 0..pattern_data[pattern_index].len() {
                let str_parts: Vec<&str> = pattern_data[pattern_index][pattern_data_index]
                    .split('\t')
                    .collect();
                if str_parts.len() <= 1 {
                    continue;
                }
                let set_index = set_bmp_index[pattern_index] + set_color as usize - 1;
                char_bmp[set_index] =
                    transparent_processing(char_bmp[set_index].take(), set_index, transparent_flag);
                let set_bmp = match char_bmp[set_index].as_ref() {
                    Some(t) => t.clone(),
                    None => continue,
                };

                let mut motion = i32::MIN;
                let mut dst = [String::new(), String::new(), String::new(), String::new()];
                let data = pm_parse_str(&str_parts);
                if data.len() > 1 {
                    motion = pm_parse_int(&data[1]);
                }
                for i in 0..dst.len() {
                    if data.len() > i + 2 {
                        // replaceAll("[^0-9a-zA-Z-]", "")
                        dst[i] = data[i + 2]
                            .chars()
                            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                            .collect();
                    }
                }

                let mut timer = i32::MIN;
                let mut op = [0_i32; 3];
                if set_motion != i32::MIN && set_motion == motion {
                    timer = dsttimer;
                    op[0] = dst_op1;
                    op[1] = dst_op2;
                    op[2] = dst_op3;
                } else if set_motion == i32::MIN {
                    if side != 2 {
                        if motion == 1 {
                            timer = TIMER_PM_CHARA_1P_NEUTRAL;
                        } else if motion == 6 {
                            timer = TIMER_PM_CHARA_1P_FEVER;
                        } else if motion == 7 {
                            timer = TIMER_PM_CHARA_1P_GREAT;
                        } else if motion == 8 {
                            timer = TIMER_PM_CHARA_1P_GOOD;
                        } else if motion == 10 {
                            timer = TIMER_PM_CHARA_1P_BAD;
                        } else if (15..=17).contains(&motion) {
                            timer = TIMER_MUSIC_END;
                            if motion == 15 {
                                // WIN
                                op[0] = OPTION_1P_BORDER_OR_MORE;
                                op[1] = -OPTION_1P_100;
                            } else if motion == 16 {
                                // LOSE
                                op[0] = -OPTION_1P_BORDER_OR_MORE;
                            } else if motion == 17 {
                                // FEVERWIN
                                op[0] = OPTION_1P_100;
                            }
                        }
                    } else if motion == 1 {
                        timer = TIMER_PM_CHARA_2P_NEUTRAL;
                    } else if motion == 7 {
                        timer = TIMER_PM_CHARA_2P_GREAT;
                    } else if motion == 10 {
                        timer = TIMER_PM_CHARA_2P_BAD;
                    } else if motion == 15 || motion == 16 {
                        timer = TIMER_MUSIC_END;
                        if motion == 15 {
                            // WIN (2P side: reversed)
                            op[0] = -OPTION_1P_BORDER_OR_MORE;
                        } else if motion == 16 {
                            // LOSE (2P side: reversed)
                            op[0] = OPTION_1P_BORDER_OR_MORE;
                        }
                    }
                }

                if timer != i32::MIN
                    && !dst[0].is_empty()
                    && dst[0].len().is_multiple_of(2)
                    && (dst[1].is_empty() || dst[1].len() == dst[0].len())
                    && (dst[2].is_empty() || dst[2].len() == dst[0].len())
                    && (dst[3].is_empty() || dst[3].len() == dst[0].len())
                {
                    // Clamp loop values
                    if loop_val[motion as usize] >= (dst[0].len() / 2 - 1) as i32 {
                        loop_val[motion as usize] = (dst[0].len() / 2 - 2) as i32;
                    } else if loop_val[motion as usize] < -1 {
                        loop_val[motion as usize] = -1;
                    }

                    let cycle = frame[motion as usize] * (dst[0].len() / 2) as i32;
                    let loop_time = frame[motion as usize] * (loop_val[motion as usize] + 1);

                    if set_motion == i32::MIN
                        && (TIMER_PM_CHARA_1P_NEUTRAL..TIMER_MUSIC_END).contains(&timer)
                    {
                        self.skin
                            .pomyu
                            .set_pm_chara_time(timer - TIMER_PM_CHARA_1P_NEUTRAL, cycle);
                    }

                    // Check for hyphen interpolation flag
                    let mut hyphen_flag = false;
                    for i in 1..dst.len() {
                        if dst[i].contains('-') {
                            hyphen_flag = true;
                            break;
                        }
                    }

                    // Frame interpolation when hyphen exists, 60FPS 17ms threshold
                    let mut increase_rate = 1;
                    if hyphen_flag && frame[motion as usize] >= increase_rate_threshold {
                        for i in 1..=frame[motion as usize] {
                            if frame[motion as usize] / i < increase_rate_threshold
                                && frame[motion as usize] % i == 0
                            {
                                increase_rate = i;
                                break;
                            }
                        }
                        // Expand dst[1..] by increase_rate
                        for i in 1..dst.len() {
                            let mut chars = Vec::new();
                            let bytes = dst[i].as_bytes();
                            let mut j = 0;
                            while j + 1 < bytes.len() {
                                for _k in 0..increase_rate {
                                    chars.push(bytes[j] as char);
                                    chars.push(bytes[j + 1] as char);
                                }
                                j += 2;
                            }
                            dst[i] = chars.into_iter().collect();
                        }
                    }

                    // DST loading
                    let frame_time = frame[motion as usize] as f64 / increase_rate as f64;
                    let loop_frame = loop_val[motion as usize] * increase_rate;
                    let dstxywh_len = if !dst[1].is_empty() {
                        dst[1].len() / 2
                    } else {
                        dst[0].len() / 2
                    };
                    let mut dstxywh = vec![[0, 0, size[0], size[1]]; dstxywh_len];

                    // Parse dst[1] position data with interpolation
                    let mut start_xywh = [0, 0, size[0], size[1]];
                    let mut end_xywh = [0, 0, size[0], size[1]];
                    {
                        let mut i = 0;
                        while i < dst[1].len() {
                            if i + 2 <= dst[1].len() {
                                if &dst[1][i..i + 2] == "--" {
                                    let mut count = 0;
                                    let mut j = i;
                                    while j < dst[1].len()
                                        && j + 2 <= dst[1].len()
                                        && &dst[1][j..j + 2] == "--"
                                    {
                                        count += 1;
                                        j += 2;
                                    }
                                    // Read end value
                                    if i + count * 2 + 2 <= dst[1].len() {
                                        let end_str = &dst[1][i + count * 2..i + count * 2 + 2];
                                        let parsed = pm_parse_int_radix(end_str, 36);
                                        if parsed >= 0 && (parsed as usize) < xywh.len() {
                                            end_xywh = xywh[parsed as usize];
                                        }
                                    }
                                    // Interpolate
                                    j = i;
                                    while j < dst[1].len()
                                        && j + 2 <= dst[1].len()
                                        && &dst[1][j..j + 2] == "--"
                                    {
                                        for k in 0..4 {
                                            dstxywh[j / 2][k] = start_xywh[k]
                                                + (end_xywh[k] - start_xywh[k])
                                                    * (((j - i) / 2 + 1) as i32)
                                                    / (count as i32 + 1);
                                        }
                                        j += 2;
                                    }
                                    i += (count - 1) * 2;
                                } else {
                                    let substr = &dst[1][i..i + 2];
                                    let parsed = pm_parse_int_radix(substr, 36);
                                    if parsed >= 0 && (parsed as usize) < xywh.len() {
                                        start_xywh = xywh[parsed as usize];
                                        dstxywh[i / 2] = start_xywh;
                                    }
                                }
                            }
                            i += 2;
                        }
                    }

                    // Alpha and angle loading
                    let mut alpha_angle = vec![[255_i32, 0_i32]; dstxywh_len];
                    for index in 2..dst.len() {
                        let mut start_value = 0;
                        let mut end_value;
                        let mut i = 0;
                        while i < dst[index].len() {
                            if i + 2 <= dst[index].len() {
                                if &dst[index][i..i + 2] == "--" {
                                    let mut count = 0;
                                    let mut j = i;
                                    while j < dst[index].len()
                                        && j + 2 <= dst[index].len()
                                        && &dst[index][j..j + 2] == "--"
                                    {
                                        count += 1;
                                        j += 2;
                                    }
                                    end_value = 0;
                                    if i + count * 2 + 2 <= dst[index].len() {
                                        let end_str = &dst[index][i + count * 2..i + count * 2 + 2];
                                        let parsed = pm_parse_int_radix(end_str, 16);
                                        if (0..=255).contains(&parsed) {
                                            end_value = parsed;
                                            if index == 3 {
                                                end_value = (end_value as f32 * 360.0 / 256.0)
                                                    .round()
                                                    as i32;
                                            }
                                        }
                                    }
                                    j = i;
                                    while j < dst[index].len()
                                        && j + 2 <= dst[index].len()
                                        && &dst[index][j..j + 2] == "--"
                                    {
                                        alpha_angle[j / 2][index - 2] = start_value
                                            + (end_value - start_value)
                                                * (((j - i) / 2 + 1) as i32)
                                                / (count as i32 + 1);
                                        j += 2;
                                    }
                                    i += (count - 1) * 2;
                                } else {
                                    let substr = &dst[index][i..i + 2];
                                    let parsed = pm_parse_int_radix(substr, 16);
                                    if (0..=255).contains(&parsed) {
                                        start_value = parsed;
                                        if index == 3 {
                                            start_value =
                                                (start_value as f32 * 360.0 / 256.0).round() as i32;
                                        }
                                        alpha_angle[i / 2][index - 2] = start_value;
                                    }
                                }
                            }
                            i += 2;
                        }
                    }

                    // Guard against size[0] or size[1] being zero (division)
                    if size[0] == 0 || size[1] == 0 {
                        continue;
                    }

                    // Pre-loop frames (up to loop start)
                    if (loop_frame + increase_rate) != 0 {
                        let mut images =
                            Vec::with_capacity((loop_val[motion as usize] + 1) as usize);
                        let mut i = 0;
                        while i < (loop_val[motion as usize] + 1) * 2 {
                            if i + 2 <= dst[0].len() as i32 {
                                let idx =
                                    pm_parse_int_radix(&dst[0][i as usize..(i + 2) as usize], 36);
                                if idx >= 0
                                    && (idx as usize) < xywh.len()
                                    && xywh[idx as usize][2] > 0
                                    && xywh[idx as usize][3] > 0
                                {
                                    images.push(TextureRegion::from_texture_region(
                                        set_bmp.clone(),
                                        xywh[idx as usize][0],
                                        xywh[idx as usize][1],
                                        xywh[idx as usize][2],
                                        xywh[idx as usize][3],
                                    ));
                                } else {
                                    images.push(TextureRegion::from_texture_region(
                                        transparent_tex.clone(),
                                        0,
                                        0,
                                        1,
                                        1,
                                    ));
                                }
                            }
                            i += 2;
                        }

                        let mut part = SkinImage::new_with_int_timer(images, timer, loop_time);

                        for i in 0..(loop_frame + increase_rate) as usize {
                            part.data.set_destination_with_int_timer_and_single_offset(
                                (frame_time * i as f64) as i64,
                                dstx + dstxywh[i][0] as f32 * dstw / size[0] as f32,
                                dsty + dsth
                                    - (dstxywh[i][1] + dstxywh[i][3]) as f32 * dsth
                                        / size[1] as f32,
                                dstxywh[i][2] as f32 * dstw / size[0] as f32,
                                dstxywh[i][3] as f32 * dsth / size[1] as f32,
                                3,
                                alpha_angle[i][0],
                                255,
                                255,
                                255,
                                1,
                                0,
                                alpha_angle[i][1],
                                0,
                                -1,
                                timer,
                                op[0],
                                op[1],
                                op[2],
                                0,
                            );
                        }
                        let last_pre = (loop_frame + increase_rate - 1) as usize;
                        part.data.set_destination_with_int_timer_and_single_offset(
                            (loop_time - 1) as i64,
                            dstx + dstxywh[last_pre][0] as f32 * dstw / size[0] as f32,
                            dsty + dsth
                                - (dstxywh[last_pre][1] + dstxywh[last_pre][3]) as f32 * dsth
                                    / size[1] as f32,
                            dstxywh[last_pre][2] as f32 * dstw / size[0] as f32,
                            dstxywh[last_pre][3] as f32 * dsth / size[1] as f32,
                            3,
                            alpha_angle[last_pre][0],
                            255,
                            255,
                            255,
                            1,
                            0,
                            alpha_angle[last_pre][1],
                            0,
                            -1,
                            timer,
                            op[0],
                            op[1],
                            op[2],
                            dst_offset,
                        );
                        self.skin.add(part);
                    }

                    // Loop frames (from loop start to end)
                    let loop_start = (loop_val[motion as usize] + 1) as usize;
                    let total_frames = dst[0].len() / 2;
                    let loop_image_count = total_frames - loop_start;
                    let mut images = Vec::with_capacity(loop_image_count);
                    let mut i = loop_start * 2;
                    while i < dst[0].len() {
                        if i + 2 <= dst[0].len() {
                            let idx = pm_parse_int_radix(&dst[0][i..i + 2], 36);
                            if idx >= 0
                                && (idx as usize) < xywh.len()
                                && xywh[idx as usize][2] > 0
                                && xywh[idx as usize][3] > 0
                            {
                                images.push(TextureRegion::from_texture_region(
                                    set_bmp.clone(),
                                    xywh[idx as usize][0],
                                    xywh[idx as usize][1],
                                    xywh[idx as usize][2],
                                    xywh[idx as usize][3],
                                ));
                            } else {
                                images.push(TextureRegion::from_texture_region(
                                    transparent_tex.clone(),
                                    0,
                                    0,
                                    1,
                                    1,
                                ));
                            }
                        }
                        i += 2;
                    }

                    let mut part = SkinImage::new_with_int_timer(images, timer, cycle - loop_time);

                    for i in (loop_frame + increase_rate) as usize..dstxywh.len() {
                        part.data.set_destination_with_int_timer_and_single_offset(
                            (frame_time * i as f64) as i64,
                            dstx + dstxywh[i][0] as f32 * dstw / size[0] as f32,
                            dsty + dsth
                                - (dstxywh[i][1] + dstxywh[i][3]) as f32 * dsth / size[1] as f32,
                            dstxywh[i][2] as f32 * dstw / size[0] as f32,
                            dstxywh[i][3] as f32 * dsth / size[1] as f32,
                            3,
                            alpha_angle[i][0],
                            255,
                            255,
                            255,
                            1,
                            0,
                            alpha_angle[i][1],
                            0,
                            loop_time,
                            timer,
                            op[0],
                            op[1],
                            op[2],
                            0,
                        );
                    }
                    let last = dstxywh.len() - 1;
                    part.data.set_destination_with_int_timer_and_single_offset(
                        cycle as i64,
                        dstx + dstxywh[last][0] as f32 * dstw / size[0] as f32,
                        dsty + dsth
                            - (dstxywh[last][1] + dstxywh[last][3]) as f32 * dsth / size[1] as f32,
                        dstxywh[last][2] as f32 * dstw / size[0] as f32,
                        dstxywh[last][3] as f32 * dsth / size[1] as f32,
                        3,
                        alpha_angle[last][0],
                        255,
                        255,
                        255,
                        1,
                        0,
                        alpha_angle[last][1],
                        0,
                        loop_time,
                        timer,
                        op[0],
                        op[1],
                        op[2],
                        dst_offset,
                    );
                    self.skin.add(part);
                }
            }
        }
    }
}

fn pm_parse_int(s: &str) -> i32 {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    cleaned.parse::<i32>().unwrap_or(0)
}

fn pm_parse_int_radix(s: &str, radix: i32) -> i32 {
    if radix == 36 {
        if s.len() < 2 {
            return -1;
        }
        let mut result = 0_i32;
        let c1 = s.as_bytes()[0] as char;
        if c1.is_ascii_digit() {
            result = ((c1 as i32) - ('0' as i32)) * 36;
        } else if c1.is_ascii_lowercase() {
            result = (((c1 as i32) - ('a' as i32)) + 10) * 36;
        } else if c1.is_ascii_uppercase() {
            result = (((c1 as i32) - ('A' as i32)) + 10) * 36;
        }
        let c2 = s.as_bytes()[1] as char;
        if c2.is_ascii_digit() {
            result += (c2 as i32) - ('0' as i32);
        } else if c2.is_ascii_lowercase() {
            result += ((c2 as i32) - ('a' as i32)) + 10;
        } else if c2.is_ascii_uppercase() {
            result += ((c2 as i32) - ('A' as i32)) + 10;
        }
        return result;
    }
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_hexdigit() || *c == '-')
        .collect();
    i32::from_str_radix(&cleaned, radix as u32).unwrap_or(-1)
}

fn pm_parse_str(s: &[&str]) -> Vec<String> {
    let mut list = Vec::new();
    for i in 0..s.len() {
        if !s[i].is_empty() {
            if s[i].starts_with('/') {
                break;
            } else if let Some(pos) = s[i].find("//") {
                list.push(s[i][..pos].to_string());
                break;
            } else {
                list.push(s[i].to_string());
            }
        }
    }
    list
}

fn transparent_processing(
    tex: Option<Texture>,
    index: usize,
    flag: &mut [bool; 8],
) -> Option<Texture> {
    // Transparent processing: bottom-right 1 pixel is transparent color
    // SelectCG icons are not made transparent
    let tex = tex?;
    if flag[index] {
        return Some(tex);
    }

    let w = tex.get_width();
    let h = tex.get_height();
    if w <= 0 || h <= 0 {
        flag[index] = true;
        return Some(tex);
    }

    // Access pixel data from rgba_data
    let rgba_data = match tex.rgba_data.as_ref() {
        Some(data) => data,
        None => {
            flag[index] = true;
            return Some(tex);
        }
    };

    // Get transparent color from bottom-right pixel
    let br_idx = ((h as usize - 1) * w as usize + (w as usize - 1)) * 4;
    if br_idx + 3 >= rgba_data.len() {
        flag[index] = true;
        return Some(tex);
    }
    let tr = rgba_data[br_idx];
    let tg = rgba_data[br_idx + 1];
    let tb = rgba_data[br_idx + 2];
    let ta = rgba_data[br_idx + 3];

    // Create new pixmap with transparent color removed
    let mut new_data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h as usize {
        for x in 0..w as usize {
            let idx = (y * w as usize + x) * 4;
            if idx + 3 < rgba_data.len() {
                let pr = rgba_data[idx];
                let pg = rgba_data[idx + 1];
                let pb = rgba_data[idx + 2];
                let pa = rgba_data[idx + 3];
                if pr != tr || pg != tg || pb != tb || pa != ta {
                    new_data[idx] = pr;
                    new_data[idx + 1] = pg;
                    new_data[idx + 2] = pb;
                    new_data[idx + 3] = pa;
                }
                // else: leave as 0,0,0,0 (transparent)
            }
        }
    }

    flag[index] = true;

    Some(Texture {
        width: w,
        height: h,
        disposed: false,
        path: tex.path.clone(),
        rgba_data: Some(Arc::new(new_data)),
        gpu_texture: None,
        gpu_view: None,
        sampler: None,
    })
}
