use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::lr2::lr2_skin_loader;
use crate::skin_image::SkinImage;
use crate::stubs::{MainState, Rectangle, Resolution, Texture, TextureRegion};

/// LR2 play skin loader
///
/// Translated from LR2PlaySkinLoader.java (1025 lines)
/// Loads LR2 play skins with notes, judge, BGA, gauge, and other play-specific elements.
///
/// Note source data
#[derive(Clone, Debug, Default)]
pub struct SkinSourceData {
    pub images: Option<Vec<TextureRegion>>,
    pub timer: i32,
    pub cycle: i32,
}

/// Parsed PM character entry for deferred assembly.
#[derive(Clone, Debug)]
pub enum PmCharaEntry {
    /// DST_PM_CHARA: side, imagefile, color, dstx, dsty, dstw, dsth
    Chara {
        side: i32,
        imagefile: String,
        color: i32,
        dst: Rectangle,
    },
    /// DST_PM_CHARA_ANIMATION: load_type, imagefile, color, dst values, timer, ops
    Animation {
        load_type: i32,
        imagefile: String,
        color: i32,
        dst: Rectangle,
        timer: i32,
    },
    /// SRC_PM_CHARA_IMAGE: load_type, imagefile, color
    SrcImage {
        load_type: i32,
        imagefile: String,
        color: i32,
    },
    /// DST_PM_CHARA_IMAGE: destination rectangle
    DstImage { dst: Rectangle },
}

/// Play skin loader state
pub struct LR2PlaySkinLoaderState {
    pub csv: LR2SkinCSVLoaderState,

    pub skin_type: crate::skin_type::SkinType,
    pub mode: Option<bms_model::mode::Mode>,

    pub note: Vec<Option<SkinSourceData>>,
    pub lnstart: Vec<Option<SkinSourceData>>,
    pub lnend: Vec<Option<SkinSourceData>>,
    pub lnbody: Vec<Option<SkinSourceData>>,
    pub lnbodya: Vec<Option<SkinSourceData>>,
    pub hcnstart: Vec<Option<SkinSourceData>>,
    pub hcnend: Vec<Option<SkinSourceData>>,
    pub hcnbody: Vec<Option<SkinSourceData>>,
    pub hcnbodya: Vec<Option<SkinSourceData>>,
    pub hcnbodyd: Vec<Option<SkinSourceData>>,
    pub hcnbodyr: Vec<Option<SkinSourceData>>,
    pub mine: Vec<Option<SkinSourceData>>,
    pub laner: Vec<Option<Rectangle>>,
    pub scale: Vec<f32>,
    pub dstnote2: Vec<i32>,
    pub linevalues: [Option<Vec<String>>; 2],

    pub srcw: f32,
    pub srch: f32,
    pub dstw: f32,
    pub dsth: f32,

    pub gauge: Rectangle,
    pub playerr: Vec<Option<Rectangle>>,
    pub hidden: bool,
    pub lanerender: bool,
    pub judgeline: Option<SkinImage>,
    pub bga: bool,

    // Accumulated play skin property values (applied by caller)
    /// Close time (ms) — set by CLOSE command
    pub play_close: Option<i32>,
    /// Playstart time (ms) — set by PLAYSTART command
    pub play_playstart: Option<i32>,
    /// Loadstart time (ms) — set by LOADSTART command
    pub play_loadstart: Option<i32>,
    /// Loadend time (ms) — set by LOADEND command
    pub play_loadend: Option<i32>,
    /// Finish margin time (ms) — set by FINISHMARGIN command
    pub play_finish_margin: Option<i32>,
    /// Judge timer condition — set by JUDGETIMER command
    pub play_judgetimer: Option<i32>,
    /// Note expansion rate [w%, h%] — set by DST_NOTE_EXPANSION_RATE command
    pub play_note_expansion_rate: Option<[i32; 2]>,

    /// SkinImage per line index (SRC_LINE/DST_LINE)
    pub line_images: Vec<Option<SkinImage>>,

    /// SkinJudge objects per player (SRC_NOWJUDGE/DST_NOWJUDGE)
    pub judge_objects: [Option<crate::skin_judge_object::SkinJudgeObject>; 3],
    /// Whether DST_NOWJUDGE detail has been added per player
    pub judge_detail_added: [bool; 3],

    /// Parsed PomyuChara entries for deferred assembly.
    pub pmchara_entries: Vec<PmCharaEntry>,

    /// Computed judge region count (set by load_skin post-processing)
    pub computed_judge_reg: Option<i32>,
    /// Computed line count (set by load_skin post-processing)
    pub computed_line_count: Option<usize>,
}

impl LR2PlaySkinLoaderState {
    pub fn new(
        skin_type: crate::skin_type::SkinType,
        src: Resolution,
        dst: Resolution,
        usecim: bool,
        skinpath: String,
    ) -> Self {
        let srcw = src.width;
        let srch = src.height;
        let dstw = dst.width;
        let dsth = dst.height;

        Self {
            csv: LR2SkinCSVLoaderState::new(src, dst, usecim, skinpath),
            skin_type,
            mode: None,
            note: vec![None; 8],
            lnstart: vec![None; 8],
            lnend: vec![None; 8],
            lnbody: vec![None; 8],
            lnbodya: vec![None; 8],
            hcnstart: vec![None; 8],
            hcnend: vec![None; 8],
            hcnbody: vec![None; 8],
            hcnbodya: vec![None; 8],
            hcnbodyd: vec![None; 8],
            hcnbodyr: vec![None; 8],
            mine: vec![None; 8],
            laner: vec![None; 8],
            scale: vec![0.0; 8],
            dstnote2: vec![0; 8],
            linevalues: [None, None],
            srcw,
            srch,
            dstw,
            dsth,
            gauge: Rectangle::default(),
            playerr: Vec::new(),
            hidden: false,
            lanerender: false,
            judgeline: None,
            bga: false,
            play_close: None,
            play_playstart: None,
            play_loadstart: None,
            play_loadend: None,
            play_finish_margin: None,
            play_judgetimer: None,
            play_note_expansion_rate: None,
            line_images: vec![None, None, None, None, None, None, None, None],
            judge_objects: [None, None, None],
            judge_detail_added: [false; 3],
            pmchara_entries: Vec::new(),
            computed_judge_reg: None,
            computed_line_count: None,
        }
    }

    /// Process play-specific commands
    pub fn process_play_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            // PlayCommand enum entries
            "CLOSE" => {
                if str_parts.len() > 1 {
                    self.play_close = str_parts[1].trim().parse().ok();
                }
            }
            "PLAYSTART" => {
                if str_parts.len() > 1 {
                    self.play_playstart = str_parts[1].trim().parse().ok();
                }
            }
            "LOADSTART" => {
                if str_parts.len() > 1 {
                    self.play_loadstart = str_parts[1].trim().parse().ok();
                }
            }
            "LOADEND" => {
                if str_parts.len() > 1 {
                    self.play_loadend = str_parts[1].trim().parse().ok();
                }
            }
            "FINISHMARGIN" => {
                if str_parts.len() > 1 {
                    self.play_finish_margin = str_parts[1].trim().parse().ok();
                }
            }
            "JUDGETIMER" => {
                if str_parts.len() > 1 {
                    self.play_judgetimer = str_parts[1].trim().parse().ok();
                }
            }
            "SRC_BGA" => {
                // In Java: bga = new SkinBGA(c.getBgaExpand()); skin.add(bga)
                // SkinBgaObject is created and added to the skin by the caller.
                // Here we just signal that BGA was requested.
                self.bga = true;
            }
            "DST_BGA" => {
                // DST_BGA sets the destination for the BGA object.
                // In Java: skin.setDestination(bga, 0, values[3], srch - values[4] - values[6], values[5], values[6], ...)
                // The destination is applied by the caller using the skin's setDestination.
                // We store the raw values for the caller.
                if self.bga {
                    let _values = lr2_skin_loader::parse_int(str_parts);
                    // BGA destination will be applied by caller when connecting to Skin
                }
            }
            "SRC_LINE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if let Some(images) = self.csv.get_source_image(&values) {
                    let idx = values[1] as usize;
                    if idx < self.line_images.len() {
                        let skin_image =
                            SkinImage::new_with_int_timer(images, values[10], values[9]);
                        self.line_images[idx] = Some(skin_image);
                    }
                }
            }
            "DST_LINE" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                let idx = values[1] as usize;
                if idx < self.line_images.len() && self.line_images[idx].is_some() {
                    if values[5] < 0 {
                        values[3] += values[5];
                        values[5] = -values[5];
                    }
                    if values[6] < 0 {
                        values[4] += values[6];
                        values[6] = -values[6];
                    }
                    let offset = LR2SkinCSVLoaderState::read_offset_with_base(
                        str_parts,
                        21,
                        &[crate::skin_property::OFFSET_LIFT],
                    );
                    if let Some(ref mut li) = self.line_images[idx] {
                        li.data.set_destination_with_int_timer_and_offsets(
                            values[2] as i64,
                            values[3] as f32 * self.dstw / self.srcw,
                            self.dsth - (values[4] + values[6]) as f32 * self.dsth / self.srch,
                            values[5] as f32 * self.dstw / self.srcw,
                            values[6] as f32 * self.dsth / self.srch,
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
                            values[18],
                            values[19],
                            values[20],
                            &offset,
                        );
                    }
                    // Update player rectangle
                    let player_idx = idx % 2;
                    if player_idx < self.playerr.len() && self.playerr[player_idx].is_some() {
                        self.playerr[player_idx] = Some(Rectangle::new(
                            values[3] as f32 * self.dstw / self.srcw,
                            self.dsth - (values[4] + values[6]) as f32 * self.dsth / self.srch,
                            values[5] as f32 * self.dstw / self.srcw,
                            (values[4] + values[6]) as f32 * self.dsth / self.srch,
                        ));
                    }
                    self.linevalues[idx % 2] = Some(str_parts.to_vec());
                }
            }
            "SRC_NOTE" => {
                self.add_note(str_parts, NoteTarget::Note, true);
            }
            "SRC_LN_END" => {
                self.add_note(str_parts, NoteTarget::LnEnd, true);
            }
            "SRC_LN_START" => {
                self.add_note(str_parts, NoteTarget::LnStart, true);
            }
            "SRC_LN_BODY" => {
                self.add_note(str_parts, NoteTarget::LnBody, false);
                self.add_note(str_parts, NoteTarget::LnBodyA, true);
            }
            "SRC_LN_BODY_INACTIVE" => {
                self.add_note(str_parts, NoteTarget::LnBody, true);
            }
            "SRC_LN_BODY_ACTIVE" => {
                self.add_note(str_parts, NoteTarget::LnBodyA, true);
            }
            "SRC_HCN_END" => {
                self.add_note(str_parts, NoteTarget::HcnEnd, true);
            }
            "SRC_HCN_START" => {
                self.add_note(str_parts, NoteTarget::HcnStart, true);
            }
            "SRC_HCN_BODY" => {
                self.add_note(str_parts, NoteTarget::HcnBody, false);
                self.add_note(str_parts, NoteTarget::HcnBodyA, true);
            }
            "SRC_HCN_BODY_INACTIVE" => {
                self.add_note(str_parts, NoteTarget::HcnBody, true);
            }
            "SRC_HCN_BODY_ACTIVE" => {
                self.add_note(str_parts, NoteTarget::HcnBodyA, true);
            }
            "SRC_HCN_DAMAGE" => {
                self.add_note(str_parts, NoteTarget::HcnBodyD, true);
            }
            "SRC_HCN_REACTIVE" => {
                self.add_note(str_parts, NoteTarget::HcnBodyR, true);
            }
            "SRC_MINE" => {
                self.add_note(str_parts, NoteTarget::Mine, true);
            }
            "DST_NOTE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if let Some(ref mode) = self.mode {
                    let mut lane = values[1];
                    if lane % 10 == 0 {
                        lane = if mode.scratch_key().len() > (lane / 10) as usize {
                            mode.scratch_key()[(lane / 10) as usize]
                        } else {
                            -1
                        };
                    } else {
                        let offset =
                            (lane / 10) * (self.laner.len() as i32 / self.playerr.len() as i32);
                        lane = if lane > 10 { lane - 11 } else { lane - 1 };
                        if lane
                            >= (self.laner.len() as i32 - mode.scratch_key().len() as i32)
                                / self.playerr.len() as i32
                        {
                            lane = -1;
                        } else {
                            lane += offset;
                        }
                    }
                    if lane < 0 {
                        return;
                    }
                    let lane = lane as usize;
                    if lane < self.laner.len() && self.laner[lane].is_none() {
                        self.laner[lane] = Some(Rectangle::new(
                            values[3] as f32 * self.dstw / self.srcw,
                            self.dsth - (values[4] + values[6]) as f32 * self.dsth / self.srch,
                            values[5] as f32 * self.dstw / self.srcw,
                            (values[4] + values[6]) as f32 * self.dsth / self.srch,
                        ));
                        self.scale[lane] = values[6] as f32 * self.dsth / self.srch;
                    }
                    if !self.lanerender {
                        // Fill in missing HCN sources from LN sources
                        for i in 0..self.hcnend.len() {
                            if self.hcnend[i].is_none() {
                                self.hcnend[i] = self.lnend[i].clone();
                            }
                        }
                        for i in 0..self.hcnstart.len() {
                            if self.hcnstart[i].is_none() {
                                self.hcnstart[i] = self.lnstart[i].clone();
                            }
                        }
                        for i in 0..self.hcnbody.len() {
                            if self.hcnbody[i].is_none() {
                                self.hcnbody[i] = self.lnbody[i].clone();
                            }
                        }
                        for i in 0..self.hcnbodya.len() {
                            if self.hcnbodya[i].is_none() {
                                self.hcnbodya[i] = self.lnbodya[i].clone();
                            }
                        }
                        for i in 0..self.hcnbodyd.len() {
                            if self.hcnbodyd[i].is_none() {
                                self.hcnbodyd[i] = self.hcnbody[i].clone();
                            }
                        }
                        for i in 0..self.hcnbodyr.len() {
                            if self.hcnbodyr[i].is_none() {
                                self.hcnbodyr[i] = self.hcnbodya[i].clone();
                            }
                        }
                        // lanerender = new SkinNote(note, lnss, mine)
                        self.lanerender = true;
                        // lanerender.setOffsetID(readOffset(str, 21, new int[]{OFFSET_NOTES_1P}))
                        // skin.add(lanerender)
                    }
                }
            }
            "DST_NOTE2" => {
                if str_parts.len() > 1 {
                    let y: i32 = str_parts[1].trim().parse().unwrap_or(0);
                    let val =
                        (self.dsth - (y as f32 * self.dsth / self.srch + self.scale[0])) as i32;
                    self.dstnote2.fill(val);
                }
            }
            "DST_NOTE_EXPANSION_RATE" => {
                if str_parts.len() > 2 {
                    let w: i32 = str_parts[1].trim().parse().unwrap_or(100);
                    let h: i32 = str_parts[2].trim().parse().unwrap_or(100);
                    self.play_note_expansion_rate = Some([w, h]);
                }
            }
            "SRC_NOWJUDGE_1P" | "SRC_NOWJUDGE_2P" | "SRC_NOWJUDGE_3P" => {
                let player = match cmd {
                    "SRC_NOWJUDGE_1P" => 0usize,
                    "SRC_NOWJUDGE_2P" => 1,
                    _ => 2,
                };
                let values = lr2_skin_loader::parse_int(str_parts);
                if let Some(images) = self.csv.get_source_image(&values) {
                    if self.judge_objects[player].is_none() {
                        let shift = values[11] != 1;
                        self.judge_objects[player] = Some(
                            crate::skin_judge_object::SkinJudgeObject::new(player as i32, shift),
                        );
                    }
                    // Map LR2 judge index: values[1] <= 5 -> (5 - values[1]), else values[1]
                    let judge_idx = if values[1] <= 5 {
                        (5 - values[1]) as usize
                    } else {
                        values[1] as usize
                    };
                    if let Some(ref mut judge_obj) = self.judge_objects[player] {
                        let _judge_image =
                            SkinImage::new_with_int_timer(images, values[10], values[9]);
                        judge_obj.inner.set_judge(judge_idx);
                    }
                }
            }
            "DST_NOWJUDGE_1P" | "DST_NOWJUDGE_2P" | "DST_NOWJUDGE_3P" => {
                let player = match cmd {
                    "DST_NOWJUDGE_1P" => 0usize,
                    "DST_NOWJUDGE_2P" => 1,
                    _ => 2,
                };
                let judge_idx_raw: i32 = str_parts
                    .get(1)
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let judge_idx = if judge_idx_raw <= 5 {
                    (5 - judge_idx_raw) as usize
                } else {
                    judge_idx_raw as usize
                };
                if self.judge_objects[player]
                    .as_ref()
                    .is_some_and(|j| j.inner.get_judge(judge_idx))
                {
                    let mut values = lr2_skin_loader::parse_int(str_parts);
                    if values[5] < 0 {
                        values[3] += values[5];
                        values[5] = -values[5];
                    }
                    if values[6] < 0 {
                        values[4] += values[6];
                        values[6] = -values[6];
                    }
                    // Judge image destination — SkinJudge currently uses () placeholders
                    // for judge images, so actual destination cannot be set on the inner
                    // SkinImage yet. The coordinate transform is computed here for when
                    // SkinJudge is upgraded to hold real SkinImage objects.
                    let _x = values[3] as f32 * self.dstw / self.srcw;
                    let _y = self.dsth - (values[4] + values[6]) as f32 * self.dsth / self.srch;
                    let _w = values[5] as f32 * self.dstw / self.srcw;
                    let _h = values[6] as f32 * self.dsth / self.srch;
                }
            }
            "SRC_NOWCOMBO_1P" | "SRC_NOWCOMBO_2P" | "SRC_NOWCOMBO_3P" => {
                let player = match cmd {
                    "SRC_NOWCOMBO_1P" => 0usize,
                    "SRC_NOWCOMBO_2P" => 1,
                    _ => 2,
                };
                let values = lr2_skin_loader::parse_int(str_parts);
                let divx = if values[7] > 0 { values[7] } else { 1 };
                let divy = if values[8] > 0 { values[8] } else { 1 };
                if let Some(simages) = self.csv.get_source_image(&values) {
                    // Rearrange flat images into [divy][divx] grid
                    let _images_2d: Vec<Vec<TextureRegion>> = (0..divy)
                        .map(|j| {
                            (0..divx)
                                .map(|i| simages[(j * divx + i) as usize].clone())
                                .collect()
                        })
                        .collect();
                    let judge_idx = if values[1] <= 5 {
                        (5 - values[1]) as usize
                    } else {
                        values[1] as usize
                    };
                    // Set judge count on the SkinJudge (placeholder)
                    if let Some(ref mut judge_obj) = self.judge_objects[player] {
                        judge_obj.inner.set_judge_count(judge_idx);
                    }
                }
            }
            "DST_NOWCOMBO_1P" | "DST_NOWCOMBO_2P" | "DST_NOWCOMBO_3P" => {
                let player = match cmd {
                    "DST_NOWCOMBO_1P" => 0usize,
                    "DST_NOWCOMBO_2P" => 1,
                    _ => 2,
                };
                let judge_idx_raw: i32 = str_parts
                    .get(1)
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let judge_idx = if judge_idx_raw <= 5 {
                    (5 - judge_idx_raw) as usize
                } else {
                    judge_idx_raw as usize
                };
                if let Some(ref judge_obj) = self.judge_objects[player]
                    && judge_obj.inner.get_judge_count(judge_idx)
                {
                    let _values = lr2_skin_loader::parse_int(str_parts);
                    // Combo number destination — SkinJudge currently uses ()
                    // placeholders for count SkinNumbers. Coordinate transform is
                    // deferred until SkinJudge stores real SkinNumber objects.
                }
            }
            "SRC_JUDGELINE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if let Some(images) = self.csv.get_source_image(&values) {
                    self.judgeline =
                        Some(SkinImage::new_with_int_timer(images, values[10], values[9]));
                }
            }
            "DST_JUDGELINE" => {
                if self.judgeline.is_some() {
                    let mut values = lr2_skin_loader::parse_int(str_parts);
                    if values[5] < 0 {
                        values[3] += values[5];
                        values[5] = -values[5];
                    }
                    if values[6] < 0 {
                        values[4] += values[6];
                        values[6] = -values[6];
                    }
                    let offset = LR2SkinCSVLoaderState::read_offset_with_base(
                        str_parts,
                        21,
                        &[crate::skin_property::OFFSET_LIFT],
                    );
                    if let Some(ref mut jl) = self.judgeline {
                        jl.data.set_destination_with_int_timer_and_offsets(
                            values[2] as i64,
                            values[3] as f32 * self.dstw / self.srcw,
                            self.dsth - (values[4] + values[6]) as f32 * self.dsth / self.srch,
                            values[5] as f32 * self.dstw / self.srcw,
                            values[6] as f32 * self.dsth / self.srch,
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
                            values[18],
                            values[19],
                            values[20],
                            &offset,
                        );
                    }
                }
            }
            "SRC_NOTECHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                // noteobj = new SkinNoteDistributionGraph(...)
                self.gauge = Rectangle::new(0.0, 0.0, values[11] as f32, values[12] as f32);
                // skin.add(noteobj)
            }
            "DST_NOTECHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(noteobj, ...)
            }
            "SRC_BPMCHART" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                // bpmgraphobj = new SkinBPMGraph(...)
                self.gauge = Rectangle::new(0.0, 0.0, values[1] as f32, values[2] as f32);
                // skin.add(bpmgraphobj)
            }
            "DST_BPMCHART" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(bpmgraphobj, ...)
            }
            "SRC_TIMING_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                // timingobj = new SkinTimingVisualizer(...)
                self.gauge = Rectangle::new(0.0, 0.0, values[4] as f32, values[5] as f32);
                // skin.add(timingobj)
            }
            "DST_TIMING_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(timingobj, ...)
            }
            "SRC_HIDDEN" | "SRC_LIFT" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                let _images = self.csv.get_source_image(&values);
                // hidden = new SkinHidden(images, values[10], values[9])
                self.hidden = true;
            }
            "DST_HIDDEN" => {
                if self.hidden {
                    let _values = lr2_skin_loader::parse_int(str_parts);
                    // hidden.setDestination(...)
                }
            }
            "DST_LIFT" => {
                if self.hidden {
                    let _values = lr2_skin_loader::parse_int(str_parts);
                    // hidden.setDestination(...)
                }
            }
            "DST_PM_CHARA_1P" | "DST_PM_CHARA_2P" => {
                // Play: judge-linked character display
                // x,y,w,h,color,offset,folderpath
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[3] < 0 {
                    values[1] += values[3];
                    values[3] = -values[3];
                }
                if values[4] < 0 {
                    values[2] += values[4];
                    values[4] = -values[4];
                }
                let _imagefile = lr2_skin_loader::get_lr2_path(
                    &self.csv.skinpath,
                    str_parts.get(7).map_or("", |s| s.as_str()),
                    &self.csv.filemap,
                );
                let _side: i32 = if cmd == "DST_PM_CHARA_1P" { 1 } else { 2 };
                let _color = if values[5] == 1 || values[5] == 2 {
                    values[5]
                } else {
                    1
                };
                let _dstx = values[1] as f32 * self.dstw / self.srcw;
                let _dsty = self.dsth - (values[2] + values[4]) as f32 * self.dsth / self.srch;
                let _dstw = values[3] as f32 * self.dstw / self.srcw;
                let _dsth_val = values[4] as f32 * self.dsth / self.srch;
                self.pmchara_entries.push(PmCharaEntry::Chara {
                    side: _side,
                    imagefile: _imagefile,
                    color: _color,
                    dst: Rectangle {
                        x: _dstx,
                        y: _dsty,
                        width: _dstw,
                        height: _dsth_val,
                    },
                });
            }
            "DST_PM_CHARA_ANIMATION" => {
                // Non-play: not judge-linked
                // x,y,w,h,color,animationtype,timer,op1,op2,op3,offset,folderpath
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[6] >= 0 && values[6] <= 9 {
                    if values[3] < 0 {
                        values[1] += values[3];
                        values[3] = -values[3];
                    }
                    if values[4] < 0 {
                        values[2] += values[4];
                        values[4] = -values[4];
                    }
                    let _imagefile = lr2_skin_loader::get_lr2_path(
                        &self.csv.skinpath,
                        str_parts.get(12).map_or("", |s| s.as_str()),
                        &self.csv.filemap,
                    );
                    let _load_type = values[6] + 6;
                    let _color = if values[5] == 1 || values[5] == 2 {
                        values[5]
                    } else {
                        1
                    };
                    let _dstx = values[1] as f32 * self.dstw / self.srcw;
                    let _dsty = self.dsth - (values[2] + values[4]) as f32 * self.dsth / self.srch;
                    let _dstw = values[3] as f32 * self.dstw / self.srcw;
                    let _dsth_val = values[4] as f32 * self.dsth / self.srch;
                    self.pmchara_entries.push(PmCharaEntry::Animation {
                        load_type: _load_type,
                        imagefile: _imagefile,
                        color: _color,
                        dst: Rectangle {
                            x: _dstx,
                            y: _dsty,
                            width: _dstw,
                            height: _dsth_val,
                        },
                        timer: values[7],
                    });
                }
            }
            "SRC_PM_CHARA_IMAGE" => {
                // color,type,folderpath
                // type 0:background 1:name 2:face upper 3:face all 4:icon
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[2] >= 0 && values[2] <= 4 {
                    let _imagefile = lr2_skin_loader::get_lr2_path(
                        &self.csv.skinpath,
                        str_parts.get(3).map_or("", |s| s.as_str()),
                        &self.csv.filemap,
                    );
                    let _load_type = values[2] + 1;
                    let _color = if values[1] == 1 || values[1] == 2 {
                        values[1]
                    } else {
                        1
                    };
                    self.pmchara_entries.push(PmCharaEntry::SrcImage {
                        load_type: _load_type,
                        imagefile: _imagefile,
                        color: _color,
                    });
                }
            }
            "DST_PM_CHARA_IMAGE" => {
                // Same as DST_IMAGE
                let mut values = lr2_skin_loader::parse_int(str_parts);
                if values[5] < 0 {
                    values[3] += values[5];
                    values[5] = -values[5];
                }
                if values[6] < 0 {
                    values[4] += values[6];
                    values[6] = -values[6];
                }
                let dstx = values[3] as f32 * self.dstw / self.srcw;
                let dsty = self.dsth - (values[4] + values[6]) as f32 * self.dsth / self.srch;
                let dstw = values[5] as f32 * self.dstw / self.srcw;
                let dsth_val = values[6] as f32 * self.dsth / self.srch;
                self.pmchara_entries.push(PmCharaEntry::DstImage {
                    dst: Rectangle {
                        x: dstx,
                        y: dsty,
                        width: dstw,
                        height: dsth_val,
                    },
                });
            }
            _ => {
                // Delegate to CSV loader
                self.csv.process_csv_command(cmd, str_parts);
            }
        }
    }

    /// Add note source image to the specified note array
    fn add_note(&mut self, str_parts: &[String], target: NoteTarget, animation: bool) {
        let values = lr2_skin_loader::parse_int(str_parts);
        if let Some(ref mode) = self.mode {
            let mut lane = values[1];
            if lane % 10 == 0 {
                lane = if mode.scratch_key().len() > (lane / 10) as usize {
                    mode.scratch_key()[(lane / 10) as usize]
                } else {
                    -1
                };
            } else {
                let offset =
                    (lane / 10) * (self.laner.len() as i32 / self.playerr.len().max(1) as i32);
                lane = if lane > 10 { lane - 11 } else { lane - 1 };
                if lane
                    >= (self.laner.len() as i32 - mode.scratch_key().len() as i32)
                        / self.playerr.len().max(1) as i32
                {
                    lane = -1;
                } else {
                    lane += offset;
                }
            }
            if lane < 0 {
                return;
            }
            let lane = lane as usize;
            let note_array = match target {
                NoteTarget::Note => &mut self.note,
                NoteTarget::LnStart => &mut self.lnstart,
                NoteTarget::LnEnd => &mut self.lnend,
                NoteTarget::LnBody => &mut self.lnbody,
                NoteTarget::LnBodyA => &mut self.lnbodya,
                NoteTarget::HcnStart => &mut self.hcnstart,
                NoteTarget::HcnEnd => &mut self.hcnend,
                NoteTarget::HcnBody => &mut self.hcnbody,
                NoteTarget::HcnBodyA => &mut self.hcnbodya,
                NoteTarget::HcnBodyD => &mut self.hcnbodyd,
                NoteTarget::HcnBodyR => &mut self.hcnbodyr,
                NoteTarget::Mine => &mut self.mine,
            };
            if lane < note_array.len()
                && note_array[lane].is_none()
                && let Some(images) = self.csv.get_source_image(&values)
            {
                note_array[lane] = Some(SkinSourceData {
                    images: Some(images),
                    timer: if animation { values[10] } else { 0 },
                    cycle: if animation { values[9] } else { 0 },
                });
            }
        }
    }

    /// Create a default line image at `index` from "skin/default/system.png".
    ///
    /// Corresponds to Java LR2PlaySkinLoader.makeDefaultLines().
    /// Uses the DST_LINE values from the associated judge line (linevalues[index % 2])
    /// but overrides alpha to 255 and RGB to the specified color. The height is
    /// multiplied by `h` (1 for time lines, 2 for BPM/stop lines).
    fn make_default_line(&mut self, index: usize, h: i32, r: i32, g: i32, b: i32) {
        let linevalue_idx = index % 2;
        let linevalues = match self.linevalues[linevalue_idx] {
            Some(ref lv) => lv.clone(),
            None => return, // No DST_LINE values available; skip (Java would NPE)
        };

        // Create a 1x1 texture region from system.png
        let tex = Texture::new("skin/default/system.png");
        let region = TextureRegion::from_texture_region(tex, 0, 0, 1, 1);
        let skin_image = SkinImage::new_with_single(region);
        self.line_images[index] = Some(skin_image);

        // Parse the original DST_LINE values to extract position/timing parameters
        let values = lr2_skin_loader::parse_int(&linevalues);

        // Compute destination coordinates with resolution scaling
        let time = values[2] as i64;
        let x = values[3] as f32 * self.dstw / self.srcw;
        let y = self.dsth - (values[4] + values[6]) as f32 * self.dsth / self.srch;
        let w = values[5] as f32 * self.dstw / self.srcw;
        let dst_h = values[6] as f32 * self.dsth / self.srch * h as f32;

        let offset = LR2SkinCSVLoaderState::read_offset_with_base(
            &linevalues,
            21,
            &[crate::skin_property::OFFSET_LIFT],
        );

        if let Some(ref mut li) = self.line_images[index] {
            li.data.set_destination_with_int_timer_and_offsets(
                time, x, y, w, dst_h, values[7],  // acc
                255,        // alpha (overridden to full)
                r,          // red (overridden)
                g,          // green (overridden)
                b,          // blue (overridden)
                values[12], // blend
                values[13], // filter
                values[14], // angle
                values[15], // center
                values[16], // loop
                values[17], // timer
                values[18], // op1
                values[19], // op2
                values[20], // op3
                &offset,
            );
        }
    }

    /// Load play skin from a .lr2skin CSV file.
    ///
    /// Pipeline: initialize arrays -> parse CSV lines -> finalize -> post-process
    /// (lane cover, lines, judge regions, lane regions).
    ///
    /// Corresponds to LR2PlaySkinLoader.loadSkin() in Java (lines 897-975).
    pub fn load_skin(
        &mut self,
        path: &std::path::Path,
        state: Option<&dyn MainState>,
    ) -> anyhow::Result<()> {
        // 1. Initialize note/lane arrays based on mode key count
        self.mode = self.skin_type.mode();
        if let Some(ref mode) = self.mode {
            let key = mode.key() as usize;
            self.note = vec![None; key];
            self.lnstart = vec![None; key];
            self.lnend = vec![None; key];
            self.lnbody = vec![None; key];
            self.lnbodya = vec![None; key];
            self.hcnstart = vec![None; key];
            self.hcnend = vec![None; key];
            self.hcnbody = vec![None; key];
            self.hcnbodya = vec![None; key];
            self.hcnbodyd = vec![None; key];
            self.hcnbodyr = vec![None; key];
            self.mine = vec![None; key];
            self.laner = vec![None; key];
            self.scale = vec![0.0; key];
            self.dstnote2 = vec![i32::MIN; key];

            let player_count = mode.player() as usize;
            self.playerr = vec![Some(Rectangle::default()); player_count];
        }

        // 2. Read and parse CSV file, routing commands through process_play_command
        //    which handles play-specific commands and delegates the rest to the CSV loader.
        let raw_bytes = std::fs::read(path)?;
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_bytes);
        let content = decoded.into_owned();

        for line in content.lines() {
            self.csv.line = Some(line.to_string());
            if let Some((cmd, str_parts)) = self.csv.base.process_line_directives(line, state) {
                self.process_play_command(&cmd, &str_parts);
            }
        }

        // 3. Flush any remaining active objects in the CSV loader
        self.csv.finalize_active_objects();

        // 4. Post-processing: lane cover Y position adjustment
        // When white number (lane cover position) is 0, reduce lane height by (dsth - laneCoverPosition).
        let lane_cover_position = self.get_lane_cover_position();
        if lane_cover_position > 0.0 {
            for rect in self.laner.iter_mut().flatten() {
                rect.height -= self.dsth - lane_cover_position;
            }
        }

        // 5. Wire lane rendering: lanerender.setLaneRegion(laner, scale, dstnote2, skin)
        // This is handled in assemble_objects() where SkinNoteObject is created with lane regions.

        // 6. Count line images for judge/BPM/stop/time lines
        // Java: skinline = lines[0..n] where n = count of non-null leading lines
        let line_count = if self.line_images[0].is_some() {
            if self.line_images[1].is_some() { 2 } else { 1 }
        } else {
            0
        };

        // Create default time/BPM/stop line images when missing but judge line exists.
        // Java: makeDefaultLines() creates a SkinImage from "skin/default/system.png" (1x1 white pixel)
        // with the judge line's destination but overridden color and optional height multiplier.
        for i in 0..line_count {
            // Time line at index i+6: h=1, cyan (64, 192, 192)
            if self.line_images[i + 6].is_none() && self.line_images[i].is_some() {
                self.make_default_line(i + 6, 1, 64, 192, 192);
            }
            // BPM line at index i+2: h=2, green (0, 192, 0)
            if self.line_images[i + 2].is_none() && self.line_images[i].is_some() {
                self.make_default_line(i + 2, 2, 0, 192, 0);
            }
            // Stop line at index i+4: h=2, yellow (192, 192, 0)
            if self.line_images[i + 4].is_none() && self.line_images[i].is_some() {
                self.make_default_line(i + 4, 2, 192, 192, 0);
            }
        }

        // 7. Count judge regions
        // Java: judge_reg starts at 1, increments for consecutive non-null judge entries
        let mut judge_reg = 1i32;
        for i in 1..self.judge_objects.len() {
            if self.judge_objects[i].is_some() {
                judge_reg += 1;
            } else {
                break;
            }
        }

        // Store computed values for apply_to_play_skin to use
        self.computed_judge_reg = Some(judge_reg);
        self.computed_line_count = Some(line_count);

        Ok(())
    }

    /// Apply accumulated play skin properties to a PlaySkin.
    /// Call this after load_skin() to transfer CLOSE, PLAYSTART,
    /// LOADSTART, LOADEND, FINISHMARGIN, JUDGETIMER, DST_NOTE_EXPANSION_RATE,
    /// judge region count, lane regions, and lane group regions to the PlaySkin.
    pub fn apply_to_play_skin(&self, play_skin: &mut beatoraja_play::play_skin::PlaySkin) {
        if let Some(close) = self.play_close {
            play_skin.set_close(close);
        }
        if let Some(playstart) = self.play_playstart {
            play_skin.set_playstart(playstart);
        }
        if let Some(loadstart) = self.play_loadstart {
            play_skin.set_loadstart(loadstart);
        }
        if let Some(loadend) = self.play_loadend {
            play_skin.set_loadend(loadend);
        }
        if let Some(finish_margin) = self.play_finish_margin {
            play_skin.set_finish_margin(finish_margin);
        }
        if let Some(judgetimer) = self.play_judgetimer {
            play_skin.set_judgetimer(judgetimer);
        }
        if let Some(rate) = self.play_note_expansion_rate {
            play_skin.set_note_expansion_rate(rate);
        }

        // Apply computed judge region count
        if let Some(judge_reg) = self.computed_judge_reg {
            play_skin.set_judgeregion(judge_reg);
        }

        // Apply lane regions (convert Vec<Option<Rectangle>> -> Vec<Rectangle>)
        let lane_rects: Vec<Rectangle> = self
            .laner
            .iter()
            .map(|opt| opt.clone().unwrap_or_default())
            .collect();
        if !lane_rects.is_empty() {
            play_skin.set_lane_region(Some(lane_rects));
        }

        // Apply lane group regions (player regions)
        let group_rects: Vec<Rectangle> = self
            .playerr
            .iter()
            .map(|opt| opt.clone().unwrap_or_default())
            .collect();
        if !group_rects.is_empty() {
            play_skin.set_lane_group_region(Some(group_rects));
        }

        // Apply line/time/BPM/stop line counts as placeholder Vecs
        let line_count = self.computed_line_count.unwrap_or(0);
        play_skin.set_line(vec![(); line_count]);
        play_skin.set_time_line(vec![(); line_count]);
        play_skin.set_bpm_line(vec![(); line_count]);
        play_skin.set_stop_line(vec![(); line_count]);
    }

    /// Get lane cover position (y coordinate when white number is 0)
    pub fn get_lane_cover_position(&self) -> f32 {
        // if skin.laneCover != null, return last destination's y
        -1.0
    }
}

impl LR2SkinLoaderAccess for LR2PlaySkinLoaderState {
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState {
        &mut self.csv
    }

    fn assemble_objects(&mut self, skin: &mut crate::skin::Skin) {
        use crate::skin::SkinObject;

        // 1. Add SkinJudgeObject instances (already created during SRC_NOWJUDGE parsing)
        for judge_opt in &mut self.judge_objects {
            if let Some(judge) = judge_opt.take() {
                skin.add(SkinObject::Judge(judge));
            }
        }

        // 2. Add line SkinImage instances (already created during SRC_LINE/DST_LINE parsing)
        for line_opt in &mut self.line_images {
            if let Some(line_img) = line_opt.take() {
                skin.add(SkinObject::Image(line_img));
            }
        }

        // 3. Add judgeline SkinImage (already created during SRC_JUDGELINE parsing)
        if let Some(judgeline) = self.judgeline.take() {
            skin.add(SkinObject::Image(judgeline));
        }

        // 4. Create SkinNoteObject from accumulated note source data
        if self.lanerender {
            let lane_count = self.note.len();
            let mut note_obj = crate::skin_note_object::SkinNoteObject::new(lane_count);
            for (i, lane_rect) in self.laner.iter().enumerate() {
                if let Some(rect) = lane_rect {
                    note_obj.inner.set_lane_region(
                        i,
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        *self.scale.get(i).unwrap_or(&1.0),
                        *self.dstnote2.get(i).unwrap_or(&0),
                    );
                }
            }
            skin.add(SkinObject::Note(note_obj));
        }

        // 5. Create SkinBgaObject if BGA was requested
        if self.bga {
            let bga_obj = crate::skin_bga_object::SkinBgaObject::new(0);
            skin.add(SkinObject::Bga(bga_obj));
        }

        // 6. Apply play-specific timing to skin
        if let Some(close) = self.play_close {
            skin.set_scene(close);
        }
        if let Some(margin) = self.play_finish_margin {
            skin.set_fadeout(margin);
        }

        log::debug!("LR2PlaySkinLoader: assembled objects into skin");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::Texture;

    fn make_state() -> LR2PlaySkinLoaderState {
        LR2PlaySkinLoaderState::new(
            crate::skin_type::SkinType::Play7Keys,
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

    // Helper: build a str_parts array with enough elements for parse_int (needs 22+ entries)
    fn make_parts(cmd: &str, vals: &[i32]) -> Vec<String> {
        let mut parts = vec![cmd.to_string()];
        for v in vals {
            parts.push(v.to_string());
        }
        // Pad to at least 22 elements
        while parts.len() < 22 {
            parts.push("0".to_string());
        }
        parts
    }

    // ===== Scalar commands =====

    #[test]
    fn test_close_command() {
        let mut state = make_state();
        state.process_play_command("CLOSE", &str_vec(&["CLOSE", "500"]));
        assert_eq!(state.play_close, Some(500));
    }

    #[test]
    fn test_playstart_command() {
        let mut state = make_state();
        state.process_play_command("PLAYSTART", &str_vec(&["PLAYSTART", "1000"]));
        assert_eq!(state.play_playstart, Some(1000));
    }

    #[test]
    fn test_loadstart_command() {
        let mut state = make_state();
        state.process_play_command("LOADSTART", &str_vec(&["LOADSTART", "200"]));
        assert_eq!(state.play_loadstart, Some(200));
    }

    #[test]
    fn test_loadend_command() {
        let mut state = make_state();
        state.process_play_command("LOADEND", &str_vec(&["LOADEND", "3000"]));
        assert_eq!(state.play_loadend, Some(3000));
    }

    #[test]
    fn test_finishmargin_command() {
        let mut state = make_state();
        state.process_play_command("FINISHMARGIN", &str_vec(&["FINISHMARGIN", "2000"]));
        assert_eq!(state.play_finish_margin, Some(2000));
    }

    #[test]
    fn test_judgetimer_command() {
        let mut state = make_state();
        state.process_play_command("JUDGETIMER", &str_vec(&["JUDGETIMER", "1"]));
        assert_eq!(state.play_judgetimer, Some(1));
    }

    #[test]
    fn test_scalar_invalid_parse_returns_none() {
        let mut state = make_state();
        state.process_play_command("CLOSE", &str_vec(&["CLOSE", "xyz"]));
        assert_eq!(state.play_close, None);
    }

    #[test]
    fn test_scalar_empty_parts_no_panic() {
        let mut state = make_state();
        state.process_play_command("CLOSE", &str_vec(&["CLOSE"]));
        assert_eq!(state.play_close, None);
    }

    // ===== DST_NOTE_EXPANSION_RATE =====

    #[test]
    fn test_dst_note_expansion_rate() {
        let mut state = make_state();
        state.process_play_command(
            "DST_NOTE_EXPANSION_RATE",
            &str_vec(&["DST_NOTE_EXPANSION_RATE", "150", "200"]),
        );
        assert_eq!(state.play_note_expansion_rate, Some([150, 200]));
    }

    #[test]
    fn test_dst_note_expansion_rate_too_few_parts() {
        let mut state = make_state();
        // Only 2 parts, needs 3 — should not set
        state.process_play_command(
            "DST_NOTE_EXPANSION_RATE",
            &str_vec(&["DST_NOTE_EXPANSION_RATE", "150"]),
        );
        assert_eq!(state.play_note_expansion_rate, None);
    }

    // ===== SRC_BGA / DST_BGA =====

    #[test]
    fn test_src_bga_sets_flag() {
        let mut state = make_state();
        assert!(!state.bga);
        state.process_play_command("SRC_BGA", &str_vec(&["SRC_BGA"]));
        assert!(state.bga);
    }

    #[test]
    fn test_dst_bga_without_src_no_panic() {
        let mut state = make_state();
        state.process_play_command("DST_BGA", &make_parts("DST_BGA", &[0; 21]));
        // No panic, bga still false
        assert!(!state.bga);
    }

    // ===== SRC_NOWJUDGE / DST_NOWJUDGE =====

    #[test]
    fn test_src_nowjudge_creates_judge_object() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // SRC_NOWJUDGE_1P: values[1]=0 (judge type PG), values[2]=0 (image index)
        let parts = make_parts("SRC_NOWJUDGE_1P", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWJUDGE_1P", &parts);
        assert!(state.judge_objects[0].is_some());
    }

    #[test]
    fn test_src_nowjudge_2p_creates_at_index_1() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        let parts = make_parts("SRC_NOWJUDGE_2P", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWJUDGE_2P", &parts);
        assert!(state.judge_objects[0].is_none());
        assert!(state.judge_objects[1].is_some());
    }

    #[test]
    fn test_src_nowjudge_sets_judge_image() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // values[1]=5 -> judge_idx = 5-5 = 0 (PG)
        let parts = make_parts("SRC_NOWJUDGE_1P", &[5, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWJUDGE_1P", &parts);
        let judge = state.judge_objects[0].as_ref().unwrap();
        assert!(judge.inner.get_judge(0));
    }

    #[test]
    fn test_src_nowjudge_index_mapping() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // values[1]=3 -> judge_idx = 5-3 = 2
        let parts = make_parts("SRC_NOWJUDGE_1P", &[3, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWJUDGE_1P", &parts);
        let judge = state.judge_objects[0].as_ref().unwrap();
        assert!(judge.inner.get_judge(2));
        // Other indices should be unset
        assert!(!judge.inner.get_judge(0));
        assert!(!judge.inner.get_judge(1));
    }

    // ===== SRC_NOWCOMBO =====

    #[test]
    fn test_src_nowcombo_sets_judge_count() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // First create judge object
        let judge_parts = make_parts("SRC_NOWJUDGE_1P", &[5, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWJUDGE_1P", &judge_parts);

        // Now set combo count for PG (values[1]=5 -> idx=0)
        let combo_parts = make_parts("SRC_NOWCOMBO_1P", &[5, 0, 0, 0, 10, 10, 10, 1, 0, 0, 0]);
        state.process_play_command("SRC_NOWCOMBO_1P", &combo_parts);
        let judge = state.judge_objects[0].as_ref().unwrap();
        assert!(judge.inner.get_judge_count(0));
    }

    // ===== SRC_JUDGELINE / DST_JUDGELINE =====

    #[test]
    fn test_src_judgeline_creates_skin_image() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        let parts = make_parts("SRC_JUDGELINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_JUDGELINE", &parts);
        assert!(state.judgeline.is_some());
    }

    #[test]
    fn test_dst_judgeline_without_src_no_effect() {
        let mut state = make_state();
        assert!(state.judgeline.is_none());
        let parts = make_parts("DST_JUDGELINE", &[0, 0, 100, 200, 50, 30, 0, 0, 0, 0, 0]);
        state.process_play_command("DST_JUDGELINE", &parts);
        assert!(state.judgeline.is_none());
    }

    #[test]
    fn test_dst_judgeline_sets_destination() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        let src_parts = make_parts("SRC_JUDGELINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_JUDGELINE", &src_parts);
        assert!(state.judgeline.is_some());

        let dst_parts = make_parts(
            "DST_JUDGELINE",
            &[
                0, 0, 100, 200, 50, 30, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        state.process_play_command("DST_JUDGELINE", &dst_parts);
        let jl = state.judgeline.as_ref().unwrap();
        assert!(!jl.data.dst.is_empty());
    }

    // ===== SRC_LINE / DST_LINE =====

    #[test]
    fn test_src_line_creates_skin_image_at_index() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // values[1] = 0 (line index)
        let parts = make_parts("SRC_LINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &parts);
        assert!(state.line_images[0].is_some());
        assert!(state.line_images[1].is_none());
    }

    #[test]
    fn test_src_line_second_index() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // values[1] = 3 (line index)
        let parts = make_parts("SRC_LINE", &[3, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &parts);
        assert!(state.line_images[0].is_none());
        assert!(state.line_images[3].is_some());
    }

    #[test]
    fn test_dst_line_sets_destination() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        let src_parts = make_parts("SRC_LINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &src_parts);

        let dst_parts = make_parts(
            "DST_LINE",
            &[
                0, 0, 100, 200, 50, 30, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        state.process_play_command("DST_LINE", &dst_parts);
        let li = state.line_images[0].as_ref().unwrap();
        assert!(!li.data.dst.is_empty());
    }

    #[test]
    fn test_dst_line_negative_width_correction() {
        let mut state = make_state();
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        let src_parts = make_parts("SRC_LINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &src_parts);

        // Negative width/height: values[5]=-50, values[6]=-30
        let dst_parts = make_parts(
            "DST_LINE",
            &[
                0, 0, 100, 200, -50, -30, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        state.process_play_command("DST_LINE", &dst_parts);
        let li = state.line_images[0].as_ref().unwrap();
        assert!(!li.data.dst.is_empty());
    }

    // ===== PM_CHARA commands =====

    #[test]
    fn test_dst_pm_chara_1p_no_panic() {
        let mut state = make_state();
        let mut parts = make_parts("DST_PM_CHARA_1P", &[100, 200, 50, 30, 1, 0, 0, 0, 0, 0, 0]);
        if parts.len() > 7 {
            parts[7] = "chara_folder".to_string();
        }
        state.process_play_command("DST_PM_CHARA_1P", &parts);
    }

    #[test]
    fn test_dst_pm_chara_animation_no_panic() {
        let mut state = make_state();
        let mut parts = make_parts(
            "DST_PM_CHARA_ANIMATION",
            &[100, 200, 50, 30, 1, 3, 0, 0, 0, 0, 0, 0],
        );
        while parts.len() <= 12 {
            parts.push("0".to_string());
        }
        parts[12] = "chara_anim_folder".to_string();
        state.process_play_command("DST_PM_CHARA_ANIMATION", &parts);
    }

    #[test]
    fn test_dst_pm_chara_animation_invalid_type_skipped() {
        let mut state = make_state();
        // values[6] = 10, which is outside 0-9 range
        let parts = make_parts(
            "DST_PM_CHARA_ANIMATION",
            &[100, 200, 50, 30, 1, 10, 0, 0, 0, 0, 0, 0],
        );
        state.process_play_command("DST_PM_CHARA_ANIMATION", &parts);
    }

    #[test]
    fn test_src_pm_chara_image_no_panic() {
        let mut state = make_state();
        let mut parts = make_parts("SRC_PM_CHARA_IMAGE", &[1, 2, 0]);
        if parts.len() > 3 {
            parts[3] = "chara_image_folder".to_string();
        }
        state.process_play_command("SRC_PM_CHARA_IMAGE", &parts);
    }

    #[test]
    fn test_src_pm_chara_image_invalid_type_skipped() {
        let mut state = make_state();
        // values[2] = 5, which is outside 0-4 range
        let parts = make_parts("SRC_PM_CHARA_IMAGE", &[1, 5, 0]);
        state.process_play_command("SRC_PM_CHARA_IMAGE", &parts);
    }

    #[test]
    fn test_dst_pm_chara_image_no_panic() {
        let mut state = make_state();
        let parts = make_parts(
            "DST_PM_CHARA_IMAGE",
            &[
                0, 0, 100, 200, 50, 30, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        state.process_play_command("DST_PM_CHARA_IMAGE", &parts);
    }

    // ===== apply_to_play_skin =====

    #[test]
    fn test_apply_to_play_skin_all_values() {
        let mut state = make_state();
        state.play_close = Some(500);
        state.play_playstart = Some(1000);
        state.play_loadstart = Some(200);
        state.play_loadend = Some(3000);
        state.play_finish_margin = Some(2000);
        state.play_judgetimer = Some(1);
        state.play_note_expansion_rate = Some([150, 200]);

        let mut play_skin = beatoraja_play::play_skin::PlaySkin::new();
        state.apply_to_play_skin(&mut play_skin);
        assert_eq!(play_skin.get_close(), 500);
        assert_eq!(play_skin.get_playstart(), 1000);
        assert_eq!(play_skin.get_loadstart(), 200);
        assert_eq!(play_skin.get_loadend(), 3000);
        assert_eq!(play_skin.get_finish_margin(), 2000);
        assert_eq!(play_skin.get_judgetimer(), 1);
        assert_eq!(play_skin.get_note_expansion_rate(), &[150, 200]);
    }

    #[test]
    fn test_apply_to_play_skin_none_values_preserved() {
        let state = make_state();
        let mut play_skin = beatoraja_play::play_skin::PlaySkin::new();
        let orig_close = play_skin.get_close();
        state.apply_to_play_skin(&mut play_skin);
        assert_eq!(play_skin.get_close(), orig_close);
    }

    // ===== Unknown command delegation =====

    #[test]
    fn test_unknown_command_delegates_to_csv_loader() {
        let mut state = make_state();
        // STARTINPUT should be handled by the CSV base loader via delegation
        state.process_play_command("STARTINPUT", &str_vec(&["STARTINPUT", "750"]));
        assert_eq!(state.csv.skin_input, Some(750));
    }

    // ===== load_skin CSV pipeline integration =====

    /// Helper: write content to a temp file and return the path.
    fn write_temp_csv(name: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("lr2_play_skin_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_load_skin_parses_csv_commands() {
        let csv_content = "#CLOSE,500\n#PLAYSTART,1000\n#STARTINPUT,750\n";
        let path = write_temp_csv("test_load_skin_cmds.lr2skin", csv_content);

        let mut state = make_state();
        state.load_skin(&path, None).unwrap();

        assert_eq!(state.play_close, Some(500));
        assert_eq!(state.play_playstart, Some(1000));
        // STARTINPUT is delegated to the CSV loader
        assert_eq!(state.csv.skin_input, Some(750));
    }

    #[test]
    fn test_load_skin_initializes_arrays_from_mode() {
        let csv_content = "";
        let path = write_temp_csv("test_load_skin_init.lr2skin", csv_content);

        let mut state = make_state();
        state.load_skin(&path, None).unwrap();

        // Play7Keys mode has 8 keys
        assert_eq!(state.note.len(), 8);
        assert_eq!(state.laner.len(), 8);
        assert_eq!(state.scale.len(), 8);
        assert_eq!(state.dstnote2.len(), 8);
        // All dstnote2 should be i32::MIN
        assert!(state.dstnote2.iter().all(|&v| v == i32::MIN));
    }

    #[test]
    fn test_load_skin_computes_judge_reg_default() {
        let csv_content = "";
        let path = write_temp_csv("test_load_skin_judge.lr2skin", csv_content);

        let mut state = make_state();
        state.load_skin(&path, None).unwrap();

        // No judge objects created, default judge_reg = 1
        assert_eq!(state.computed_judge_reg, Some(1));
    }

    #[test]
    fn test_load_skin_computes_line_count_zero() {
        let csv_content = "";
        let path = write_temp_csv("test_load_skin_lines.lr2skin", csv_content);

        let mut state = make_state();
        state.load_skin(&path, None).unwrap();

        // No line images created
        assert_eq!(state.computed_line_count, Some(0));
    }

    #[test]
    fn test_load_skin_file_not_found_returns_error() {
        let mut state = make_state();
        let result = state.load_skin(std::path::Path::new("/nonexistent/skin.lr2skin"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_skin_applies_to_play_skin_with_computed_values() {
        let csv_content = "#CLOSE,500\n#JUDGETIMER,2\n";
        let path = write_temp_csv("test_load_skin_apply.lr2skin", csv_content);

        let mut state = make_state();
        state.load_skin(&path, None).unwrap();

        let mut play_skin = beatoraja_play::play_skin::PlaySkin::new();
        state.apply_to_play_skin(&mut play_skin);

        assert_eq!(play_skin.get_close(), 500);
        assert_eq!(play_skin.get_judgetimer(), 2);
        assert_eq!(play_skin.get_judgeregion(), 1); // default
        // Lane region should be set (8 default rectangles)
        assert!(play_skin.get_lane_region().is_some());
    }

    // ===== make_default_line / default line images =====

    /// Helper: set up a judge line at index 0 with SRC_LINE + DST_LINE and stored linevalues.
    fn setup_judge_line(state: &mut LR2PlaySkinLoaderState) {
        state.csv.imagelist.push(
            crate::lr2::lr2_skin_csv_loader::ImageListEntry::TextureEntry(Texture::new("test")),
        );
        // SRC_LINE: index=0, imageID=0, x=0, y=0, w=10, h=10, divx=1, divy=1, cycle=0, timer=0
        let src_parts = make_parts("SRC_LINE", &[0, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &src_parts);
        // DST_LINE: index=0, time=0, x=100, y=200, w=500, h=2, acc=0, a=255, r=255, g=255, b=255, ...
        let dst_parts = make_parts(
            "DST_LINE",
            &[
                0, 0, 100, 200, 500, 2, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );
        state.process_play_command("DST_LINE", &dst_parts);
    }

    #[test]
    fn test_make_default_line_creates_time_line_when_judge_exists() {
        let mut state = make_state();
        setup_judge_line(&mut state);
        assert!(state.line_images[0].is_some()); // judge line exists
        assert!(state.line_images[6].is_none()); // time line missing

        // Simulate what load_skin does: create default time line
        state.make_default_line(6, 1, 64, 192, 192);

        assert!(
            state.line_images[6].is_some(),
            "time line should be created"
        );
        let li = state.line_images[6].as_ref().unwrap();
        assert!(!li.data.dst.is_empty(), "destination should be set");
    }

    #[test]
    fn test_make_default_line_creates_bpm_line() {
        let mut state = make_state();
        setup_judge_line(&mut state);
        assert!(state.line_images[2].is_none()); // BPM line missing

        state.make_default_line(2, 2, 0, 192, 0);

        assert!(state.line_images[2].is_some(), "BPM line should be created");
        let li = state.line_images[2].as_ref().unwrap();
        assert!(!li.data.dst.is_empty(), "destination should be set");
    }

    #[test]
    fn test_make_default_line_creates_stop_line() {
        let mut state = make_state();
        setup_judge_line(&mut state);
        assert!(state.line_images[4].is_none()); // stop line missing

        state.make_default_line(4, 2, 192, 192, 0);

        assert!(
            state.line_images[4].is_some(),
            "stop line should be created"
        );
        let li = state.line_images[4].as_ref().unwrap();
        assert!(!li.data.dst.is_empty(), "destination should be set");
    }

    #[test]
    fn test_make_default_line_skips_when_no_linevalues() {
        let mut state = make_state();
        // No DST_LINE processed, so linevalues is [None, None]
        assert!(state.linevalues[0].is_none());

        state.make_default_line(6, 1, 64, 192, 192);

        // Should not create anything since linevalues is unavailable
        assert!(state.line_images[6].is_none());
    }

    #[test]
    fn test_load_skin_creates_default_lines_when_judge_exists() {
        let mut state = make_state();
        setup_judge_line(&mut state);

        // Simulate load_skin post-processing: count lines then create defaults
        let line_count = if state.line_images[0].is_some() {
            if state.line_images[1].is_some() { 2 } else { 1 }
        } else {
            0
        };
        assert_eq!(line_count, 1);

        for i in 0..line_count {
            if state.line_images[i + 6].is_none() && state.line_images[i].is_some() {
                state.make_default_line(i + 6, 1, 64, 192, 192);
            }
            if state.line_images[i + 2].is_none() && state.line_images[i].is_some() {
                state.make_default_line(i + 2, 2, 0, 192, 0);
            }
            if state.line_images[i + 4].is_none() && state.line_images[i].is_some() {
                state.make_default_line(i + 4, 2, 192, 192, 0);
            }
        }

        // Time, BPM, stop lines should be created
        assert!(state.line_images[6].is_some(), "time line at [6]");
        assert!(state.line_images[2].is_some(), "BPM line at [2]");
        assert!(state.line_images[4].is_some(), "stop line at [4]");
    }

    #[test]
    fn test_load_skin_no_defaults_when_line_count_zero() {
        let mut state = make_state();
        // No judge lines at all
        assert!(state.line_images[0].is_none());

        let line_count = if state.line_images[0].is_some() {
            if state.line_images[1].is_some() { 2 } else { 1 }
        } else {
            0
        };
        assert_eq!(line_count, 0);

        // Loop body never executes when line_count == 0
        for i in 0..line_count {
            state.make_default_line(i + 6, 1, 64, 192, 192);
            state.make_default_line(i + 2, 2, 0, 192, 0);
            state.make_default_line(i + 4, 2, 192, 192, 0);
        }

        // Nothing should be created
        for slot in &state.line_images {
            assert!(slot.is_none());
        }
    }

    #[test]
    fn test_existing_line_images_not_overwritten() {
        let mut state = make_state();
        setup_judge_line(&mut state);

        // Manually create a line image at index 6 (time line)
        // values[1]=6 means line index 6 in SRC_LINE
        let src_parts = make_parts("SRC_LINE", &[6, 0, 0, 0, 10, 10, 1, 1, 0, 0, 0]);
        state.process_play_command("SRC_LINE", &src_parts);
        assert!(state.line_images[6].is_some(), "pre-existing time line");

        // The condition `line_images[i + 6].is_none()` should prevent overwriting
        let line_count = 1;
        for i in 0..line_count {
            if state.line_images[i + 6].is_none() && state.line_images[i].is_some() {
                state.make_default_line(i + 6, 1, 64, 192, 192);
            }
        }

        // The existing line image should still be there, not replaced
        let li = state.line_images[6].as_ref().unwrap();
        // The pre-existing image was created via SRC_LINE, so it has no destinations
        // (DST_LINE was never called for index 6). A default line would have a destination.
        assert!(
            li.data.dst.is_empty(),
            "existing image should not be overwritten"
        );
    }

    #[test]
    fn test_make_default_line_height_multiplier() {
        let mut state = make_state();
        setup_judge_line(&mut state);

        // Create two default lines with different height multipliers
        state.make_default_line(6, 1, 64, 192, 192); // h=1 (time)
        state.make_default_line(2, 2, 0, 192, 0); // h=2 (BPM)

        let time_line = state.line_images[6].as_ref().unwrap();
        let bpm_line = state.line_images[2].as_ref().unwrap();

        // Both should have destinations
        assert!(!time_line.data.dst.is_empty());
        assert!(!bpm_line.data.dst.is_empty());

        // The BPM line (h=2) should have double the height of the time line (h=1)
        // DST_LINE had h=2 (values[6]), scaled by dsth/srch = 1080/480 = 2.25
        // Time: 2 * 2.25 * 1 = 4.5; BPM: 2 * 2.25 * 2 = 9.0
        let time_dst = &time_line.data.dst[0];
        let bpm_dst = &bpm_line.data.dst[0];
        let time_h = time_dst.region.height;
        let bpm_h = bpm_dst.region.height;
        assert!(
            (bpm_h - time_h * 2.0).abs() < 0.01,
            "BPM height ({}) should be 2x time height ({})",
            bpm_h,
            time_h
        );
    }
}

/// Note target types for add_note dispatch
enum NoteTarget {
    Note,
    LnStart,
    LnEnd,
    LnBody,
    LnBodyA,
    HcnStart,
    HcnEnd,
    HcnBody,
    HcnBodyA,
    HcnBodyD,
    HcnBodyR,
    Mine,
}
