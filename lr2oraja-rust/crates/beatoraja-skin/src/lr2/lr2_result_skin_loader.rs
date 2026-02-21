use crate::lr2::lr2_skin_csv_loader::LR2SkinCSVLoaderState;
use crate::lr2::lr2_skin_loader::{self, LR2SkinLoaderState};
use crate::stubs::{MainState, Rectangle, Resolution};

/// LR2 result skin loader
///
/// Translated from LR2ResultSkinLoader.java (114 lines)
/// Loads LR2 result skins with gauge chart, note chart, BPM chart,
/// and timing chart elements.
///
/// Result skin loader state
pub struct LR2ResultSkinLoaderState {
    pub csv: LR2SkinCSVLoaderState,
    pub gauge: Rectangle,
    pub gaugeobj: Option<()>,       // SkinGaugeGraphObject placeholder
    pub noteobj: Option<()>,        // SkinNoteDistributionGraph placeholder
    pub bpmgraphobj: Option<()>,    // SkinBPMGraph placeholder
    pub timinggraphobj: Option<()>, // SkinTimingDistributionGraph placeholder
}

impl LR2ResultSkinLoaderState {
    pub fn new(src: Resolution, dst: Resolution, usecim: bool, skinpath: String) -> Self {
        Self {
            csv: LR2SkinCSVLoaderState::new(src, dst, usecim, skinpath),
            gauge: Rectangle::default(),
            gaugeobj: None,
            noteobj: None,
            bpmgraphobj: None,
            timinggraphobj: None,
        }
    }

    /// Process result-specific commands
    pub fn process_result_command(&mut self, cmd: &str, str_parts: &[String]) {
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
                // skin.setDestination(gaugeobj, ...)
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
            "SRC_BPMCHART" => {
                // #SRC_BPMCHART, field_w, field_h, delay, lineWidth, mainBPMColor, minBPMColor, maxBPMColor, otherBPMColor, stopLineColor, transitionLineColor
                let values = lr2_skin_loader::parse_int(str_parts);
                // bpmgraphobj = new SkinBPMGraph(values[3], values[4], str[5], str[6], str[7], str[8], str[9], str[10])
                self.gauge = Rectangle::new(0.0, 0.0, values[1] as f32, values[2] as f32);
                self.bpmgraphobj = Some(());
                // skin.add(bpmgraphobj)
            }
            "DST_BPMCHART" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(bpmgraphobj, ...)
            }
            "SRC_TIMINGCHART_1P" => {
                // #SRC_TIMINGCHART_1P,(index),(gr),(x),width,height,lineWidth,graphColor,averageColor,devColor,PGColor,GRColor,GDColor,BDColor,PRColor,drawAverage,drawDev
                let values = lr2_skin_loader::parse_int(str_parts);
                // timinggraphobj = new SkinTimingDistributionGraph(values[4], values[6], str[7], str[8], str[9], str[10], str[11], str[12], str[13], str[14], values[15], values[16])
                self.gauge = Rectangle::new(0.0, 0.0, values[4] as f32, values[5] as f32);
                self.timinggraphobj = Some(());
                // skin.add(timinggraphobj)
            }
            "DST_TIMINGCHART_1P" => {
                let values = lr2_skin_loader::parse_int(str_parts);
                self.gauge.x = values[3] as f32;
                self.gauge.y = self.csv.src.height - values[4] as f32;
                // skin.setDestination(timinggraphobj, ...)
            }
            _ => {
                // Delegate to CSV loader
                self.csv.process_csv_command(cmd, str_parts);
            }
        }
    }
}
