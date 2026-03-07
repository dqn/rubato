// PomyuCharaLoader.java -> pomyu_chara_loader.rs
// Mechanical line-by-line translation.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;

use crate::objects::skin_image::SkinImage;
use crate::property::timer_property::TimerProperty;
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

    #[allow(clippy::too_many_arguments)]
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
                                if let Some(slot) = char_bmp.get_mut(char_bmp_index) {
                                    *slot = SkinLoaderStub::texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharBMP2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(char_bmp_index + 1) {
                                    *slot = SkinLoaderStub::texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharTex") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(char_tex_index) {
                                    *slot = SkinLoaderStub::texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharTex2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(char_tex_index + 1) {
                                    *slot = SkinLoaderStub::texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharFace") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(char_face_index) {
                                    *slot = SkinLoaderStub::texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharFace2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(char_face_index + 1) {
                                    *slot = SkinLoaderStub::texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#SelectCG") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(select_cg_index) {
                                    *slot = SkinLoaderStub::texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#SelectCG2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(select_cg_index + 1) {
                                    *slot = SkinLoaderStub::texture(&path, usecim);
                                }
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
        char_bmp.get(char_bmp_index)?.as_ref()?;

        // Check 2P color availability
        if color == 2
            && char_bmp
                .get(char_bmp_index + 1)
                .and_then(|t| t.as_ref())
                .is_some()
            && (pattern_data[texture_idx].is_empty()
                || (!pattern_data[texture_idx].is_empty()
                    && char_bmp
                        .get(char_tex_index + 1)
                        .and_then(|t| t.as_ref())
                        .is_some()))
        {
            set_color = 2;
        }

        // If #Texture definition exists but #CharTex is absent, return null
        if set_color == 1
            && !pattern_data[texture_idx].is_empty()
            && char_bmp
                .get(char_tex_index)
                .and_then(|t| t.as_ref())
                .is_none()
        {
            return None;
        }

        let mut set_motion = i32::MIN;

        match load_type {
            BACKGROUND => {
                if set_color < 1 {
                    return None;
                }
                let set_index = char_bmp_index + set_color as usize - 1;
                if set_index >= char_bmp.len() {
                    return None;
                }
                let taken = char_bmp.get_mut(set_index).and_then(|s| s.take());
                if let Some(slot) = char_bmp.get_mut(set_index) {
                    *slot = transparent_processing(taken, set_index, &mut transparent_flag);
                }
                let set_bmp = char_bmp.get(set_index)?.as_ref()?;
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
                if set_color < 1 {
                    return None;
                }
                let set_index = char_bmp_index + set_color as usize - 1;
                if set_index >= char_bmp.len() {
                    return None;
                }
                let taken = char_bmp.get_mut(set_index).and_then(|s| s.take());
                if let Some(slot) = char_bmp.get_mut(set_index) {
                    *slot = transparent_processing(taken, set_index, &mut transparent_flag);
                }
                let set_bmp = char_bmp.get(set_index)?.as_ref()?;
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
                let set_index = if set_color == 2
                    && char_bmp
                        .get(char_face_index + 1)
                        .and_then(|t| t.as_ref())
                        .is_some()
                {
                    char_face_index + 1
                } else {
                    char_face_index
                };
                if set_index >= char_bmp.len() {
                    return None;
                }
                let taken = char_bmp.get_mut(set_index).and_then(|s| s.take());
                if let Some(slot) = char_bmp.get_mut(set_index) {
                    *slot = transparent_processing(taken, set_index, &mut transparent_flag);
                }
                let set_bmp = char_bmp.get(set_index)?.as_ref()?;
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
                let set_index = if set_color == 2
                    && char_bmp
                        .get(char_face_index + 1)
                        .and_then(|t| t.as_ref())
                        .is_some()
                {
                    char_face_index + 1
                } else {
                    char_face_index
                };
                if set_index >= char_bmp.len() {
                    return None;
                }
                let taken = char_bmp.get_mut(set_index).and_then(|s| s.take());
                if let Some(slot) = char_bmp.get_mut(set_index) {
                    *slot = transparent_processing(taken, set_index, &mut transparent_flag);
                }
                let set_bmp = char_bmp.get(set_index)?.as_ref()?;
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
                let set_bmp = if set_color == 2
                    && char_bmp
                        .get(select_cg_index + 1)
                        .and_then(|t| t.as_ref())
                        .is_some()
                {
                    char_bmp.get(select_cg_index + 1)?.as_ref()?
                } else {
                    char_bmp.get(select_cg_index)?.as_ref()?
                };
                let w = set_bmp.width;
                let h = set_bmp.height;
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
}

include!("load_play_type.rs");
include!("helpers.rs");

#[cfg(test)]
mod tests;
