use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::lr2::lr2_skin_loader::{self};
use crate::skin_bpm_graph::SkinBPMGraph;
use crate::skin_image::SkinImage;
use crate::skin_note_distribution_graph::SkinNoteDistributionGraph;
use crate::stubs::{Rectangle, Resolution, TextureRegion};

/// LR2 select skin loader
///
/// Translated from LR2SelectSkinLoader.java (561 lines)
/// Loads LR2 music select skins with bar elements (body, lamp, level, title, etc.)
///
/// LR2 LAMP ID mapping
/// 0:NO PLAY, 1:FAILED, 2:EASY, 3:NORMAL, 4:HARD, 5:EXH, 6:FC, 7:PERFECT, 8:MAX, 9:ASSIST, 10:L-ASSIST
const LAMPG: &[&[i32]] = &[
    &[0],
    &[1],
    &[4, 2, 3],
    &[5],
    &[6, 7],
    &[7],
    &[8, 9, 10],
    &[9],
    &[10],
    &[2],
    &[3],
];

/// BAR_COUNT constant
const BAR_COUNT: usize = 20;
/// BARLEVEL_COUNT constant
const BARLEVEL_COUNT: usize = 10;
/// BARLAMP_COUNT constant
const BARLAMP_COUNT: usize = 11;
/// BARTROPHY_COUNT constant
const BARTROPHY_COUNT: usize = 3;
/// BARLABEL_COUNT constant
const BARLABEL_COUNT: usize = 3;
/// BARTEXT_COUNT constant
const BARTEXT_COUNT: usize = 11;

/// Select skin loader state
pub struct LR2SelectSkinLoaderState {
    pub csv: LR2SkinCSVLoaderState,

    pub barimage: Vec<Option<Vec<TextureRegion>>>,
    pub barimageon: Vec<Option<SkinImage>>,
    pub barimageoff: Vec<Option<SkinImage>>,
    pub barcycle: i32,

    pub barlevel: Vec<Option<crate::skin_number::SkinNumber>>,
    pub bartext: Vec<Option<Box<dyn crate::skin_text::SkinText>>>,

    pub barlamp: Vec<Option<SkinImage>>,
    pub barmylamp: Vec<Option<SkinImage>>,
    pub barrivallamp: Vec<Option<SkinImage>>,
    pub bartrophy: Vec<Option<SkinImage>>,
    pub barlabel: Vec<Option<SkinImage>>,
    pub bargraph_type: Option<i32>,
    pub bargraph_images: Option<Vec<TextureRegion>>,
    pub bargraph_region: Rectangle,

    pub noteobj: Option<SkinNoteDistributionGraph>,
    pub bpmgraphobj: Option<SkinBPMGraph>,
    pub gauge: Rectangle,

    pub center_bar: i32,
    pub clickable_bar: Vec<i32>,

    pub srcw: f32,
    pub srch: f32,
    pub dstw: f32,
    pub dsth: f32,
}

impl LR2SelectSkinLoaderState {
    pub fn new(src: Resolution, dst: Resolution, usecim: bool, skinpath: String) -> Self {
        let srcw = src.width;
        let srch = src.height;
        let dstw = dst.width;
        let dsth = dst.height;

        Self {
            csv: LR2SkinCSVLoaderState::new(src, dst, usecim, skinpath),
            barimage: vec![None; 10],
            barimageon: (0..BAR_COUNT).map(|_| None).collect(),
            barimageoff: (0..BAR_COUNT).map(|_| None).collect(),
            barcycle: 0,
            barlevel: (0..BARLEVEL_COUNT).map(|_| None).collect(),
            bartext: (0..BARTEXT_COUNT).map(|_| None).collect(),
            barlamp: (0..BARLAMP_COUNT).map(|_| None).collect(),
            barmylamp: (0..BARLAMP_COUNT).map(|_| None).collect(),
            barrivallamp: (0..BARLAMP_COUNT).map(|_| None).collect(),
            bartrophy: (0..BARTROPHY_COUNT).map(|_| None).collect(),
            barlabel: (0..BARLABEL_COUNT).map(|_| None).collect(),
            bargraph_type: None,
            bargraph_images: None,
            bargraph_region: Rectangle::default(),
            noteobj: None,
            bpmgraphobj: None,
            gauge: Rectangle::default(),
            center_bar: 0,
            clickable_bar: Vec::new(),
            srcw,
            srch,
            dstw,
            dsth,
        }
    }

    /// Process select-specific commands
    #[allow(unused_assignments)]
    pub fn process_select_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            "SRC_BAR_BODY" => {
                let gr: i32 = str_parts
                    .get(2)
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                if gr < 100 {
                    let values = lr2_skin_loader::parse_int(str_parts);
                    let images = self.csv.source_image(&values);
                    if let Some(images) = images {
                        let idx = values[1] as usize;
                        if idx < self.barimage.len() {
                            self.barimage[idx] = Some(images);
                            self.barcycle = values[9];
                        }
                    }
                }
            }
            "DST_BAR_BODY_OFF" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[5] < 0 {
                    values[3] += values[5];
                    values[5] = -values[5];
                }
                if values[6] < 0 {
                    values[4] += values[6];
                    values[6] = -values[6];
                }
                let idx = values[1] as usize;
                if idx < self.barimageoff.len() && self.barimageoff[idx].is_none() {
                    let images_2d: Vec<Vec<TextureRegion>> = self
                        .barimage
                        .iter()
                        .map(|opt| opt.clone().unwrap_or_default())
                        .collect();
                    self.barimageoff[idx] = Some(SkinImage::new_with_int_timer_ref(
                        images_2d,
                        0,
                        self.barcycle,
                        None,
                    ));
                }
                if idx < self.barimageoff.len()
                    && let Some(ref mut img) = self.barimageoff[idx]
                {
                    let dstw = self.csv.dst.width / self.csv.src.width;
                    let dsth = self.csv.dst.height / self.csv.src.height;
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    img.data.set_destination_with_int_timer_ops(
                        values[2] as i64,
                        values[3] as f32 * dstw,
                        self.csv.dst.height - (values[4] + values[6]) as f32 * dsth,
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
            "DST_BAR_BODY_ON" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[5] < 0 {
                    values[3] += values[5];
                    values[5] = -values[5];
                }
                if values[6] < 0 {
                    values[4] += values[6];
                    values[6] = -values[6];
                }
                let idx = values[1] as usize;
                if idx < self.barimageon.len() && self.barimageon[idx].is_none() {
                    let images_2d: Vec<Vec<TextureRegion>> = self
                        .barimage
                        .iter()
                        .map(|opt| opt.clone().unwrap_or_default())
                        .collect();
                    self.barimageon[idx] = Some(SkinImage::new_with_int_timer_ref(
                        images_2d,
                        0,
                        self.barcycle,
                        None,
                    ));
                }
                if idx < self.barimageon.len()
                    && let Some(ref mut img) = self.barimageon[idx]
                {
                    let dstw = self.csv.dst.width / self.csv.src.width;
                    let dsth = self.csv.dst.height / self.csv.src.height;
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    img.data.set_destination_with_int_timer_ops(
                        values[2] as i64,
                        values[3] as f32 * dstw,
                        self.csv.dst.height - (values[4] + values[6]) as f32 * dsth,
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
            "BAR_CENTER" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.center_bar = values[1];
            }
            "BAR_AVAILABLE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                let mut clickable = vec![0i32; (values[2] - values[1] + 1) as usize];
                for (i, c) in clickable.iter_mut().enumerate() {
                    *c = values[1] + i as i32;
                }
                self.clickable_bar = clickable;
            }
            "SRC_BAR_FLASH" | "DST_BAR_FLASH" => {
                // No-op
            }
            "SRC_BAR_LEVEL" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARLEVEL_COUNT as i32 {
                    return;
                }
                let mut divx = values[7];
                if divx <= 0 {
                    divx = 1;
                }
                let mut divy = values[8];
                if divy <= 0 {
                    divy = 1;
                }

                if divx * divy >= 10 {
                    let images = self.csv.source_image(&values);
                    if let Some(images) = images {
                        let idx = values[1] as usize;
                        if images.len() % 24 == 0 {
                            // Split into positive (12 digits) and negative (12 digits) images
                            let d = 12;
                            let frames = images.len() / (d * 2);
                            let mut pimages = Vec::with_capacity(frames);
                            let mut mimages = Vec::with_capacity(frames);
                            for f in 0..frames {
                                let base = f * d * 2;
                                pimages.push(images[base..base + d].to_vec());
                                mimages.push(images[base + d..base + d * 2].to_vec());
                            }
                            let sn = crate::skin_number::SkinNumber::new_with_int_timer(
                                pimages,
                                Some(mimages),
                                values[9],
                                values[10],
                                divx,
                                values[11],
                                0,
                                0,
                                0,
                            );
                            if idx < self.barlevel.len() {
                                self.barlevel[idx] = Some(sn);
                            }
                        } else {
                            let d = if images.len() % 10 == 0 { 10 } else { 11 };
                            let frames = images.len() / d;
                            let mut nimages = Vec::with_capacity(frames);
                            for f in 0..frames {
                                let base = f * d;
                                nimages.push(images[base..base + d].to_vec());
                            }
                            let sn = crate::skin_number::SkinNumber::new_with_int_timer(
                                nimages, None, values[9], values[10], divx, values[11], 0, 0, 0,
                            );
                            if idx < self.barlevel.len() {
                                self.barlevel[idx] = Some(sn);
                            }
                        }
                    }
                }
            }
            "DST_BAR_LEVEL" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                let idx = values[1] as usize;
                if idx < self.barlevel.len()
                    && let Some(ref mut sn) = self.barlevel[idx]
                {
                    let dstw = self.csv.dst.width / self.csv.src.width;
                    let dsth = self.csv.dst.height / self.csv.src.height;
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    sn.data.set_destination_with_int_timer_ops(
                        values[2] as i64,
                        values[3] as f32 * dstw,
                        self.csv.dst.height - (values[4] + values[6]) as f32 * dsth,
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
            "SRC_BAR_LAMP" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARLAMP_COUNT as i32 {
                    return;
                }
                let images = self.csv.source_image(&values);
                if let Some(images) = images {
                    let lamp_idx = values[1] as usize;
                    if lamp_idx < LAMPG.len() {
                        for &lid in LAMPG[lamp_idx] {
                            let img = SkinImage::new_with_int_timer(
                                images.clone(),
                                values[10],
                                values[9],
                            );
                            let uid = lid as usize;
                            if uid < self.barlamp.len() {
                                self.barlamp[uid] = Some(img);
                            }
                        }
                    }
                }
            }
            "DST_BAR_LAMP" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[5] < 0 {
                    values[3] += values[5];
                    values[5] = -values[5];
                }
                if values[6] < 0 {
                    values[4] += values[6];
                    values[6] = -values[6];
                }
                let lamp_idx = values[1] as usize;
                if lamp_idx < LAMPG.len() {
                    let dstw = self.csv.dst.width / self.csv.src.width;
                    let dsth = self.csv.dst.height / self.csv.src.height;
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    for &lid in LAMPG[lamp_idx] {
                        let uid = lid as usize;
                        if uid < self.barlamp.len()
                            && let Some(ref mut lamp) = self.barlamp[uid]
                        {
                            lamp.data.set_destination_with_int_timer_ops(
                                values[2] as i64,
                                values[3] as f32 * dstw,
                                self.csv.dst.height - (values[4] + values[6]) as f32 * dsth,
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
                }
            }
            "SRC_BAR_MY_LAMP" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARLAMP_COUNT as i32 {
                    return;
                }
                let images = self.csv.source_image(&values);
                if let Some(images) = images {
                    let lamp_idx = values[1] as usize;
                    if lamp_idx < LAMPG.len() {
                        for &lid in LAMPG[lamp_idx] {
                            let img = SkinImage::new_with_int_timer(
                                images.clone(),
                                values[10],
                                values[9],
                            );
                            let uid = lid as usize;
                            if uid < self.barmylamp.len() {
                                self.barmylamp[uid] = Some(img);
                            }
                        }
                    }
                }
            }
            "DST_BAR_MY_LAMP" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[5] < 0 {
                    values[3] += values[5];
                    values[5] = -values[5];
                }
                if values[6] < 0 {
                    values[4] += values[6];
                    values[6] = -values[6];
                }
                let lamp_idx = values[1] as usize;
                if lamp_idx < LAMPG.len() {
                    let dstw = self.csv.dst.width / self.csv.src.width;
                    let dsth = self.csv.dst.height / self.csv.src.height;
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    for &lid in LAMPG[lamp_idx] {
                        let uid = lid as usize;
                        if uid < self.barmylamp.len()
                            && let Some(ref mut lamp) = self.barmylamp[uid]
                        {
                            lamp.data.set_destination_with_int_timer_ops(
                                values[2] as i64,
                                values[3] as f32 * dstw,
                                self.csv.dst.height - (values[4] + values[6]) as f32 * dsth,
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
                }
            }
            "SRC_BAR_RIVAL_LAMP" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARLAMP_COUNT as i32 {
                    return;
                }
                let images = self.csv.source_image(&values);
                if let Some(images) = images {
                    let lamp_idx = values[1] as usize;
                    if lamp_idx < LAMPG.len() {
                        for &lid in LAMPG[lamp_idx] {
                            let img = SkinImage::new_with_int_timer(
                                images.clone(),
                                values[10],
                                values[9],
                            );
                            let uid = lid as usize;
                            if uid < self.barrivallamp.len() {
                                self.barrivallamp[uid] = Some(img);
                            }
                        }
                    }
                }
            }
            "DST_BAR_RIVAL_LAMP" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[5] < 0 {
                    values[3] += values[5];
                    values[5] = -values[5];
                }
                if values[6] < 0 {
                    values[4] += values[6];
                    values[6] = -values[6];
                }
                let lamp_idx = values[1] as usize;
                if lamp_idx < LAMPG.len() {
                    let dstw = self.csv.dst.width / self.csv.src.width;
                    let dsth = self.csv.dst.height / self.csv.src.height;
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    for &lid in LAMPG[lamp_idx] {
                        let uid = lid as usize;
                        if uid < self.barrivallamp.len()
                            && let Some(ref mut lamp) = self.barrivallamp[uid]
                        {
                            lamp.data.set_destination_with_int_timer_ops(
                                values[2] as i64,
                                values[3] as f32 * dstw,
                                self.csv.dst.height - (values[4] + values[6]) as f32 * dsth,
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
                }
            }
            "SRC_BAR_TROPHY" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARTROPHY_COUNT as i32 {
                    return;
                }
                let images = self.csv.source_image(&values);
                if let Some(images) = images {
                    let idx = values[1] as usize;
                    let img = SkinImage::new_with_int_timer(images, values[10], values[9]);
                    if idx < self.bartrophy.len() {
                        self.bartrophy[idx] = Some(img);
                    }
                }
            }
            "DST_BAR_TROPHY" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[5] < 0 {
                    values[3] += values[5];
                    values[5] = -values[5];
                }
                if values[6] < 0 {
                    values[4] += values[6];
                    values[6] = -values[6];
                }
                let idx = values[1] as usize;
                if idx < self.bartrophy.len()
                    && let Some(ref mut trophy) = self.bartrophy[idx]
                {
                    let dstw = self.csv.dst.width / self.csv.src.width;
                    let dsth = self.csv.dst.height / self.csv.src.height;
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    trophy.data.set_destination_with_int_timer_ops(
                        values[2] as i64,
                        values[3] as f32 * dstw,
                        self.csv.dst.height - (values[4] + values[6]) as f32 * dsth,
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
            "SRC_BAR_LABEL" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARLABEL_COUNT as i32 {
                    return;
                }
                let images = self.csv.source_image(&values);
                if let Some(images) = images {
                    let idx = values[1] as usize;
                    let img = SkinImage::new_with_int_timer(images, values[10], values[9]);
                    if idx < self.barlabel.len() {
                        self.barlabel[idx] = Some(img);
                    }
                }
            }
            "DST_BAR_LABEL" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[5] < 0 {
                    values[3] += values[5];
                    values[5] = -values[5];
                }
                if values[6] < 0 {
                    values[4] += values[6];
                    values[6] = -values[6];
                }
                let idx = values[1] as usize;
                if idx < self.barlabel.len()
                    && let Some(ref mut label) = self.barlabel[idx]
                {
                    let dstw = self.csv.dst.width / self.csv.src.width;
                    let dsth = self.csv.dst.height / self.csv.src.height;
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    label.data.set_destination_with_int_timer_ops(
                        values[2] as i64,
                        values[3] as f32 * dstw,
                        self.csv.dst.height - (values[4] + values[6]) as f32 * dsth,
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
            "SRC_BAR_GRAPH" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                let images = self.csv.source_image(&values);
                if let Some(images) = images {
                    self.bargraph_type = Some(values[1]);
                    self.bargraph_images = Some(images);
                }
            }
            "DST_BAR_GRAPH" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[5] < 0 {
                    values[3] += values[5];
                    values[5] = -values[5];
                }
                if values[6] < 0 {
                    values[4] += values[6];
                    values[6] = -values[6];
                }
                let dstw = self.csv.dst.width / self.csv.src.width;
                let dsth = self.csv.dst.height / self.csv.src.height;
                self.bargraph_region = Rectangle::new(
                    values[3] as f32 * dstw,
                    self.csv.dst.height - (values[4] + values[6]) as f32 * dsth,
                    values[5] as f32 * dstw,
                    values[6] as f32 * dsth,
                );
            }
            "SRC_NOTECHART" => {
                lr2_skin_loader::process_src_notechart(
                    str_parts,
                    &mut self.gauge,
                    &mut self.noteobj,
                );
            }
            "DST_NOTECHART" => {
                lr2_skin_loader::process_dst_notechart(
                    str_parts,
                    self.csv.src.height,
                    self.csv.dst.width,
                    self.csv.dst.height,
                    self.csv.src.width,
                    &mut self.gauge,
                    &mut self.noteobj,
                );
            }
            "SRC_BPMCHART" => {
                lr2_skin_loader::process_src_bpmchart(
                    str_parts,
                    &mut self.gauge,
                    &mut self.bpmgraphobj,
                );
            }
            "DST_BPMCHART" => {
                lr2_skin_loader::process_dst_bpmchart(
                    str_parts,
                    self.csv.src.height,
                    self.csv.dst.width,
                    self.csv.dst.height,
                    self.csv.src.width,
                    &mut self.gauge,
                    &mut self.bpmgraphobj,
                );
            }
            "SRC_BAR_TITLE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARTEXT_COUNT as i32 {
                    return;
                }
                let font_idx = values[2] as usize;
                if font_idx < self.csv.fontlist.len()
                    && let Some(source) = self.csv.fontlist[font_idx].take()
                {
                    let text = crate::skin_text_image::SkinTextImage::new(source);
                    let idx = values[1] as usize;
                    if idx < self.bartext.len() {
                        self.bartext[idx] = Some(Box::new(text));
                    }
                }
            }
            "DST_BAR_TITLE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                let idx = values[1] as usize;
                if idx < self.bartext.len()
                    && let Some(ref mut text) = self.bartext[idx]
                {
                    let dstw = self.csv.dst.width / self.csv.src.width;
                    let dsth = self.csv.dst.height / self.csv.src.height;
                    let offsets = lr2_skin_loader::read_offset(str_parts, 21);
                    text.get_text_data_mut()
                        .data
                        .set_destination_with_int_timer_ops(
                            values[2] as i64,
                            values[3] as f32 * dstw,
                            self.csv.dst.height - (values[4] + values[6]) as f32 * dsth,
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
            "SRC_BAR_RANK" | "DST_BAR_RANK" | "SRC_README" | "DST_README" => {
                // No-op
            }
            _ => {
                // Delegate to CSV loader
                self.csv.process_csv_command(cmd, str_parts);
            }
        }
    }
}

impl LR2SkinLoaderAccess for LR2SelectSkinLoaderState {
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState {
        &mut self.csv
    }

    fn assemble_objects(&mut self, skin: &mut crate::skin::Skin) {
        use crate::skin::SkinObject;

        // Create SkinBarObject to register with the skin pipeline.
        // The SkinBarObject itself is a minimal wrapper — actual bar rendering
        // is handled by BarRenderer in rubato-state/select.
        let has_bars = self
            .barimageon
            .iter()
            .chain(self.barimageoff.iter())
            .any(|b| b.is_some());
        if has_bars {
            let bar_obj = crate::skin_bar_object::SkinBarObject::new(self.center_bar);
            skin.add(SkinObject::Bar(bar_obj));

            // Transfer bar data so MusicSelector can build SkinBar + BarRenderer
            skin.select_bar_data = Some(crate::select_bar_data::SelectBarData {
                barimageon: std::mem::take(&mut self.barimageon),
                barimageoff: std::mem::take(&mut self.barimageoff),
                center_bar: self.center_bar,
                clickable_bar: std::mem::take(&mut self.clickable_bar),
                barlevel: std::mem::take(&mut self.barlevel),
                bartext: std::mem::take(&mut self.bartext),
                barlamp: std::mem::take(&mut self.barlamp),
                barmylamp: std::mem::take(&mut self.barmylamp),
                barrivallamp: std::mem::take(&mut self.barrivallamp),
                bartrophy: std::mem::take(&mut self.bartrophy),
                barlabel: std::mem::take(&mut self.barlabel),
                graph_type: self.bargraph_type.take(),
                graph_images: self.bargraph_images.take(),
                graph_region: std::mem::take(&mut self.bargraph_region),
            });
        }

        // Add graph objects
        if let Some(obj) = self.noteobj.take() {
            skin.add(SkinObject::NoteDistributionGraph(obj));
        }
        if let Some(obj) = self.bpmgraphobj.take() {
            skin.add(SkinObject::BpmGraph(obj));
        }

        log::debug!("LR2SelectSkinLoader: assembled objects into skin");
    }
}
