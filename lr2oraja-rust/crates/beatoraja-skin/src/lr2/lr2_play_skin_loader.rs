use std::collections::HashMap;

use log::warn;

use crate::lr2::lr2_skin_csv_loader::LR2SkinCSVLoaderState;
use crate::lr2::lr2_skin_loader::{self, LR2SkinLoaderState};
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
    pub lines: Vec<Option<()>>, // SkinImage placeholder
    pub linevalues: [Option<Vec<String>>; 2],

    pub judge: [Option<()>; 3], // SkinJudge placeholder

    pub srcw: f32,
    pub srch: f32,
    pub dstw: f32,
    pub dsth: f32,

    pub gauge: Rectangle,
    pub playerr: Vec<Option<Rectangle>>,
    pub hidden: Option<()>,
    pub lane_cover: Option<()>,
    pub lanerender: Option<()>,
    pub line: Option<()>,
    pub bga: Option<()>,
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
            lines: vec![None; 8],
            linevalues: [None, None],
            judge: [None, None, None],
            srcw,
            srch,
            dstw,
            dsth,
            gauge: Rectangle::default(),
            playerr: Vec::new(),
            hidden: None,
            lane_cover: None,
            lanerender: None,
            line: None,
            bga: None,
        }
    }

    /// Process play-specific commands
    pub fn process_play_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            // PlayCommand enum entries
            "CLOSE" => {
                // skin.setClose(parseInt(str[1]))
                warn!("not yet implemented: CLOSE requires skin reference");
            }
            "PLAYSTART" => {
                // skin.setPlaystart(parseInt(str[1]))
                warn!("not yet implemented: PLAYSTART requires skin reference");
            }
            "LOADSTART" => {
                // skin.setLoadstart(parseInt(str[1]))
                warn!("not yet implemented: LOADSTART requires skin reference");
            }
            "LOADEND" => {
                // skin.setLoadend(parseInt(str[1]))
                warn!("not yet implemented: LOADEND requires skin reference");
            }
            "FINISHMARGIN" => {
                // skin.setFinishMargin(parseInt(str[1]))
                warn!("not yet implemented: FINISHMARGIN requires skin reference");
            }
            "JUDGETIMER" => {
                // skin.setJudgetimer(parseInt(str[1]))
                warn!("not yet implemented: JUDGETIMER requires skin reference");
            }
            "SRC_BGA" => {
                // bga = new SkinBGA(c.getBgaExpand())
                self.bga = Some(());
                warn!("not yet implemented: SRC_BGA requires SkinBGA");
            }
            "DST_BGA" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                if self.bga.is_some() {
                    warn!("not yet implemented: DST_BGA requires skin.setDestination");
                }
            }
            "SRC_LINE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                let _images = self.csv.get_source_image(&values);
                // lines[values[1]] = new SkinImage(images, values[10], values[9])
                warn!("not yet implemented: SRC_LINE requires SkinImage creation");
            }
            "DST_LINE" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                warn!("not yet implemented: DST_LINE requires SkinImage.setDestination");
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
                    if self.lanerender.is_none() {
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
                        self.lanerender = Some(());
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
                // skin.setNoteExpansionRate(new int[]{parseInt(str[1]), parseInt(str[2])})
                warn!("not yet implemented: DST_NOTE_EXPANSION_RATE requires skin reference");
            }
            "SRC_NOWJUDGE_1P" | "SRC_NOWJUDGE_2P" | "SRC_NOWJUDGE_3P" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                let _images = self.csv.get_source_image(&_values);
                // Create SkinJudge and add judge images
                warn!("not yet implemented: SRC_NOWJUDGE requires SkinJudge");
            }
            "DST_NOWJUDGE_1P" | "DST_NOWJUDGE_2P" | "DST_NOWJUDGE_3P" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                warn!("not yet implemented: DST_NOWJUDGE requires SkinJudge.setDestination");
            }
            "SRC_NOWCOMBO_1P" | "SRC_NOWCOMBO_2P" | "SRC_NOWCOMBO_3P" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                warn!("not yet implemented: SRC_NOWCOMBO requires SkinNumber creation");
            }
            "DST_NOWCOMBO_1P" | "DST_NOWCOMBO_2P" | "DST_NOWCOMBO_3P" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                warn!("not yet implemented: DST_NOWCOMBO requires SkinNumber.setDestination");
            }
            "SRC_JUDGELINE" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                // line = new SkinImage(images, values[10], values[9])
                warn!("not yet implemented: SRC_JUDGELINE requires SkinImage");
            }
            "DST_JUDGELINE" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                warn!("not yet implemented: DST_JUDGELINE requires SkinImage.setDestination");
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
                self.hidden = Some(());
            }
            "DST_HIDDEN" => {
                if self.hidden.is_some() {
                    let _values = lr2_skin_loader::parse_int(str_parts);
                    // hidden.setDestination(...)
                }
            }
            "DST_LIFT" => {
                if self.hidden.is_some() {
                    let _values = lr2_skin_loader::parse_int(str_parts);
                    // hidden.setDestination(...)
                }
            }
            "DST_PM_CHARA_1P" | "DST_PM_CHARA_2P" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                // PomyuCharaLoader load
                warn!("not yet implemented: DST_PM_CHARA requires PomyuCharaLoader");
            }
            "DST_PM_CHARA_ANIMATION" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                warn!("not yet implemented: DST_PM_CHARA_ANIMATION requires PomyuCharaLoader");
            }
            "SRC_PM_CHARA_IMAGE" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                warn!("not yet implemented: SRC_PM_CHARA_IMAGE requires PomyuCharaLoader");
            }
            "DST_PM_CHARA_IMAGE" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                warn!("not yet implemented: DST_PM_CHARA_IMAGE requires PomyuCharaLoader");
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

    /// Load play skin
    pub fn load_skin(&mut self, _state: Option<&dyn MainState>) -> anyhow::Result<()> {
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

        // Would call self.csv.load_skin0(...) here
        // Then set up lane rendering, lines, etc.

        Ok(())
    }

    /// Get lane cover position (y coordinate when white number is 0)
    pub fn get_lane_cover_position(&self) -> f32 {
        // if skin.laneCover != null, return last destination's y
        -1.0
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
