use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::lr2::lr2_skin_loader::{self, LR2SkinLoaderState};
use crate::stubs::{MainState, Rectangle, Resolution};

/// LR2 course result skin loader
///
/// Translated from LR2CourseResultSkinLoader.java (87 lines)
/// Loads LR2 course result skins with gauge chart and note chart elements.
///
/// Course result skin loader state
pub struct LR2CourseResultSkinLoaderState {
    pub csv: LR2SkinCSVLoaderState,
    pub gauge: Rectangle,
    pub gaugeobj: Option<()>, // SkinGaugeGraphObject placeholder
    pub noteobj: Option<()>,  // SkinNoteDistributionGraph placeholder
}

impl LR2CourseResultSkinLoaderState {
    pub fn new(src: Resolution, dst: Resolution, usecim: bool, skinpath: String) -> Self {
        Self {
            csv: LR2SkinCSVLoaderState::new(src, dst, usecim, skinpath),
            gauge: Rectangle::default(),
            gaugeobj: None,
            noteobj: None,
        }
    }

    /// Process course result-specific commands
    pub fn process_course_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            "STARTINPUT" => {
                // skin.setInput(parseInt(str[1]))
                // skin.setRankTime(parseInt(str[2]))
            }
            "SRC_GAUGECHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                // gaugeobj = new SkinGaugeGraphObject()
                // gaugeobj.setLineWidth(values[6])
                // gaugeobj.setDelay(values[14] - values[13])
                self.gauge = Rectangle::new(0.0, 0.0, values[11] as f32, values[12] as f32);
                self.gaugeobj = Some(());
                // skin.add(gaugeobj)
            }
            "DST_GAUGECHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(gaugeobj, values[2], gauge.x, gauge.y, gauge.width, gauge.height, ...)
            }
            "SRC_NOTECHART_1P" => {
                // #SRC_NOTECHART_1P,(index),(gr),(x),(y),(w),(h),(div_x),(div_y),(cycle),(timer),field_w,field_h,(start),(end),delay,backTexOff,orderReverse,noGap
                let values = lr2_skin_loader::parse_int(str_parts);
                // noteobj = new SkinNoteDistributionGraph(values[1], values[15], values[16], values[17], values[18], values[19])
                self.gauge = Rectangle::new(0.0, 0.0, values[11] as f32, values[12] as f32);
                self.noteobj = Some(());
                // skin.add(noteobj)
            }
            "DST_NOTECHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(noteobj, ...)
            }
            _ => {
                // Delegate to CSV loader
                self.csv.process_csv_command(cmd, str_parts);
            }
        }
    }
}

impl LR2SkinLoaderAccess for LR2CourseResultSkinLoaderState {
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState {
        &mut self.csv
    }
}
