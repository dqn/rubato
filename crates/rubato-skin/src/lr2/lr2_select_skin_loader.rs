use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::lr2::lr2_skin_loader::{self};
use crate::skin_image::SkinImage;
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

    pub gauge: Rectangle,

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
            gauge: Rectangle::default(),
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
                    let images = self.csv.get_source_image(&values);
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
                // barimageoff[values[1]].setDestination(...)
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
                // barimageon[values[1]].setDestination(...)
            }
            "BAR_CENTER" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                // skin.setCenterBar(values[1])
                let _ = values[1];
            }
            "BAR_AVAILABLE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                let mut clickable = vec![0i32; (values[2] - values[1] + 1) as usize];
                for i in 0..clickable.len() {
                    clickable[i] = values[1] + i as i32;
                }
                // skin.setClickableBar(clickable)
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
                    let images = self.csv.get_source_image(&values);
                    if let Some(images) = images {
                        if images.len() % 24 == 0 {
                            // Split into positive/negative number images
                            let _pn_count = images.len() / 24;
                            // skinbar.setBarlevel(values[1], new SkinNumber(pn, mn, ...))
                        } else {
                            let d = if images.len() % 10 == 0 { 10 } else { 11 };
                            // skinbar.setBarlevel(values[1], new SkinNumber(nimages, ...))
                            let _ = d;
                        }
                    }
                }
            }
            "DST_BAR_LEVEL" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                // skinbar.getBarlevel(values[1])?.setDestination(...)
            }
            "SRC_BAR_LAMP" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARLAMP_COUNT as i32 {
                    return;
                }
                let _images = self.csv.get_source_image(&values);
                // skinbar.setLamp(lampg[values[1]][0], new SkinImage(images, values[10], values[9]))
            }
            "DST_BAR_LAMP" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                let lamp_idx = values[1] as usize;
                if lamp_idx < LAMPG.len() {
                    let lamps = LAMPG[lamp_idx];
                    for _i in 0..lamps.len() {
                        if values[5] < 0 {
                            values[3] += values[5];
                            values[5] = -values[5];
                        }
                        if values[6] < 0 {
                            values[4] += values[6];
                            values[6] = -values[6];
                        }
                        // skinbar.getLamp(lamps[i])?.setDestination(...)
                    }
                }
            }
            "SRC_BAR_MY_LAMP" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARLAMP_COUNT as i32 {
                    return;
                }
                let _images = self.csv.get_source_image(&values);
                // skinbar.setPlayerLamp(...)
            }
            "DST_BAR_MY_LAMP" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                let lamp_idx = values[1] as usize;
                if lamp_idx < LAMPG.len() {
                    let lamps = LAMPG[lamp_idx];
                    for _i in 0..lamps.len() {
                        if values[5] < 0 {
                            values[3] += values[5];
                            values[5] = -values[5];
                        }
                        if values[6] < 0 {
                            values[4] += values[6];
                            values[6] = -values[6];
                        }
                        // skinbar.getPlayerLamp(lamps[i])?.setDestination(...)
                    }
                }
            }
            "SRC_BAR_RIVAL_LAMP" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARLAMP_COUNT as i32 {
                    return;
                }
                let _images = self.csv.get_source_image(&values);
                // skinbar.setRivalLamp(...)
            }
            "DST_BAR_RIVAL_LAMP" => {
                let mut values = lr2_skin_loader::parse_int(str_parts);
                let lamp_idx = values[1] as usize;
                if lamp_idx < LAMPG.len() {
                    let lamps = LAMPG[lamp_idx];
                    for _i in 0..lamps.len() {
                        if values[5] < 0 {
                            values[3] += values[5];
                            values[5] = -values[5];
                        }
                        if values[6] < 0 {
                            values[4] += values[6];
                            values[6] = -values[6];
                        }
                        // skinbar.getRivalLamp(lamps[i])?.setDestination(...)
                    }
                }
            }
            "SRC_BAR_TROPHY" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARTROPHY_COUNT as i32 {
                    return;
                }
                let _images = self.csv.get_source_image(&values);
                // skinbar.setTrophy(values[1], new SkinImage(images, values[10], values[9]))
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
                // skinbar.getTrophy(values[1])?.setDestination(...)
            }
            "SRC_BAR_LABEL" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARLABEL_COUNT as i32 {
                    return;
                }
                let _images = self.csv.get_source_image(&values);
                // skinbar.setLabel(values[1], new SkinImage(images, values[10], values[9]))
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
                // skinbar.getLabel(values[1])?.setDestination(...)
            }
            "SRC_BAR_GRAPH" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                let _images = self.csv.get_source_image(&values);
                // Build distribution graph images and set on skinbar
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
                // skinbar.getGraph()?.setDestination(...)
            }
            "SRC_NOTECHART" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                // noteobj = new SkinNoteDistributionGraph(...)
                self.gauge = Rectangle::new(0.0, 0.0, values[11] as f32, values[12] as f32);
                // skin.add(noteobj)
            }
            "DST_NOTECHART" => {
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
            "SRC_BAR_TITLE" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                if values[1] < 0 || values[1] >= BARTEXT_COUNT as i32 {
                    return;
                }
                // Create SkinText based on fontlist availability
                // skinbar.setText(values[1], bartext)
                let _ = values;
            }
            "DST_BAR_TITLE" => {
                let _values = lr2_skin_loader::parse_int(str_parts);
                // skinbar.getText(values[1])?.setDestination(...)
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
        // The bar body on/off images (barimageon/barimageoff) have been parsed
        // as placeholders; full SkinImage construction requires texture sources
        // from get_source_image() which are consumed during CSV parsing.
        // The SkinBarObject itself is a minimal wrapper — actual bar rendering
        // is handled by BarRenderer in beatoraja-select.
        let has_bars = self
            .barimageon
            .iter()
            .chain(self.barimageoff.iter())
            .any(|b| b.is_some());
        if has_bars {
            let bar_obj = crate::skin_bar_object::SkinBarObject::new(0);
            skin.add(SkinObject::Bar(bar_obj));
        }

        log::debug!("LR2SelectSkinLoader: assembled objects into skin");
    }
}
