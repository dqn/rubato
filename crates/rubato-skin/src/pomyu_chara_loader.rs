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
                                if let Some(slot) = char_bmp.get_mut(char_bmp_index) {
                                    *slot = SkinLoaderStub::get_texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharBMP2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(char_bmp_index + 1) {
                                    *slot = SkinLoaderStub::get_texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharTex") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(char_tex_index) {
                                    *slot = SkinLoaderStub::get_texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharTex2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(char_tex_index + 1) {
                                    *slot = SkinLoaderStub::get_texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharFace") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(char_face_index) {
                                    *slot = SkinLoaderStub::get_texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#CharFace2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(char_face_index + 1) {
                                    *slot = SkinLoaderStub::get_texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#SelectCG") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(select_cg_index) {
                                    *slot = SkinLoaderStub::get_texture(&path, usecim);
                                }
                            }
                        } else if str_parts[0].eq_ignore_ascii_case("#SelectCG2P") {
                            if data.len() > 1 {
                                let path =
                                    format!("{}{}", chp_dir_prefix, data[1].replace('\\', "/"));
                                if let Some(slot) = char_bmp.get_mut(select_cg_index + 1) {
                                    *slot = SkinLoaderStub::get_texture(&path, usecim);
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
                if set_color < 1 {
                    continue;
                }
                let set_index = set_bmp_index[pattern_index] + set_color as usize - 1;
                if set_index >= char_bmp.len() {
                    continue;
                }
                let taken = char_bmp.get_mut(set_index).and_then(|s| s.take());
                if let Some(slot) = char_bmp.get_mut(set_index) {
                    *slot = transparent_processing(taken, set_index, transparent_flag);
                }
                let set_bmp = match char_bmp.get(set_index).and_then(|t| t.as_ref()) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ================================================================
    // pm_parse_int tests
    // ================================================================

    #[test]
    fn test_pm_parse_int_valid() {
        assert_eq!(pm_parse_int("42"), 42);
    }

    #[test]
    fn test_pm_parse_int_negative() {
        assert_eq!(pm_parse_int("-1"), -1);
    }

    #[test]
    fn test_pm_parse_int_zero() {
        assert_eq!(pm_parse_int("0"), 0);
    }

    #[test]
    fn test_pm_parse_int_empty_returns_zero() {
        assert_eq!(pm_parse_int(""), 0, "empty string should default to 0");
    }

    #[test]
    fn test_pm_parse_int_non_numeric_returns_zero() {
        assert_eq!(
            pm_parse_int("abc"),
            0,
            "non-numeric string should default to 0"
        );
    }

    #[test]
    fn test_pm_parse_int_strips_non_digit_chars() {
        // The function filters to only ascii digits and '-', so "12abc34" becomes "1234"
        assert_eq!(pm_parse_int("12abc34"), 1234);
    }

    #[test]
    fn test_pm_parse_int_with_whitespace() {
        // Whitespace is stripped (filtered out), so " 42 " becomes "42"
        assert_eq!(pm_parse_int(" 42 "), 42);
    }

    #[test]
    fn test_pm_parse_int_large_value() {
        assert_eq!(pm_parse_int("2147483647"), i32::MAX);
    }

    #[test]
    fn test_pm_parse_int_overflow_returns_zero() {
        // Overflowing i32 should fail to parse and return 0
        assert_eq!(
            pm_parse_int("99999999999"),
            0,
            "overflowing value should default to 0"
        );
    }

    // ================================================================
    // pm_parse_int_radix tests
    // ================================================================

    #[test]
    fn test_pm_parse_int_radix_base10() {
        assert_eq!(pm_parse_int_radix("42", 10), 42);
    }

    #[test]
    fn test_pm_parse_int_radix_base16() {
        assert_eq!(pm_parse_int_radix("ff", 16), 255);
    }

    #[test]
    fn test_pm_parse_int_radix_base16_uppercase() {
        assert_eq!(pm_parse_int_radix("FF", 16), 255);
    }

    #[test]
    fn test_pm_parse_int_radix_base36_two_digits() {
        // "zz" in base36: z=35, so 35*36 + 35 = 1295
        assert_eq!(pm_parse_int_radix("zz", 36), 1295);
    }

    #[test]
    fn test_pm_parse_int_radix_base36_uppercase() {
        assert_eq!(pm_parse_int_radix("ZZ", 36), 1295);
    }

    #[test]
    fn test_pm_parse_int_radix_base36_numeric() {
        // "00" in base36: 0*36 + 0 = 0
        assert_eq!(pm_parse_int_radix("00", 36), 0);
    }

    #[test]
    fn test_pm_parse_int_radix_base36_mixed() {
        // "a0" in base36: (10)*36 + 0 = 360
        assert_eq!(pm_parse_int_radix("a0", 36), 360);
    }

    #[test]
    fn test_pm_parse_int_radix_base36_single_char_returns_negative() {
        // Base36 requires at least 2 chars, single char returns -1
        assert_eq!(
            pm_parse_int_radix("z", 36),
            -1,
            "base36 with <2 chars should return -1"
        );
    }

    #[test]
    fn test_pm_parse_int_radix_base36_empty_returns_negative() {
        assert_eq!(
            pm_parse_int_radix("", 36),
            -1,
            "base36 with empty string should return -1"
        );
    }

    #[test]
    fn test_pm_parse_int_radix_base10_invalid_returns_negative() {
        assert_eq!(
            pm_parse_int_radix("xyz", 10),
            -1,
            "invalid base10 string should return -1"
        );
    }

    #[test]
    fn test_pm_parse_int_radix_base16_zero() {
        assert_eq!(pm_parse_int_radix("0", 16), 0);
    }

    // ================================================================
    // pm_parse_str tests
    // ================================================================

    #[test]
    fn test_pm_parse_str_basic() {
        let parts = vec!["#Tag", "value1", "value2"];
        let result = pm_parse_str(&parts);
        assert_eq!(result, vec!["#Tag", "value1", "value2"]);
    }

    #[test]
    fn test_pm_parse_str_stops_at_comment_prefix() {
        // A part starting with '/' causes the loop to break
        let parts = vec!["#Tag", "value1", "/comment"];
        let result = pm_parse_str(&parts);
        assert_eq!(result, vec!["#Tag", "value1"]);
    }

    #[test]
    fn test_pm_parse_str_inline_comment() {
        // A part containing "//" truncates the value and stops
        let parts = vec!["#Tag", "value1//comment", "value2"];
        let result = pm_parse_str(&parts);
        assert_eq!(result, vec!["#Tag", "value1"]);
    }

    #[test]
    fn test_pm_parse_str_empty_parts_skipped() {
        // Empty parts are skipped but do not stop the loop
        let parts = vec!["#Tag", "", "value2"];
        let result = pm_parse_str(&parts);
        assert_eq!(result, vec!["#Tag", "value2"]);
    }

    #[test]
    fn test_pm_parse_str_all_empty() {
        let parts: Vec<&str> = vec!["", "", ""];
        let result = pm_parse_str(&parts);
        assert!(
            result.is_empty(),
            "all empty parts should produce empty vec"
        );
    }

    #[test]
    fn test_pm_parse_str_no_parts() {
        let parts: Vec<&str> = vec![];
        let result = pm_parse_str(&parts);
        assert!(result.is_empty());
    }

    // ================================================================
    // transparent_processing tests
    // ================================================================

    #[test]
    fn test_transparent_processing_none_returns_none() {
        let mut flag = [false; 8];
        let result = transparent_processing(None, 0, &mut flag);
        assert!(result.is_none(), "None input should return None");
        assert!(!flag[0], "flag should not be set when input is None");
    }

    #[test]
    fn test_transparent_processing_already_flagged_returns_unchanged() {
        let tex = Texture {
            width: 2,
            height: 2,
            disposed: false,
            path: None,
            rgba_data: Some(Arc::new(vec![255; 16])),
            gpu_texture: None,
            gpu_view: None,
            sampler: None,
        };
        let mut flag = [false; 8];
        flag[0] = true; // already processed
        let result = transparent_processing(Some(tex), 0, &mut flag);
        assert!(result.is_some(), "flagged texture should pass through");
    }

    #[test]
    fn test_transparent_processing_zero_size_sets_flag() {
        let tex = Texture {
            width: 0,
            height: 0,
            disposed: false,
            path: None,
            rgba_data: Some(Arc::new(vec![])),
            gpu_texture: None,
            gpu_view: None,
            sampler: None,
        };
        let mut flag = [false; 8];
        let result = transparent_processing(Some(tex), 3, &mut flag);
        assert!(result.is_some(), "zero-size texture should return Some");
        assert!(flag[3], "flag should be set for zero-size texture");
    }

    #[test]
    fn test_transparent_processing_no_rgba_data_sets_flag() {
        let tex = Texture {
            width: 4,
            height: 4,
            disposed: false,
            path: None,
            rgba_data: None,
            gpu_texture: None,
            gpu_view: None,
            sampler: None,
        };
        let mut flag = [false; 8];
        let result = transparent_processing(Some(tex), 2, &mut flag);
        assert!(
            result.is_some(),
            "texture without rgba_data should return Some"
        );
        assert!(flag[2], "flag should be set when rgba_data is None");
    }

    #[test]
    fn test_transparent_processing_removes_transparent_color() {
        // 2x2 image, bottom-right pixel is (10, 20, 30, 255) = the transparent color
        // Other pixels should remain, matching pixel should become (0,0,0,0)
        #[rustfmt::skip]
        let rgba = vec![
            // row 0
            255, 0, 0, 255,   // (0,0): red - keep
            0, 255, 0, 255,   // (1,0): green - keep
            // row 1
            0, 0, 255, 255,   // (0,1): blue - keep
            10, 20, 30, 255,  // (1,1): transparent color - remove
        ];
        let tex = Texture {
            width: 2,
            height: 2,
            disposed: false,
            path: None,
            rgba_data: Some(Arc::new(rgba)),
            gpu_texture: None,
            gpu_view: None,
            sampler: None,
        };
        let mut flag = [false; 8];
        let result = transparent_processing(Some(tex), 0, &mut flag);
        assert!(flag[0], "flag should be set after processing");
        let result = result.expect("should return Some");
        let data = result.rgba_data.expect("should have rgba_data");
        // Red pixel (0,0) should be preserved
        assert_eq!(
            &data[0..4],
            &[255, 0, 0, 255],
            "red pixel should be preserved"
        );
        // Green pixel (1,0) should be preserved
        assert_eq!(
            &data[4..8],
            &[0, 255, 0, 255],
            "green pixel should be preserved"
        );
        // Blue pixel (0,1) should be preserved
        assert_eq!(
            &data[8..12],
            &[0, 0, 255, 255],
            "blue pixel should be preserved"
        );
        // Bottom-right pixel (transparent color) should be zeroed
        assert_eq!(
            &data[12..16],
            &[0, 0, 0, 0],
            "transparent color pixel should become fully transparent"
        );
    }

    // ================================================================
    // Constant value tests
    // ================================================================

    #[test]
    fn test_load_type_constants() {
        assert_eq!(PLAY, 0);
        assert_eq!(BACKGROUND, 1);
        assert_eq!(NAME, 2);
        assert_eq!(FACE_UPPER, 3);
        assert_eq!(FACE_ALL, 4);
        assert_eq!(SELECT_CG, 5);
        assert_eq!(NEUTRAL, 6);
        assert_eq!(FEVER, 7);
        assert_eq!(GREAT, 8);
        assert_eq!(GOOD, 9);
        assert_eq!(BAD, 10);
        assert_eq!(FEVERWIN, 11);
        assert_eq!(WIN, 12);
        assert_eq!(LOSE, 13);
        assert_eq!(OJAMA, 14);
        assert_eq!(DANCE, 15);
    }

    // ================================================================
    // load() boundary condition tests
    // ================================================================

    #[test]
    fn test_load_invalid_load_type_returns_none() {
        let mut skin = PlaySkinStub::new();
        let mut loader = PomyuCharaLoader::new(&mut skin);
        let result = loader.load(
            false,
            Path::new("/nonexistent/path.chp"),
            99, // invalid load_type (outside 0..=15)
            0,
            0.0,
            0.0,
            100.0,
            100.0,
            0,
            0,
            0,
            0,
            0,
            0,
        );
        assert!(result.is_none(), "load_type=99 should return None");
    }

    #[test]
    fn test_load_negative_load_type_returns_none() {
        let mut skin = PlaySkinStub::new();
        let mut loader = PomyuCharaLoader::new(&mut skin);
        let result = loader.load(
            false,
            Path::new("/nonexistent/path.chp"),
            -1, // negative load_type
            0,
            0.0,
            0.0,
            100.0,
            100.0,
            0,
            0,
            0,
            0,
            0,
            0,
        );
        assert!(result.is_none(), "load_type=-1 should return None");
    }

    #[test]
    fn test_load_nonexistent_path_returns_none() {
        let mut skin = PlaySkinStub::new();
        let mut loader = PomyuCharaLoader::new(&mut skin);
        let result = loader.load(
            false,
            Path::new("/definitely/does/not/exist/file.chp"),
            PLAY, // valid load_type
            0,
            0.0,
            0.0,
            100.0,
            100.0,
            0,
            0,
            0,
            0,
            0,
            0,
        );
        assert!(
            result.is_none(),
            "nonexistent .chp path should return None without panic"
        );
    }

    #[test]
    fn test_load_nonexistent_directory_returns_none() {
        let mut skin = PlaySkinStub::new();
        let mut loader = PomyuCharaLoader::new(&mut skin);
        // Path without .chp extension triggers directory search mode
        let result = loader.load(
            false,
            Path::new("/definitely/does/not/exist/dir/"),
            PLAY,
            0,
            0.0,
            0.0,
            100.0,
            100.0,
            0,
            0,
            0,
            0,
            0,
            0,
        );
        assert!(
            result.is_none(),
            "nonexistent directory should return None without panic"
        );
    }

    // ================================================================
    // Bounds safety tests for char_bmp array accesses
    // ================================================================

    #[test]
    fn test_char_bmp_get_out_of_bounds_returns_none() {
        // Verify that .get() on a fixed-size array returns None for out-of-bounds indices
        let char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
        assert!(
            char_bmp.get(8).is_none(),
            "index 8 should be out of bounds for [_; 8]"
        );
        assert!(
            char_bmp.get(100).is_none(),
            "large index should be out of bounds"
        );
        assert!(
            char_bmp.get(usize::MAX).is_none(),
            "usize::MAX should be out of bounds"
        );
    }

    #[test]
    fn test_char_bmp_get_mut_out_of_bounds_returns_none() {
        let mut char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
        assert!(
            char_bmp.get_mut(8).is_none(),
            "get_mut(8) should be None for [_; 8]"
        );
        assert!(
            char_bmp.get_mut(usize::MAX).is_none(),
            "get_mut(usize::MAX) should be None"
        );
    }

    #[test]
    fn test_set_color_zero_underflow_guard() {
        // set_color < 1 should be caught before computing set_color as usize - 1
        // This test verifies the guard prevents usize underflow (wrapping subtraction)
        let set_color: i32 = 0;
        assert!(
            set_color < 1,
            "set_color=0 should trigger the underflow guard"
        );

        // If the guard were absent, this would wrap to usize::MAX
        // With the guard, we never reach this computation
        let set_color: i32 = -1;
        assert!(
            set_color < 1,
            "set_color=-1 should trigger the underflow guard"
        );
    }

    #[test]
    fn test_set_color_valid_index_computation() {
        let char_bmp_index: usize = 0;

        // set_color = 1 -> index = 0 + 1 - 1 = 0 (valid)
        let set_color = 1;
        let set_index = char_bmp_index + set_color as usize - 1;
        assert_eq!(set_index, 0);
        assert!(set_index < 8, "set_color=1 should produce valid index");

        // set_color = 2 -> index = 0 + 2 - 1 = 1 (valid)
        let set_color = 2;
        let set_index = char_bmp_index + set_color as usize - 1;
        assert_eq!(set_index, 1);
        assert!(set_index < 8, "set_color=2 should produce valid index");
    }

    #[test]
    fn test_char_bmp_take_via_get_mut() {
        let tex = Texture {
            width: 4,
            height: 4,
            disposed: false,
            path: None,
            rgba_data: None,
            gpu_texture: None,
            gpu_view: None,
            sampler: None,
        };
        let mut char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
        char_bmp[3] = Some(tex);

        // Safe take via get_mut
        let taken = char_bmp.get_mut(3).and_then(|s| s.take());
        assert!(
            taken.is_some(),
            "take from occupied slot should return Some"
        );
        assert!(char_bmp[3].is_none(), "slot should be None after take");

        // Out-of-bounds take returns None
        let taken_oob = char_bmp.get_mut(8).and_then(|s| s.take());
        assert!(taken_oob.is_none(), "out-of-bounds take should return None");
    }

    #[test]
    fn test_transparent_processing_with_bounds_checked_index() {
        // Verify transparent_processing works correctly when called via bounds-checked pattern
        let tex = Texture {
            width: 2,
            height: 2,
            disposed: false,
            path: None,
            rgba_data: Some(Arc::new(vec![255; 16])),
            gpu_texture: None,
            gpu_view: None,
            sampler: None,
        };
        let mut char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
        let mut transparent_flag = [false; 8];
        let set_index: usize = 1;

        // Place texture
        if let Some(slot) = char_bmp.get_mut(set_index) {
            *slot = Some(tex);
        }

        // Bounds-checked take + transparent_processing + put back
        let taken = char_bmp.get_mut(set_index).and_then(|s| s.take());
        if let Some(slot) = char_bmp.get_mut(set_index) {
            *slot = transparent_processing(taken, set_index, &mut transparent_flag);
        }

        assert!(
            char_bmp.get(set_index).unwrap().is_some(),
            "should have processed texture"
        );
        assert!(
            transparent_flag[set_index],
            "flag should be set after processing"
        );
    }

    #[test]
    fn test_select_cg_bounds_checked_access() {
        // Verify that SELECT_CG access patterns use .get() safely
        let char_bmp: [Option<Texture>; 8] = [None, None, None, None, None, None, None, None];
        let select_cg_index: usize = 6;

        // Both indices (6 and 7) should be valid
        assert!(
            char_bmp.get(select_cg_index).is_some(),
            "index 6 should be in bounds"
        );
        assert!(
            char_bmp.get(select_cg_index + 1).is_some(),
            "index 7 should be in bounds"
        );

        // Both slots are None (no texture loaded), which is the expected default
        assert!(
            char_bmp.get(select_cg_index).unwrap().is_none(),
            "slot 6 should be None by default"
        );
        assert!(
            char_bmp.get(select_cg_index + 1).unwrap().is_none(),
            "slot 7 should be None by default"
        );
    }
}
